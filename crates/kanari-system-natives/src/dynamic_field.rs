// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use better_any::{Tid, TidAble};
use move_core_types::gas_algebra::InternalGas;
use move_core_types::vm_status::StatusCode;
use move_vm_runtime::native_charge_gas_early_exit;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::{NativeResult, PartialVMError, PartialVMResult};
use move_vm_types::pop_arg;
use move_vm_types::values::Value;
use move_vm_types::values::values_impl::Reference;
use smallvec::smallvec;
use std::collections::VecDeque;
use std::sync::Arc;

use crate::helpers::make_module_natives;

// ==============================================================================
// Error Codes (ตรงกับที่ประกาศไว้ใน dynamic_field.move)
// ==============================================================================
const E_FIELD_ALREADY_EXISTS: u64 = 1;
const E_FIELD_DOES_NOT_EXIST: u64 = 2;

// ==============================================================================
// Gas Parameters
// ==============================================================================

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub add: InternalGas,
    pub borrow: InternalGas,
    pub borrow_mut: InternalGas,
    pub remove: InternalGas,
    pub exists_: InternalGas,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            add: 0.into(),
            borrow: 0.into(),
            borrow_mut: 0.into(),
            remove: 0.into(),
            exists_: 0.into(),
        }
    }
}

// ==============================================================================
// State Extensions (Tid) for ObjectRuntime
// ==============================================================================

#[derive(Clone, Debug)]
pub enum DynamicFieldOp {
    Add {
        object_id: String,
        name_bytes: Vec<u8>,
        value_bytes: Vec<u8>,
    },
    Remove {
        object_id: String,
        name_bytes: Vec<u8>,
    },
}

#[derive(Tid, Default)]
pub struct DynamicFieldsExt {
    pub ops: Vec<DynamicFieldOp>,
}

impl DynamicFieldsExt {
    pub fn record(&mut self, op: DynamicFieldOp) {
        self.ops.push(op);
    }
    pub fn take_all(&mut self) -> Vec<DynamicFieldOp> {
        std::mem::take(&mut self.ops)
    }
}

// ==============================================================================
// Native Function Registrations
// ==============================================================================

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let add_gas = gas_params.add;
    let borrow_gas = gas_params.borrow;
    let borrow_mut_gas = gas_params.borrow_mut;
    let remove_gas = gas_params.remove;
    let exists_gas = gas_params.exists_;

    let add: NativeFunction =
        Arc::new(move |context, ty_args, args| native_add(add_gas, context, ty_args, args));
    let borrow_mut: NativeFunction = Arc::new(move |context, ty_args, args| {
        native_borrow_mut(borrow_mut_gas, context, ty_args, args)
    });
    let borrow: NativeFunction =
        Arc::new(move |context, ty_args, args| native_borrow(borrow_gas, context, ty_args, args));
    let remove: NativeFunction =
        Arc::new(move |context, ty_args, args| native_remove(remove_gas, context, ty_args, args));
    let exists_: NativeFunction =
        Arc::new(move |context, ty_args, args| native_exists_(exists_gas, context, ty_args, args));

    make_module_natives([
        ("add", add),
        ("borrow_mut", borrow_mut),
        ("borrow", borrow),
        ("remove", remove),
        ("exists_", exists_),
    ])
}

// ==============================================================================
// Native Implementations (Safe Mode)
// ==============================================================================

