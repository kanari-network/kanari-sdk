// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use better_any::{Tid, TidAble};
use move_core_types::account_address::AccountAddress;
use move_core_types::gas_algebra::{InternalGas, InternalGasPerByte, NumBytes};
use move_vm_runtime::native_charge_gas_early_exit;
use move_vm_runtime::native_functions::NativeContext;
use move_vm_runtime::native_functions::{
    NativeFunction, NativeFunctionTable, make_table_from_iter,
};
use move_vm_types::natives::function::NativeResult;
use move_vm_types::natives::function::PartialVMResult;
use smallvec::smallvec;
use std::collections::VecDeque;
use std::sync::Arc;

use crate::helpers::make_module_natives;

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub save_object: SaveObjectGasParameters,
    pub delete_object: DeleteObjectGasParameters,
}

#[derive(Debug, Clone)]
pub struct SaveObjectGasParameters {
    pub base: InternalGas,
    pub per_byte_serialized: InternalGasPerByte,
}

#[derive(Debug, Clone)]
pub struct DeleteObjectGasParameters {
    pub base: InternalGas,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            save_object: SaveObjectGasParameters {
                base: 0.into(),
                per_byte_serialized: 0.into(),
            },
            delete_object: DeleteObjectGasParameters { base: 0.into() },
        }
    }
}

fn uid_address_bytes(uid_val: &move_vm_types::values::Value) -> Option<Vec<u8>> {
    use move_core_types::runtime_value::{MoveStructLayout, MoveTypeLayout};

    // Support both:
    // - UID { addr: address }  (legacy / simplified)
    // - UID { id: ID }, ID { bytes: address } (Sui-style)
    let nested = MoveTypeLayout::Struct(MoveStructLayout::new(vec![MoveTypeLayout::Struct(
        MoveStructLayout::new(vec![MoveTypeLayout::Address]),
    )]));
    let flat = MoveTypeLayout::Struct(MoveStructLayout::new(vec![MoveTypeLayout::Address]));

    uid_val
        .simple_serialize(&nested)
        .or_else(|| uid_val.simple_serialize(&flat))
}

#[derive(Clone, Debug)]
pub struct SavedObject {
    pub object_id: String,
    pub object_type: String,
    pub data: Vec<u8>,
}

#[derive(Tid, Default)]
pub struct SavedObjectsExt {
    pub objects: Vec<SavedObject>,
}

impl SavedObjectsExt {
    pub fn record(&mut self, obj: SavedObject) {
        self.objects.push(obj);
    }
    pub fn take_all(&mut self) -> Vec<SavedObject> {
        std::mem::take(&mut self.objects)
    }
}

#[derive(Clone, Debug)]
pub struct DeletedObject {
    pub object_id: String,
}

#[derive(Tid, Default)]
pub struct DeletedObjectsExt {
    pub objects: Vec<DeletedObject>,
}

impl DeletedObjectsExt {
    pub fn record(&mut self, obj: DeletedObject) {
        self.objects.push(obj);
    }
    pub fn take_all(&mut self) -> Vec<DeletedObject> {
        std::mem::take(&mut self.objects)
    }
}

pub fn all_natives(move_addr: AccountAddress) -> NativeFunctionTable {
    make_table_from_iter(
        move_addr,
        make_all(GasParameters::zeros())
            .map(|(func_name, func)| ("object".to_string(), func_name, func)),
    )
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let save_params = gas_params.save_object;
    let delete_params = gas_params.delete_object;

    let save_object: NativeFunction = Arc::new(move |context, ty_args, args| {
        native_save_object(&save_params, context, ty_args, args)
    });
    let delete_impl: NativeFunction = Arc::new(move |context, ty_args, args| {
        native_delete_object(&delete_params, context, ty_args, args)
    });
    make_module_natives([("save_object", save_object), ("delete_impl", delete_impl)])
}

fn native_delete_object(
    gas_params: &DeleteObjectGasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<move_vm_types::loaded_data::runtime_types::Type>,
    mut arguments: VecDeque<move_vm_types::values::Value>,
) -> PartialVMResult<NativeResult> {
    use move_core_types::vm_status::StatusCode;
    use move_vm_types::natives::function::NativeResult as NR;
    use move_vm_types::natives::function::PartialVMError;

    native_charge_gas_early_exit!(context, gas_params.base);

    // Arguments: uid (UID) - passed by value
    let uid_val = arguments.pop_back().ok_or_else(|| {
        PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
            .with_message("Missing uid argument".to_string())
    })?;

    // Serialize UID to get the address bytes (supports multiple UID encodings)
    let uid_bytes = uid_address_bytes(&uid_val).ok_or_else(|| {
        PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
            .with_message("Failed to serialize UID".to_string())
    })?;

    let object_id = format!("0x{}", hex::encode(&uid_bytes));

    // Record the deleted object
    crate::native_ext::with_ext_mut_or_default::<DeletedObjectsExt, _>(context, |ext| {
        ext.record(DeletedObject { object_id })
    });

    Ok(NR::ok(context.gas_used(), smallvec![]))
}

fn native_save_object(
    gas_params: &SaveObjectGasParameters,
    context: &mut NativeContext,
    ty_args: Vec<move_vm_types::loaded_data::runtime_types::Type>,
    mut arguments: VecDeque<move_vm_types::values::Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;
    use move_vm_types::pop_arg;
    use move_vm_types::values::values_impl::Reference;

    native_charge_gas_early_exit!(context, gas_params.base);

    if ty_args.is_empty() {
        return Ok(NR::err(context.gas_used(), 1));
    }

    let type_tag = context.type_to_type_tag(&ty_args[0])?;
    let type_str = format!("{}", type_tag);

    // Arguments: obj (&T)
    let obj_ref = pop_arg!(arguments, Reference);

    // Read the reference to get the value
    let obj_val = obj_ref.read_ref()?;

    let mut obj_data = Vec::new();
    if let Some(layout) = context.type_to_type_layout(&ty_args[0])? {
        obj_data = obj_val.simple_serialize(&layout).unwrap_or_default();
    }

    native_charge_gas_early_exit!(
        context,
        gas_params.per_byte_serialized * NumBytes::new(obj_data.len() as u64)
    );

    // Extract ID (first 32 bytes)
    let object_id_hex = if obj_data.len() >= 32 {
        let uid_bytes = &obj_data[0..32];
        format!("0x{}", hex::encode(uid_bytes))
    } else {
        String::new()
    };

    if !object_id_hex.is_empty() {
        let saved = SavedObject {
            object_id: object_id_hex,
            object_type: type_str,
            data: obj_data,
        };

        // Record the saved object
        crate::native_ext::with_ext_mut_or_default::<SavedObjectsExt, _>(context, |ext| {
            ext.record(saved)
        });

        // Log for debugging
        // println!("[DEBUG] native_save_object: Saved {}", saved.object_id);
    }

    Ok(NR::ok(context.gas_used(), smallvec![]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_vm_types::values::{Struct, Value};

    #[test]
    fn uid_address_bytes_supports_flat_and_nested() {
        let addr = AccountAddress::new([7u8; AccountAddress::LENGTH]);
        let expected = addr.into_bytes().to_vec();

        // UID { addr: address }
        let flat_uid = Value::struct_(Struct::pack([Value::address(addr)]));
        assert_eq!(uid_address_bytes(&flat_uid).unwrap(), expected);

        // UID { id: ID }, ID { bytes: address }
        let nested_uid = Value::struct_(Struct::pack([Value::struct_(Struct::pack([
            Value::address(addr),
        ]))]));
        assert_eq!(uid_address_bytes(&nested_uid).unwrap(), expected);
    }
}
