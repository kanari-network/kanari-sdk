// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use kanari_move_runtime_v1::changeset::{ChangeSet, CreatedObject};
use kanari_move_runtime_v1::state::{OwnerState, StateManager};
use kanari_types::balance::BalanceRecord;
use kanari_types::object::{IDRecord, UIDRecord};
use kanari_types::transaction::ObjectOwnerKind;
use libfuzzer_sys::fuzz_target;
use move_core_types::account_address::AccountAddress;
use std::collections::BTreeSet;

/// Arbitrary OwnerState generator
#[derive(Debug, Arbitrary)]
struct ArbitraryOwnerState {
    address_bytes: [u8; 32],
    nonce: u64,
    module_count: u8,
    token_balance_count: u8,
}

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);

    // Create a StateManager
    let mut state = StateManager::new_in_memory();

    // Generate multiple random owner states
    let owner_count = std::cmp::min(unstructured.int_in_range(1..=10).unwrap_or(1), 10);

    let mut owners = Vec::new();

    for _ in 0..owner_count {
        if let Ok(arb_owner) = ArbitraryOwnerState::arbitrary(&mut unstructured) {
            let address = AccountAddress::new(arb_owner.address_bytes);
            let mut owner_state = OwnerState::new(address);
            owner_state.nonce = arb_owner.nonce;

            // Add some modules
            for i in 0..arb_owner.module_count {
                owner_state.add_module(format!("0x1::module_{}", i));
            }

            // Add some token balances
            for i in 0..arb_owner.token_balance_count {
                owner_state.set_token_balance(
                    format!("0x1::coin::Coin<0x1::token{}>", i),
                    BalanceRecord::new(i as u64 * 100),
                );
            }

            owners.push((address, owner_state));
        }
    }

    // Apply owner states
    for (address, owner_state) in owners {
        let _ = state.set_owner_state(address, owner_state);
    }

    // Generate random changesets and apply them
    let changeset_count = std::cmp::min(unstructured.int_in_range(1..=5).unwrap_or(1), 5);

    for _ in 0..changeset_count {
        let mut cs = ChangeSet::new();

        // Add some random token balance changes
        if let Ok(owner_idx) = unstructured.int_in_range(0..=owners.len() as i32) {
            let owner = owners[owner_idx as usize].0;
            let token_type = format!(
                "0x1::coin::TEST{}",
                unstructured.int_in_range(0..=100).unwrap_or(0)
            );
            let balance = unstructured.int_in_range(0..=10000).unwrap_or(0) as u64;
            cs.add_token_balance_set(owner, token_type, balance);

            // Add some created objects
            let obj_count = std::cmp::min(unstructured.int_in_range(0..=5).unwrap_or(0), 5);
            for _ in 0..obj_count {
                let owner_kind = match unstructured.int_in_range(0..=4).unwrap_or(0) {
                    0 => ObjectOwnerKind::AddressOwner(owner.to_hex_literal()),
                    1 => ObjectOwnerKind::ObjectOwner(format!("0x{}", owner)),
                    2 => ObjectOwnerKind::SharedOwner {
                        initial_shared_version: 1,
                    },
                    3 => ObjectOwnerKind::Immutable,
                    _ => ObjectOwnerKind::AddressOwner(owner.to_hex_literal()),
                };

                let type_str = format!(
                    "0x1::test::Object{}",
                    unstructured.int_in_range(0..=100).unwrap_or(0)
                );
                let data = unstructured
                    .bytes(unstructured.int_in_range(0..=256).unwrap_or(0) as usize)
                    .unwrap_or_default();
                let version = unstructured.int_in_range(1..=100).unwrap_or(1) as u64;

                cs.created_objects.push((
                    format!(
                        "0x{:016x}",
                        unstructured.int_in_range(0..=u64::MAX).unwrap_or(0)
                    ),
                    CreatedObject {
                        owner,
                        owner_kind,
                        uid: None,
                        id: None,
                        type_: type_str,
                        data,
                        version,
                    },
                ));
            }

            // Add some state access
            for _ in 0..unstructured.int_in_range(0..=10).unwrap_or(0) {
                let key = unstructured.bytes(32).unwrap_or_default();
                cs.state_access_set.read(key.clone());
                if unstructured.int_in_range(0..=2).unwrap_or(0) > 0 {
                    cs.state_access_set.write(key);
                }
            }
        }

        // Try to apply changeset
        let _ = state.apply_changeset(&cs);
    }

    // Verify state consistency
    for (address, _) in &owners {
        let _owner_state = state.get_owner_state(*address);
    }

    // Try to get state root hash
    let _state_root = state.compute_state_root();
});
