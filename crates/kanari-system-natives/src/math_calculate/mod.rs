// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::make_native;
use move_core_types::account_address::AccountAddress;
use move_vm_runtime::native_functions::make_table_from_iter;

pub mod math;

pub fn all_natives(
    core_addr: AccountAddress,
) -> move_vm_runtime::native_functions::NativeFunctionTable {
    // สร้างลิสต์ของ Native Functions โดยใช้ `make_native` เพื่อจัดการเรื่อง Type
    let natives = vec![
        ("math", "sqrt_u128", make_native(math::native_sqrt_u128)),
        ("math", "sqrt_u64", make_native(math::native_sqrt_u64)),
        ("math", "pow_u64", make_native(math::native_pow_u64)),
        (
            "math",
            "mul_div_u128",
            make_native(math::native_mul_div_u128),
        ),
    ];

    // สร้างตารางลงทะเบียน (Table) คืนค่าให้ Runtime
    make_table_from_iter(core_addr, natives)
}
