// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Object-input construction and inference from resolved object records.

use super::*;

/// Only `0x2::coin::Coin<T>` (and legacy nested spelling) is a canonical
/// coin. A bare `ends_with("::coin::Coin")` would accept `0xevil::coin::Coin`
/// and let worthless attacker tokens appear as transfers/holders.
fn is_canonical_coin_outer(outer: &str) -> bool {
    let outer = outer.trim();
    if !CoinModule::is_coin_outer(outer) {
        return false;
    }
    // Outer address must be the kanari system address (0x2, any spelling).
    let addr_str = outer.split("::").next().unwrap_or("");
    if let Ok(addr) = AccountAddress::from_hex_literal(addr_str) {
        return addr
            == AccountAddress::from_hex_literal(
                kanari_types::address::Address::KANARI_SYSTEM_ADDRESS,
            )
            .expect("KANARI_SYSTEM_ADDRESS constant is invalid");
    }
    // Fallback for non-hex spellings: accept only literal 0x2 variants.
    matches!(addr_str.trim(), "0x2" | "0x02" | "0X2" | "0X02")
}

pub(crate) fn coin_token_type_from_object_type(object_type: &str) -> Option<String> {
    if let Some(start) = object_type.find('<')
        && let Some(end) = object_type.rfind('>')
        && end > start
    {
        let outer = &object_type[..start];
        if is_canonical_coin_outer(outer) {
            return Some(CoinModule::normalize_token_type(
                &object_type[start + 1..end],
            ));
        }
        return None;
    }

    if let Ok(TypeTag::Struct(st)) = TypeTag::from_str(object_type)
        && st.module.as_str() == CoinModule::COIN_MODULE
        && st.name.as_str() == CoinModule::COIN_STRUCT
        && let Some(TypeTag::Struct(inner)) = st.type_params.first()
    {
        let expected =
            AccountAddress::from_hex_literal(kanari_types::address::Address::KANARI_SYSTEM_ADDRESS)
                .expect("KANARI_SYSTEM_ADDRESS constant is invalid");
        if st.address == expected {
            return Some(format!("{}", inner));
        }
    }

    None
}

pub(crate) fn build_object_input(
    object: &kanari_rpc_api::ObjectInfo,
    sender: &str,
) -> anyhow::Result<ObjectInput> {
    match &object.owner_kind {
        ObjectOwnerKind::AddressOwner(address) => {
            // Object owners are always canonical hex addresses, while wallet
            // senders may be curve-tagged public keys (for example
            // `Ed25519Dilithium3:...`). Compare their derived account
            // addresses instead of comparing the strings verbatim.
            let same_account = match (
                Address::parse_to_account_address(address),
                Address::parse_to_account_address(sender),
            ) {
                (Ok(owner), Ok(sender)) => owner == sender,
                _ => normalize_addr(address) == normalize_addr(sender),
            };
            if !same_account {
                anyhow::bail!(
                    "Object input {} is owned by {}, not sender {}",
                    object.id,
                    address,
                    sender
                );
            }
        }
        ObjectOwnerKind::Shared | ObjectOwnerKind::Immutable => (),
    }

    Ok(object_input_from_info(object))
}

/// Convert a resolved object record into the canonical transaction input.
///
/// View calls do not have a transaction sender, so they preserve the
/// authoritative owner-kind from storage without the submitted-transaction
/// ownership check performed by [`build_object_input`].
fn object_input_from_info(object: &kanari_rpc_api::ObjectInfo) -> ObjectInput {
    ObjectInput {
        object_ref: ObjectRef::new(
            object.id.clone(),
            Some(object.version),
            object.digest.clone(),
        ),
        owner: Some(object.owner_kind.clone()),
        mutable: !matches!(object.owner_kind, ObjectOwnerKind::Immutable),
    }
}

pub(crate) fn infer_object_inputs(
    state: &RpcServerState,
    sender: &str,
    args: &[Vec<u8>],
    candidate_arg_indices: Option<&HashSet<usize>>,
) -> anyhow::Result<Vec<ObjectInput>> {
    let state_guard = state.engine.state_read();
    let mut seen = HashSet::new();
    let mut inputs = Vec::new();

    for (index, arg) in args.iter().enumerate() {
        if let Some(indices) = candidate_arg_indices
            && !indices.contains(&index)
        {
            continue;
        }
        if arg.len() != 32 {
            continue;
        }
        let Some(object_id) = object_id_from_address_bytes(arg) else {
            continue;
        };
        if !seen.insert(object_id.clone()) {
            continue;
        }
        let Some(stored) = state_guard.get_object(&object_id)? else {
            continue;
        };
        let digest = stored.digest();
        let object = kanari_rpc_api::ObjectInfo {
            id: object_id,
            owner: format!("{:#x}", stored.owner),
            owner_kind: stored.owner_kind,
            type_: stored.type_,
            data: stored.data,
            version: stored.version,
            digest: Some(digest),
        };
        inputs.push(build_object_input(&object, sender)?);
    }

    Ok(inputs)
}

/// Resolve a view function's object parameters from raw 32-byte object ids.
/// Signature inspection ensures a pure `address` argument is never mistaken
/// for an object input merely because both have the same BCS representation.
pub(crate) fn infer_view_object_inputs(
    state: &RpcServerState,
    args: &[Vec<u8>],
    candidate_arg_indices: Option<&HashSet<usize>>,
) -> anyhow::Result<Vec<ObjectInput>> {
    let Some(indices) = candidate_arg_indices else {
        anyhow::bail!(
            "unable to inspect view function signature for automatic object resolution; provide explicit object inputs"
        );
    };

    let state_guard = state.engine.state_read();
    let mut inputs = Vec::with_capacity(indices.len());
    let mut seen = HashSet::new();
    for index in indices {
        let arg = args.get(*index).ok_or_else(|| {
            anyhow::anyhow!("view function object parameter {index} is missing from --arg values")
        })?;
        if arg.len() != AccountAddress::LENGTH {
            anyhow::bail!(
                "view function object parameter {index} must be a 32-byte object id; received {} bytes",
                arg.len()
            );
        }
        let object_id = object_id_from_address_bytes(arg).ok_or_else(|| {
            anyhow::anyhow!("view function object parameter {index} is not a valid address")
        })?;
        if !seen.insert(object_id.clone()) {
            anyhow::bail!(
                "view function reuses object input {object_id}; pass it once in the function signature instead"
            );
        }
        let stored = state_guard.get_object(&object_id)?.ok_or_else(|| {
            anyhow::anyhow!("object parameter {index} references unknown object {object_id}")
        })?;
        let digest = stored.digest();
        let object = kanari_rpc_api::ObjectInfo {
            id: object_id,
            owner: format!("{:#x}", stored.owner),
            owner_kind: stored.owner_kind,
            type_: stored.type_,
            data: stored.data,
            version: stored.version,
            digest: Some(digest),
        };
        inputs.push(object_input_from_info(&object));
    }
    Ok(inputs)
}
