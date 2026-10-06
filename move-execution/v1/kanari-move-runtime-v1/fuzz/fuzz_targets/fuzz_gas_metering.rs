// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use arbitrary::Unstructured;
use kanari_move_runtime_v1::move_runtime::MoveRuntime;
use kanari_move_runtime_v1::validation::validate_gas_info;
use kanari_types::gas::GasOperation;
use libfuzzer_sys::fuzz_target;
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::{ModuleId, TypeTag};

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);

    // 1. Gas parameter validation must handle edge values without panicking.
    let gas_limit = unstructured.int_in_range(0..=u64::MAX).unwrap_or(0);
    let gas_price = unstructured.int_in_range(0..=u64::MAX).unwrap_or(1);
    let _ = validate_gas_info(gas_limit, gas_price);
    let _ = validate_gas_info(0, 0);
    let _ = validate_gas_info(u64::MAX, u64::MAX);
    let _ = validate_gas_info(100, 1000);

    // 2. Gas accounting for every operation kind. `module_size` stays inside the
    //    range the bytecode verifier accepts (64 KB), so the arithmetic below only
    //    ever sees inputs the protocol can produce.
    const MAX_FUZZ_MODULE_SIZE: u64 = 65_535;
    let module_size = unstructured
        .int_in_range(0..=MAX_FUZZ_MODULE_SIZE)
        .unwrap_or(0) as usize;
    let complexity = unstructured.int_in_range(0..=u32::MAX as u64).unwrap_or(0) as u32;
    let operations = [
        GasOperation::Transfer,
        GasOperation::PublishModule { module_size },
        GasOperation::ExecuteFunction { complexity },
        GasOperation::CreateAccount,
        GasOperation::UpdateAccount,
    ];
    for operation in &operations {
        let units = operation.gas_units();
        assert!(units > 0, "every operation must cost gas");
    }

    // 3. Executing an entry function must stay within gas parameters, whatever
    //    the fuzzer picks (including limit < price and zero budgets).
    if let Ok(runtime) = MoveRuntime::new_with_kanari_natives_in_memory() {
        let mut sender_bytes = [0u8; 32];
        let _ = unstructured.fill_buffer(&mut sender_bytes);
        let sender = AccountAddress::new(sender_bytes);
        let module_id = ModuleId::new(
            sender,
            Identifier::new("test").expect("static module name is a valid identifier"),
        );

        let executions: u32 = unstructured.int_in_range(1..=4).unwrap_or(1);
        for _ in 0..executions {
            let budget = unstructured.int_in_range(0..=u64::MAX).unwrap_or(1000);
            let price = unstructured.int_in_range(0..=u64::MAX).unwrap_or(1);
            let _ = runtime.execute_entry_function(
                &module_id,
                "test",
                Vec::<TypeTag>::new(),
                Vec::<Vec<u8>>::new(),
                Some(sender),
                Some((budget, price)),
                None,
            );
        }
    }
});
