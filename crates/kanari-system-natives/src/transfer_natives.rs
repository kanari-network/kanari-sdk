// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use better_any::{Tid, TidAble};
/// Native functions for object transfer with proper tracking
/// Uses per-execution native context extensions to track transferred objects
use move_core_types::account_address::AccountAddress;
use move_core_types::gas_algebra::{InternalGas, InternalGasPerByte, NumBytes};
use move_vm_runtime::native_charge_gas_early_exit;
use move_vm_runtime::native_functions::NativeContext;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::natives::function::PartialVMError;
use move_vm_types::natives::function::PartialVMResult;
use move_vm_types::pop_arg;
use smallvec::smallvec;
use std::collections::VecDeque;
use std::sync::Arc;

use crate::helpers::{expect_native_signature, make_module_natives};

// Error codes returned by this native (keep values small and stable)
const E_TYPE_NAME_TOO_LONG: u64 = 2;

const TYPE_NAME_MAX_LEN: usize = 256;

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub transfer_with_uid: TransferWithUidGasParameters,
    pub freeze_object: FreezeObjectGasParameters,
}

#[derive(Debug, Clone)]
pub struct TransferWithUidGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

#[derive(Debug, Clone)]
pub struct FreezeObjectGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            transfer_with_uid: TransferWithUidGasParameters {
                base: 0.into(),
                per_byte: 0.into(),
            },
            freeze_object: FreezeObjectGasParameters {
                base: 0.into(),
                per_byte: 0.into(),
            },
        }
    }
}

fn serialize_or_placeholder(
    context: &NativeContext,
    ty: &Type,
    val: &move_vm_types::values::Value,
    placeholder: impl FnOnce(&mut Vec<u8>),
) -> (Vec<u8>, bool) {
    if let Ok(Some(layout)) = context.type_to_type_layout(ty)
        && let Some(bytes) = val.simple_serialize(&layout)
    {
        return (bytes, true);
    }

    let mut data = Vec::new();
    placeholder(&mut data);
    (data, false)
}

fn object_id_hex_from_data(
    recipient: AccountAddress,
    type_str: &str,
    obj_data: &[u8],
    has_full_serialized_data: bool,
    hash_prefix: Option<&[u8]>,
) -> String {
    if has_full_serialized_data && obj_data.len() >= AccountAddress::LENGTH {
        let uid_bytes = &obj_data[..AccountAddress::LENGTH];
        return format!("0x{}", hex::encode(uid_bytes));
    }

    use kanari_crypto::hash_data_blake3;

    let mut input = Vec::new();
    if let Some(prefix) = hash_prefix {
        input.extend_from_slice(prefix);
    } else {
        input.extend_from_slice(recipient.as_ref());
    }
    input.extend_from_slice(type_str.as_bytes());
    input.extend_from_slice(obj_data);

    let hash = hash_data_blake3(&input);
    format!("0x{}", hex::encode(&hash[..AccountAddress::LENGTH]))
}

fn record_transferred_object(context: &mut NativeContext, obj: TransferredObject) {
    crate::native_ext::with_ext_mut_or_default::<TransferredObjectsExt, _>(context, |ext| {
        ext.record(obj)
    });
}

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

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let transfer_params = gas_params.transfer_with_uid;
    let freeze_params = gas_params.freeze_object;

    let transfer_with_uid: NativeFunction = Arc::new(move |context, ty_args, args| {
        native_transfer_with_uid(&transfer_params, context, ty_args, args)
    });
    let freeze_object: NativeFunction = Arc::new(move |context, ty_args, args| {
        native_freeze_object(&freeze_params, context, ty_args, args)
    });
    make_module_natives([
        ("transfer_with_uid", transfer_with_uid),
        ("freeze_object", freeze_object),
    ])
}

