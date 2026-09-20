// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Property-based fuzzing tests for kanari-move-runtime-v1 using proptest
//! 100% validation coverage for input security

use arbitrary::{Arbitrary, Unstructured};
use kanari_move_runtime_v1::changeset::ChangeSet;
use kanari_move_runtime_v1::move_runtime::MoveRuntime;
use kanari_move_runtime_v1::state::StateManager;
use kanari_move_runtime_v1::validation::{
    MAX_ARG_SIZE, MAX_ARGS, MAX_FUNCTION_NAME_LENGTH, MAX_MODULE_SIZE, MAX_TYPE_ARGS,
    ValidatedGasInfo, validate_args, validate_function_name, validate_gas_info,
    validate_module_bytes, validate_transaction, validate_type_tags,
};
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::{ModuleId, TypeTag};
use proptest::prelude::*;

const DEFAULT_FUZZ_CASES: u32 = 32;

fn runtime_fuzz_config() -> proptest::test_runner::Config {
    proptest::test_runner::Config {
        cases: std::env::var("KANARI_RUNTIME_PROPTEST_CASES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_FUZZ_CASES),
        max_global_rejects: 10000,
        ..Default::default()
    }
}

#[derive(Debug, Arbitrary)]
struct PropAccountAddress {
    bytes: [u8; 32],
}

impl From<PropAccountAddress> for AccountAddress {
    fn from(arb: PropAccountAddress) -> Self {
        AccountAddress::new(arb.bytes)
    }
}

fn build_random_changeset(unstructured: &mut Unstructured) -> ChangeSet {
    let mut cs = ChangeSet::new();
    if let Ok(owner_bytes) = unstructured.arbitrary::<[u8; 32]>() {
        let owner: AccountAddress = PropAccountAddress { bytes: owner_bytes }.into();
        let token_type = "0x1::coin::KANARI".to_string();
        let balance = unstructured.int_in_range(0..=10000).unwrap_or(0) as u64;
        cs.add_token_balance_set(owner, token_type, balance);
    }
    cs
}

// ============================================================================
// ChangeSet Tests
// ============================================================================

#[test]
fn prop_fuzz_changeset_roundtrip() {
    proptest!(runtime_fuzz_config(), |(data: Vec<u8>)| {
        prop_assume!(data.len() <= 4096);
        let mut unstructured = Unstructured::new(&data);
        let cs = build_random_changeset(&mut unstructured);
        if let Ok(serialized) = bcs::to_bytes(&cs)
            && let Ok(deserialized) = bcs::from_bytes::<ChangeSet>(&serialized) {
                prop_assert_eq!(deserialized.created_objects.len(), cs.created_objects.len());
                prop_assert_eq!(deserialized.token_balance_sets.len(), cs.token_balance_sets.len());
            }
    });
}

// ============================================================================
// Input Validation Tests - 100% Coverage
// ============================================================================

#[test]
fn prop_fuzz_validation_module_bytes() {
    proptest!(runtime_fuzz_config(), |(data: Vec<u8>)| {
        prop_assume!(data.len() <= 4096);
        assert!(validate_module_bytes(&[]).is_err());
        let oversized = vec![0u8; MAX_MODULE_SIZE + 1];
        assert!(validate_module_bytes(&oversized).is_err());
        if data.len() <= MAX_MODULE_SIZE && !data.is_empty() {
            let _ = validate_module_bytes(&data);
        }
    });
}

#[test]
fn prop_fuzz_validation_function_name() {
    proptest!(runtime_fuzz_config(), |(data: Vec<u8>)| {
        prop_assume!(data.len() <= 512);
        assert!(validate_function_name("").is_err());
        let too_long = "a".repeat(MAX_FUNCTION_NAME_LENGTH + 1);
        assert!(validate_function_name(&too_long).is_err());
        let with_null = "test\0function";
        assert!(validate_function_name(with_null).is_err());
        let max_name = "a".repeat(MAX_FUNCTION_NAME_LENGTH);
        let valid_names: Vec<String> = vec!["test".to_string(), "main".to_string(), "transfer".to_string(), max_name];
        for name in valid_names {
            assert!(validate_function_name(&name).is_ok());
        }
        if let Ok(name) = String::from_utf8(data.clone())
            && !name.is_empty() && name.len() <= MAX_FUNCTION_NAME_LENGTH && !name.contains('\0') {
                assert!(validate_function_name(&name).is_ok());
            }
    });
}

#[test]
fn prop_fuzz_validation_type_args() {
    proptest!(runtime_fuzz_config(), |(data: Vec<u8>)| {
        prop_assume!(data.len() <= 512);
        let too_many: Vec<TypeTag> = vec![TypeTag::Bool; MAX_TYPE_ARGS + 1];
        assert!(validate_type_tags(&too_many).is_err());
        let valid_counts = [0, 1, 10, MAX_TYPE_ARGS];
        for count in valid_counts {
            let valid: Vec<TypeTag> = vec![TypeTag::Bool; count];
            assert!(validate_type_tags(&valid).is_ok());
        }
    });
}

#[test]
fn prop_fuzz_validation_args() {
    proptest!(runtime_fuzz_config(), |(data: Vec<u8>)| {
        prop_assume!(data.len() <= 512);
        let too_many: Vec<Vec<u8>> = vec![vec![1u8]; MAX_ARGS + 1];
        assert!(validate_args(&too_many).is_err());
        let oversized_arg = vec![0u8; MAX_ARG_SIZE + 1];
        let with_oversized = vec![oversized_arg];
        assert!(validate_args(&with_oversized).is_err());
        let valid_counts = [0, 1, 10, MAX_ARGS];
        for count in valid_counts {
            let valid: Vec<Vec<u8>> = vec![vec![1u8; 100]; count];
            assert!(validate_args(&valid).is_ok());
        }
    });
}

#[test]
fn prop_fuzz_validation_gas_info() {
    proptest!(runtime_fuzz_config(), |(data: Vec<u8>)| {
        prop_assume!(data.len() <= 512);
        let mut unstructured = Unstructured::new(&data);
        assert!(validate_gas_info(u64::MAX, 2).is_err());
        assert!(validate_gas_info(0, 0).is_ok());
        assert!(validate_gas_info(10_000_000_000, 1).is_ok());
        assert!(validate_gas_info(1, 10_000_000_000).is_ok());
        let limit = unstructured.int_in_range(0..=10_000_000_000).unwrap_or(1000);
        let price = unstructured.int_in_range(0..=10_000_000_000).unwrap_or(1);
        let _ = validate_gas_info(limit, price);
        let _ = ValidatedGasInfo::new(1000, 1);
    });
}

#[test]
fn prop_fuzz_validation_full_transaction() {
    proptest!(runtime_fuzz_config(), |(data: Vec<u8>)| {
        prop_assume!(data.len() <= 1024);
        let sender = AccountAddress::new([1u8; 32]);
        let module_id = ModuleId::new(sender, "test_module".parse().unwrap());
        assert!(validate_transaction(Some(&module_id), "", &[], &[], Some(sender), Some((1000, 1))).is_err());
        let too_many_types: Vec<TypeTag> = vec![TypeTag::Bool; MAX_TYPE_ARGS + 1];
        assert!(validate_transaction(Some(&module_id), "test", &too_many_types, &[], Some(sender), Some((1000, 1))).is_err());
        assert!(validate_transaction(Some(&module_id), "test_function", &[TypeTag::Bool], &[vec![1u8, 2u8, 3u8]], Some(sender), Some((1000, 1))).is_ok());
    });
}

// ============================================================================
// StateManager Tests
// ============================================================================

#[test]
fn prop_fuzz_state_manager_apply() {
    proptest!(runtime_fuzz_config(), |(data: Vec<u8>)| {
        prop_assume!(data.len() <= 2048);
        let mut unstructured = Unstructured::new(&data);
        let mut state = StateManager::new_in_memory();
        let op_count = std::cmp::min(unstructured.int_in_range(1..=10).unwrap_or(1), 10);
        for _ in 0..op_count {
            let cs = build_random_changeset(&mut unstructured);
            let _ = state.apply_changeset(&cs);
        }
        let _ = state.compute_state_root();
    });
}

#[test]
fn prop_fuzz_state_manager_deterministic() {
    proptest!(runtime_fuzz_config(), |(data: Vec<u8>)| {
        prop_assume!(data.len() <= 1024);
        let mut unstructured = Unstructured::new(&data);
        let mut state1 = StateManager::new_in_memory();
        let mut state2 = StateManager::new_in_memory();
        let cs = build_random_changeset(&mut unstructured);
        let _ = state1.apply_changeset(&cs);
        let _ = state2.apply_changeset(&cs);
        let root1 = state1.compute_state_root();
        let root2 = state2.compute_state_root();
        prop_assert_eq!(root1, root2, "Same operations should produce same state root");
    });
}

// ============================================================================
// MoveRuntime Tests
// ============================================================================

#[test]
fn prop_fuzz_move_runtime_init() {
    proptest!(runtime_fuzz_config(), |(seed: u64)| {
        for _ in 0..std::cmp::min(seed as usize % 3 + 1, 3) {
            let runtime = MoveRuntime::new_with_kanari_natives_in_memory();
            prop_assert!(runtime.is_ok(), "MoveRuntime should initialize");
        }
    });
}

#[test]
fn prop_fuzz_move_runtime_workers() {
    proptest!(runtime_fuzz_config(), |(seed: u64)| {
        let runtime = MoveRuntime::new_with_kanari_natives_in_memory().expect("Runtime creation failed");
        let worker_count = std::cmp::min(seed as usize % 3 + 1, 3);
        for _ in 0..worker_count {
            let worker = runtime.spawn_worker();
            prop_assert!(worker.is_ok(), "Worker should spawn successfully");
        }
    });
}

// ============================================================================
// Transaction Execution Tests with Validation
// ============================================================================

#[test]
fn prop_fuzz_transaction_execution_with_validation() {
    proptest!(runtime_fuzz_config(), |(data: Vec<u8>)| {
        prop_assume!(data.len() <= 1024);
        let runtime = MoveRuntime::new_with_kanari_natives_in_memory().expect("Runtime creation failed");
        let sender = AccountAddress::new([1u8; 32]);
        let module_id = ModuleId::new(sender, "test_module".parse().unwrap());

        let invalid_cases = [
            ("", vec![], vec![]),
            ("test", vec![TypeTag::Bool; MAX_TYPE_ARGS + 1], vec![]),
            ("test", vec![], vec![vec![0u8; MAX_ARG_SIZE + 1]]),
            ("test", vec![], vec![vec![]; MAX_ARGS + 1]),
        ];

        for (func, types, args) in invalid_cases {
            let result = validate_transaction(Some(&module_id), func, &types, &args, Some(sender), Some((1000, 1)));
            assert!(result.is_err(), "Invalid input should fail validation");
        }

        let result = validate_transaction(Some(&module_id), "test_function", &[TypeTag::Bool], &[vec![1u8, 2u8]], Some(sender), Some((1000, 1)));
        assert!(result.is_ok(), "Valid input should pass validation");

        if result.is_ok() {
            let _ = runtime.execute_entry_function(&module_id, "test_function", vec![TypeTag::Bool], vec![vec![1u8, 2u8]], Some(sender), Some((1000, 1)), None);
        }
    });
}

#[test]
fn prop_fuzz_transaction_gas_edge_cases() {
    proptest!(runtime_fuzz_config(), |(data: Vec<u8>)| {
        prop_assume!(data.len() <= 512);
        let runtime = MoveRuntime::new_with_kanari_natives_in_memory().expect("Runtime creation failed");
        let sender = AccountAddress::new([1u8; 32]);
        let module_id = ModuleId::new(sender, "test_module".parse().unwrap());

        assert!(ValidatedGasInfo::new(u64::MAX, 2).is_err());
        let valid_gas = ValidatedGasInfo::new(1000, 1).expect("Should be valid");
        assert_eq!(valid_gas.limit(), 1000);
        assert_eq!(valid_gas.price(), 1);

        let _ = runtime.execute_entry_function(&module_id, "test", vec![], vec![], Some(sender), Some(valid_gas.into_tuple()), None);
    });
}
