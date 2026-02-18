// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use better_any::{Tid, TidAble};
/// Native functions for object transfer with proper tracking
/// Uses per-execution native context extensions to track transferred objects
use move_core_types::account_address::AccountAddress;
use move_core_types::gas_algebra::GasQuantity;
use move_vm_runtime::native_functions::NativeContext;
use move_vm_runtime::native_functions::make_table_from_iter;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::natives::function::PartialVMError;
use move_vm_types::natives::function::PartialVMResult;
use move_vm_types::pop_arg;
use smallvec::smallvec;
use std::collections::VecDeque;

use crate::make_native;

// Error codes returned by this native (keep values small and stable)
const E_MISSING_TYPE_ARGUMENT: u64 = 1;
const E_TYPE_NAME_TOO_LONG: u64 = 2;

/// Information about a transferred object with full data
#[derive(Clone, Debug)]
pub struct TransferredObject {
    pub object_id: String,
    pub object_type: String,
    pub recipient: AccountAddress,
    pub data: Vec<u8>,
    pub should_persist: bool, // Flag to indicate if object should be stored persistently
    pub is_frozen: bool,      // Flag to indicate if object is immutable/frozen
}

/// Extension stored in the Move VM native context for this execution
#[derive(Tid, Default)]
pub struct TransferredObjectsExt {
    pub objects: Vec<TransferredObject>,
}

impl TransferredObjectsExt {
    pub fn record(&mut self, obj: TransferredObject) {
        self.objects.push(obj);
    }

    pub fn take_all(&mut self) -> Vec<TransferredObject> {
        std::mem::take(&mut self.objects)
    }
}

/// Get all transfer native functions
pub fn all_natives(
    move_addr: AccountAddress,
) -> move_vm_runtime::native_functions::NativeFunctionTable {
    let natives = vec![
        (
            "transfer",
            "transfer_with_uid",
            make_native(native_transfer_with_uid),
        ),
        (
            "transfer",
            "freeze_object",
            make_native(native_freeze_object),
        ),
    ];

    make_table_from_iter(move_addr, natives)
}

// transfer::transfer_with_uid<T: key + store>(obj: T, recipient: address)
// Tracks transferred objects in the transaction-local native context extensions
fn native_transfer_with_uid(
    context: &mut NativeContext,
    ty_args: Vec<move_vm_types::loaded_data::runtime_types::Type>,
    mut arguments: VecDeque<move_vm_types::values::Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    // Snapshot gas used early to avoid borrowing `context` multiple times
    // (prevents borrow conflicts when mutably accessing extensions).
    let gas_used_now = context.gas_used();

    // Pop arguments: recipient (address), obj (generic T with key+store)
    let recipient = pop_arg!(arguments, AccountAddress);
    let obj_val = arguments.pop_back().ok_or_else(|| {
        PartialVMError::new(move_core_types::vm_status::StatusCode::INTERNAL_TYPE_ERROR)
            .with_message("Missing object argument".to_string())
    })?;

    // Get type argument (the object type T)
    if ty_args.is_empty() {
        return Ok(NR::err(gas_used_now, E_MISSING_TYPE_ARGUMENT)); // Missing type argument
    }

    // Extract type information and convert runtime Type -> TypeTag -> human-readable string
    let type_tag = context.type_to_type_tag(&ty_args[0])?;
    let type_str = format!("{}", type_tag);

    // Guard against pathological type name lengths to avoid excessive native
    // memory usage. Typical Move type names are small; reject overly long
    // values (treat as Move-level error). Adjust threshold if needed.
    if type_str.len() > 256 {
        return Ok(NR::err(gas_used_now, E_TYPE_NAME_TOO_LONG));
    }

    // We DO NOT generate random object IDs or serialize full object data here.
    // Instead we record a deterministic identifier per-execution and leave full
    // object data to be retrieved from the VM changeset / write-set by external systems.

    // Attempt to serialize the transferred object value into BCS so external
    // callers (CLI/RPC) can fetch the object's bytes and use them as function
    // arguments. If serialization fails, fall back to a minimal placeholder
    // (recipient + type) to preserve existing behavior.
    let mut obj_data: Vec<u8> = Vec::new();

    // Try to obtain the layout for the type and serialize the value.
    if let Ok(layout_opt) = context.type_to_type_layout(&ty_args[0]) {
        if let Some(layout) = layout_opt {
            // `obj_val` is the moved Value; attempt simple_serialize
            if let Some(serialized) = obj_val.simple_serialize(&layout) {
                obj_data = serialized;
            } else {
                // serialization failed - fall back to minimal placeholder
                obj_data.extend_from_slice(recipient.as_ref());
                obj_data.extend_from_slice(type_str.as_bytes());
            }
        } else {
            // No layout available -> fallback to placeholder
            obj_data.extend_from_slice(recipient.as_ref());
            obj_data.extend_from_slice(type_str.as_bytes());
        }
    } else {
        // Failed to query layout -> fallback to placeholder
        obj_data.extend_from_slice(recipient.as_ref());
        obj_data.extend_from_slice(type_str.as_bytes());
    }

    // Capture data length for gas metering before moving obj_data
    let data_len = obj_data.len() as u64;

    // Extract real UID from object data (first 32 bytes)
    // In Kanari/Sui Move, objects with `key` have `UID` as the first field.
    // `UID` -> `ID` -> `address` (32 bytes).
    // So the first 32 bytes of the BCS serialized data represent the Object ID.
    let object_id_hex = if obj_data.len() >= 32 {
        let uid_bytes = &obj_data[0..32];
        format!("0x{}", hex::encode(uid_bytes))
    } else {
        // Fallback for non-standard objects or errors: compute hash
        use kanari_crypto::hash_data_blake3;
        let mut input = Vec::new();
        input.extend_from_slice(recipient.as_ref());
        input.extend_from_slice(type_str.as_bytes());
        input.extend_from_slice(&obj_data);
        let hash = hash_data_blake3(&input);
        format!("0x{}", hex::encode(&hash[0..32]))
    };

    // Store transfer in the transaction-local native context extension
    // Limit the mutable borrow of the extensions to a small scope to avoid
    // borrow conflicts with later calls on `context`.
    {
        // Access the native-extensions container mutably so we can record
        // transferred objects. This borrows `context` mutably, which is why
        // we snapshot gas earlier into `gas_used_now`.
        let exts = context.extensions_mut();
        let ext = exts.get_mut::<TransferredObjectsExt>();

        let obj = TransferredObject {
            object_id: object_id_hex,
            object_type: type_str.clone(),
            recipient,
            data: obj_data,
            should_persist: true,
            is_frozen: false,
        };
        ext.record(obj);
    }

    // Consume the object (it's been transferred)
    drop(obj_val);

    // Gas cost: 2000 gas units for transfer tracking + size-dependent cost
    let cost = 2000 + (data_len * 10);
    let gas_cost = GasQuantity::new(cost);

    Ok(NR::ok(gas_used_now + gas_cost, smallvec![]))
}

