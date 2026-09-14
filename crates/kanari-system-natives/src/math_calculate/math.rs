// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use move_core_types::gas_algebra::InternalGas;
use move_core_types::u256::U256 as MU256;
use move_vm_runtime::native_charge_gas_early_exit;
use move_vm_runtime::native_functions::NativeContext;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::{NativeResult, PartialVMResult},
    pop_arg,
    values::Value,
};
use num_integer::Roots;
use primitive_types::{U256 as P256, U512 as P512};
use smallvec::smallvec;
use std::collections::VecDeque;

use crate::helpers::expect_native_signature;

// =================================================================
// Error Codes (must match math.move)
// =================================================================
const E_OVERFLOW: u64 = 1;
const E_DIVIDE_BY_ZERO: u64 = 2;
const E_INVALID_ARG: u64 = 3;

// =================================================================
// Gas Parameters Structs
// =================================================================

#[derive(Debug, Clone)]
pub struct SqrtU128GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct SqrtU64GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct PowU64GasParameters {
    pub base: InternalGas,
    pub per_exponent: InternalGas,
}

#[derive(Debug, Clone)]
pub struct PowU128GasParameters {
    pub base: InternalGas,
    pub per_exponent: InternalGas,
}

#[derive(Debug, Clone)]
pub struct MulDivU128GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct MulDivRoundU128GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct MinU128GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct MaxU128GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct AverageU64GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct AverageU128GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct ClampU64GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct ClampU128GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct Log2U64GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct Log2U128GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct MulAddU128GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct TryMulDivU128GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct TryPowU64GasParameters {
    pub base: InternalGas,
    pub per_exponent: InternalGas,
}

#[derive(Debug, Clone)]
pub struct TryPowU128GasParameters {
    pub base: InternalGas,
    pub per_exponent: InternalGas,
}

// --- u256 gas parameters (U512 intermediate via primitive-types) ---

#[derive(Debug, Clone)]
pub struct SqrtU256GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct PowU256GasParameters {
    pub base: InternalGas,
    pub per_exponent: InternalGas,
}

#[derive(Debug, Clone)]
pub struct TryPowU256GasParameters {
    pub base: InternalGas,
    pub per_exponent: InternalGas,
}

#[derive(Debug, Clone)]
pub struct MulDivU256GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct MulDivRoundU256GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct TryMulDivU256GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct MulAddU256GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct MinU256GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct MaxU256GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct AverageU256GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct ClampU256GasParameters {
    pub base: InternalGas,
}

#[derive(Debug, Clone)]
pub struct Log2U256GasParameters {
    pub base: InternalGas,
}

// =================================================================
// Internal Helper Functions (pure, unit-testable)
// =================================================================

fn mul_div_u128_checked(x: u128, y: u128, z: u128) -> Result<u128, u64> {
    if z == 0 {
        return Err(E_DIVIDE_BY_ZERO);
    }
    // U256 intermediate: exact, no intermediate overflow.
    let result_256 = ethnum::U256::from(x) * ethnum::U256::from(y) / ethnum::U256::from(z);
    if result_256 > ethnum::U256::from(u128::MAX) {
        Err(E_OVERFLOW)
    } else {
        Ok(result_256.as_u128())
    }
}

fn mul_div_u128_round_checked(x: u128, y: u128, z: u128, round_up: bool) -> Result<u128, u64> {
    if z == 0 {
        return Err(E_DIVIDE_BY_ZERO);
    }
    let prod_256 = ethnum::U256::from(x) * ethnum::U256::from(y);
    let z_256 = ethnum::U256::from(z);
    let (q, r) = prod_256.div_rem(z_256);
    let result_256 = if round_up && r != ethnum::U256::ZERO {
        q + ethnum::U256::ONE
    } else {
        q
    };
    if result_256 > ethnum::U256::from(u128::MAX) {
        Err(E_OVERFLOW)
    } else {
        Ok(result_256.as_u128())
    }
}

fn mul_add_u128_checked(x: u128, y: u128, z: u128) -> Result<u128, u64> {
    let result_256 = ethnum::U256::from(x) * ethnum::U256::from(y) + ethnum::U256::from(z);
    if result_256 > ethnum::U256::from(u128::MAX) {
        Err(E_OVERFLOW)
    } else {
        Ok(result_256.as_u128())
    }
}

/// Overflow-safe average: (a & b) + ((a ^ b) >> 1). Never overflows.
fn average_u64(a: u64, b: u64) -> u64 {
    (a & b) + ((a ^ b) >> 1)
}

fn average_u128(a: u128, b: u128) -> u128 {
    (a & b) + ((a ^ b) >> 1)
}

fn clamp_u64(x: u64, lo: u64, hi: u64) -> Result<u64, u64> {
    if lo > hi {
        return Err(E_INVALID_ARG);
    }
    Ok(x.clamp(lo, hi))
}

fn clamp_u128(x: u128, lo: u128, hi: u128) -> Result<u128, u64> {
    if lo > hi {
        return Err(E_INVALID_ARG);
    }
    Ok(x.clamp(lo, hi))
}

fn log2_u64_checked(x: u64) -> Result<u64, u64> {
    if x == 0 {
        return Err(E_INVALID_ARG);
    }
    Ok(63 - x.leading_zeros() as u64)
}

fn log2_u128_checked(x: u128) -> Result<u128, u64> {
    if x == 0 {
        return Err(E_INVALID_ARG);
    }
    Ok(127 - x.leading_zeros() as u128)
}

