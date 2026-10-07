// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! MoveRuntime is the core runtime for executing Move transactions in Kanari. It provides an interface for executing Move entry functions, publishing and upgrading modules, and managing the state of Move objects. The runtime handles the interaction between the Move VM, the underlying storage, and the Kanari-specific extensions such as object ownership and event handling.
//!
//! The MoveRuntime is designed to be thread-safe and supports concurrent execution of transactions. It maintains a cache of published modules and provides mechanisms for validating module compatibility during upgrades. The runtime also includes utilities for parsing Move changesets into Kanari-specific state changes, ensuring that the effects of Move transactions are correctly reflected in the Kanari state model.
//!
//! The main components of the MoveRuntime include:
//! - `MoveVM`: The Move virtual machine instance used for executing Move bytecode.
//! - `KanariMoveResolver`: A resolver that provides access to Move modules and resources stored in the Kanari state.
//! - `ObjectStorage`: An abstraction for storing and retrieving Move objects, supporting both in-memory and persistent storage backends.
//!
//! The MoveRuntime provides a high-level API for executing transactions, publishing modules, and managing state, while encapsulating the complexities of the underlying Move VM and storage mechanisms.
//!
//! The MoveRuntime is intended to be used by higher-level components such as the TransactionScheduler, which orchestrates the execution of transactions and manages the overall state of the blockchain.
//!
//! The MoveRuntime is also responsible for ensuring that transactions are executed in a manner consistent with the Kanari protocol, including enforcing ownership semantics, validating object inputs, and applying changesets to the state. It provides a robust foundation for building applications and services on top of the Kanari blockchain platform.
use crate::state::default_owner_kind_for_type;
use crate::storage::resolver::KanariMoveResolver;
use anyhow::{Context, Result, ensure};
use kanari_crypto::hash_data_blake3;
use kanari_system_natives::dynamic_field::{DynamicFieldOp, DynamicFieldsExt};
use kanari_system_natives::event::CapturedEvent;
use kanari_system_natives::event::EventsExt;
use kanari_system_natives::object::{
    BorrowedObject, BorrowedObjectsExt, DeletedObject, DeletedObjectsExt, LoadedObjectsExt,
    SavedObject, SavedObjectsExt,
};
use kanari_system_natives::transfer_natives::{TransferredObject, TransferredObjectsExt};
use kanari_types::clock::{Clock, ClockModule};

use kanari_types::error::KanariUnwrapExt;
use kanari_types::event::Event;
use log::debug;
use move_binary_format::file_format::CompiledModule;
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::{IdentStr, Identifier};
use move_core_types::language_storage::{ModuleId, StructTag, TypeTag};
use move_core_types::runtime_value::{MoveStruct, MoveTypeLayout, MoveValue};
use move_vm_runtime::move_vm::MoveVM;
use move_vm_runtime::native_extensions::NativeContextExtensions;
use move_vm_runtime::native_functions::NativeFunctionTable;
use move_vm_runtime::session::Session;
use sha3::{Digest, Sha3_256};
mod gas_ops;
mod helpers;
pub mod load_system_modules;
mod object_ops;
mod parsers;
use kanari_types::address::Address as KanariAddress;
use kanari_types::gas::GasOperation;
use kanari_types::object::UIDRecord;
use kanari_types::transaction::{ObjectInput, ObjectOwnerKind, ObjectRef};
use kanari_types::tx_context::TxContextModule;
mod move_runtime_extensions;
use crate::changeset::ChangeSet;
use crate::state::StateManager;
use crate::storage::move_vm_state::MoveVMState;
use crate::storage::object_storage::{ObjectStorage, ObjectStore, StoredObject};
use crate::storage::persistent_store::PersistentStore;
use move_binary_format::compatibility::Compatibility;
use move_binary_format::normalized;
use move_bytecode_verifier::verifier::verify_module_unmetered;
use move_vm_types::loaded_data::runtime_types::Type as RuntimeType;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

mod constructors;
mod entry;
mod init;
mod object_lifecycle;
mod publish;
mod view;

