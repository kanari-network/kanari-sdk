// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::native_charge_gas_early_exit;
use move_vm_runtime::native_functions::NativeContext;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::{NativeResult, PartialVMResult},
    pop_arg,
    values::Value,
};
use num_integer::Integer;
use num_integer::Roots;
use smallvec::smallvec;
use std::collections::VecDeque;

// =================================================================
// Error Codes (must match math.move)
// =================================================================
const E_OVERFLOW: u64 = 1;
const E_DIVIDE_BY_ZERO: u64 = 2;

// =================================================================
// Native Implementations
// =================================================================

/// Calculate square root of u128
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
}

#[derive(Debug, Clone)]
pub struct MulDivU128GasParameters {
    pub base: InternalGas,
}

fn mul_div_u128_checked(x: u128, y: u128, z: u128) -> Result<u128, u64> {
    if z == 0 {
        return Err(E_DIVIDE_BY_ZERO);
    }

    // Reduce before multiplication to avoid intermediate overflow.
    // (x * y) / z == (x' * y') / z' where we cancel common factors with z.
    let mut x = x;
    let mut y = y;
    let mut z = z;

    let g1 = x.gcd(&z);
    x /= g1;
    z /= g1;

    let g2 = y.gcd(&z);
    y /= g2;
    z /= g2;

    match x.checked_mul(y) {
        Some(xy) => Ok(xy / z),
        None => Err(E_OVERFLOW),
    }
}

pub fn native_sqrt_u128(
    gas_params: &SqrtU128GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    debug_assert!(args.len() == 1);
    native_charge_gas_early_exit!(context, gas_params.base);
    let x: u128 = pop_arg!(args, u128);
    let result = x.sqrt();

    Ok(NR::ok(context.gas_used(), smallvec![Value::u128(result)]))
}

/// Calculate square root of u64
pub fn native_sqrt_u64(
    gas_params: &SqrtU64GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    debug_assert!(args.len() == 1);
    native_charge_gas_early_exit!(context, gas_params.base);
    let x: u64 = pop_arg!(args, u64);
    let result = x.sqrt();

    Ok(NR::ok(context.gas_used(), smallvec![Value::u64(result)]))
}

/// Power function for u64 (base ^ exponent)
pub fn native_pow_u64(
    gas_params: &PowU64GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    debug_assert!(args.len() == 2);
    native_charge_gas_early_exit!(context, gas_params.base);
    // Pop arguments in LIFO order
    let exponent: u8 = pop_arg!(args, u8);
    let base: u64 = pop_arg!(args, u64);

    // Use checked_pow to prevent overflow with high exponents
    match base.checked_pow(exponent as u32) {
        Some(result) => Ok(NR::ok(context.gas_used(), smallvec![Value::u64(result)])),
        None => Ok(NR::err(context.gas_used(), E_OVERFLOW)),
    }
}

/// Calculate (x * y) / z safely for u128, preventing intermediate overflow
pub fn native_mul_div_u128(
    gas_params: &MulDivU128GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    debug_assert!(args.len() == 3);
    native_charge_gas_early_exit!(context, gas_params.base);
    // Pop arguments in LIFO order
    let z: u128 = pop_arg!(args, u128);
    let y: u128 = pop_arg!(args, u128);
    let x: u128 = pop_arg!(args, u128);

    // Prevent division by zero
    // Calculate x * y with bounds checking (checked_mul)
    // Note: For production scale, if x*y exceeds u128 ceiling, consider using U256 temporarily (e.g., primitive_types::U256)
    match mul_div_u128_checked(x, y, z) {
        Ok(result) => Ok(NR::ok(context.gas_used(), smallvec![Value::u128(result)])),
        Err(code) => Ok(NR::err(context.gas_used(), code)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mul_div_u128_avoids_intermediate_overflow() {
        assert_eq!(mul_div_u128_checked(u128::MAX, 2, 2).unwrap(), u128::MAX);
    }

    #[test]
    fn mul_div_u128_basic() {
        assert_eq!(mul_div_u128_checked(10, 20, 5).unwrap(), 40);
    }

    #[test]
    fn mul_div_u128_divide_by_zero() {
        assert_eq!(mul_div_u128_checked(1, 2, 0).unwrap_err(), E_DIVIDE_BY_ZERO);
    }

    #[test]
    fn mul_div_u128_overflow_still_errors() {
        assert_eq!(
            mul_div_u128_checked(u128::MAX, u128::MAX, 1).unwrap_err(),
            E_OVERFLOW
        );
    }
}
