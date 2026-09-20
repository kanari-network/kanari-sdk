// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use arbitrary::Unstructured;
use kanari_move_runtime_v1::move_runtime::MoveRuntime;
use kanari_move_runtime_v1::state::StateManager;
use libfuzzer_sys::fuzz_target;
use move_binary_format::file_format::CompiledModule;
use move_core_types::account_address::AccountAddress;

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);
    
    // Create MoveRuntime
    if let Ok(runtime) = MoveRuntime::new_with_kanari_natives_in_memory() {
        let mut state = StateManager::new_in_memory();
        
        // Initialize system clock
        let _ = runtime.ensure_system_clock(&mut state);
        
        // Generate random sender
        let mut sender_bytes = [0u8; 32];
        unstructured.fill_bytes(&mut sender_bytes).unwrap_or_default();
        let sender = AccountAddress::new(sender_bytes);
        
        // Test 1: Try to publish with empty module bytes
        let empty_module = vec![];
        let _ = runtime.publish_module_with_context_and_persistence(
            empty_module.clone(),
            sender,
            None,
            true,
        );
        
        // Test 2: Try to publish with random bytes (invalid module)
        let random_module = unstructured.bytes(1024).unwrap_or_default();
        let _ = runtime.publish_module_with_context_and_persistence(
            random_module,
            sender,
            None,
            true,
        );
        
        // Test 3: Try to publish with very large module
        let mut large_module = vec![0u8; 1024 * 1024]; // 1MB
        unstructured.fill_bytes(&mut large_module).unwrap_or_default();
        let _ = runtime.publish_module_with_context_and_persistence(
            large_module,
            sender,
            None,
            true,
        );
        
        // Test 4: Try to publish with malicious bytes
        // Module with invalid magic bytes
        let mut malicious_module = vec![0xFF, 0xFE, 0xFD, 0xFC];
        malicious_module.extend_from_slice(&unstructured.bytes(256).unwrap_or_default());
        let _ = runtime.publish_module_with_context_and_persistence(
            malicious_module,
            sender,
            None,
            true,
        );
        
        // Test 5: Try to publish with all possible byte patterns
        let byte_pattern_count = unstructured.int_in_range(1..=10).unwrap_or(1);
        for _ in 0..byte_pattern_count {
            let pattern_byte = unstructured.int_in_range(0..=255).unwrap_or(0) as u8;
            let pattern_module = vec![pattern_byte; unstructured.int_in_range(1..=100).unwrap_or(1) as usize];
            let _ = runtime.publish_module_with_context_and_persistence(
                pattern_module,
                sender,
                None,
                true,
            );
        }
        
        // Test 6: Module upgrade with invalid existing module
        let invalid_module_id = format!("0x{}", unstructured.int_in_range(0..=u64::MAX).unwrap_or(0));
        let new_module = unstructured.bytes(512).unwrap_or_default();
        let _ = runtime.upgrade_module_with_context_and_persistence(
            invalid_module_id,
            new_module,
            sender,
            None,
            true,
        );
    }
});
