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
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);

    // Test CreatedObject serialization/deserialization
    if let Ok(type_str) = unstructured.arbitrary::<String>() {
        if let Ok(data) = unstructured.arbitrary::<Vec<u8>>() {
            if let Ok(version) = unstructured.arbitrary::<u64>() {
                let owner = AccountAddress::from_hex_literal("0x11111111111111111111111111111111")
                    .unwrap_or_default();
                let owner_kind = ObjectOwnerKind::AddressOwner(owner.to_hex_literal());

                let created = CreatedObject {
                    owner,
                    owner_kind,
                    uid: Some(UIDRecord { bytes: [1u8; 32] }),
                    id: Some(IDRecord { bytes: [2u8; 32] }),
                    type_: type_str,
                    data: data.clone(),
                    version,
                };

                // Test serialization
                if let Ok(serialized) = bcs::to_bytes(&created) {
                    // Test deserialization
                    let _deserialized: Result<CreatedObject, _> = bcs::from_bytes(&serialized);
                }

                // Test JSON serialization if the type supports it
                if let Ok(json) = serde_json::to_string(&created) {
                    let _: Result<CreatedObject, _> = serde_json::from_str(&json);
                }

                // Test digest computation
                let _digest = created.digest();

                // Test object_ref
                let _obj_ref = created.object_ref("0xtest");

                // Test with no UID/ID
                let created_no_uid = CreatedObject {
                    owner,
                    owner_kind,
                    uid: None,
                    id: None,
                    type_: type_str,
                    data: data.clone(),
                    version,
                };

                let _digest2 = created_no_uid.digest();
                let _obj_ref2 = created_no_uid.object_ref("0xtest2");
            }
        }
    }

    // Test OwnerState serialization
    if let Ok(address_bytes) = unstructured.arbitrary::<[u8; 32]>() {
        let address = AccountAddress::new(address_bytes);
        let mut owner_state = OwnerState::new(address);

        if let Ok(nonce) = unstructured.arbitrary::<u64>() {
            owner_state.nonce = nonce;
        }

        if let Ok(module_count) = unstructured.int_in_range(0..=10) {
            for i in 0..module_count {
                owner_state.add_module(format!("0x1::module_{}", i));
            }
        }

        if let Ok(token_count) = unstructured.int_in_range(0..=10) {
            for i in 0..token_count {
                let token_type = format!("0x1::coin::Token{}", i);
                let amount = unstructured.int_in_range(0..=1000).unwrap_or(0) as u64;
                owner_state.set_token_balance(token_type, BalanceRecord::new(amount));
            }
        }

        // Test serialization
        if let Ok(serialized) = bcs::to_bytes(&owner_state) {
            let _: Result<OwnerState, _> = bcs::from_bytes(&serialized);
        }

        // Test JSON serialization
        if let Ok(json) = serde_json::to_string(&owner_state) {
            let _: Result<OwnerState, _> = serde_json::from_str(&json);
        }

        // Test cloning
        let _cloned = owner_state.clone();
    }

    // Test various ObjectOwnerKind variants
    for kind in [
        ObjectOwnerKind::AddressOwner(
            AccountAddress::from_hex_literal("0x1111")
                .unwrap_or_default()
                .to_hex_literal(),
        ),
        ObjectOwnerKind::ObjectOwner("0x2222".to_string()),
        ObjectOwnerKind::SharedOwner {
            initial_shared_version: 1,
        },
        ObjectOwnerKind::Immutable,
    ] {
        let _ = kind;
        // Just ensure they can be created and used
    }
});
