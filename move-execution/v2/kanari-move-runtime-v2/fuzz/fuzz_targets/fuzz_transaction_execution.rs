// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use arbitrary::Unstructured;
use kanari_move_runtime_v2::move_runtime::MoveRuntime;
use kanari_move_runtime_v2::state::StateManager;
use kanari_move_runtime_v2::TransactionScheduler;
use kanari_types::transaction::{
    ObjectInput, ObjectOwnerKind, ObjectRef, SignedTransaction, Transaction,
};
use libfuzzer_sys::fuzz_target;
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::{ModuleId, TypeTag};
use std::collections::BTreeSet;

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);

    let mut sender_bytes = [0u8; 32];
    let _ = unstructured.fill_buffer(&mut sender_bytes);
    let sender = AccountAddress::new(sender_bytes);

    // Object inputs for the entry-function context.
    let input_count: u32 = unstructured.int_in_range(0..=8).unwrap_or(0);
    let mut object_inputs = Vec::new();
    for _ in 0..input_count {
        let mut object_bytes = [0u8; 32];
        let _ = unstructured.fill_buffer(&mut object_bytes);
        let object_id = AccountAddress::new(object_bytes).to_hex_literal();

        let version = unstructured.int_in_range(1..=100).unwrap_or(1) as u64;
        let digest = format!(
            "0x{:064x}",
            unstructured.int_in_range(0..=u64::MAX).unwrap_or(0)
        );
        let owner_kind = match unstructured.int_in_range::<u32>(0..=3).unwrap_or(0) {
            0 => ObjectOwnerKind::AddressOwner(sender.to_hex_literal()),
            1 => ObjectOwnerKind::AddressOwner(object_id.clone()),
            2 => ObjectOwnerKind::Shared,
            _ => ObjectOwnerKind::Immutable,
        };
        let mutable = unstructured.int_in_range::<u32>(0..=1).unwrap_or(0) == 1;

        object_inputs.push(ObjectInput {
            object_ref: ObjectRef::new(object_id, Some(version), Some(digest)),
            owner: Some(owner_kind),
            mutable,
        });
    }

    if let Ok(runtime) = MoveRuntime::new_with_kanari_natives_in_memory() {
        let mut state = StateManager::new_in_memory();
        let _ = runtime.ensure_system_clock(&mut state);

        let module_id = ModuleId::new(
            sender,
            Identifier::new("test").expect("static module name is a valid identifier"),
        );

        // Entry function with an explicit gas budget and timestamp.
        let _ = runtime.execute_entry_function(
            &module_id,
            "test",
            Vec::<TypeTag>::new(),
            Vec::new(),
            Some(sender),
            Some((1000, 1000)),
            Some(12345),
        );

        // Entry function with fuzzed object inputs.
        let _ = runtime.execute_entry_function(
            &module_id,
            "test",
            Vec::<TypeTag>::new(),
            vec![vec![1, 2, 3, 4]],
            Some(sender),
            Some((2000, 1)),
            None,
        );

        // Read-only view call against the fuzzed object inputs.
        let _ = runtime.execute_view_function("0x1", "test", "test", &[], &[], &object_inputs);

        // Zero-ish budget must be rejected gracefully, never panic.
        let _ = runtime.execute_entry_function(
            &module_id,
            "test",
            Vec::<TypeTag>::new(),
            Vec::new(),
            Some(sender),
            Some((0, 0)),
            None,
        );
    }

    // Scheduling fuzzed transactions must preserve order, bound wave size, and
    // never place two conflicting transactions in the same speculative wave.
    let tx_count: u64 = unstructured.int_in_range(1..=16).unwrap_or(1);
    let mut txs = Vec::new();
    for index in 0..tx_count {
        let mut object_bytes = [0u8; 32];
        let _ = unstructured.fill_buffer(&mut object_bytes);
        let object_id = AccountAddress::new(object_bytes).to_hex_literal();

        let object_inputs = if unstructured.int_in_range::<u32>(0..=1).unwrap_or(0) == 1 {
            vec![ObjectInput {
                object_ref: ObjectRef::new(object_id, None, None),
                owner: Some(ObjectOwnerKind::AddressOwner(sender.to_hex_literal())),
                mutable: true,
            }]
        } else {
            Vec::new()
        };

        txs.push(SignedTransaction::new(Transaction::ExecuteFunction {
            sender: sender.to_hex_literal(),
            module: format!("0x1::module_{}", index % 4),
            function: "run".to_string(),
            type_args: Vec::new(),
            args: Vec::new(),
            object_inputs,
            gas_payment: None,
            gas_limit: 1000,
            gas_price: 1,
            nonce: index,
        }));
    }

    let expected_hashes: Vec<Vec<u8>> = txs
        .iter()
        .map(|tx| tx.transaction_hash().to_vec())
        .collect();
    let waves = TransactionScheduler::schedule(txs);

    let scheduled_hashes: Vec<Vec<u8>> = waves
        .iter()
        .flatten()
        .map(|tx| tx.transaction_hash().to_vec())
        .collect();
    assert_eq!(
        scheduled_hashes, expected_hashes,
        "scheduling must preserve the global transaction order"
    );

    for wave in &waves {
        assert!(
            wave.len() <= TransactionScheduler::MAX_SPECULATIVE_WAVE_SIZE,
            "wave size must stay within MAX_SPECULATIVE_WAVE_SIZE"
        );

        let keys: Vec<BTreeSet<String>> = wave
            .iter()
            .map(|tx| tx.transaction.get_conflict_keys().into_iter().collect())
            .collect();
        for (i, left) in keys.iter().enumerate() {
            for right in keys.iter().skip(i + 1) {
                let conflict = left.intersection(right).next();
                assert!(
                    conflict.is_none(),
                    "conflicting transactions scheduled in one wave: {}",
                    conflict.cloned().unwrap_or_default()
                );
            }
        }
    }
});