fn pow_gas_cost(base: &InternalGas, per_exponent: &InternalGas, exponent: u64) -> InternalGas {
    // cost = base + per_exponent * exponent (saturating, cheap approximation of
    // repeated-multiply work). Avoids charging O(1) for pow(2, 2^32).
    InternalGas::new(
        u64::from(*base).saturating_add(u64::from(*per_exponent).saturating_mul(exponent)),
    )
}

// =================================================================
// u256 Helpers (U512 intermediate via primitive-types)
// =================================================================

/// Move U256 -> primitive U256 (both little-endian based).
fn m2p(x: &MU256) -> P256 {
    P256::from_little_endian(&x.to_le_bytes())
}

/// Primitive U256 -> Move U256.
fn p2m(v: P256) -> MU256 {
    MU256::from_le_bytes(&v.to_little_endian())
}

fn fit_u256(v: P512) -> Result<MU256, u64> {
    if v > P512::from(P256::MAX) {
        return Err(E_OVERFLOW);
    }
    let wide = v.to_little_endian();
    let mut narrow = [0u8; 32];
    narrow.copy_from_slice(&wide[..32]);
    Ok(MU256::from_le_bytes(&narrow))
}

fn mul_div_u256_checked(x: &MU256, y: &MU256, z: &MU256) -> Result<MU256, u64> {
    if *z == MU256::zero() {
        return Err(E_DIVIDE_BY_ZERO);
    }
    // 512-bit intermediate: exact, no intermediate overflow.
    let result = P512::from(m2p(x)) * P512::from(m2p(y)) / P512::from(m2p(z));
    fit_u256(result)
}

fn mul_div_u256_round_checked(
    x: &MU256,
    y: &MU256,
    z: &MU256,
    round_up: bool,
) -> Result<MU256, u64> {
    if *z == MU256::zero() {
        return Err(E_DIVIDE_BY_ZERO);
    }
    let prod = P512::from(m2p(x)) * P512::from(m2p(y));
    let den = P512::from(m2p(z));
    let (q, r) = prod.div_mod(den);
    // q + 1 cannot panic: saturate to overflow error instead (a panic in a
    // native would crash the node rather than abort the transaction).
    let result = if round_up && r != P512::zero() {
        match q.checked_add(P512::one()) {
            Some(v) => v,
            None => return Err(E_OVERFLOW),
        }
    } else {
        q
    };
    fit_u256(result)
}

fn mul_add_u256_checked(x: &MU256, y: &MU256, z: &MU256) -> Result<MU256, u64> {
    let result = P512::from(m2p(x)) * P512::from(m2p(y)) + P512::from(m2p(z));
    fit_u256(result)
}

fn pow_u256_checked(base: P256, exponent: u32) -> Option<P256> {
    // Exponentiation by squaring with overflow checks.
    let mut result = P256::one();
    let mut b = base;
    let mut e = exponent;
    while e > 0 {
        if e & 1 == 1 {
            result = result.checked_mul(b)?;
        }
        e >>= 1;
        if e > 0 {
            b = b.checked_mul(b)?;
        }
    }
    Some(result)
}

/// Floor integer square root via binary search (no shifts needed).
fn isqrt_u256(n: P256) -> P256 {
    let one = P256::one();
    let two = P256::from(2u64);
    if n <= one {
        return n;
    }
    // sqrt(n) <= n/2 + 1 for all n >= 2.
    let mut lo = one;
    let mut hi = n / two + one;
    while lo < hi {
        // Upper midpoint; `hi - lo >= 1` here so no overflow/underflow.
        let mid = lo + (hi - lo + one) / two;
        if mid <= n / mid {
            lo = mid;
        } else {
            // mid >= 2 whenever lo < hi, so mid - 1 cannot underflow.
            hi = mid - one;
        }
    }
    lo
}

/// Overflow-safe average: (a & b) + ((a ^ b) / 2). Never overflows.
fn average_u256(a: P256, b: P256) -> P256 {
    let two = P256::from(2u64);
    (a & b) + ((a ^ b) / two)
}

fn clamp_u256(x: P256, lo: P256, hi: P256) -> Result<P256, u64> {
    if lo > hi {
        return Err(E_INVALID_ARG);
    }
    Ok(x.clamp(lo, hi))
}

fn log2_u256_checked(x: &MU256) -> Result<P256, u64> {
    if *x == MU256::zero() {
        return Err(E_INVALID_ARG);
    }
    Ok(P256::from(m2p(x).bits() as u64 - 1))
}

// =================================================================
// Native Functions Implementations
// =================================================================

/// Calculate square root of u128 (floor)
pub fn native_sqrt_u128(
    gas_params: &SqrtU128GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 1, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let x: u128 = pop_arg!(args, u128);
    Ok(NR::ok(context.gas_used(), smallvec![Value::u128(x.sqrt())]))
}

/// Calculate square root of u64 (floor)
pub fn native_sqrt_u64(
    gas_params: &SqrtU64GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 1, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let x: u64 = pop_arg!(args, u64);
    Ok(NR::ok(context.gas_used(), smallvec![Value::u64(x.sqrt())]))
}

/// Power function for u64 (base ^ exponent). Aborts with E_OVERFLOW on overflow.
pub fn native_pow_u64(
    gas_params: &PowU64GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 2, _ty_args.len(), 0)?;
    // Pop first so we can charge proportional to work.
    let exponent: u8 = pop_arg!(args, u8);
    let base: u64 = pop_arg!(args, u64);
    native_charge_gas_early_exit!(
        context,
        pow_gas_cost(&gas_params.base, &gas_params.per_exponent, exponent as u64)
    );

    match base.checked_pow(exponent as u32) {
        Some(result) => Ok(NR::ok(context.gas_used(), smallvec![Value::u64(result)])),
        None => Ok(NR::err(context.gas_used(), E_OVERFLOW)),
    }
}

