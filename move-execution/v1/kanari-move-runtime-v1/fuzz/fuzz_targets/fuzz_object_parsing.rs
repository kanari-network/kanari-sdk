// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use arbitrary::Unstructured;
use kanari_move_runtime_v1::changeset::CreatedObject;
use kanari_move_runtime_v1::state::OwnerState;
use kanari_types::balance::BalanceRecord;
use kanari_types::object::{IDRecord, UIDRecord};
use kanari_types::transaction::ObjectOwnerKind;
use libfuzzer_sys::fuzz_target;
use move_core_types::account_address::AccountAddress;

fn sample_owner_address() -> AccountAddress {
    AccountAddress::from_hex_literal("0x11111111111111111111111111111111")
        .unwrap_or_else(|_| AccountAddress::new([0x11u8; 32]))
}

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);

    // CreatedObject serialization / deserialization roundtrip.
    if let Ok(type_str) = unstructured.arbitrary::<String>()
        && let Ok(payload) = unstructured.arbitrary::<Vec<u8>>()
        && let Ok(version) = unstructured.arbitrary::<u64>()
    {
        let owner = sample_owner_address();
        let owner_kind = ObjectOwnerKind::AddressOwner(owner.to_hex_literal());

        let created = CreatedObject {
            owner,
            owner_kind: owner_kind.clone(),
            uid: Some(UIDRecord::new(AccountAddress::new([1u8; 32]))),
            id: Some(IDRecord::new(AccountAddress::new([2u8; 32]))),
            type_: type_str.clone(),
            data: payload.clone(),
            version,
        };

        if let Ok(serialized) = bcs::to_bytes(&created) {
            let deserialized: Result<CreatedObject, _> = bcs::from_bytes(&serialized);
            if let Ok(roundtripped) = deserialized {
                assert_eq!(roundtripped.data, created.data);
                assert_eq!(roundtripped.version, created.version);
                assert_eq!(roundtripped.type_, created.type_);
            }
        }

        if let Ok(json) = serde_json::to_string(&created) {
            let deserialized: Result<CreatedObject, _> = serde_json::from_str(&json);
            if let Ok(roundtripped) = deserialized {
                assert_eq!(roundtripped.data, created.data);
                assert_eq!(roundtripped.version, created.version);
            }
        }

        // Digest and object_ref must match the stored payload.
        let digest = created.digest();
        assert!(!digest.is_empty(), "digest must never be empty");
        let object_ref = created.object_ref("0xtest");
        assert_eq!(object_ref.object_id, "0xtest");
        assert_eq!(object_ref.version, Some(created.version));
        assert_eq!(object_ref.digest.as_ref(), Some(&digest));

        // The same payload without UID/ID must produce the same digest.
        let created_no_uid = CreatedObject {
            owner,
            owner_kind,
            uid: None,
            id: None,
            type_: type_str,
            data: payload,
            version,
        };
        assert_eq!(created_no_uid.digest(), digest);
        let _ = created_no_uid.object_ref("0xtest2");
    }

    // OwnerState construction, serialization, and cloning.
    if let Ok(address_bytes) = unstructured.arbitrary::<[u8; 32]>() {
        let address = AccountAddress::new(address_bytes);
        let mut owner_state = OwnerState::new(address);

        if let Ok(nonce) = unstructured.arbitrary::<u64>() {
            owner_state.nonce = nonce;
        }

        if let Ok(module_count) = unstructured.int_in_range(0..=10u32) {
            for i in 0..module_count {
                owner_state.modules.insert(format!("0x1::module_{}", i));
            }
        }

        if let Ok(token_count) = unstructured.int_in_range(0..=10u32) {
            for i in 0..token_count {
                let token_type = format!("0x1::coin::Token{}", i);
                let amount = unstructured.int_in_range(0..=1000).unwrap_or(0) as u64;
                owner_state.set_token_balance(token_type, BalanceRecord::new(amount));
            }
        }

        if let Ok(serialized) = bcs::to_bytes(&owner_state) {
            let deserialized: Result<OwnerState, _> = bcs::from_bytes(&serialized);
            if let Ok(roundtripped) = deserialized {
                assert_eq!(roundtripped.nonce, owner_state.nonce);
                assert_eq!(roundtripped.modules, owner_state.modules);
            }
        }

        if let Ok(json) = serde_json::to_string(&owner_state) {
            let _: Result<OwnerState, _> = serde_json::from_str(&json);
        }

        let cloned = owner_state.clone();
        assert_eq!(cloned.address, owner_state.address);
        assert_eq!(cloned.modules, owner_state.modules);
        for (token_type, _) in &owner_state.token_balances {
            assert_eq!(
                cloned.get_token_balance(token_type),
                owner_state.get_token_balance(token_type),
                "cloning must preserve the token balance"
            );
        }
    }

    // Every ObjectOwnerKind variant must survive a serde roundtrip.
    let owner_hex = sample_owner_address().to_hex_literal();
    for kind in [
        ObjectOwnerKind::AddressOwner(owner_hex.clone()),
        ObjectOwnerKind::Shared,
        ObjectOwnerKind::Immutable,
    ] {
        if let Ok(json) = serde_json::to_string(&kind) {
            let parsed: Result<ObjectOwnerKind, _> = serde_json::from_str(&json);
            assert_eq!(
                parsed.ok().as_ref(),
                Some(&kind),
                "owner kind serde roundtrip must preserve the value"
            );
        }
    }
});
