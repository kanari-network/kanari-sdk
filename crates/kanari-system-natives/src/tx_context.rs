// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use move_core_types::account_address::AccountAddress;
use move_core_types::gas_algebra::{InternalGas, InternalGasPerByte, NumBytes};
use move_vm_runtime::native_charge_gas_early_exit;
use move_vm_runtime::native_functions::NativeContext;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::{NativeResult, PartialVMError, PartialVMResult};
use move_vm_types::pop_arg;
use move_vm_types::values::Value;
use sha3::{Digest, Sha3_256};
use smallvec::smallvec;
use std::collections::VecDeque;
use std::sync::Arc;

use crate::helpers::make_module_natives;

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub derive_id: DeriveIdGasParameters,
}

#[derive(Debug, Clone)]
pub struct DeriveIdGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            derive_id: DeriveIdGasParameters {
                base: 0.into(),
                per_byte: 0.into(),
            },
        }
    }
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let derive_params = gas_params.derive_id;
    let derive_id: NativeFunction = Arc::new(move |context, ty_args, args| {
        native_derive_id(&derive_params, context, ty_args, args)
    });
    make_module_natives([("derive_id", derive_id)])
}

fn native_derive_id(
    gas_params: &DeriveIdGasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    debug_assert!(arguments.len() == 2);

    let ids_created = pop_arg!(arguments, u64);
    let tx_hash = pop_arg!(arguments, Vec<u8>);

    native_charge_gas_early_exit!(context, gas_params.base);
    native_charge_gas_early_exit!(
        context,
        gas_params.per_byte * NumBytes::new(tx_hash.len() as u64)
    );

    // Hash(tx_hash || ids_created)
    let mut hasher = Sha3_256::new();
    hasher.update(&tx_hash);
    hasher.update(ids_created.to_le_bytes());
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

    Ok(NR::ok(context.gas_used(), smallvec![Value::address(addr)]))
}
