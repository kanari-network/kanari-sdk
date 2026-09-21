// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use arbitrary::Unstructured;
use kanari_move_runtime_v1::move_runtime::MoveRuntime;
use kanari_move_runtime_v1::state::StateManager;
use kanari_move_runtime_v1::TransactionScheduler;
use kanari_types::transaction::{ObjectInput, ObjectOwnerKind, ObjectRef};
use libfuzzer_sys::fuzz_target;
use move_core_types::account_address::AccountAddress;

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);
    
    // Create MoveRuntime - this is the main runtime
    if let Ok(runtime) = MoveRuntime::new_with_kanari_natives_in_memory() {
        let mut state = StateManager::new_in_memory();
        
        // Initialize system clock if needed
        let _ = runtime.ensure_system_clock(&mut state);
        
        // Generate random sender addresses
        let mut senders = Vec::new();
        for _ in 0..std::cmp::min(unstructured.int_in_range(1..=5).unwrap_or(1), 5) {
            let mut addr_bytes = [0u8; 32];
            unstructured.fill_bytes(&mut addr_bytes).unwrap_or_default();
            senders.push(AccountAddress::new(addr_bytes));
        }
        
        // Create some object refs for inputs
        let mut object_inputs = Vec::new();
        let num_inputs = std::cmp::min(unstructured.int_in_range(0..=10).unwrap_or(0), 10);
        
        for _ in 0..num_inputs {
            let mut obj_id_bytes = [0u8; 32];
            unstructured.fill_bytes(&mut obj_id_bytes).unwrap_or_default();
            let obj_id = AccountAddress::new(obj_id_bytes).to_hex_literal();
            
            let version = unstructured.int_in_range(1..=100).unwrap_or(1) as u64;
            let digest = format!("0x{:064x}", unstructured.int_in_range(0..=u64::MAX).unwrap_or(0));
            
            let owner_kind = match unstructured.int_in_range(0..=4).unwrap_or(0) {
                0 => ObjectOwnerKind::AddressOwner(senders[0].to_hex_literal()),
                1 => ObjectOwnerKind::ObjectOwner(obj_id.clone()),
                2 => ObjectOwnerKind::SharedOwner { initial_shared_version: 1 },
                3 => ObjectOwnerKind::Immutable,
                _ => ObjectOwnerKind::AddressOwner(senders[0].to_hex_literal()),
            };
            
            object_inputs.push(ObjectInput::ObjectRef(ObjectRef::new(
                obj_id,
                Some(version),
                Some(digest),
            )));
        }
        
        // Try to execute with different configurations
        if !senders.is_empty() {
            let sender = senders[0];
            
            // Try executing a simple entry function
            // We'll use a simple function that doesn't require specific modules
            let _result = runtime.execute_entry_function(
                "test_function",
                vec![],
                vec![],
                sender,
                Some((1000, 1000)),
                Some(12345),
                Some(vec![1, 2, 3, 4]),
            );
            
            // Try with object context
            let _result2 = runtime.execute_entry_function(
                "another_function",
                vec![],
                object_inputs.clone(),
                sender,
                Some((2000, 2000)),
                Some(54321),
                Some(vec![5, 6, 7, 8]),
            );
        }
        
        // Test view function execution (read-only)
        if !senders.is_empty() {
            let sender = senders[0];
            let _view_result = runtime.execute_view_function(
                "test_view",
                vec![],
                vec![],
                sender,
                Some(1000),
            );
        }
        
        // Test with empty inputs
        let _empty_result = runtime.execute_entry_function(
            "empty_function",
            vec![],
            vec![],
            AccountAddress::from_hex_literal("0x1111").unwrap_or_default(),
            Some((100, 100)),
            None,
            None,
        );
        
        // Test TransactionScheduler if we can create one
        let scheduler = TransactionScheduler::new(runtime.clone());
        
        // Try to schedule a transaction
        let _schedule_result = scheduler.schedule_transaction(
            vec![],
            vec![],
            senders.get(0).copied().unwrap_or_default(),
            Some((500, 500)),
        );
    }
});
