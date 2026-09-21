// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::make_module_natives;
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::{
    native_charge_gas_early_exit,
    native_functions::{NativeContext, NativeFunction},
};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::NativeResult,
    pop_arg,
    values::{Value, values_impl::SignerRef},
};
use smallvec::smallvec;
use std::{collections::VecDeque, sync::Arc};

/***************************************************************************************************
 * native fun borrow_address
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct BorrowAddressGasParameters {
    pub base: InternalGas,
}

#[inline]
fn native_borrow_address(
    gas_params: &BorrowAddressGasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    native_charge_gas_early_exit!(context, gas_params.base);
    let signer_reference = pop_arg!(arguments, SignerRef);

    Ok(NativeResult::ok(
        context.gas_used(),
        smallvec![signer_reference.borrow_signer()?],
    ))
}

pub fn make_native_borrow_address(gas_params: BorrowAddressGasParameters) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_borrow_address(&gas_params, context, ty_args, args)
        },
    )
}

/***************************************************************************************************
 * native fun address_to_bytes
 *
 *   gas cost: base_cost
 *
 * Returns the 32-byte big-endian representation of an address
 * (same encoding as `to_u256` address conversions elsewhere).
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct AddressToBytesGasParameters {
    pub base: InternalGas,
}

#[inline]
fn native_address_to_bytes(
    gas_params: &AddressToBytesGasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_core_types::account_address::AccountAddress;

    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    native_charge_gas_early_exit!(context, gas_params.base);
    let addr = pop_arg!(arguments, AccountAddress);

    Ok(NativeResult::ok(
        context.gas_used(),
        smallvec![Value::vector_u8(addr.to_vec())],
    ))
}

pub fn make_native_address_to_bytes(gas_params: AddressToBytesGasParameters) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_address_to_bytes(&gas_params, context, ty_args, args)
        },
    )
}

/***************************************************************************************************
 * native fun address_to_u64
 *
 *   gas cost: base_cost
 *
 * Returns the value of the low 8 bytes (big-endian) of an address.
 * Distinct low bytes give distinct results; high bytes are ignored.
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct AddressToU64GasParameters {
    pub base: InternalGas,
}

#[inline]
fn native_address_to_u64(
    gas_params: &AddressToU64GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_core_types::account_address::AccountAddress;

    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    native_charge_gas_early_exit!(context, gas_params.base);
    let addr = pop_arg!(arguments, AccountAddress);
    let bytes = addr.to_vec();
    let mut low = [0u8; 8];
    low.copy_from_slice(&bytes[24..32]);
    let value = u64::from_be_bytes(low);

    Ok(NativeResult::ok(
        context.gas_used(),
        smallvec![Value::u64(value)],
    ))
}

pub fn make_native_address_to_u64(gas_params: AddressToU64GasParameters) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_address_to_u64(&gas_params, context, ty_args, args)
        },
    )
}

/***************************************************************************************************
 * module
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub borrow_address: BorrowAddressGasParameters,
    pub address_to_bytes: AddressToBytesGasParameters,
    pub address_to_u64: AddressToU64GasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        (
            "borrow_address",
            make_native_borrow_address(gas_params.borrow_address),
        ),
        (
            "address_to_bytes",
            make_native_address_to_bytes(gas_params.address_to_bytes),
        ),
        (
            "address_to_u64",
            make_native_address_to_u64(gas_params.address_to_u64),
        ),
    ];

    make_module_natives(natives)
}
