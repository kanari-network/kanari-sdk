// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use arbitrary::Unstructured;
use kanari_move_runtime_v1::kanari_gas_meter::KanariGasMeter;
use kanari_move_runtime_v1::move_runtime::MoveRuntime;
use kanari_types::gas::GasOperation;
use libfuzzer_sys::fuzz_target;
use move_core_types::account_address::AccountAddress;

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);
    
    // Create MoveRuntime
    if let Ok(runtime) = MoveRuntime::new_with_kanari_natives_in_memory() {
        
        // Test 1: Execute with zero gas limit
        let sender = AccountAddress::new([1u8; 32]);
        let _ = runtime.execute_entry_function(
            "test_function",
            vec![],
            vec![],
            sender,
            Some((0, 0)),  // Zero gas limit
            None,
            None,
        );
        
        // Test 2: Execute with maximum gas limit
        let sender = AccountAddress::new([2u8; 32]);
        let _ = runtime.execute_entry_function(
            "test_function",
            vec![],
            vec![],
            sender,
            Some((u64::MAX, u64::MAX)),  // Max gas limit
            None,
            None,
        );
        
        // Test 3: Execute with mismatched gas limits (limit < price)
        let sender = AccountAddress::new([3u8; 32]);
        let _ = runtime.execute_entry_function(
            "test_function",
            vec![],
            vec![],
            sender,
            Some((100, 1000)),  // limit < price
            None,
            None,
        );
        
        // Test 4: Random gas limit combinations
        let test_count = unstructured.int_in_range(1..=20).unwrap_or(1);
        for _ in 0..test_count {
            let sender = AccountAddress::new([
                unstructured.int_in_range(0..=255).unwrap_or(0) as u8;
                32
            ]);
            
            let gas_limit = unstructured.int_in_range(0..=u64::MAX).unwrap_or(1000);
            let gas_price = unstructured.int_in_range(0..=u64::MAX).unwrap_or(1);
            
            let _ = runtime.execute_entry_function(
                "test_function",
                vec![],
                vec![],
                sender,
                Some((gas_limit, gas_price)),
                None,
                None,
            );
        }
        
        // Test 5: Gas meter operations with edge cases
        let mut gas_meter = KanariGasMeter::new(1000, 1);
        
        // Try to charge more than available
        let _ = gas_meter.charge_gas(2000);
        
        // Try to charge with overflow values
        let charge_amount = unstructured.int_in_range(0..=u64::MAX).unwrap_or(100);
        let _ = gas_meter.charge_gas(charge_amount);
        
        // Test 6: Multiple gas operations
        let op_count = unstructured.int_in_range(1..=100).unwrap_or(10);
        for _ in 0..op_count {
            let amount = unstructured.int_in_range(0..=10000).unwrap_or(10);
            let _ = gas_meter.charge_gas(amount);
        }
        
        // Test 7: Gas operations with random GasOperation
        let op_type = unstructured.int_in_range(0..=10).unwrap_or(0);
        let gas_op = match op_type {
            0 => GasOperation::ExecuteEntryFunction,
            1 => GasOperation::PublishModule,
            2 => GasOperation::EmitEvent,
            3 => GasOperation::CreateObject,
            4 => GasOperation::TransferObject,
            5 => GasOperation::DeleteObject,
            6 => GasOperation::BorrowObject,
            7 => GasOperation::MutateObject,
            8 => GasOperation::LoadModule,
            9 => GasOperation::CallNativeFunction,
            10 => GasOperation::CreateObject,
            _ => GasOperation::ExecuteEntryFunction,
        };
        
        let _ = runtime.execute_entry_function(
            "test_function",
            vec![],
            vec![],
            AccountAddress::new([4u8; 32]),
            Some((500, 1)),
            None,
            None,
        );
    }
});
