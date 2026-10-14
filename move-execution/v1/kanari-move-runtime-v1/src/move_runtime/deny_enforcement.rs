// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Transfer-time enforcement of the shared deny list (Sui parity).
//!
//! Every `Coin<T>` transfer funnels through the transfer native, so checking
//! recipients here covers all Move paths — including plain
//! `transfer::public_transfer`, which the `*_checked` helpers cannot see.
//!
//! Reads (fail-closed, deterministic):
//! 1. this transaction's pending dynamic-field ops (a same-tx deny+transfer
//!    is blocked, matching execution-time reads),
//! 2. the speculative state overlay,
//! 3. durable storage under the fixed deny-list id.
//!
//! Notes on soundness trade-offs (documented, not hidden):
//! - Only transfers are checked (`freeze`/`share` carry no recipient).
//! - A dynamic-field slot that exists but does not strictly decode as a
//!   `TypeDenySet` is treated as ABSENT (logged): only the coin paths write
//!   well-formed sets, and bricking transfers on foreign garbage would hand
//!   anyone a cheap per-type DoS. (Dynamic fields here are keyed by name
//!   bytes alone, so same-layout key types can collide; the deny module uses
//!   a single key type precisely to avoid this.)
//! - Coin types with generic arguments normalize only their leading address;
//!   regulated coins are concrete types in practice.

use super::*;
use anyhow::{Context, Result, bail};
use kanari_system_natives::dynamic_field::DynamicFieldOp;
use kanari_system_natives::transfer_natives::TransferredObject;
use move_core_types::account_address::AccountAddress;
use std::collections::{BTreeMap, BTreeSet};

/// Marker searched for in transferred object type strings.
const COIN_MARKER: &str = "::coin::Coin<";
/// `COIN_INDEX` from `kanari_system::deny_list` (coin deny entries).
const COIN_INDEX: u64 = 0;

impl MoveRuntime {
    /// Reject transfers of regulated coins to denied (or paused-type)
    /// recipients. Runs before the changeset records anything, so a denied
    /// transfer persists nothing.
    pub(crate) fn enforce_coin_deny_list(
        &self,
        transferred: &[TransferredObject],
        pending_fields: &[DynamicFieldOp],
        state_overlay: crate::StateOverlayView<'_>,
        sender: Option<AccountAddress>,
    ) -> Result<()> {
        // Group recipients by normalized coin-type key so each deny set is
        // loaded at most once per transaction.
        let mut by_type: BTreeMap<Vec<u8>, Vec<AccountAddress>> = BTreeMap::new();
        for obj in transferred {
            if obj.is_frozen || obj.is_shared {
                continue;
            }
            let Some(inner) = coin_inner_type(&obj.object_type) else {
                continue;
            };
            let Some(type_key) = normalize_type_key(inner) else {
                continue;
            };
            by_type.entry(type_key).or_default().push(obj.recipient);
        }
        if by_type.is_empty() {
            return Ok(());
        }

        for (type_key, recipients) in &by_type {
            let Some((denied, paused)) =
                self.lookup_type_deny_set(type_key, pending_fields, state_overlay)?
            else {
                continue;
            };
            for recipient in recipients {
                if paused {
                    bail!(
                        "transfer of regulated coin to {} blocked: coin type is globally paused",
                        recipient.to_hex_literal()
                    );
                }
                if !is_reserved_address(recipient) && denied_contains(&denied, recipient) {
                    bail!(
                        "transfer of regulated coin to {} blocked: recipient is denied",
                        recipient.to_hex_literal()
                    );
                }
            }
            // A denied address may not move the coin either — not just
            // receive it. (Pause is already covered above: every transfer
            // has a recipient.)
            if let Some(sender) = sender
                && !is_reserved_address(&sender)
                && denied_contains(&denied, &sender)
            {
                bail!(
                    "transfer of regulated coin blocked: sender {} is denied",
                    sender.to_hex_literal()
                );
            }
        }
        Ok(())
    }

