// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::make_module_natives;
use move_vm_runtime::native_functions::NativeFunction;
use std::sync::Arc;

pub mod math;

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub sqrt_u128: math::SqrtU128GasParameters,
    pub sqrt_u64: math::SqrtU64GasParameters,
    pub sqrt_u256: math::SqrtU256GasParameters,
    pub pow_u64: math::PowU64GasParameters,
    pub pow_u128: math::PowU128GasParameters,
    pub pow_u256: math::PowU256GasParameters,
    pub try_pow_u64: math::TryPowU64GasParameters,
    pub try_pow_u128: math::TryPowU128GasParameters,
    pub try_pow_u256: math::TryPowU256GasParameters,
    pub mul_div_u128: math::MulDivU128GasParameters,
    pub mul_div_round_u128: math::MulDivRoundU128GasParameters,
    pub try_mul_div_u128: math::TryMulDivU128GasParameters,
    pub mul_add_u128: math::MulAddU128GasParameters,
    pub mul_div_u256: math::MulDivU256GasParameters,
    pub mul_div_round_u256: math::MulDivRoundU256GasParameters,
    pub try_mul_div_u256: math::TryMulDivU256GasParameters,
    pub mul_add_u256: math::MulAddU256GasParameters,
    pub min_u128: math::MinU128GasParameters,
    pub max_u128: math::MaxU128GasParameters,
    pub min_u256: math::MinU256GasParameters,
    pub max_u256: math::MaxU256GasParameters,
    pub average_u64: math::AverageU64GasParameters,
    pub average_u128: math::AverageU128GasParameters,
    pub average_u256: math::AverageU256GasParameters,
    pub clamp_u64: math::ClampU64GasParameters,
    pub clamp_u128: math::ClampU128GasParameters,
    pub clamp_u256: math::ClampU256GasParameters,
    pub log2_u64: math::Log2U64GasParameters,
    pub log2_u128: math::Log2U128GasParameters,
    pub log2_u256: math::Log2U256GasParameters,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            sqrt_u128: math::SqrtU128GasParameters { base: 0.into() },
            sqrt_u64: math::SqrtU64GasParameters { base: 0.into() },
            sqrt_u256: math::SqrtU256GasParameters { base: 0.into() },
            pow_u64: math::PowU64GasParameters {
                base: 0.into(),
                per_exponent: 0.into(),
            },
            pow_u128: math::PowU128GasParameters {
                base: 0.into(),
                per_exponent: 0.into(),
            },
            pow_u256: math::PowU256GasParameters {
                base: 0.into(),
                per_exponent: 0.into(),
            },
            try_pow_u64: math::TryPowU64GasParameters {
                base: 0.into(),
                per_exponent: 0.into(),
            },
            try_pow_u128: math::TryPowU128GasParameters {
                base: 0.into(),
                per_exponent: 0.into(),
            },
            try_pow_u256: math::TryPowU256GasParameters {
                base: 0.into(),
                per_exponent: 0.into(),
            },
            mul_div_u128: math::MulDivU128GasParameters { base: 0.into() },
            mul_div_round_u128: math::MulDivRoundU128GasParameters { base: 0.into() },
            try_mul_div_u128: math::TryMulDivU128GasParameters { base: 0.into() },
            mul_add_u128: math::MulAddU128GasParameters { base: 0.into() },
            mul_div_u256: math::MulDivU256GasParameters { base: 0.into() },
            mul_div_round_u256: math::MulDivRoundU256GasParameters { base: 0.into() },
            try_mul_div_u256: math::TryMulDivU256GasParameters { base: 0.into() },
            mul_add_u256: math::MulAddU256GasParameters { base: 0.into() },
            min_u128: math::MinU128GasParameters { base: 0.into() },
            max_u128: math::MaxU128GasParameters { base: 0.into() },
            min_u256: math::MinU256GasParameters { base: 0.into() },
            max_u256: math::MaxU256GasParameters { base: 0.into() },
            average_u64: math::AverageU64GasParameters { base: 0.into() },
            average_u128: math::AverageU128GasParameters { base: 0.into() },
            average_u256: math::AverageU256GasParameters { base: 0.into() },
            clamp_u64: math::ClampU64GasParameters { base: 0.into() },
            clamp_u128: math::ClampU128GasParameters { base: 0.into() },
            clamp_u256: math::ClampU256GasParameters { base: 0.into() },
            log2_u64: math::Log2U64GasParameters { base: 0.into() },
            log2_u128: math::Log2U128GasParameters { base: 0.into() },
            log2_u256: math::Log2U256GasParameters { base: 0.into() },
        }
    }
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let sqrt_u128_params = gas_params.sqrt_u128;
    let sqrt_u64_params = gas_params.sqrt_u64;
    let sqrt_u256_params = gas_params.sqrt_u256;
    let pow_u64_params = gas_params.pow_u64;
    let pow_u128_params = gas_params.pow_u128;
    let pow_u256_params = gas_params.pow_u256;
    let try_pow_u64_params = gas_params.try_pow_u64;
    let try_pow_u128_params = gas_params.try_pow_u128;
    let try_pow_u256_params = gas_params.try_pow_u256;
    let mul_div_u128_params = gas_params.mul_div_u128;
    let mul_div_round_u128_params = gas_params.mul_div_round_u128;
    let try_mul_div_u128_params = gas_params.try_mul_div_u128;
    let mul_add_u128_params = gas_params.mul_add_u128;
    let mul_div_u256_params = gas_params.mul_div_u256;
    let mul_div_round_u256_params = gas_params.mul_div_round_u256;
    let try_mul_div_u256_params = gas_params.try_mul_div_u256;
    let mul_add_u256_params = gas_params.mul_add_u256;
    let min_u128_params = gas_params.min_u128;
    let max_u128_params = gas_params.max_u128;
    let min_u256_params = gas_params.min_u256;
    let max_u256_params = gas_params.max_u256;
    let average_u64_params = gas_params.average_u64;
    let average_u128_params = gas_params.average_u128;
    let average_u256_params = gas_params.average_u256;
    let clamp_u64_params = gas_params.clamp_u64;
    let clamp_u128_params = gas_params.clamp_u128;
    let clamp_u256_params = gas_params.clamp_u256;
    let log2_u64_params = gas_params.log2_u64;
    let log2_u128_params = gas_params.log2_u128;
    let log2_u256_params = gas_params.log2_u256;

    let sqrt_u128: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_sqrt_u128(&sqrt_u128_params, context, ty_args, args)
    });
    let sqrt_u64: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_sqrt_u64(&sqrt_u64_params, context, ty_args, args)
    });
    let sqrt_u256: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_sqrt_u256(&sqrt_u256_params, context, ty_args, args)
    });
    let pow_u64: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_pow_u64(&pow_u64_params, context, ty_args, args)
    });
    let pow_u128: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_pow_u128(&pow_u128_params, context, ty_args, args)
    });
    let pow_u256: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_pow_u256(&pow_u256_params, context, ty_args, args)
    });
    let try_pow_u64: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_try_pow_u64(&try_pow_u64_params, context, ty_args, args)
    });
    let try_pow_u128: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_try_pow_u128(&try_pow_u128_params, context, ty_args, args)
    });
    let try_pow_u256: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_try_pow_u256(&try_pow_u256_params, context, ty_args, args)
    });
    let mul_div_u128: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_mul_div_u128(&mul_div_u128_params, context, ty_args, args)
    });
    let mul_div_round_u128: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_mul_div_round_u128(&mul_div_round_u128_params, context, ty_args, args)
    });
    let try_mul_div_u128: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_try_mul_div_u128(&try_mul_div_u128_params, context, ty_args, args)
    });
    let mul_add_u128: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_mul_add_u128(&mul_add_u128_params, context, ty_args, args)
    });
    let mul_div_u256: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_mul_div_u256(&mul_div_u256_params, context, ty_args, args)
    });
    let mul_div_round_u256: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_mul_div_round_u256(&mul_div_round_u256_params, context, ty_args, args)
    });
    let try_mul_div_u256: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_try_mul_div_u256(&try_mul_div_u256_params, context, ty_args, args)
    });
    let mul_add_u256: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_mul_add_u256(&mul_add_u256_params, context, ty_args, args)
    });
    let min_u128: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_min_u128(&min_u128_params, context, ty_args, args)
    });
    let max_u128: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_max_u128(&max_u128_params, context, ty_args, args)
    });
    let min_u256: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_min_u256(&min_u256_params, context, ty_args, args)
    });
    let max_u256: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_max_u256(&max_u256_params, context, ty_args, args)
    });
    let average_u64: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_average_u64(&average_u64_params, context, ty_args, args)
    });
    let average_u128: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_average_u128(&average_u128_params, context, ty_args, args)
    });
    let average_u256: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_average_u256(&average_u256_params, context, ty_args, args)
    });
    let clamp_u64: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_clamp_u64(&clamp_u64_params, context, ty_args, args)
    });
    let clamp_u128: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_clamp_u128(&clamp_u128_params, context, ty_args, args)
    });
    let clamp_u256: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_clamp_u256(&clamp_u256_params, context, ty_args, args)
    });
    let log2_u64: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_log2_u64(&log2_u64_params, context, ty_args, args)
    });
    let log2_u128: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_log2_u128(&log2_u128_params, context, ty_args, args)
    });
    let log2_u256: NativeFunction = Arc::new(move |context, ty_args, args| {
        math::native_log2_u256(&log2_u256_params, context, ty_args, args)
    });

    make_module_natives([
        ("sqrt_u128", sqrt_u128),
        ("sqrt_u64", sqrt_u64),
        ("sqrt_u256", sqrt_u256),
        ("pow_u64", pow_u64),
        ("pow_u128", pow_u128),
        ("pow_u256", pow_u256),
        ("try_pow_u64", try_pow_u64),
        ("try_pow_u128", try_pow_u128),
        ("try_pow_u256", try_pow_u256),
        ("mul_div_u128", mul_div_u128),
        ("mul_div_round_u128", mul_div_round_u128),
        ("try_mul_div_u128", try_mul_div_u128),
        ("mul_add_u128", mul_add_u128),
        ("mul_div_u256", mul_div_u256),
        ("mul_div_round_u256", mul_div_round_u256),
        ("try_mul_div_u256", try_mul_div_u256),
        ("mul_add_u256", mul_add_u256),
        ("min_u128", min_u128),
        ("max_u128", max_u128),
        ("min_u256", min_u256),
        ("max_u256", max_u256),
        ("average_u64", average_u64),
        ("average_u128", average_u128),
        ("average_u256", average_u256),
        ("clamp_u64", clamp_u64),
        ("clamp_u128", clamp_u128),
        ("clamp_u256", clamp_u256),
        ("log2_u64", log2_u64),
        ("log2_u128", log2_u128),
        ("log2_u256", log2_u256),
    ])
}
