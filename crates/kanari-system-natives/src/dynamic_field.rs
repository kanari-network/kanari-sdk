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

use crate::helpers::{expect_native_args, expect_native_signature, make_module_natives};

// ==============================================================================
// Error Codes (must match declarations in dynamic_field.move)
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

    expect_native_signature(arguments.len(), 3, ty_args.len(), 2)?;

    let value = arguments.pop_back().ok_or_else(|| {
        PartialVMError::new(StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH)
            .with_message("Missing dynamic field value argument".to_string())
    })?;
    let name = arguments.pop_back().ok_or_else(|| {
        PartialVMError::new(StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH)
            .with_message("Missing dynamic field name argument".to_string())
    })?;
    let uid_ref = pop_arg!(arguments, Reference);

    // 1. Serialize Name safely (avoid unwrap)
    let name_layout = match context.type_to_type_layout(&ty_args[0]) {
        Ok(Some(layout)) => layout,
        _ => return Err(PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE)),
    };
    let name_bytes = match name.simple_serialize(&name_layout) {
        Some(bytes) => bytes,
        None => return Err(PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR)),
    };

    // 2. Serialize Value safely
    let value_layout = match context.type_to_type_layout(&ty_args[1]) {
        Ok(Some(layout)) => layout,
        _ => return Err(PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE)),
    };
    let value_bytes = match value.simple_serialize(&value_layout) {
        Some(bytes) => bytes,
        None => return Err(PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR)),
    };

    // Extract temporary Object ID from Reference (placeholder until full DB integration)
    let object_id_str = format!("{:?}", uid_ref);

    let mut already_exists = false;

    // Record data in Context safely
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

    // If duplicate Key is added, return Error gracefully to Move VM (Abort but Node does not crash)
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

    expect_native_args(arguments.len(), 2)?;
    let _name = arguments.pop_back().ok_or_else(|| {
        PartialVMError::new(StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH)
            .with_message("Missing dynamic field name argument".to_string())
    })?;
    let _uid_ref = pop_arg!(arguments, Reference);

    // Safest approach: Creating fake Reference will crash VM
    // Returning Error `E_FIELD_DOES_NOT_EXIST` is the safest and correct approach
    // Until full DB Reference connection system is implemented
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

    expect_native_args(arguments.len(), 2)?;
    let _name = arguments.pop_back().ok_or_else(|| {
        PartialVMError::new(StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH)
            .with_message("Missing dynamic field name argument".to_string())
    })?;
    let _uid_ref = pop_arg!(arguments, Reference);

    // Safest approach: Abort contract if called
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

    expect_native_args(arguments.len(), 2)?;
    let _name = arguments.pop_back().ok_or_else(|| {
        PartialVMError::new(StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH)
            .with_message("Missing dynamic field name argument".to_string())
    })?;
    let _uid_ref = pop_arg!(arguments, Reference);

    // Safest approach: Abort because system cannot yet convert data from Bytes back to 'Value' for Move
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

    expect_native_signature(arguments.len(), 2, ty_args.len(), 1)?;

    let name = arguments.pop_back().ok_or_else(|| {
        PartialVMError::new(StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH)
            .with_message("Missing dynamic field name argument".to_string())
    })?;
    let _uid_ref = pop_arg!(arguments, Reference);

    // Serialize Name safely
    let name_layout = match context.type_to_type_layout(&ty_args[0]) {
        Ok(Some(layout)) => layout,
        _ => return Err(PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE)),
    };
    let name_bytes = match name.simple_serialize(&name_layout) {
        Some(bytes) => bytes,
        None => return Err(PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR)),
    };

    let mut is_exist = false;

    // Check in Extension if it has been Added in current Transaction
    crate::native_ext::with_ext_mut_or_default::<DynamicFieldsExt, _>(context, |ext| {
        is_exist = ext.ops.iter().any(|op| match op {
            DynamicFieldOp::Add {
                name_bytes: existing_name,
                ..
            } => existing_name == &name_bytes,
            _ => false,
        });
    });

    // TODO: Check from RocksDB in future

    Ok(NR::ok(context.gas_used(), smallvec![Value::bool(is_exist)]))
}