/// Power function for u128 (base ^ exponent). Aborts with E_OVERFLOW on overflow.
pub fn native_pow_u128(
    gas_params: &PowU128GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 2, _ty_args.len(), 0)?;
    let exponent: u32 = pop_arg!(args, u32);
    let base: u128 = pop_arg!(args, u128);
    native_charge_gas_early_exit!(
        context,
        pow_gas_cost(&gas_params.base, &gas_params.per_exponent, exponent as u64)
    );

    match base.checked_pow(exponent) {
        Some(result) => Ok(NR::ok(context.gas_used(), smallvec![Value::u128(result)])),
        None => Ok(NR::err(context.gas_used(), E_OVERFLOW)),
    }
}

/// Non-aborting pow for u64. Returns (success, value): (false, 0) on overflow.
pub fn native_try_pow_u64(
    gas_params: &TryPowU64GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 2, _ty_args.len(), 0)?;
    let exponent: u8 = pop_arg!(args, u8);
    let base: u64 = pop_arg!(args, u64);
    native_charge_gas_early_exit!(
        context,
        pow_gas_cost(&gas_params.base, &gas_params.per_exponent, exponent as u64)
    );

    match base.checked_pow(exponent as u32) {
        Some(v) => Ok(NR::ok(
            context.gas_used(),
            smallvec![Value::bool(true), Value::u64(v)],
        )),
        None => Ok(NR::ok(
            context.gas_used(),
            smallvec![Value::bool(false), Value::u64(0)],
        )),
    }
}

/// Non-aborting pow for u128. Returns (success, value): (false, 0) on overflow.
pub fn native_try_pow_u128(
    gas_params: &TryPowU128GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 2, _ty_args.len(), 0)?;
    let exponent: u32 = pop_arg!(args, u32);
    let base: u128 = pop_arg!(args, u128);
    native_charge_gas_early_exit!(
        context,
        pow_gas_cost(&gas_params.base, &gas_params.per_exponent, exponent as u64)
    );

    match base.checked_pow(exponent) {
        Some(v) => Ok(NR::ok(
            context.gas_used(),
            smallvec![Value::bool(true), Value::u128(v)],
        )),
        None => Ok(NR::ok(
            context.gas_used(),
            smallvec![Value::bool(false), Value::u128(0)],
        )),
    }
}

/// Calculate (x * y) / z safely for u128 (floor). Aborts on div-by-zero / overflow.
pub fn native_mul_div_u128(
    gas_params: &MulDivU128GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 3, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let z: u128 = pop_arg!(args, u128);
    let y: u128 = pop_arg!(args, u128);
    let x: u128 = pop_arg!(args, u128);

    match mul_div_u128_checked(x, y, z) {
        Ok(result) => Ok(NR::ok(context.gas_used(), smallvec![Value::u128(result)])),
        Err(code) => Ok(NR::err(context.gas_used(), code)),
    }
}

/// Calculate (x * y) / z with explicit rounding: false = floor, true = ceil.
pub fn native_mul_div_round_u128(
    gas_params: &MulDivRoundU128GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 4, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let round_up: bool = pop_arg!(args, bool);
    let z: u128 = pop_arg!(args, u128);
    let y: u128 = pop_arg!(args, u128);
    let x: u128 = pop_arg!(args, u128);

    match mul_div_u128_round_checked(x, y, z, round_up) {
        Ok(result) => Ok(NR::ok(context.gas_used(), smallvec![Value::u128(result)])),
        Err(code) => Ok(NR::err(context.gas_used(), code)),
    }
}

/// Non-aborting (x * y) / z (floor). Returns (success, value).
/// (false, 0) when z == 0 or result overflows u128.
pub fn native_try_mul_div_u128(
    gas_params: &TryMulDivU128GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 3, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let z: u128 = pop_arg!(args, u128);
    let y: u128 = pop_arg!(args, u128);
    let x: u128 = pop_arg!(args, u128);

    match mul_div_u128_checked(x, y, z) {
        Ok(v) => Ok(NR::ok(
            context.gas_used(),
            smallvec![Value::bool(true), Value::u128(v)],
        )),
        Err(_) => Ok(NR::ok(
            context.gas_used(),
            smallvec![Value::bool(false), Value::u128(0)],
        )),
    }
}

/// Calculate (x * y) + z safely for u128. Aborts with E_OVERFLOW on overflow.
pub fn native_mul_add_u128(
    gas_params: &MulAddU128GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 3, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let z: u128 = pop_arg!(args, u128);
    let y: u128 = pop_arg!(args, u128);
    let x: u128 = pop_arg!(args, u128);

    match mul_add_u128_checked(x, y, z) {
        Ok(result) => Ok(NR::ok(context.gas_used(), smallvec![Value::u128(result)])),
        Err(code) => Ok(NR::err(context.gas_used(), code)),
    }
}

/// Get minimum of two u128 values
pub fn native_min_u128(
    gas_params: &MinU128GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 2, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let b: u128 = pop_arg!(args, u128);
    let a: u128 = pop_arg!(args, u128);

    Ok(NR::ok(
        context.gas_used(),
        smallvec![Value::u128(std::cmp::min(a, b))],
    ))
}

/// Get maximum of two u128 values
pub fn native_max_u128(
    gas_params: &MaxU128GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 2, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let b: u128 = pop_arg!(args, u128);
    let a: u128 = pop_arg!(args, u128);

    Ok(NR::ok(
        context.gas_used(),
        smallvec![Value::u128(std::cmp::max(a, b))],
    ))
}