    /// Reject mutable uses of regulated coins by denied owners at input
    /// binding time: a denied address may not split/join/burn (or otherwise
    /// mutate) coins of the denied type, not just receive them. Shared or
    /// frozen inputs have no single owner and are skipped; pure reads are
    /// unaffected (only `&mut`-bound inputs are listed here).
    /// Runs after object binding, before execution. No pending writes exist
    /// yet, so only the overlay and durable state are consulted.
    pub(crate) fn enforce_coin_inputs(
        &self,
        loaded: &[(usize, String, AccountAddress, ObjectOwnerKind, String, u64)],
        state_overlay: crate::StateOverlayView<'_>,
    ) -> Result<()> {
        use std::collections::{BTreeMap, BTreeSet};

        let mut by_type: BTreeMap<Vec<u8>, BTreeSet<AccountAddress>> = BTreeMap::new();
        for (_, _, owner, kind, type_name, _) in loaded {
            if !matches!(kind, ObjectOwnerKind::AddressOwner(_)) {
                continue;
            }
            let Some(inner) = coin_inner_type(type_name) else {
                continue;
            };
            let Some(type_key) = normalize_type_key(inner) else {
                continue;
            };
            by_type.entry(type_key).or_default().insert(*owner);
        }
        if by_type.is_empty() {
            return Ok(());
        }

        for (type_key, owners) in &by_type {
            let Some((denied, paused)) = self.lookup_type_deny_set(type_key, &[], state_overlay)?
            else {
                continue;
            };
            for owner in owners {
                if paused {
                    bail!(
                        "use of regulated coin by {} blocked: coin type is globally paused",
                        owner.to_hex_literal()
                    );
                }
                if !is_reserved_address(owner) && denied_contains(&denied, owner) {
                    bail!(
                        "use of regulated coin by {} blocked: owner is denied",
                        owner.to_hex_literal()
                    );
                }
            }
        }
        Ok(())
    }
    /// Load and strictly decode the deny set for one normalized coin-type
    /// key. `Ok(None)` means "no usable entry": absent anywhere, deleted in
    /// this transaction, or present-but-undecodable (see module docs).
    fn lookup_type_deny_set(
        &self,
        type_key: &[u8],
        pending_fields: &[DynamicFieldOp],
        state_overlay: crate::StateOverlayView<'_>,
    ) -> Result<Option<(BTreeSet<[u8; 32]>, bool)>> {
        let mut name = Vec::with_capacity(8 + 5 + type_key.len());
        name.extend_from_slice(&COIN_INDEX.to_le_bytes());
        append_uleb128(&mut name, type_key.len() as u64);
        name.extend_from_slice(type_key);
        let deny_id = Self::deny_list_object_id();

        // 1. This transaction's pending dynamic-field ops (last write wins).
        let mut pending: Option<Option<Vec<u8>>> = None;
        for op in pending_fields {
            match op {
                DynamicFieldOp::Add {
                    object_id,
                    name_bytes,
                    value_bytes,
                } if object_id == &deny_id && name_bytes == &name => {
                    pending = Some(Some(value_bytes.clone()));
                }
                DynamicFieldOp::Remove {
                    object_id,
                    name_bytes,
                } if object_id == &deny_id && name_bytes == &name => {
                    pending = Some(None);
                }
                _ => {}
            }
        }
        if let Some(found) = pending {
            return match found {
                Some(bytes) => Ok(decode_type_deny_set(&bytes)),
                None => Ok(None),
            };
        }

        // 2. Speculative overlay (values are BCS(length-prefixed bytes)).
        let df_key = crate::common::keys::dynamic_field_key(&deny_id, &name);
        if let Some(overlay) = state_overlay
            && let Some(slot) = overlay.get(&df_key)
        {
            return match slot {
                Some(bytes) => {
                    let raw: Vec<u8> = bcs::from_bytes(bytes)
                        .context("deny-list overlay entry failed to decode")?;
                    Ok(decode_type_deny_set(&raw))
                }
                None => Ok(None),
            };
        }

        // 3. Durable storage (`load` already BCS-decodes into the raw
        // value bytes, mirroring the dynamic-field resolver).
        let stored: Option<Vec<u8>> = self
            .state
            .store()
            .load(&df_key)
            .context("deny-list registry read failed")?;
        match stored {
            Some(raw) => Ok(decode_type_deny_set(&raw)),
            None => Ok(None),
        }
    }
}

