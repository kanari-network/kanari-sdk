// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use better_any::{Tid, TidAble};
use move_core_types::gas_algebra::{InternalGas, InternalGasPerByte, NumBytes};
use move_vm_runtime::native_charge_gas_early_exit;
use move_vm_runtime::native_functions::NativeContext;
use move_vm_runtime::native_functions::NativeFunction;
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
    pub borrow_global_mut: BorrowGlobalMutGasParameters,
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

#[derive(Debug, Clone)]
pub struct BorrowGlobalMutGasParameters {
    pub base: InternalGas,
    pub per_byte_loaded: InternalGasPerByte,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            save_object: SaveObjectGasParameters {
                base: 0.into(),
                per_byte_serialized: 0.into(),
            },
            delete_object: DeleteObjectGasParameters { base: 0.into() },
            borrow_global_mut: BorrowGlobalMutGasParameters {
                base: 0.into(),
                per_byte_loaded: 0.into(),
            },
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

/// Extension for loaded objects from storage
#[derive(Tid, Default)]
pub struct LoadedObjectsExt {
    pub objects: std::collections::HashMap<String, (String, Vec<u8>)>,
}

impl LoadedObjectsExt {
    pub fn insert(&mut self, object_id: String, type_str: String, data: Vec<u8>) {
        self.objects.insert(object_id, (type_str, data));
    }
    pub fn get(&self, object_id: &str) -> Option<&(String, Vec<u8>)> {
        self.objects.get(object_id)
    }
}

/// Extension for tracking borrowed mutable objects
#[derive(Tid, Default)]
pub struct BorrowedObjectsExt {
    pub objects: Vec<(String, String, Vec<u8>)>, // (object_id, type_str, original_data)
}

impl BorrowedObjectsExt {
    pub fn record(&mut self, object_id: String, type_str: String, data: Vec<u8>) {
        self.objects.push((object_id, type_str, data));
    }
    pub fn take_all(&mut self) -> Vec<(String, String, Vec<u8>)> {
        std::mem::take(&mut self.objects)
    }
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let save_params = gas_params.save_object.clone();
    let delete_params = gas_params.delete_object.clone();
    let borrow_params = gas_params.borrow_global_mut.clone();

    let save_object: NativeFunction = Arc::new(move |context, ty_args, args| {
        native_save_object(&save_params, context, ty_args, args)
    });
    let delete_impl: NativeFunction = Arc::new(move |context, ty_args, args| {
        native_delete_object(&delete_params, context, ty_args, args)
    });
    let borrow_global_mut: NativeFunction = Arc::new(move |context, ty_args, args| {
        native_borrow_global_mut(&borrow_params, context, ty_args, args)
    });
    make_module_natives([
        ("save_object", save_object),
        ("delete_impl", delete_impl),
        ("borrow_global_mut", borrow_global_mut),
    ])
}

/// Native function: borrow_global_mut<T>(address): &mut T
/// Loads an object from storage and returns a mutable reference.
/// This enables CLI to pass object IDs and have them resolved at runtime.
fn native_borrow_global_mut(
    gas_params: &BorrowGlobalMutGasParameters,
    context: &mut NativeContext,
    ty_args: Vec<move_vm_types::loaded_data::runtime_types::Type>,
    mut arguments: VecDeque<move_vm_types::values::Value>,
) -> PartialVMResult<NativeResult> {
    use move_core_types::account_address::AccountAddress;
    use move_core_types::vm_status::StatusCode;
    use move_vm_types::natives::function::NativeResult as NR;
    use move_vm_types::pop_arg;

    native_charge_gas_early_exit!(context, gas_params.base);

    // Arguments: address (32 bytes)
    let addr_bytes = pop_arg!(arguments, Vec<u8>);
    if addr_bytes.len() != 32 {
        return Ok(NR::err(
            context.gas_used(),
            StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT as u64,
        ));
    }

    let object_id = format!("0x{}", hex::encode(&addr_bytes));
    let object_addr = AccountAddress::new(addr_bytes.try_into().unwrap());
    // Load object from storage via context's extension
    let loaded_data =
        crate::native_ext::with_ext_mut_or_default::<LoadedObjectsExt, _>(context, |ext| {
            ext.get(&object_id).cloned()
        });

    let Some((type_str, obj_data)) = loaded_data.flatten() else {
        return Ok(NR::err(
            context.gas_used(),
            StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT as u64,
        ));
    };

    native_charge_gas_early_exit!(
        context,
        gas_params.per_byte_loaded * NumBytes::new(obj_data.len() as u64)
    );

    // Deserialize the object data into a Move value
    let layout = match context.type_to_type_layout(&ty_args[0]) {
        Ok(Some(layout)) => layout,
        _ => {
            return Ok(NR::err(
                context.gas_used(),
                StatusCode::TYPE_MISMATCH as u64,
            ));
        }
    };

    // Verify that the loaded object type matches the requested type
    let requested_type = match context.type_to_type_tag(&ty_args[0]) {
        Ok(tag) => format!("{}", tag),
        Err(_) => {
            return Ok(NR::err(
                context.gas_used(),
                StatusCode::TYPE_MISMATCH as u64,
            ));
        }
    };

    if type_str != requested_type {
        return Ok(NR::err(
            context.gas_used(),
            StatusCode::TYPE_MISMATCH as u64,
        ));
    }

    // Deserialize object data into Move value
    let obj_val = match move_vm_types::values::Value::simple_deserialize(&obj_data, &layout) {
        Some(val) => val,
        None => {
            return Ok(NR::err(
                context.gas_used(),
                StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT as u64,
            ));
        }
    };

    // Create a mutable reference to the object
    // The VM will manage the lifecycle of this reference
    let obj_ref = move_vm_types::values::Value::mutable_borrow_global(
        object_addr,
        ty_args[0].clone(),
        obj_val,
    )?;

    // Track borrowed objects for later writeback
    crate::native_ext::with_ext_mut_or_default::<BorrowedObjectsExt, _>(context, |ext| {
        ext.record(object_id.clone(), type_str, obj_data);
    });

    Ok(NR::ok(context.gas_used(), smallvec![obj_ref]))
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
    use move_core_types::account_address::AccountAddress;
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