#[derive(Clone)]
pub struct MoveRuntime {
    pub(crate) vm: Arc<RwLock<MoveVM>>,
    pub(crate) all_natives: Arc<NativeFunctionTable>,
    pub(crate) resolver: KanariMoveResolver,
    pub(crate) state: MoveVMState,
    pub(crate) published_modules: Arc<RwLock<HashSet<ModuleId>>>,
    pub(crate) object_storage: Arc<dyn ObjectStore>,
    // Cache parsed type tags to avoid repeated string parsing.
    pub(crate) type_tag_cache: Arc<RwLock<HashMap<String, TypeTag>>>,
    // Serialize module publishes so compatibility checks and storage writes cannot race.
    pub(crate) module_publish_lock: Arc<Mutex<()>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModulePublishIntent {
    Publish,
    Upgrade,
    Bootstrap,
}

type LoadedMutableObject = (
    usize,
    String,
    AccountAddress,
    kanari_types::transaction::ObjectOwnerKind,
    String,
    u64,
);

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObjectParamBindingRequirement {
    param_index: usize,
    mutable: bool,
}

/// Everything a finished Move session left behind in its native extensions.
///
/// Draining these once, into a named struct, keeps every execution path
/// (publish `init`, entry call, ...) from re-deriving the same six-field tuple
/// inline and from silently drifting when a new extension is recorded.
struct SessionNativeOutputs {
    transferred: Vec<TransferredObject>,
    events: Vec<CapturedEvent>,
    saved: Vec<SavedObject>,
    deleted: Vec<DeletedObject>,
    dynamic_fields: Vec<DynamicFieldOp>,
    borrowed: Vec<BorrowedObject>,
}

impl SessionNativeOutputs {
    fn drain(exts: &mut NativeContextExtensions<'_>) -> Self {
        Self {
            transferred: exts.get_mut::<TransferredObjectsExt>().take_all(),
            events: exts.get_mut::<EventsExt>().take_all(),
            saved: exts.get_mut::<SavedObjectsExt>().take_all(),
            deleted: exts.get_mut::<DeletedObjectsExt>().take_all(),
            dynamic_fields: exts.get_mut::<DynamicFieldsExt>().take_all(),
            borrowed: exts.get_mut::<BorrowedObjectsExt>().take_all(),
        }
    }

    /// Record events, deletions and dynamic-field ops into the changeset.
    ///
    /// These three carry no ownership or version resolution, so they translate
    /// one-to-one and are safe to apply in any order.
    fn apply_simple_effects(
        cs: &mut ChangeSet,
        events: Vec<CapturedEvent>,
        deleted: Vec<DeletedObject>,
        dynamic_fields: Vec<DynamicFieldOp>,
    ) {
        for ev in events {
            cs.add_event(Event {
                key: ev.key,
                sequence_number: ev.sequence_number,
                type_tag: ev.type_tag,
                event_data: ev.event_data,
            });
        }
        for deleted in deleted {
            cs.add_deleted_object(deleted.object_id);
        }
        for op in dynamic_fields {
            match op {
                DynamicFieldOp::Add {
                    object_id,
                    name_bytes,
                    value_bytes,
                } => {
                    cs.added_dynamic_fields
                        .push((object_id, name_bytes, value_bytes));
                }
                DynamicFieldOp::Remove {
                    object_id,
                    name_bytes,
                } => {
                    cs.removed_dynamic_fields.push((object_id, name_bytes));
                }
            }
        }
    }
}

/// Reads type tags and abilities for a loaded function's parameters.
///
/// Entry-function execution and view calls both need the same three questions
/// per parameter ("what is its type tag?", "is it a `key` struct?", "does it bind
/// a declared object input?"). Bundling them behind one value keeps those
/// answers defined once instead of re-derived as closures at every call site.
struct ParamProbe<'s, 'r, 'l> {
    session: &'s Session<'r, 'l, KanariMoveResolver>,
}

