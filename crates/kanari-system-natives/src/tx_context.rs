// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::make_native;
use move_core_types::account_address::AccountAddress;
use move_vm_runtime::native_functions::NativeContext;
use move_vm_runtime::native_functions::make_table_from_iter;
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::{NativeResult, PartialVMError, PartialVMResult};
use move_vm_types::pop_arg;
use move_vm_types::values::Value;
use sha3::{Digest, Sha3_256};
use smallvec::smallvec;
use std::collections::VecDeque;

pub fn all_natives(
    move_addr: AccountAddress,
) -> move_vm_runtime::native_functions::NativeFunctionTable {
    let natives = vec![("tx_context", "derive_id", make_native(native_derive_id))];
    make_table_from_iter(move_addr, natives)
}

fn native_derive_id(
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(arguments.len() == 2);

    let ids_created = pop_arg!(arguments, u64);
    let tx_hash = pop_arg!(arguments, Vec<u8>);

    // Hash(tx_hash || ids_created)
    let mut hasher = Sha3_256::new();
    hasher.update(&tx_hash);
    hasher.update(&ids_created.to_le_bytes());
    let hash = hasher.finalize();

    // Convert hash to address (take first 32 bytes)
    // AccountAddress::LENGTH is 32 (typically).
    // We safeguard against length mismatch if AccountAddress length changes.
    let addr_bytes = if hash.len() >= AccountAddress::LENGTH {
        &hash[..AccountAddress::LENGTH]
    } else {
        // Should not happen with Sha3_256 (32 bytes) and AccountAddress (32 bytes or less)
        return Err(PartialVMError::new(
            move_core_types::vm_status::StatusCode::INTERNAL_TYPE_ERROR,
        )
        .with_message("Hash length insufficient for address".to_string()));
    };

    let addr = AccountAddress::from_bytes(addr_bytes).map_err(|e| {
        PartialVMError::new(move_core_types::vm_status::StatusCode::INTERNAL_TYPE_ERROR)
            .with_message(format!("Failed to create address from hash: {}", e))
    })?;

    // Charge a small amount of gas for the hashing
    // In a real system, this should be proportional to input size (tx_hash is 32 bytes usually)
    let cost = move_core_types::gas_algebra::GasQuantity::new(1000);

    Ok(NativeResult::ok(cost, smallvec![Value::address(addr)]))
}
