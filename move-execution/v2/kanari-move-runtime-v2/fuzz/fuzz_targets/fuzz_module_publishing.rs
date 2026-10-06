// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use arbitrary::Unstructured;
use kanari_move_runtime_v2::move_runtime::MoveRuntime;
use kanari_move_runtime_v2::state::StateManager;
use libfuzzer_sys::fuzz_target;
use move_binary_format::file_format::CompiledModule;
use move_core_types::account_address::AccountAddress;

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);

    let mut sender_bytes = [0u8; 32];
    let _ = unstructured.fill_buffer(&mut sender_bytes);
    let sender = AccountAddress::new(sender_bytes);

    // Fuzzed module candidate; the size stays bounded by the fuzzer input.
    let module_bytes: Vec<u8> = unstructured.arbitrary().unwrap_or_default();

    // Fast path: bytecode deserialization must never panic.
    let _ = CompiledModule::deserialize_with_defaults(&module_bytes);

    if let Ok(runtime) = MoveRuntime::new_with_kanari_natives_in_memory() {
        let mut state = StateManager::new_in_memory();
        let _ = runtime.ensure_system_clock(&mut state);

        // Empty module bytes.
        let _ = runtime.publish_module_with_context_and_persistence(
            Vec::new(),
            sender,
            None,
            None,
            None,
            true,
        );

        // Random bytes (invalid module).
        let _ = runtime.publish_module_with_context_and_persistence(
            module_bytes.clone(),
            sender,
            Some((500, 1)),
            None,
            None,
            true,
        );

        // Invalid magic prefix followed by fuzzed bytes.
        let mut malicious_module = vec![0xFF, 0xFE, 0xFD, 0xFC];
        malicious_module.extend_from_slice(&module_bytes);
        let _ = runtime.publish_module_with_context_and_persistence(
            malicious_module,
            sender,
            None,
            None,
            None,
            true,
        );

        // Uniform byte patterns of fuzz-chosen length.
        let pattern_count: u32 = unstructured.int_in_range(1..=4).unwrap_or(1);
        for _ in 0..pattern_count {
            let pattern_byte = unstructured.int_in_range(0..=255).unwrap_or(0) as u8;
            let pattern_len: u32 = unstructured.int_in_range(1..=64).unwrap_or(1);
            let pattern_module = vec![pattern_byte; pattern_len as usize];
            let _ = runtime.publish_module_with_context_and_persistence(
                pattern_module,
                sender,
                None,
                None,
                None,
                true,
            );
        }

        // Upgrade path with fuzzed bytes.
        let _ = runtime.upgrade_module_with_context_and_persistence(
            module_bytes,
            sender,
            None,
            None,
            None,
            true,
        );
    }
});
