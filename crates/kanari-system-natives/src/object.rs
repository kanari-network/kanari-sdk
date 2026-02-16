// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::make_native;
use better_any::{Tid, TidAble};
use move_core_types::account_address::AccountAddress;
use move_core_types::gas_algebra::GasQuantity;
use move_vm_runtime::native_functions::NativeContext;
use move_vm_runtime::native_functions::make_table_from_iter;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::natives::function::PartialVMResult;
use smallvec::smallvec;
use std::collections::VecDeque;

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

pub fn all_natives(
    move_addr: AccountAddress,
) -> move_vm_runtime::native_functions::NativeFunctionTable {
    let natives = vec![("object", "save_object", make_native(native_save_object))];
    make_table_from_iter(move_addr, natives)
}

fn native_save_object(
    context: &mut NativeContext,
    ty_args: Vec<move_vm_types::loaded_data::runtime_types::Type>,
    mut arguments: VecDeque<move_vm_types::values::Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;
    use move_vm_types::pop_arg;
    use move_vm_types::values::values_impl::Reference;

    let gas_used_now = context.gas_used();

    if ty_args.is_empty() {
        return Ok(NR::err(gas_used_now, 1));
    }

    let type_tag = context.type_to_type_tag(&ty_args[0])?;
    let type_str = format!("{}", type_tag);

    // Arguments: obj (&T)
    let obj_ref = pop_arg!(arguments, Reference);

    // Read the reference to get the value
    let obj_val = obj_ref.read_ref()?;

    let mut obj_data = Vec::new();
    if let Ok(layout_opt) = context.type_to_type_layout(&ty_args[0]) {
        if let Some(layout) = layout_opt {
            if let Some(serialized) = obj_val.simple_serialize(&layout) {
                obj_data = serialized;
            }
        }
    }

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
        let exts = context.extensions_mut();
        let ext = exts.get_mut::<SavedObjectsExt>();
        ext.record(saved);

        // Log for debugging
        // println!("[DEBUG] native_save_object: Saved {}", saved.object_id);
    }

    // Gas cost
    let cost = 1000;
    let gas_cost = GasQuantity::new(cost);

    Ok(NR::ok(gas_used_now + gas_cost, smallvec![]))
}
