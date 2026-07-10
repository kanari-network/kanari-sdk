// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Address conversion native functions
//!
//! Provides native implementations for:
//! - `address::to_u256`: Convert address to u256 (big-endian)
//! - `address::from_u256`: Convert u256 to address (big-endian)
//! - `address::from_bytes`: Create address from bytes

use move_core_types::{account_address::AccountAddress, gas_algebra::InternalGas, u256::U256};
use move_vm_runtime::native_charge_gas_early_exit;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::natives::function::{NativeResult, PartialVMError};
use move_vm_types::{pop_arg, values::Value};
use smallvec::smallvec;

use crate::crypto::make_native;
use crate::helpers::expect_native_signature;

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub to_u256: InternalGas,
    pub from_u256: InternalGas,
    pub from_bytes: InternalGas,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            to_u256: InternalGas::new(0),
            from_u256: InternalGas::new(0),
            from_bytes: InternalGas::new(0),
        }
    }
}

/// Convert big-endian bytes to U256 by reversing to little-endian first
fn from_be_bytes(bytes: &[u8; 32]) -> U256 {
    let mut le = *bytes;
    le.reverse();
    U256::from_le_bytes(&le)
}

/// Convert U256 to big-endian bytes by reversing from little-endian
fn to_be_bytes(val: U256) -> [u8; 32] {
    let mut le = val.to_le_bytes();
    le.reverse();
    le
}

/// Convert an address to u256 by interpreting its 32 bytes as a big-endian integer
pub fn make_to_u256_native(gas_cost: InternalGas) -> NativeFunction {
    make_native(move |context, ty_args, mut args| {
        native_charge_gas_early_exit!(context, gas_cost);
        expect_native_signature(args.len(), 1, ty_args.len(), 0)?;

        let addr: AccountAddress = pop_arg!(args, AccountAddress);
        let addr_bytes = addr.to_vec();

        // Interpret the 32 bytes as a big-endian integer
        let mut be_bytes = [0u8; 32];
        be_bytes.copy_from_slice(&addr_bytes);
        let value = from_be_bytes(&be_bytes);

        Ok(NativeResult::ok(
            context.gas_used(),
            smallvec![Value::u256(value)],
        ))
    })
}

/// Convert a u256 value to an address (big-endian encoding).
/// Aborts if the value exceeds the maximum address (2^256 - 1).
pub fn make_from_u256_native(gas_cost: InternalGas) -> NativeFunction {
    make_native(move |context, ty_args, mut args| {
        native_charge_gas_early_exit!(context, gas_cost);
        expect_native_signature(args.len(), 1, ty_args.len(), 0)?;

        let val: U256 = pop_arg!(args, U256);
        let be_bytes = to_be_bytes(val);
        let addr = AccountAddress::new(be_bytes);

        Ok(NativeResult::ok(
            context.gas_used(),
            smallvec![Value::address(addr)],
        ))
    })
}

/// Create an address from a byte vector.
/// Aborts if the length of the byte vector is not exactly 32.
pub fn make_from_bytes_native(gas_cost: InternalGas) -> NativeFunction {
    make_native(move |context, ty_args, mut args| {
        native_charge_gas_early_exit!(context, gas_cost);
        expect_native_signature(args.len(), 1, ty_args.len(), 0)?;

        let bytes: Vec<u8> = pop_arg!(args, Vec<u8>);

        // Convert to AccountAddress (enforces 32-byte length)
        let addr = AccountAddress::from_bytes(bytes).map_err(|_| {
            PartialVMError::new(move_core_types::vm_status::StatusCode::ABORTED)
                .with_message("Address must be exactly 32 bytes".to_string())
        })?;

        Ok(NativeResult::ok(
            context.gas_used(),
            smallvec![Value::address(addr)],
        ))
    })
}

/// Creates the address native functions iterator
pub fn make_address_natives(
    gas_params: GasParameters,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = vec![
        (
            "to_u256".to_string(),
            make_to_u256_native(gas_params.to_u256),
        ),
        (
            "from_u256".to_string(),
            make_from_u256_native(gas_params.from_u256),
        ),
        (
            "from_bytes".to_string(),
            make_from_bytes_native(gas_params.from_bytes),
        ),
    ];

    crate::helpers::make_module_natives(natives)
}