/// Overflow-safe average of two u64: (a & b) + ((a ^ b) >> 1)
pub fn native_average_u64(
    gas_params: &AverageU64GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 2, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let b: u64 = pop_arg!(args, u64);
    let a: u64 = pop_arg!(args, u64);

    Ok(NR::ok(
        context.gas_used(),
        smallvec![Value::u64(average_u64(a, b))],
    ))
}

/// Overflow-safe average of two u128
pub fn native_average_u128(
    gas_params: &AverageU128GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 2, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let b: u128 = pop_arg!(args, u128);
    let a: u128 = pop_arg!(args, u128);

    Ok(NR::ok(
        context.gas_used(),
        smallvec![Value::u128(average_u128(a, b))],
    ))
}

/// Clamp x into [lo, hi]. Aborts with E_INVALID_ARG when lo > hi.
pub fn native_clamp_u64(
    gas_params: &ClampU64GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 3, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let hi: u64 = pop_arg!(args, u64);
    let lo: u64 = pop_arg!(args, u64);
    let x: u64 = pop_arg!(args, u64);

    match clamp_u64(x, lo, hi) {
        Ok(v) => Ok(NR::ok(context.gas_used(), smallvec![Value::u64(v)])),
        Err(code) => Ok(NR::err(context.gas_used(), code)),
    }
}

/// Clamp x into [lo, hi] for u128. Aborts with E_INVALID_ARG when lo > hi.
pub fn native_clamp_u128(
    gas_params: &ClampU128GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 3, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let hi: u128 = pop_arg!(args, u128);
    let lo: u128 = pop_arg!(args, u128);
    let x: u128 = pop_arg!(args, u128);

    match clamp_u128(x, lo, hi) {
        Ok(v) => Ok(NR::ok(context.gas_used(), smallvec![Value::u128(v)])),
        Err(code) => Ok(NR::err(context.gas_used(), code)),
    }
}

/// Floor log2 of u64. Aborts with E_INVALID_ARG on 0.
pub fn native_log2_u64(
    gas_params: &Log2U64GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 1, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let x: u64 = pop_arg!(args, u64);

    match log2_u64_checked(x) {
        Ok(v) => Ok(NR::ok(context.gas_used(), smallvec![Value::u64(v)])),
        Err(code) => Ok(NR::err(context.gas_used(), code)),
    }
}

/// Floor log2 of u128. Aborts with E_INVALID_ARG on 0.
pub fn native_log2_u128(
    gas_params: &Log2U128GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 1, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let x: u128 = pop_arg!(args, u128);

    match log2_u128_checked(x) {
        Ok(v) => Ok(NR::ok(context.gas_used(), smallvec![Value::u128(v)])),
        Err(code) => Ok(NR::err(context.gas_used(), code)),
    }
}

// =================================================================
// u256 Native Functions (U512 intermediate via primitive-types)
// =================================================================

/// Floor square root of u256.
pub fn native_sqrt_u256(
    gas_params: &SqrtU256GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 1, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let x: MU256 = pop_arg!(args, MU256);
    Ok(NR::ok(
        context.gas_used(),
        smallvec![Value::u256(p2m(isqrt_u256(m2p(&x))))],
    ))
}

/// base ^ exponent for u256. Aborts with E_OVERFLOW on overflow.
pub fn native_pow_u256(
    gas_params: &PowU256GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 2, _ty_args.len(), 0)?;
    let exponent: u32 = pop_arg!(args, u32);
    let base: MU256 = pop_arg!(args, MU256);
    native_charge_gas_early_exit!(
        context,
        pow_gas_cost(&gas_params.base, &gas_params.per_exponent, exponent as u64)
    );

    match pow_u256_checked(m2p(&base), exponent) {
        Some(v) => Ok(NR::ok(context.gas_used(), smallvec![Value::u256(p2m(v))])),
        None => Ok(NR::err(context.gas_used(), E_OVERFLOW)),
    }
}

/// Non-aborting pow for u256. Returns (success, value); (false, 0) on overflow.
pub fn native_try_pow_u256(
    gas_params: &TryPowU256GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 2, _ty_args.len(), 0)?;
    let exponent: u32 = pop_arg!(args, u32);
    let base: MU256 = pop_arg!(args, MU256);
    native_charge_gas_early_exit!(
        context,
        pow_gas_cost(&gas_params.base, &gas_params.per_exponent, exponent as u64)
    );

    match pow_u256_checked(m2p(&base), exponent) {
        Some(v) => Ok(NR::ok(
            context.gas_used(),
            smallvec![Value::bool(true), Value::u256(p2m(v))],
        )),
        None => Ok(NR::ok(
            context.gas_used(),
            smallvec![Value::bool(false), Value::u256(MU256::zero())],
        )),
    }
}

/// (x * y) / z for u256 with 512-bit intermediate, floor rounding.
pub fn native_mul_div_u256(
    gas_params: &MulDivU256GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 3, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let z: MU256 = pop_arg!(args, MU256);
    let y: MU256 = pop_arg!(args, MU256);
    let x: MU256 = pop_arg!(args, MU256);

    match mul_div_u256_checked(&x, &y, &z) {
        Ok(result) => Ok(NR::ok(context.gas_used(), smallvec![Value::u256(result)])),
        Err(code) => Ok(NR::err(context.gas_used(), code)),
    }
}

/// (x * y) / z for u256 with explicit rounding: false = floor, true = ceil.
pub fn native_mul_div_round_u256(
    gas_params: &MulDivRoundU256GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 4, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let round_up: bool = pop_arg!(args, bool);
    let z: MU256 = pop_arg!(args, MU256);
    let y: MU256 = pop_arg!(args, MU256);
    let x: MU256 = pop_arg!(args, MU256);

    match mul_div_u256_round_checked(&x, &y, &z, round_up) {
        Ok(result) => Ok(NR::ok(context.gas_used(), smallvec![Value::u256(result)])),
        Err(code) => Ok(NR::err(context.gas_used(), code)),
    }
}

