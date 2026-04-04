// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::make_module_natives;
use move_core_types::account_address::AccountAddress;
use move_vm_runtime::native_functions::{
    NativeFunction, NativeFunctionTable, make_table_from_iter,
};
use std::sync::Arc;

pub mod math;

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub sqrt_u128: math::SqrtU128GasParameters,
    pub sqrt_u64: math::SqrtU64GasParameters,
    pub pow_u64: math::PowU64GasParameters,
    pub mul_div_u128: math::MulDivU128GasParameters,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            sqrt_u128: math::SqrtU128GasParameters { base: 0.into() },
            sqrt_u64: math::SqrtU64GasParameters { base: 0.into() },
            pow_u64: math::PowU64GasParameters { base: 0.into() },
            mul_div_u128: math::MulDivU128GasParameters { base: 0.into() },
        }
    }
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let sqrt_u128_params = gas_params.sqrt_u128;
    let sqrt_u64_params = gas_params.sqrt_u64;
    let pow_u64_params = gas_params.pow_u64;
    let mul_div_u128_params = gas_params.mul_div_u128;

    let sqrt_u128: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_sqrt_u128(&sqrt_u128_params, context, ty_args, args)
    });
    let sqrt_u64: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_sqrt_u64(&sqrt_u64_params, context, ty_args, args)
    });
    let pow_u64: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_pow_u64(&pow_u64_params, context, ty_args, args)
    });
    let mul_div_u128: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_mul_div_u128(&mul_div_u128_params, context, ty_args, args)
    });

    make_module_natives([
        ("sqrt_u128", sqrt_u128),
        ("sqrt_u64", sqrt_u64),
        ("pow_u64", pow_u64),
        ("mul_div_u128", mul_div_u128),
    ])
}

pub fn all_natives(core_addr: AccountAddress) -> NativeFunctionTable {
    make_table_from_iter(
        core_addr,
        make_all(GasParameters::zeros())
            .map(|(func_name, func)| ("math".to_string(), func_name, func)),
    )
}
