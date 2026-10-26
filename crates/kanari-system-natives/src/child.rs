// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Child-object link index backing `kanari_system::dynamic_object_field`.
//!
//! Unlike Sui (where child objects stay live in storage), this runtime
//! stores dynamic-field values as bytes. This module keeps the difference
//! observable instead of silent: every dynamic-object-field add/remove
//! records a `(parent, name) -> child` link, and `list_children` enumerates
//! a parent's children for external tools.
//!
//! Reads merge this transaction's pending ops over the injected resolver
//! (overlay + durable state). The index is metadata only; object contents
//! still flow through the regular dynamic-field path.

use better_any::{Tid, TidAble};
use move_core_types::account_address::AccountAddress;
use move_core_types::gas_algebra::{InternalGas, InternalGasPerByte, NumBytes};
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

use crate::helpers::{expect_native_signature, make_module_natives};

/// Pending link op that the host could not record.
pub const E_CHILD_INDEX_FAILED: u64 = 1;

/// Key prefix for per-parent link lists (`children:{parent_hex}` holding
/// BCS `Vec<(name_bytes, child_addr)>`).
pub const CHILDREN_KEY_PREFIX: &[u8] = b"children:";

/// Storage key for one parent's link list. The runtime must use this exact
/// construction so reads and writes agree.
pub fn children_key(parent_hex: &str) -> Vec<u8> {
    let mut key = CHILDREN_KEY_PREFIX.to_vec();
    key.extend_from_slice(parent_hex.as_bytes());
    key
}

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub record: InternalGas,
    pub unrecord: InternalGas,
    pub list: InternalGas,
    /// Per-byte rate over serialized names/addresses.
    pub per_byte: InternalGasPerByte,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            record: 0.into(),
            unrecord: 0.into(),
            list: 0.into(),
            per_byte: 0.into(),
        }
    }

    pub fn production() -> Self {
        Self {
            record: 4_000.into(),
            unrecord: 4_000.into(),
            list: 2_000.into(),
            per_byte: 50.into(),
        }
    }
}

/// One child link: a serialized dynamic-field name paired with the child
/// object's UID address bytes.
pub type ChildLink = (Vec<u8>, Vec<u8>);

/// A parent's ordered child links, as persisted under its children key.
pub type ChildLinks = Vec<ChildLink>;

/// Transaction-local child-link mutations for one parent, in call order:
/// `(serialized name, Some(child bytes))` records a link and
/// `(name, None)` removes one.
pub type ChildLinkOps = Vec<(Vec<u8>, Option<Vec<u8>>)>;

/// Read-only view of the persisted child index, provided by the host.
pub trait ChildIndexResolver: Send + Sync {
    /// Decoded `(name_bytes, child_addr)` pairs for `parent_hex`
    /// (empty when the parent has no index entry).
    fn child_links(&self, parent_hex: &str) -> Result<ChildLinks, String>;
}

/// Transaction-local record of child-link mutations plus read access to
/// the persisted index. Flushed to durable state by the host after a
/// successful transaction.
#[derive(Tid, Clone, Default)]
pub struct ChildIndexExt {
    resolver: Option<Arc<dyn ChildIndexResolver>>,
    /// `(parent_hex, name_bytes, child_bytes|None)` in call order.
    pending: Vec<(String, Vec<u8>, Option<Vec<u8>>)>,
}

impl ChildIndexExt {
    pub fn new(resolver: Arc<dyn ChildIndexResolver>) -> Self {
        Self {
            resolver: Some(resolver),
            pending: Vec::new(),
        }
    }

    pub fn record(&mut self, parent_hex: String, name_bytes: Vec<u8>, child_bytes: Vec<u8>) {
        self.pending
            .push((parent_hex, name_bytes, Some(child_bytes)));
    }

    pub fn unrecord(&mut self, parent_hex: String, name_bytes: Vec<u8>) {
        self.pending.push((parent_hex, name_bytes, None));
    }

    /// Drain accumulated mutations for the host to persist.
    pub fn take_pending(&mut self) -> Vec<(String, Vec<u8>, Option<Vec<u8>>)> {
        std::mem::take(&mut self.pending)
    }

    /// Children of `parent_hex` in insertion order: durable base (via the
    /// injected resolver, if any) with this transaction's pending ops
    /// applied on top. With no resolver (bare unit runner) only pending
    /// ops are visible.
    pub fn read_children(&self, parent_hex: &str) -> Result<Vec<AccountAddress>, String> {
        let mut pairs: ChildLinks = Vec::new();
        if let Some(resolver) = &self.resolver {
            pairs = resolver.child_links(parent_hex)?;
        }
        for (parent, name, child) in &self.pending {
            if parent != parent_hex {
                continue;
            }
            pairs.retain(|(existing, _)| existing != name);
            if let Some(child) = child {
                pairs.push((name.clone(), child.clone()));
            }
        }
        let mut out = Vec::with_capacity(pairs.len());
        for (_, child) in pairs {
            let addr = AccountAddress::from_bytes(&child)
                .map_err(|_| "child index holds a malformed address".to_string())?;
            out.push(addr);
        }
        Ok(out)
    }
}

fn object_id_from_uid_ref(uid_ref: Reference) -> PartialVMResult<String> {
    let uid_val = uid_ref.read_ref()?;
    let uid_bytes = crate::object::uid_address_bytes(&uid_val).ok_or_else(|| {
        PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
            .with_message("Failed to serialize UID".to_string())
    })?;
    kanari_types::object::object_id_from_address_bytes(&uid_bytes).ok_or_else(|| {
        PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
            .with_message("UID did not serialize to a 32-byte address".to_string())
    })
}