fn native_add(
    gas_base: InternalGas,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    native_charge_gas_early_exit!(context, gas_base);

    // ตรวจสอบความถูกต้องของ Arguments
    if ty_args.len() != 2 || arguments.len() != 3 {
        return Err(PartialVMError::new(
            StatusCode::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH,
        ));
    }

    let value = arguments.pop_back().unwrap();
    let name = arguments.pop_back().unwrap();
    let uid_ref = pop_arg!(arguments, Reference);

    // 1. Serialize Name แบบปลอดภัย (ไม่ใช้ unwrap)
    let name_layout = match context.type_to_type_layout(&ty_args[0]) {
        Ok(Some(layout)) => layout,
        _ => return Err(PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE)),
    };
    let name_bytes = match name.simple_serialize(&name_layout) {
        Some(bytes) => bytes,
        None => return Err(PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR)),
    };

    // 2. Serialize Value แบบปลอดภัย
    let value_layout = match context.type_to_type_layout(&ty_args[1]) {
        Ok(Some(layout)) => layout,
        _ => return Err(PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE)),
    };
    let value_bytes = match value.simple_serialize(&value_layout) {
        Some(bytes) => bytes,
        None => return Err(PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR)),
    };

    // ดึง Object ID ชั่วคราวจาก Reference (เป็น placeholder ระหว่างที่รอเชื่อม DB เต็มตัว)
    let object_id_str = format!("{:?}", uid_ref);

    let mut already_exists = false;

    // บันทึกข้อมูลลง Context แบบปลอดภัย
    crate::native_ext::with_ext_mut_or_default::<DynamicFieldsExt, _>(context, |ext| {
        already_exists = ext.ops.iter().any(|op| match op {
            DynamicFieldOp::Add {
                name_bytes: existing_name,
                ..
            } => existing_name == &name_bytes,
            _ => false,
        });

        if !already_exists {
            ext.record(DynamicFieldOp::Add {
                object_id: object_id_str,
                name_bytes,
                value_bytes,
            });
        }
    });

    // หากพบว่าเพิ่ม Key ซ้ำ ให้พ่น Error กลับไปที่ Move VM แบบ Graceful (สัญญา Abort แต่ Node ไม่ดับ)
    if already_exists {
        Ok(NR::err(context.gas_used(), E_FIELD_ALREADY_EXISTS))
    } else {
        Ok(NR::ok(context.gas_used(), smallvec![]))
    }
}

fn native_borrow_mut(
    gas_base: InternalGas,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    native_charge_gas_early_exit!(context, gas_base);

    let _name = arguments.pop_back().unwrap();
    let _uid_ref = pop_arg!(arguments, Reference);

    // ปลอดภัยที่สุด: การสร้าง Reference ปลอมจะทำให้ VM พัง (Crash)
    // การคืนค่า Error `E_FIELD_DOES_NOT_EXIST` เป็นวิธีที่ถูกต้องและปลอดภัยที่สุด
    // จนกว่าเราจะเขียนระบบเชื่อม Database Reference ให้สมบูรณ์
    Ok(NativeResult::err(
        context.gas_used(),
        E_FIELD_DOES_NOT_EXIST,
    ))
}

fn native_borrow(
    gas_base: InternalGas,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    native_charge_gas_early_exit!(context, gas_base);

    let _name = arguments.pop_back().unwrap();
    let _uid_ref = pop_arg!(arguments, Reference);

    // ปลอดภัยที่สุด: สั่ง Abort สัญญาหากมีการเรียกใช้งาน
    Ok(NativeResult::err(
        context.gas_used(),
        E_FIELD_DOES_NOT_EXIST,
    ))
}

fn native_remove(
    gas_base: InternalGas,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    native_charge_gas_early_exit!(context, gas_base);

    let _name = arguments.pop_back().unwrap();
    let _uid_ref = pop_arg!(arguments, Reference);

    // ปลอดภัยที่สุด: สั่ง Abort เพราะระบบยังไม่สามารถแปลงข้อมูลจาก Byte กลับเป็น 'Value' เพื่อคืนให้ Move ได้
    Ok(NativeResult::err(
        context.gas_used(),
        E_FIELD_DOES_NOT_EXIST,
    ))
}

fn native_exists_(
    gas_base: InternalGas,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    native_charge_gas_early_exit!(context, gas_base);

    if ty_args.is_empty() || arguments.len() != 2 {
        return Err(PartialVMError::new(
            StatusCode::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH,
        ));
    }

    let name = arguments.pop_back().unwrap();
    let _uid_ref = pop_arg!(arguments, Reference);

    // Serialize Name อย่างปลอดภัย
    let name_layout = match context.type_to_type_layout(&ty_args[0]) {
        Ok(Some(layout)) => layout,
        _ => return Err(PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE)),
    };
    let name_bytes = match name.simple_serialize(&name_layout) {
        Some(bytes) => bytes,
        None => return Err(PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR)),
    };

    let mut is_exist = false;

    // ตรวจสอบใน Extension ว่าถูก Add ไปหรือยังใน Transaction ปัจจุบัน
    crate::native_ext::with_ext_mut_or_default::<DynamicFieldsExt, _>(context, |ext| {
        is_exist = ext.ops.iter().any(|op| match op {
            DynamicFieldOp::Add {
                name_bytes: existing_name,
                ..
            } => existing_name == &name_bytes,
            _ => false,
        });
    });

    // TODO: ตรวจสอบจาก RocksDB ในอนาคต

    Ok(NR::ok(context.gas_used(), smallvec![Value::bool(is_exist)]))
}