/// Framework/system addresses (`@0x0`–`@0xF`) can never be denied,
/// mirroring `deny_list::is_reserved` so system flows cannot brick on
/// adversarial state.
fn is_reserved_address(addr: &AccountAddress) -> bool {
    let bytes = addr.as_ref();
    bytes[..AccountAddress::LENGTH - 1].iter().all(|b| *b == 0)
        && bytes[AccountAddress::LENGTH - 1] <= 0x0F
}

fn denied_contains(denied: &BTreeSet<[u8; 32]>, addr: &AccountAddress) -> bool {
    denied.iter().any(|entry| entry.as_slice() == addr.as_ref())
}

/// Extract the inner `T` from a `...::coin::Coin<T>` type string, if any.
/// The substring between the first `<` and the final `>` renders exactly
/// like `T` on its own, so it matches registry keys computed from `T`.
fn coin_inner_type(object_type: &str) -> Option<&str> {
    let start = object_type.find(COIN_MARKER)? + COIN_MARKER.len();
    let end = object_type.len().checked_sub(1)?;
    if end < start || !object_type.ends_with('>') {
        return None;
    }
    Some(&object_type[start..end])
}

/// Normalize a coin type string to the `type_name` byte format the Move
/// side keys entries by: every `0x<hex>` address becomes full 32-byte
/// lowercase hex with no prefix (Move renders `0x2::m::T` as
/// `0000...0002::m::T`; verified by `type_key_format_pinned`).
fn normalize_type_key(inner: &str) -> Option<Vec<u8>> {
    let bytes = inner.as_bytes();
    let mut out = String::with_capacity(inner.len() + 32);
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'0' && i + 1 < bytes.len() && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X')
        {
            let mut j = i + 2;
            while j < bytes.len() && bytes[j].is_ascii_hexdigit() {
                j += 1;
            }
            let hex_part = &inner[i + 2..j];
            if hex_part.is_empty() || hex_part.len() > AccountAddress::LENGTH * 2 {
                return None;
            }
            out.push_str(&format!("{:0>64}", hex_part.to_ascii_lowercase()));
            i = j;
        } else {
            if !bytes[i].is_ascii() {
                return None;
            }
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    Some(out.into_bytes())
}

fn append_uleb128(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn read_uleb128(data: &[u8], pos: &mut usize) -> Option<u64> {
    let mut result = 0u64;
    let mut shift = 0u32;
    loop {
        let byte = *data.get(*pos)?;
        *pos += 1;
        result |= ((byte & 0x7F) as u64).checked_shl(shift)?;
        if byte & 0x80 == 0 {
            return Some(result);
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
}

/// Strictly decode a `TypeDenySet`: `UID[32] ++ VecSet{Vec<addr[32]>} ++
/// paused:bool`, with no trailing bytes. Returns `None` on any deviation
/// (callers treat that as absent; see module docs).
fn decode_type_deny_set(data: &[u8]) -> Option<(BTreeSet<[u8; 32]>, bool)> {
    let mut pos = 32usize;
    if data.len() < pos {
        return None;
    }
    let count = read_uleb128(data, &mut pos)? as usize;
    let mut denied = BTreeSet::new();
    for _ in 0..count {
        let bytes = data.get(pos..pos + 32)?;
        let mut addr = [0u8; 32];
        addr.copy_from_slice(bytes);
        pos += 32;
        denied.insert(addr);
    }
    let paused = match data.get(pos) {
        Some(1) => true,
        Some(0) => false,
        _ => return None,
    };
    pos += 1;
    if pos != data.len() {
        return None;
    }
    Some((denied, paused))
}