// transfer::transfer_with_uid<T: key + store>(obj: T, recipient: address)
// Tracks transferred objects in the transaction-local native context extensions
fn native_transfer_with_uid(
    gas_params: &TransferWithUidGasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<move_vm_types::values::Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(arguments.len(), 2, ty_args.len(), 1)?;

    // Pop arguments: recipient (address), obj (generic T with key+store)
    let recipient = pop_arg!(arguments, AccountAddress);
    let obj_val = arguments.pop_back().ok_or_else(|| {
        PartialVMError::new(move_core_types::vm_status::StatusCode::INTERNAL_TYPE_ERROR)
            .with_message("Missing object argument".to_string())
    })?;

    let ty = &ty_args[0];

    // Extract type information and convert runtime Type -> TypeTag -> human-readable string
    let type_tag = context.type_to_type_tag(ty)?;
    let type_str = format!("{}", type_tag);

    // Guard against pathological type name lengths to avoid excessive native
    // memory usage. Typical Move type names are small; reject overly long
    // values (treat as Move-level error). Adjust threshold if needed.
    if type_str.len() > TYPE_NAME_MAX_LEN {
        return Ok(NR::err(context.gas_used(), E_TYPE_NAME_TOO_LONG));
    }

    native_charge_gas_early_exit!(context, gas_params.base);

    // We DO NOT generate random object IDs or serialize full object data here.
    // Instead we record a deterministic identifier per-execution and leave full
    // object data to be retrieved from the VM changeset / write-set by external systems.

    // Attempt to serialize the transferred object value into BCS so external
    // callers (CLI/RPC) can fetch the object's bytes and use them as function
    // arguments. If serialization fails, fall back to a minimal placeholder
    // (recipient + type) to preserve existing behavior.
    let (obj_data, has_full_serialized_data) =
        serialize_or_placeholder(context, ty, &obj_val, |d| {
            d.extend_from_slice(recipient.as_ref());
            d.extend_from_slice(type_str.as_bytes());
        });

    // Capture data length for gas metering before moving obj_data
    let data_len = obj_data.len() as u64;
    native_charge_gas_early_exit!(context, gas_params.per_byte * NumBytes::new(data_len));

    // Extract real UID from object data (first 32 bytes)
    // In Kanari/Sui Move, objects with `key` have `UID` as the first field.
    // `UID` -> `ID` -> `address` (32 bytes).
    // So the first 32 bytes of the BCS serialized data represent the Object ID.
    let object_id_hex = object_id_hex_from_data(
        recipient,
        &type_str,
        &obj_data,
        has_full_serialized_data,
        None,
    );

    // Store transfer in the transaction-local native context extension
    // Limit the mutable borrow of the extensions to a small scope to avoid
    // borrow conflicts with later calls on `context`.
    let obj = TransferredObject {
        object_id: object_id_hex,
        object_type: type_str,
        recipient,
        data: obj_data,
        should_persist: has_full_serialized_data,
        is_frozen: false,
    };
    record_transferred_object(context, obj);

    Ok(NR::ok(context.gas_used(), smallvec![]))
}

// transfer::freeze_object<T: key + store>(obj: T)
// Freezes the object (makes it immutable) and tracks it in native context
fn native_freeze_object(
    gas_params: &FreezeObjectGasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<move_vm_types::values::Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(arguments.len(), 1, ty_args.len(), 1)?;

    // Pop argument: obj (generic T with key+store)
    let obj_val = arguments.pop_back().ok_or_else(|| {
        PartialVMError::new(move_core_types::vm_status::StatusCode::INTERNAL_TYPE_ERROR)
            .with_message("Missing object argument".to_string())
    })?;

    let ty = &ty_args[0];

    let type_tag = context.type_to_type_tag(ty)?;
    let type_str = format!("{}", type_tag);

    if type_str.len() > TYPE_NAME_MAX_LEN {
        return Ok(NR::err(context.gas_used(), E_TYPE_NAME_TOO_LONG));
    }

    native_charge_gas_early_exit!(context, gas_params.base);

    let (obj_data, has_full_serialized_data) =
        serialize_or_placeholder(context, ty, &obj_val, |d| {
            d.extend_from_slice(type_str.as_bytes())
        });

    let data_len = obj_data.len() as u64;
    native_charge_gas_early_exit!(context, gas_params.per_byte * NumBytes::new(data_len));

    // Extract real UID from object data (first 32 bytes)
    // Objects with `key` have `UID` as the first field.
    let object_id_hex = object_id_hex_from_data(
        AccountAddress::ZERO,
        &type_str,
        &obj_data,
        has_full_serialized_data,
        Some(b"Immutable"),
    );

    let obj = TransferredObject {
        object_id: object_id_hex,
        object_type: type_str,
        recipient: AccountAddress::ZERO, // Frozen objects don't have a specific owner
        data: obj_data,
        should_persist: has_full_serialized_data,
        is_frozen: true,
    };
    record_transferred_object(context, obj);

    Ok(NR::ok(context.gas_used(), smallvec![]))
}