/// Non-aborting (x * y) / z (floor) for u256. (false, 0) on div-by-zero/overflow.
pub fn native_try_mul_div_u256(
    gas_params: &TryMulDivU256GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 3, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let z: MU256 = pop_arg!(args, MU256);
    let y: MU256 = pop_arg!(args, MU256);
    let x: MU256 = pop_arg!(args, MU256);

    match mul_div_u256_checked(&x, &y, &z) {
        Ok(v) => Ok(NR::ok(
            context.gas_used(),
            smallvec![Value::bool(true), Value::u256(v)],
        )),
        Err(_) => Ok(NR::ok(
            context.gas_used(),
            smallvec![Value::bool(false), Value::u256(MU256::zero())],
        )),
    }
}

/// (x * y) + z for u256. Aborts with E_OVERFLOW on overflow.
pub fn native_mul_add_u256(
    gas_params: &MulAddU256GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 3, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let z: MU256 = pop_arg!(args, MU256);
    let y: MU256 = pop_arg!(args, MU256);
    let x: MU256 = pop_arg!(args, MU256);

    match mul_add_u256_checked(&x, &y, &z) {
        Ok(result) => Ok(NR::ok(context.gas_used(), smallvec![Value::u256(result)])),
        Err(code) => Ok(NR::err(context.gas_used(), code)),
    }
}

/// Minimum of two u256 values.
pub fn native_min_u256(
    gas_params: &MinU256GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 2, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let b: MU256 = pop_arg!(args, MU256);
    let a: MU256 = pop_arg!(args, MU256);
    let (pa, pb) = (m2p(&a), m2p(&b));

    Ok(NR::ok(
        context.gas_used(),
        smallvec![Value::u256(p2m(std::cmp::min(pa, pb)))],
    ))
}

/// Maximum of two u256 values.
pub fn native_max_u256(
    gas_params: &MaxU256GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 2, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let b: MU256 = pop_arg!(args, MU256);
    let a: MU256 = pop_arg!(args, MU256);
    let (pa, pb) = (m2p(&a), m2p(&b));

    Ok(NR::ok(
        context.gas_used(),
        smallvec![Value::u256(p2m(std::cmp::max(pa, pb)))],
    ))
}

/// Overflow-safe average (floor) of two u256 values.
pub fn native_average_u256(
    gas_params: &AverageU256GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 2, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let b: MU256 = pop_arg!(args, MU256);
    let a: MU256 = pop_arg!(args, MU256);

    Ok(NR::ok(
        context.gas_used(),
        smallvec![Value::u256(p2m(average_u256(m2p(&a), m2p(&b))))],
    ))
}

/// Clamp x into [lo, hi] for u256. Aborts with E_INVALID_ARG when lo > hi.
pub fn native_clamp_u256(
    gas_params: &ClampU256GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 3, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let hi: MU256 = pop_arg!(args, MU256);
    let lo: MU256 = pop_arg!(args, MU256);
    let x: MU256 = pop_arg!(args, MU256);

    match clamp_u256(m2p(&x), m2p(&lo), m2p(&hi)) {
        Ok(v) => Ok(NR::ok(context.gas_used(), smallvec![Value::u256(p2m(v))])),
        Err(code) => Ok(NR::err(context.gas_used(), code)),
    }
}

/// Floor log2 of u256. Aborts with E_INVALID_ARG on 0.
pub fn native_log2_u256(
    gas_params: &Log2U256GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(args.len(), 1, _ty_args.len(), 0)?;
    native_charge_gas_early_exit!(context, gas_params.base);
    let x: MU256 = pop_arg!(args, MU256);

    match log2_u256_checked(&x) {
        Ok(v) => Ok(NR::ok(context.gas_used(), smallvec![Value::u256(p2m(v))])),
        Err(code) => Ok(NR::err(context.gas_used(), code)),
    }
}