impl<'s, 'r, 'l> ParamProbe<'s, 'r, 'l> {
    fn new(session: &'s Session<'r, 'l, KanariMoveResolver>) -> Self {
        Self { session }
    }

    /// Type tag for a parameter, unwrapping a reference layer when the reference
    /// itself carries no tag.
    fn type_tag_for_param(&self, param_type: &RuntimeType) -> Option<TypeTag> {
        self.session
            .get_type_tag(param_type)
            .ok()
            .or_else(|| match param_type {
                RuntimeType::Reference(inner) | RuntimeType::MutableReference(inner) => {
                    self.session.get_type_tag(inner).ok()
                }
                _ => None,
            })
    }

    /// True when the parameter is a struct carrying the `key` ability.
    fn is_key_struct_param(&self, param_type: &RuntimeType) -> bool {
        MoveRuntime::is_runtime_struct_like(param_type)
            && self
                .session
                .get_type_abilities(param_type)
                .map(|abilities| abilities.has_key())
                .unwrap_or(false)
    }

    /// Declared-object binding requirements for a function signature, in
    /// parameter order. `TxContext` is supplied by the runtime, never bound.
    fn binding_requirements(
        &self,
        parameters: &[RuntimeType],
    ) -> Vec<ObjectParamBindingRequirement> {
        parameters
            .iter()
            .enumerate()
            .filter_map(|(param_index, param_type)| {
                let mutable = MoveRuntime::object_param_mutability(param_type, |t| {
                    self.is_key_struct_param(t)
                })?;
                if self.is_tx_context_param(param_type) {
                    return None;
                }
                Some(ObjectParamBindingRequirement {
                    param_index,
                    mutable,
                })
            })
            .collect()
    }

    fn is_tx_context_param(&self, param_type: &RuntimeType) -> bool {
        matches!(
            self.type_tag_for_param(param_type),
            Some(TypeTag::Struct(struct_tag)) if MoveRuntime::is_tx_context_struct(&struct_tag)
        )
    }
}

#[derive(Clone)]
pub struct EntryFunctionObjectContext {
    pub object_inputs: Vec<ObjectInput>,
    pub sender: Option<AccountAddress>,
    pub gas_info: Option<(u64, u64)>,
    pub timestamp: Option<u64>,
    pub tx_hash: Option<Vec<u8>>,
    pub persist_runtime_state: bool,
    pub state_overlay: Option<crate::StateOverlay>,
}

#[derive(Clone)]
struct ExecutionOptions {
    sender: Option<AccountAddress>,
    gas_info: Option<(u64, u64)>,
    timestamp: Option<u64>,
    tx_hash: Option<Vec<u8>>,
    object_inputs: Vec<ObjectInput>,
    persist_runtime_state: bool,
    bypass_entry_check: bool,
    state_overlay: Option<crate::StateOverlay>,
}

impl ExecutionOptions {
    fn new(
        sender: Option<AccountAddress>,
        gas_info: Option<(u64, u64)>,
        timestamp: Option<u64>,
        tx_hash: Option<Vec<u8>>,
    ) -> Self {
        Self {
            sender,
            gas_info,
            timestamp,
            tx_hash,
            object_inputs: Vec::new(),
            persist_runtime_state: true,
            bypass_entry_check: false,
            state_overlay: None,
        }
    }

    fn with_object_inputs(mut self, object_inputs: Vec<ObjectInput>) -> Self {
        self.object_inputs = object_inputs;
        self
    }

    fn with_persistence(mut self, persist_runtime_state: bool) -> Self {
        self.persist_runtime_state = persist_runtime_state;
        self
    }

    fn with_state_overlay(mut self, state_overlay: Option<crate::StateOverlay>) -> Self {
        self.state_overlay = state_overlay;
        self
    }

    fn bypass_entry_check(mut self) -> Self {
        self.bypass_entry_check = true;
        self
    }
}

