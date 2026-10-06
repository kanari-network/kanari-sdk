// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use kanari_move_runtime_v1::changeset::{ChangeSet, CreatedObject, StateAccessSet};
use kanari_move_runtime_v1::state::{OwnerState, StateManager};
use kanari_types::balance::BalanceRecord;
use kanari_types::transaction::ObjectOwnerKind;
use libfuzzer_sys::fuzz_target;
use move_core_types::account_address::AccountAddress;

/// Arbitrary OwnerState generator.
#[derive(Debug, Arbitrary)]
struct ArbitraryOwnerState {
    address_bytes: [u8; 32],
    nonce: u64,
    module_count: u8,
    token_balance_count: u8,
}

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);
    let mut state = StateManager::new_in_memory();

    // Build a handful of owner states from fuzzed data.
    let owner_count: u32 = unstructured.int_in_range(1..=10).unwrap_or(1);
    let mut owners = Vec::new();

    for _ in 0..owner_count {
        let Ok(arb) = ArbitraryOwnerState::arbitrary(&mut unstructured) else {
            break;
        };
        let address = AccountAddress::new(arb.address_bytes);
        let mut owner_state = OwnerState::new(address);
        owner_state.nonce = arb.nonce;

        for i in 0..arb.module_count {
            owner_state.modules.insert(format!("0x1::module_{}", i));
        }

        for i in 0..arb.token_balance_count {
            owner_state.set_token_balance(
                format!("0x1::coin::Coin<0x1::token{}>", i),
                BalanceRecord::new(i as u64 * 100),
            );
        }

        owners.push((address, owner_state));
    }

    for (_, owner_state) in &owners {
        let _ = state.save_owner_state(owner_state);
    }

    // Generate random changesets and apply them; none of this may panic.
    let changeset_count: u32 = unstructured.int_in_range(1..=5).unwrap_or(1);

    for _ in 0..changeset_count {
        let mut cs = ChangeSet::new();

        let owner_slot: u32 = unstructured
            .int_in_range(0..=(owners.len().saturating_sub(1)) as u32)
            .unwrap_or(0);
        if let Some(&(owner, _)) = owners.get(owner_slot as usize) {
            let token_type = format!(
                "0x1::coin::TEST{}",
                unstructured.int_in_range(0..=100).unwrap_or(0)
            );
            let balance = unstructured.int_in_range(0..=10000).unwrap_or(0) as u64;
            cs.add_token_balance_set(owner, token_type, balance);

            let obj_count: u32 = unstructured.int_in_range(0..=5).unwrap_or(0);
            for _ in 0..obj_count {
                let mut obj_bytes = [0u8; 32];
                let _ = unstructured.fill_buffer(&mut obj_bytes);
                let object_id = AccountAddress::new(obj_bytes);

                let owner_kind = match unstructured.int_in_range::<u32>(0..=3).unwrap_or(0) {
                    0 => ObjectOwnerKind::AddressOwner(owner.to_hex_literal()),
                    1 => ObjectOwnerKind::AddressOwner(object_id.to_hex_literal()),
                    2 => ObjectOwnerKind::Shared,
                    _ => ObjectOwnerKind::Immutable,
                };

                let type_str = format!(
                    "0x1::test::Object{}",
                    unstructured.int_in_range(0..=100).unwrap_or(0)
                );
                let data_len: u32 = unstructured.int_in_range(0..=256).unwrap_or(0);
                let data = unstructured
                    .bytes(data_len as usize)
                    .unwrap_or_default()
                    .to_vec();
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

            // Build an access manifest through the public reads/writes sets.
            let access_count: u32 = unstructured.int_in_range(0..=10).unwrap_or(0);
            let mut access = StateAccessSet::default();
            for _ in 0..access_count {
                let key_len: u32 = unstructured.int_in_range(0..=32).unwrap_or(32);
                let key = unstructured
                    .bytes(key_len as usize)
                    .unwrap_or_default()
                    .to_vec();
                access.reads.insert(key.clone());
                if unstructured.int_in_range::<u32>(0..=2).unwrap_or(0) > 0 {
                    access.writes.insert(key);
                }
            }

            let derived = cs.deterministic_access_set();
            assert_eq!(
                access.conflicts_with(&derived),
                derived.conflicts_with(&access),
                "conflicts_with must be symmetric"
            );
        }

        let _ = state.apply_changeset(&cs);
    }

    // Reading back every owner and recomputing the root must never panic.
    for (address, _) in &owners {
        let _ = state.get_owner_state(address);
    }
    let _ = state.compute_state_root();
});