fn serialize_with_layout(
    context: &mut NativeContext,
    ty: &Type,
    value: &Value,
) -> PartialVMResult<Vec<u8>> {
    let layout = context.type_to_type_layout(ty)?.ok_or_else(|| {
        PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
            .with_message("Type layout unavailable for child index key".to_string())
    })?;
    value.simple_serialize(&layout).ok_or_else(|| {
        PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
            .with_message("Child index key serialization failed".to_string())
    })
}

fn canonical_type_string(context: &mut NativeContext, ty: &Type) -> PartialVMResult<String> {
    Ok(format!("{}", context.type_to_type_tag(ty)?))
}

/// `dynamic_object_field::record_child(parent, name, child_obj)`.
/// Records the `(parent, name) -> child` link; the value itself is stored
/// by the regular dynamic-field path.
fn native_record_child(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(arguments.len(), 3, ty_args.len(), 2)?;

    let child_ref = pop_arg!(arguments, Reference);
    let name_val = arguments.pop_back().ok_or_else(|| {
        PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
            .with_message("Missing name argument".to_string())
    })?;
    let uid_ref = pop_arg!(arguments, Reference);

    let parent_hex = object_id_from_uid_ref(uid_ref)?;
    let name_bytes = serialize_with_layout(context, &ty_args[0], &name_val)?;
    let child_val = child_ref.read_ref()?;
    let child_layout = serialize_with_layout(context, &ty_args[1], &child_val)?;
    // Object identity is the leading UID address (first 32 bytes), the
    // same convention transfer tracking relies on.
    if child_layout.len() < AccountAddress::LENGTH {
        return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
            .with_message("Child object has no UID".to_string()));
    }
    let child_bytes = child_layout[..AccountAddress::LENGTH].to_vec();
    // Touch the type so malformed instantiations fail loudly here.
    let _ = canonical_type_string(context, &ty_args[1])?;

    native_charge_gas_early_exit!(context, gas_params.record);
    native_charge_gas_early_exit!(
        context,
        gas_params.per_byte * NumBytes::new((name_bytes.len() + child_bytes.len()) as u64)
    );

    crate::native_ext::with_ext_mut_or_default::<ChildIndexExt, _>(context, |ext| {
        ext.record(parent_hex, name_bytes, child_bytes);
    });
    // Absent extension (bare unit runner): record is a no-op there.
    Ok(NR::ok(context.gas_used(), smallvec![]))
}

/// `dynamic_object_field::unrecord_child(parent, name)`.
fn native_unrecord_child(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(arguments.len(), 2, ty_args.len(), 1)?;

    let name_val = arguments.pop_back().ok_or_else(|| {
        PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
            .with_message("Missing name argument".to_string())
    })?;
    let uid_ref = pop_arg!(arguments, Reference);

    let parent_hex = object_id_from_uid_ref(uid_ref)?;
    let name_bytes = serialize_with_layout(context, &ty_args[0], &name_val)?;

    native_charge_gas_early_exit!(context, gas_params.unrecord);
    native_charge_gas_early_exit!(
        context,
        gas_params.per_byte * NumBytes::new(name_bytes.len() as u64)
    );

    crate::native_ext::with_ext_mut_or_default::<ChildIndexExt, _>(context, |ext| {
        ext.unrecord(parent_hex, name_bytes);
    });
    Ok(NR::ok(context.gas_used(), smallvec![]))
}

/// `dynamic_object_field::list_children(parent): vector<address>`.
fn native_list_children(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(arguments.len(), 1, ty_args.len(), 0)?;

    let uid_ref = pop_arg!(arguments, Reference);
    let parent_hex = object_id_from_uid_ref(uid_ref)?;

    native_charge_gas_early_exit!(context, gas_params.list);

    // Reads merge pending ops over the injected resolver (overlay +
    // durable state). With no resolver (bare unit runner) only pending
    // ops are visible — documented fail-open for tests.
    let mut outcome: Option<Result<Vec<AccountAddress>, String>> = None;
    crate::native_ext::with_ext_mut_or_default::<ChildIndexExt, _>(context, |ext| {
        outcome = Some(ext.read_children(&parent_hex));
    });
    let children = match outcome {
        Some(Ok(children)) => children,
        Some(Err(error)) => {
            return Err(PartialVMError::new(StatusCode::ABORTED)
                .with_sub_status(E_CHILD_INDEX_FAILED)
                .with_message(error));
        }
        None => Vec::new(),
    };

    native_charge_gas_early_exit!(
        context,
        gas_params.per_byte * NumBytes::new((children.len() * AccountAddress::LENGTH) as u64)
    );

    Ok(NR::ok(
        context.gas_used(),
        smallvec![Value::vector_address(children)],
    ))
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let record_gas = gas_params.clone();
    let unrecord_gas = gas_params.clone();
    let list_gas = gas_params;

    let record_child: NativeFunction = Arc::new(move |context, ty_args, args| {
        native_record_child(&record_gas, context, ty_args, args)
    });
    let unrecord_child: NativeFunction = Arc::new(move |context, ty_args, args| {
        native_unrecord_child(&unrecord_gas, context, ty_args, args)
    });
    let list_children: NativeFunction = Arc::new(move |context, ty_args, args| {
        native_list_children(&list_gas, context, ty_args, args)
    });

    make_module_natives([
        ("record_child", record_child),
        ("unrecord_child", unrecord_child),
        ("list_children", list_children),
    ])
}