// transfer::freeze_object<T: key + store>(obj: T)
// Freezes the object (makes it immutable) and tracks it in native context
fn native_freeze_object(
    context: &mut NativeContext,
    ty_args: Vec<move_vm_types::loaded_data::runtime_types::Type>,
    mut arguments: VecDeque<move_vm_types::values::Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    let gas_used_now = context.gas_used();

    // Pop argument: obj (generic T with key+store)
    let obj_val = arguments.pop_back().ok_or_else(|| {
        PartialVMError::new(move_core_types::vm_status::StatusCode::INTERNAL_TYPE_ERROR)
            .with_message("Missing object argument".to_string())
    })?;

    if ty_args.is_empty() {
        return Ok(NR::err(gas_used_now, E_MISSING_TYPE_ARGUMENT));
    }

    let type_tag = context.type_to_type_tag(&ty_args[0])?;
    let type_str = format!("{}", type_tag);

    if type_str.len() > 256 {
        return Ok(NR::err(gas_used_now, E_TYPE_NAME_TOO_LONG));
    }

    // Serialize object data
    let mut obj_data: Vec<u8> = Vec::new();
    if let Ok(Some(layout)) = context.type_to_type_layout(&ty_args[0]) {
        if let Some(serialized) = obj_val.simple_serialize(&layout) {
            obj_data = serialized;
        } else {
            obj_data.extend_from_slice(type_str.as_bytes());
        }
    } else {
        obj_data.extend_from_slice(type_str.as_bytes());
    }

    let data_len = obj_data.len() as u64;

    // Extract real UID from object data (first 32 bytes)
    // Objects with `key` have `UID` as the first field.
    let object_id_hex = if obj_data.len() >= 32 {
        let uid_bytes = &obj_data[0..32];
        format!("0x{}", hex::encode(uid_bytes))
    } else {
        // Fallback
        use kanari_crypto::hash_data_blake3;
        let mut input = Vec::new();
        input.extend_from_slice(b"Immutable");
        input.extend_from_slice(type_str.as_bytes());
        input.extend_from_slice(&obj_data);
        let hash = hash_data_blake3(&input);
        format!("0x{}", hex::encode(&hash[0..32]))
    };

    {
        let exts = context.extensions_mut();
        let ext = exts.get_mut::<TransferredObjectsExt>();

        let obj = TransferredObject {
            object_id: object_id_hex,
            object_type: type_str.clone(),
            recipient: AccountAddress::ZERO, // Frozen objects don't have a specific owner
            data: obj_data,
            should_persist: true,
            is_frozen: true,
        };
        ext.record(obj);
    }

    drop(obj_val);

    let cost = 2000 + (data_len * 10);
    let gas_cost = GasQuantity::new(cost);

    Ok(NR::ok(gas_used_now + gas_cost, smallvec![]))
}