impl MoveRuntime {
    fn is_tx_context_struct(struct_tag: &StructTag) -> bool {
        let sys_addr = KanariAddress::kanari_system_account_address();
        struct_tag.address == sys_addr
            && struct_tag.module.as_str() == TxContextModule::TX_CONTEXT_MODULE
            && struct_tag.name.as_str() == TxContextModule::TX_CONTEXT_STRUCT
    }

    fn object_param_mutability<F>(param_type: &RuntimeType, is_key_struct: F) -> Option<bool>
    where
        F: Fn(&RuntimeType) -> bool,
    {
        match param_type {
            // Generic object types such as Coin<T> can lose their key-ability
            // information while resolving a published package's dependency
            // graph. They are still object references when passed by ref.
            RuntimeType::Reference(inner)
                if is_key_struct(inner)
                    || matches!(&**inner, RuntimeType::StructInstantiation(_)) =>
            {
                Some(false)
            }
            RuntimeType::MutableReference(inner)
                if is_key_struct(inner)
                    || matches!(&**inner, RuntimeType::StructInstantiation(_)) =>
            {
                Some(true)
            }
            RuntimeType::Struct(_) | RuntimeType::StructInstantiation(_)
                if is_key_struct(param_type) =>
            {
                Some(true)
            }
            _ => None,
        }
    }

    fn is_runtime_struct_like(ty: &RuntimeType) -> bool {
        matches!(
            ty,
            RuntimeType::Struct(_) | RuntimeType::StructInstantiation(_)
        )
    }

    fn object_digest(data: &[u8]) -> String {
        format!("0x{}", hex::encode(hash_data_blake3(data)))
    }

    fn load_object_by_ref_checked(&self, object_ref: &ObjectRef) -> Result<StoredObject> {
        let stored_obj = self
            .object_storage
            .get_object(&object_ref.object_id)?
            .with_context(|| format!("Object input {} was not found", object_ref.object_id))?;
        if let Some(version) = object_ref.version {
            ensure!(
                stored_obj.version == version,
                "Object input {} version mismatch: expected {}, found {}",
                object_ref.object_id,
                version,
                stored_obj.version
            );
        }
        if let Some(expected_digest) = &object_ref.digest {
            let actual_digest = Self::object_digest(&stored_obj.data);
            ensure!(
                actual_digest == *expected_digest,
                "Object input {} digest mismatch: expected {}, found {}",
                object_ref.object_id,
                expected_digest,
                actual_digest
            );
        }
        Ok(stored_obj)
    }

    #[cfg(test)]
    fn validate_declared_object_input_bindings(
        object_inputs: &[ObjectInput],
        requirements: &[ObjectParamBindingRequirement],
    ) -> Result<()> {
        Self::validate_object_input_bindings(object_inputs, requirements, false)
    }

    fn validate_object_input_bindings(
        object_inputs: &[ObjectInput],
        requirements: &[ObjectParamBindingRequirement],
        allow_extra_dependency_inputs: bool,
    ) -> Result<()> {
        if object_inputs.len() < requirements.len()
            || (!allow_extra_dependency_inputs && object_inputs.len() != requirements.len())
        {
            anyhow::bail!(
                "Declared object_inputs count mismatch: function expects {} object params, got {}",
                requirements.len(),
                object_inputs.len()
            );
        }

        for (input, requirement) in object_inputs.iter().zip(requirements.iter()) {
            ensure!(
                !requirement.mutable || input.mutable,
                "Object input {} mutability does not match function parameter {}",
                input.object_ref.object_id,
                requirement.param_index
            );
            ensure!(
                input.owner.is_some(),
                "Object input {} must declare owner semantics for function parameter {}",
                input.object_ref.object_id,
                requirement.param_index
            );
            if requirement.mutable {
                ensure!(
                    !matches!(input.owner, Some(ObjectOwnerKind::Immutable)),
                    "Immutable object input {} cannot bind to mutable function parameter {}",
                    input.object_ref.object_id,
                    requirement.param_index
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "../../tests/unit/binding_tests.rs"]
mod binding_tests;

#[cfg(test)]
#[path = "../../tests/unit/move_runtime_tests.rs"]
mod move_runtime_tests;
