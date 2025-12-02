// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::make_module_natives;
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::{InternalGas, InternalGasPerByte, NumBytes};
use move_vm_runtime::{
    native_charge_gas_early_exit,
    native_functions::{NativeContext, NativeFunction},
};
use move_vm_types::{
    loaded_data::runtime_types::Type, natives::function::NativeResult, pop_arg, values::{Reference, Value},
};
use sha2::Sha256;
use sha3::{Sha3_256, Keccak256};
use smallvec::smallvec;
use std::{collections::VecDeque, sync::Arc};
use blake3;
use blake2b_simd::Params as Blake2bParams;
use ripemd160::Ripemd160;
use sha2::Digest;

/***************************************************************************************************
 * native fun sha2_256
 *
 *   gas cost: base_cost + unit_cost * max(input_length_in_bytes, legacy_min_input_len)
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Sha2_256GasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
    pub legacy_min_input_len: NumBytes,
}

#[inline]
fn native_sha2_256(
    gas_params: &Sha2_256GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let hash_arg = pop_arg!(arguments, Vec<u8>);

    let cost = gas_params.base
        + gas_params.per_byte
            * std::cmp::max(
                NumBytes::new(hash_arg.len() as u64),
                gas_params.legacy_min_input_len,
            );
    // Charge before doing work
    native_charge_gas_early_exit!(context, cost);

    let hash_vec = Sha256::digest(hash_arg.as_slice()).to_vec();
    Ok(NativeResult::ok(
        context.gas_used(),
        smallvec![Value::vector_u8(hash_vec)],
    ))
}

pub fn make_native_sha2_256(gas_params: Sha2_256GasParameters) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_sha2_256(&gas_params, context, ty_args, args)
        },
    )
}

/***************************************************************************************************
 * native fun sha3_256
 *
 *   gas cost: base_cost + unit_cost * max(input_length_in_bytes, legacy_min_input_len)
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Sha3_256GasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
    pub legacy_min_input_len: NumBytes,
}

#[derive(Debug, Clone)]
pub struct KeccakLikeGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
    pub legacy_min_input_len: NumBytes,
}

#[derive(Debug, Clone)]
pub struct Ripemd160GasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
    pub legacy_min_input_len: NumBytes,
}

#[inline]
fn native_sha3_256(
    gas_params: &Sha3_256GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let hash_arg = pop_arg!(arguments, Vec<u8>);

    let cost = gas_params.base
        + gas_params.per_byte
            * std::cmp::max(
                NumBytes::new(hash_arg.len() as u64),
                gas_params.legacy_min_input_len,
            );
    // Charge before doing work
    native_charge_gas_early_exit!(context, cost);

    let hash_vec = Sha3_256::digest(hash_arg.as_slice()).to_vec();
    Ok(NativeResult::ok(
        context.gas_used(),
        smallvec![Value::vector_u8(hash_vec)],
    ))
}

pub fn make_native_sha3_256(gas_params: Sha3_256GasParameters) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_sha3_256(&gas_params, context, ty_args, args)
        },
    )
}

// keccak256 native
#[inline]
fn native_keccak256(
    gas_params: &KeccakLikeGasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let arg_ref = pop_arg!(arguments, Reference);
    let hash_val = arg_ref.read_ref()?;
    let hash_arg = hash_val.value_as::<Vec<u8>>()?;

    let cost = gas_params.base
        + gas_params.per_byte * std::cmp::max(NumBytes::new(hash_arg.len() as u64), gas_params.legacy_min_input_len);
    native_charge_gas_early_exit!(context, cost);

    let hash_vec = Keccak256::digest(hash_arg.as_slice()).to_vec();
    Ok(NativeResult::ok(
        context.gas_used(),
        smallvec![Value::vector_u8(hash_vec)],
    ))
}

pub fn make_native_keccak256(gas_params: KeccakLikeGasParameters) -> NativeFunction {
    Arc::new(move |context, ty_args, args| -> PartialVMResult<NativeResult> {
        native_keccak256(&gas_params, context, ty_args, args)
    })
}

// blake2b-256 native
#[inline]
fn native_blake2b256(
    gas_params: &KeccakLikeGasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let arg_ref = pop_arg!(arguments, Reference);
    let hash_val = arg_ref.read_ref()?;
    let hash_arg = hash_val.value_as::<Vec<u8>>()?;

    let cost = gas_params.base
        + gas_params.per_byte * std::cmp::max(NumBytes::new(hash_arg.len() as u64), gas_params.legacy_min_input_len);
    native_charge_gas_early_exit!(context, cost);

    let hash = Blake2bParams::new().hash_length(32).hash(&hash_arg);
    Ok(NativeResult::ok(
        context.gas_used(),
        smallvec![Value::vector_u8(hash.as_bytes().to_vec())],
    ))
}

pub fn make_native_blake2b256(gas_params: KeccakLikeGasParameters) -> NativeFunction {
    Arc::new(move |context, ty_args, args| -> PartialVMResult<NativeResult> {
        native_blake2b256(&gas_params, context, ty_args, args)
    })
}

// blake3 native
#[inline]
fn native_blake3_256(
    gas_params: &KeccakLikeGasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let arg_ref = pop_arg!(arguments, Reference);
    let hash_val = arg_ref.read_ref()?;
    let hash_arg = hash_val.value_as::<Vec<u8>>()?;

    let cost = gas_params.base
        + gas_params.per_byte * std::cmp::max(NumBytes::new(hash_arg.len() as u64), gas_params.legacy_min_input_len);
    native_charge_gas_early_exit!(context, cost);

    let hash = blake3::hash(&hash_arg);
    Ok(NativeResult::ok(
        context.gas_used(),
        smallvec![Value::vector_u8(hash.as_bytes().to_vec())],
    ))
}

pub fn make_native_blake3_256(gas_params: KeccakLikeGasParameters) -> NativeFunction {
    Arc::new(move |context, ty_args, args| -> PartialVMResult<NativeResult> {
        native_blake3_256(&gas_params, context, ty_args, args)
    })
}

// ripemd160 native
#[inline]
fn native_ripemd160(
    gas_params: &Ripemd160GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let arg_ref = pop_arg!(arguments, Reference);
    let hash_val = arg_ref.read_ref()?;
    let hash_arg = hash_val.value_as::<Vec<u8>>()?;

    let cost = gas_params.base
        + gas_params.per_byte * std::cmp::max(NumBytes::new(hash_arg.len() as u64), gas_params.legacy_min_input_len);
    native_charge_gas_early_exit!(context, cost);

    let mut hasher = Ripemd160::new();
    hasher.update(&hash_arg);
    let result = hasher.finalize();
    Ok(NativeResult::ok(
        context.gas_used(),
        smallvec![Value::vector_u8(result.to_vec())],
    ))
}

pub fn make_native_ripemd160(gas_params: Ripemd160GasParameters) -> NativeFunction {
    Arc::new(move |context, ty_args, args| -> PartialVMResult<NativeResult> {
        native_ripemd160(&gas_params, context, ty_args, args)
    })
}

/***************************************************************************************************
 * module
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub sha2_256: Sha2_256GasParameters,
    pub sha3_256: Sha3_256GasParameters,
    pub keccak256: KeccakLikeGasParameters,
    pub blake2b256: KeccakLikeGasParameters,
    pub blake3_256: KeccakLikeGasParameters,
    pub ripemd160: Ripemd160GasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        ("sha2_256", make_native_sha2_256(gas_params.sha2_256)),
        ("sha3_256", make_native_sha3_256(gas_params.sha3_256)),
        ("keccak256", make_native_keccak256(gas_params.keccak256)),
        ("blake2b256", make_native_blake2b256(gas_params.blake2b256)),
        ("blake3_256", make_native_blake3_256(gas_params.blake3_256)),
        ("ripemd160", make_native_ripemd160(gas_params.ripemd160)),
    ];

    make_module_natives(natives)
}
