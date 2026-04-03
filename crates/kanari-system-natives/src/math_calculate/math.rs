// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::native_functions::NativeContext;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::{NativeResult, PartialVMResult},
    pop_arg,
    values::Value,
};
use num_integer::Roots; // 🚨 อย่าลืมใส่ num-integer = "0.1.45" ใน Cargo.toml
use smallvec::smallvec;
use std::collections::VecDeque;

// =================================================================
// Error Codes (ต้องตรงกับใน math.move)
// =================================================================
const E_OVERFLOW: u64 = 1;
const E_DIVIDE_BY_ZERO: u64 = 2;

// =================================================================
// Native Implementations
// =================================================================

/// ถอดรากที่สองของ u128
pub fn native_sqrt_u128(
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(args.len() == 1);
    let x: u128 = pop_arg!(args, u128);
    let result = x.sqrt();

    Ok(NativeResult::ok(
        InternalGas::new(10), // ค่า Gas
        smallvec![Value::u128(result)],
    ))
}

/// ถอดรากที่สองของ u64
pub fn native_sqrt_u64(
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(args.len() == 1);
    let x: u64 = pop_arg!(args, u64);
    let result = x.sqrt();

    Ok(NativeResult::ok(
        InternalGas::new(10),
        smallvec![Value::u64(result)],
    ))
}

/// ยกกำลังสำหรับ u64 (base ^ exponent)
pub fn native_pow_u64(
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(args.len() == 2);
    // Pop argument จากหลังมาหน้า (LIFO)
    let exponent: u8 = pop_arg!(args, u8);
    let base: u64 = pop_arg!(args, u64);

    // ใช้ checked_pow เพื่อป้องกัน Overflow เวลายกกำลังสูงๆ
    match base.checked_pow(exponent as u32) {
        Some(result) => Ok(NativeResult::ok(
            InternalGas::new(15),
            smallvec![Value::u64(result)],
        )),
        None => Ok(NativeResult::err(InternalGas::new(15), E_OVERFLOW)),
    }
}

/// คำนวณ (x * y) / z แบบปลอดภัยสำหรับ u128 ป้องกัน Overflow ระหว่างทาง
pub fn native_mul_div_u128(
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(args.len() == 3);
    // Pop argument จากหลังมาหน้า (LIFO)
    let z: u128 = pop_arg!(args, u128);
    let y: u128 = pop_arg!(args, u128);
    let x: u128 = pop_arg!(args, u128);

    // ป้องกันหารด้วย 0
    if z == 0 {
        return Ok(NativeResult::err(InternalGas::new(10), E_DIVIDE_BY_ZERO));
    }

    // คำนวณ x * y แบบเช็คขอบเขต (checked_mul)
    // หมายเหตุ: สำหรับ Production Scale ถ้า x*y เกินเพดาน u128 แนะนำให้ใช้ Type U256 ชั่วคราว (เช่น primitive_types::U256)
    match x.checked_mul(y) {
        Some(xy) => {
            let result = xy / z;
            Ok(NativeResult::ok(
                InternalGas::new(20),
                smallvec![Value::u128(result)],
            ))
        }
        None => Ok(NativeResult::err(InternalGas::new(20), E_OVERFLOW)),
    }
}