// =================================================================
// Unit Tests
// =================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mul_div_u128_avoids_intermediate_overflow() {
        assert_eq!(mul_div_u128_checked(u128::MAX, 2, 2).unwrap(), u128::MAX);
        // (MAX * MAX) / MAX == MAX without intermediate overflow
        assert_eq!(
            mul_div_u128_checked(u128::MAX, u128::MAX, u128::MAX).unwrap(),
            u128::MAX
        );
    }

    #[test]
    fn test_mul_div_u128_basic() {
        assert_eq!(mul_div_u128_checked(10, 20, 5).unwrap(), 40);
        assert_eq!(mul_div_u128_checked(0, u128::MAX, 1).unwrap(), 0);
    }

    #[test]
    fn test_mul_div_u128_divide_by_zero() {
        assert_eq!(mul_div_u128_checked(1, 2, 0).unwrap_err(), E_DIVIDE_BY_ZERO);
    }

    #[test]
    fn test_mul_div_u128_overflow() {
        assert_eq!(
            mul_div_u128_checked(u128::MAX, u128::MAX, 1).unwrap_err(),
            E_OVERFLOW
        );
        // ceil path overflows while floor does not
        assert_eq!(mul_div_u128_checked(10, 10, 40).unwrap(), 2);
        assert_eq!(mul_div_u128_round_checked(10, 10, 40, false).unwrap(), 2);
        assert_eq!(mul_div_u128_round_checked(10, 10, 40, true).unwrap(), 3);
        assert_eq!(mul_div_u128_round_checked(10, 20, 5, true).unwrap(), 40);
        // round-up overflow: floor fits, ceil does not
        assert_eq!(
            mul_div_u128_round_checked(u128::MAX, 2, 2, false).unwrap(),
            u128::MAX
        );
    }

    #[test]
    fn test_mul_add_u128() {
        assert_eq!(mul_add_u128_checked(10, 20, 5).unwrap(), 205);
        assert_eq!(mul_add_u128_checked(u128::MAX, 1, 0).unwrap(), u128::MAX);
        assert_eq!(
            mul_add_u128_checked(u128::MAX, 1, 1).unwrap_err(),
            E_OVERFLOW
        );
        assert_eq!(
            mul_add_u128_checked(u128::MAX, 2, 0).unwrap_err(),
            E_OVERFLOW
        );
    }

    #[test]
    fn test_average_no_overflow() {
        assert_eq!(average_u64(u64::MAX, u64::MAX), u64::MAX);
        assert_eq!(average_u64(u64::MAX, 0), u64::MAX / 2);
        assert_eq!(average_u64(10, 20), 15);
        assert_eq!(average_u64(10, 11), 10); // floor
        assert_eq!(average_u128(u128::MAX, u128::MAX), u128::MAX);
        assert_eq!(average_u128(u128::MAX, 0), u128::MAX / 2);
        assert_eq!(average_u128(10, 20), 15);
    }

    #[test]
    fn test_clamp() {
        assert_eq!(clamp_u64(5, 0, 10).unwrap(), 5);
        assert_eq!(clamp_u64(15, 0, 10).unwrap(), 10);
        assert_eq!(clamp_u64(0, 1, 10).unwrap(), 1);
        assert_eq!(clamp_u64(5, 5, 5).unwrap(), 5);
        assert_eq!(clamp_u64(5, 10, 0).unwrap_err(), E_INVALID_ARG);
        assert_eq!(clamp_u128(5, 0, 10).unwrap(), 5);
        assert_eq!(clamp_u128(15, 0, 10).unwrap(), 10);
        assert_eq!(clamp_u128(5, 10, 0).unwrap_err(), E_INVALID_ARG);
    }

    #[test]
    fn test_log2() {
        assert_eq!(log2_u64_checked(1).unwrap(), 0);
        assert_eq!(log2_u64_checked(2).unwrap(), 1);
        assert_eq!(log2_u64_checked(3).unwrap(), 1);
        assert_eq!(log2_u64_checked(u64::MAX).unwrap(), 63);
        assert_eq!(log2_u64_checked(0).unwrap_err(), E_INVALID_ARG);
        assert_eq!(log2_u128_checked(1).unwrap(), 0);
        assert_eq!(log2_u128_checked(u128::MAX).unwrap(), 127);
        assert_eq!(log2_u128_checked(0).unwrap_err(), E_INVALID_ARG);
    }

    #[cfg(test)]
    mod prop_tests {
        //! Deterministic property tests (xorshift64 PRNG, fixed seeds).
        //! No external deps; every run reproduces the exact same sequence.
        //! Nested inside `mod tests`, so this imports the parent math module.
        use super::super::*;

        struct XorShift64(u64);

        impl XorShift64 {
            fn next_u64(&mut self) -> u64 {
                // Zero seed would stick at zero; force a nonzero state.
                let mut x = self.0 | 1;
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                self.0 = x;
                x
            }

            fn next_u128(&mut self) -> u128 {
                ((self.next_u64() as u128) << 64) | (self.next_u64() as u128)
            }

            fn next_p256(&mut self) -> P256 {
                let mut b = [0u8; 32];
                for i in 0..4 {
                    b[i * 8..(i + 1) * 8].copy_from_slice(&self.next_u64().to_le_bytes());
                }
                P256::from_little_endian(&b)
            }

            /// Biased toward edge values (0, 1, MAX, powers of two) interleaved
            /// with uniform randoms, so boundaries get hit often.
            fn next_p256_edge_biased(&mut self) -> P256 {
                match self.next_u64() % 8 {
                    0 => P256::zero(),
                    1 => P256::one(),
                    2 => P256::MAX,
                    3 => {
                        let s = (self.next_u64() % 256) as u32;
                        pow2_p256_test(s)
                    }
                    4 => P256::from(self.next_u64()),
                    _ => self.next_p256(),
                }
            }
        }

        /// 2^exp without shift operators (shared with unit tests).
        fn pow2_p256_test(exp: u32) -> P256 {
            assert!(exp <= 255);
            let mut v = P256::one();
            for _ in 0..exp {
                v = v + v;
            }
            v
        }

        const ITERS: usize = 5_000;

        #[test]
        fn prop_isqrt_square_back() {
            let mut rng = XorShift64(0x9E3779B97F4A7C15);
            for _ in 0..ITERS {
                let n = rng.next_p256_edge_biased();
                let s = isqrt_u256(n);
                // s^2 <= n: s <= 2^128 - 1 always, so the square fits.
                let sq = s.checked_mul(s).expect("s^2 fits in u256");
                assert!(sq <= n, "isqrt low bound failed");
                // n < (s+1)^2: an overflowing square also proves the bound
                // (it means (s+1)^2 >= 2^256 > n).
                match s
                    .checked_add(P256::one())
                    .unwrap()
                    .checked_mul(s.checked_add(P256::one()).unwrap())
                {
                    Some(hi) => assert!(n < hi, "isqrt high bound failed"),
                    None => {}
                }
            }
        }

        #[test]
        fn prop_sqrt_u256_agrees_u128() {
            use num_integer::Roots;
            let mut rng = XorShift64(0xC2B2801218687);
            for _ in 0..ITERS {
                let x = rng.next_u128();
                let got = isqrt_u256(m2p(&MU256::from(x)));
                let want = P256::from(x.sqrt());
                assert_eq!(got, want, "sqrt disagreement at x={x}");
            }
        }

        #[test]
        fn prop_mul_div_u256_agrees_u128() {
            let mut rng = XorShift64(0x27BB2EE687B0B0FD);
            for _ in 0..ITERS {
                let (x, y) = (rng.next_u128(), rng.next_u128());
                // z == 0 half the time to cover div-by-zero agreement.
                let z = if rng.next_u64() % 2 == 0 {
                    0
                } else {
                    rng.next_u128()
                };
                let got = mul_div_u256_checked(&MU256::from(x), &MU256::from(y), &MU256::from(z));
                let want = mul_div_u128_checked(x, y, z);
                match (got, want) {
                    // Both Ok implies the value fits u128, so the lossy
                    // accessor below is exact here.
                    (Ok(g), Ok(w)) => assert_eq!(g.unchecked_as_u128(), w),
                    // u128 overflow with a fitting u256 result is correct
                    // divergence: different widths, different MAX.
                    (Ok(g), Err(E_OVERFLOW)) => {
                        assert!(
                            g > MU256::from(u128::MAX),
                            "u128 overflow must exceed u128::MAX"
                        )
                    }
                    // Div-by-zero must agree on both widths.
                    (Err(gc), Err(wc)) => assert_eq!(gc, wc, "error code disagreement"),
                    (g, w) => panic!("ok/err disagreement: {g:?} vs {w:?}"),
                }
            }
        }

        #[test]
        fn prop_pow_u256_matches_naive_loop() {
            let mut rng = XorShift64(0x165667B19E3779F9);
            for _ in 0..2_000 {
                let base = P256::from(rng.next_u64() % 10_000);
                let exp = (rng.next_u64() % 24) as u32;
                let got = pow_u256_checked(base, exp);
                // Naive reference.
                let mut want = Some(P256::one());
                for _ in 0..exp {
                    want = want.and_then(|v| v.checked_mul(base));
                }
                assert_eq!(got, want, "pow disagreement at {base} ^ {exp}");
            }
        }

        #[test]
        fn prop_ceil_floor_bracket() {
            let mut rng = XorShift64(0xD1B54A32D192ED03);
            for _ in 0..ITERS {
                let (x, y) = (MU256::from(rng.next_u128()), MU256::from(rng.next_u128()));
                let z = MU256::from(rng.next_u128() | 1); // nonzero
                let f = mul_div_u256_round_checked(&x, &y, &z, false).unwrap();
                let c = mul_div_u256_round_checked(&x, &y, &z, true).unwrap();
                let (pf, pc) = (m2p(&f), m2p(&c));
                assert!(pf <= pc);
                // ceil - floor is 0 (exact) or 1.
                let d = pc.checked_sub(pf).expect("c >= f");
                assert!(d <= P256::one(), "ceil/floor gap > 1");
            }
        }

        #[test]
        fn prop_average_bounded_and_agrees_u128() {
            let mut rng = XorShift64(0x8A51E04D45043063);
            for _ in 0..ITERS {
                let (a, b) = (rng.next_p256_edge_biased(), rng.next_p256_edge_biased());
                let avg = average_u256(a, b);
                let (lo, hi) = (std::cmp::min(a, b), std::cmp::max(a, b));
                assert!(avg >= lo && avg <= hi, "average out of [min, max]");
            }
            // Differential on values that fit u128.
            for _ in 0..1_000 {
                let (a, b) = (rng.next_u128(), rng.next_u128());
                let got = average_u256(m2p(&MU256::from(a)), m2p(&MU256::from(b)));
                assert_eq!(got, P256::from(average_u128(a, b)));
            }
        }

        #[test]
        fn prop_log2_brackets_value() {
            let mut rng = XorShift64(0xB5297A4D68EFE35D);
            for _ in 0..ITERS {
                let x = rng.next_p256_edge_biased();
                if x == P256::zero() {
                    assert_eq!(
                        log2_u256_checked(&MU256::zero()).unwrap_err(),
                        E_INVALID_ARG
                    );
                    continue;
                }
                let l: u32 = {
                    let lp = log2_u256_checked(&p2m(x)).unwrap();
                    // log2 result always fits u32; convert via LE bytes.
                    let b = lp.to_little_endian();
                    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
                };
                assert!(l <= 255);
                let plo = pow2_p256_test(l);
                assert!(plo <= x, "2^log2(x) <= x failed");
                if l < 255 {
                    // 2^256 is not representable: only bracket from above
                    // when the upper power fits.
                    let phi = pow2_p256_test(l + 1);
                    assert!(x < phi, "x < 2^(log2(x)+1) failed");
                }
            }
        }

        #[test]
        fn prop_clamp_contract() {
            let mut rng = XorShift64(0x7647EDD6F9B0D21D);
            for _ in 0..ITERS {
                let (x, a, b) = (
                    rng.next_p256_edge_biased(),
                    rng.next_p256_edge_biased(),
                    rng.next_p256_edge_biased(),
                );
                let (lo, hi) = (std::cmp::min(a, b), std::cmp::max(a, b));
                let c = clamp_u256(x, lo, hi).unwrap();
                assert!(c >= lo && c <= hi);
                assert_eq!(c, x.clamp(lo, hi));
                // Inverted bounds always rejected.
                if a != b {
                    assert_eq!(clamp_u256(x, hi, lo).unwrap_err(), E_INVALID_ARG);
                }
            }
        }

        // --- u256 tests ---

        fn m(v: u128) -> MU256 {
            MU256::from(v)
        }

        fn max256() -> MU256 {
            MU256::max_value()
        }

        /// 2^exp as P256 without shift operators (exp <= 255).
        fn pow2_p256(exp: u32) -> P256 {
            let mut v = P256::one();
            for _ in 0..exp {
                v = v + v;
            }
            v
        }

        #[test]
        fn test_u256_roundtrip() {
            assert_eq!(p2m(m2p(&max256())), max256());
            assert_eq!(p2m(m2p(&m(0))), m(0));
            assert_eq!(p2m(m2p(&m(u128::MAX))), m(u128::MAX));
        }

        #[test]
        fn test_mul_div_u256_no_intermediate_overflow() {
            // (MAX * 2) / 2 == MAX and (MAX * MAX) / MAX == MAX
            assert_eq!(
                mul_div_u256_checked(&max256(), &m(2), &m(2)).unwrap(),
                max256()
            );
            assert_eq!(
                mul_div_u256_checked(&max256(), &max256(), &max256()).unwrap(),
                max256()
            );
            assert_eq!(mul_div_u256_checked(&m(10), &m(20), &m(5)).unwrap(), m(40));
            assert_eq!(
                mul_div_u256_checked(&m(1), &m(2), &MU256::zero()).unwrap_err(),
                E_DIVIDE_BY_ZERO
            );
            assert_eq!(
                mul_div_u256_checked(&max256(), &max256(), &m(1)).unwrap_err(),
                E_OVERFLOW
            );
        }

        #[test]
        fn test_mul_div_u256_round() {
            // 10 * 10 / 40 = 2.5
            assert_eq!(
                mul_div_u256_round_checked(&m(10), &m(10), &m(40), false).unwrap(),
                m(2)
            );
            assert_eq!(
                mul_div_u256_round_checked(&m(10), &m(10), &m(40), true).unwrap(),
                m(3)
            );
            assert_eq!(
                mul_div_u256_round_checked(&m(10), &m(20), &m(5), true).unwrap(),
                m(40)
            );
        }

        #[test]
        fn test_mul_add_u256() {
            assert_eq!(mul_add_u256_checked(&m(10), &m(20), &m(5)).unwrap(), m(205));
            assert_eq!(
                mul_add_u256_checked(&max256(), &m(1), &m(0)).unwrap(),
                max256()
            );
            assert_eq!(
                mul_add_u256_checked(&max256(), &m(1), &m(1)).unwrap_err(),
                E_OVERFLOW
            );
            assert_eq!(
                mul_add_u256_checked(&max256(), &m(2), &m(0)).unwrap_err(),
                E_OVERFLOW
            );
        }

        #[test]
        fn test_pow_u256() {
            assert_eq!(p2m(pow_u256_checked(m2p(&m(2)), 10).unwrap()), m(1024));
            assert_eq!(pow_u256_checked(m2p(&m(5)), 0).unwrap(), P256::one());
            assert!(pow_u256_checked(m2p(&max256()), 2).is_none());
            // 2^256 overflows u256, 2^255 does not
            assert!(pow_u256_checked(P256::from(2u64), 256).is_none());
            assert_eq!(
                p2m(pow_u256_checked(P256::from(2u64), 255).unwrap()),
                p2m(pow2_p256(255))
            );
        }

        #[test]
        fn test_isqrt_u256() {
            assert_eq!(isqrt_u256(P256::zero()), P256::zero());
            assert_eq!(isqrt_u256(P256::one()), P256::one());
            assert_eq!(isqrt_u256(P256::from(15u64)), P256::from(3u64));
            assert_eq!(isqrt_u256(P256::from(16u64)), P256::from(4u64));
            assert_eq!(isqrt_u256(m2p(&m(u128::MAX))), m2p(&m(u64::MAX as u128)));
            // sqrt(MAX) == 2^128 - 1
            let sqrt_max = isqrt_u256(m2p(&max256()));
            assert_eq!(sqrt_max, pow2_p256(128) - P256::one());
            // square-back check: s^2 <= MAX < (s+1)^2
            let sq = sqrt_max.checked_mul(sqrt_max).expect("s^2 fits in u256");
            assert!(sq <= m2p(&max256()));
            let sp1 = sqrt_max.checked_add(P256::one()).expect("s+1 fits");
            assert!(sp1.checked_mul(sp1).is_none()); // (2^128)^2 = 2^256 overflows
            // perfect square roundtrip
            let base = P256::from(123456789u64);
            assert_eq!(isqrt_u256(base * base), base);
        }

        #[test]
        fn test_average_u256() {
            let max = m2p(&max256());
            assert_eq!(average_u256(max, max), max);
            assert_eq!(average_u256(max, P256::zero()), max / P256::from(2u64));
            assert_eq!(
                average_u256(P256::from(10u64), P256::from(20u64)),
                P256::from(15u64)
            );
        }

        #[test]
        fn test_clamp_log2_u256() {
            let (a, b, c) = (P256::from(5u64), P256::zero(), P256::from(10u64));
            assert_eq!(clamp_u256(a, b, c).unwrap(), a);
            assert_eq!(clamp_u256(P256::from(99u64), b, c).unwrap(), c);
            assert_eq!(clamp_u256(a, c, b).unwrap_err(), E_INVALID_ARG);
            assert_eq!(log2_u256_checked(&MU256::from(1u64)).unwrap(), P256::zero());
            assert_eq!(log2_u256_checked(&max256()).unwrap(), P256::from(255u64));
            assert_eq!(
                log2_u256_checked(&MU256::zero()).unwrap_err(),
                E_INVALID_ARG
            );
        }
    }
}
