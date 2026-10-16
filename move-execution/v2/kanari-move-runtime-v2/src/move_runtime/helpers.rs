// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Internal helper functions for Move execution.

use kanari_system_natives::child::{ChildIndexExt, ChildIndexResolver, ChildLinks, children_key};
use kanari_system_natives::dynamic_field::{DynamicFieldResolver, DynamicFieldStorageExt};
use kanari_system_natives::types::{
    CLAIMED_WITNESSES_KEY, ClaimedWitnessesExt, WitnessClaimResolver,
};
use kanari_types::coin::CoinModule;
use kanari_types::transaction::{ObjectInput, ObjectOwnerKind};

use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::StructTag;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

struct RuntimeDynamicFieldResolver {
    store: Arc<crate::storage::persistent_store::PersistentStore>,
    overlay: Option<crate::StateOverlay>,
}

impl RuntimeDynamicFieldResolver {
    fn dynamic_field_key(object_id: &str, name_bytes: &[u8]) -> Vec<u8> {
        crate::common::keys::dynamic_field_key(object_id, name_bytes)
    }
}

impl DynamicFieldResolver for RuntimeDynamicFieldResolver {
    fn get_dynamic_field(
        &self,
        object_id: &str,
        name_bytes: &[u8],
    ) -> Result<Option<Vec<u8>>, String> {
        let key = Self::dynamic_field_key(object_id, name_bytes);
        if let Some(value) = self.overlay.as_ref().and_then(|overlay| overlay.get(&key)) {
            return match value {
                Some(bytes) => bcs::from_bytes(bytes)
                    .map(Some)
                    .map_err(|error| format!("overlay dynamic-field decode failed: {error}")),
                None => Ok(None),
            };
        }
        self.store
            .load::<Vec<u8>>(&key)
            .map_err(|error| format!("persistent dynamic-field read failed: {error}"))
    }
}

/// Read a durable metadata entry, letting a transaction's state overlay shadow
/// the persistent store. Returns `None` when the key is absent (or tombstones
/// a store entry with overlay `None`).
///
/// Canonical storage keys written through `ChangeSet::move_writes` hold a
/// length-prefixed `Vec<u8>` blob (see `StateManager::save_internal`), so the
/// raw bytes are unwrapped before decoding the typed metadata value.
fn load_metadata<T: serde::de::DeserializeOwned>(
    store: &crate::storage::persistent_store::PersistentStore,
    overlay: Option<&BTreeMap<Vec<u8>, Option<Vec<u8>>>>,
    key: &[u8],
) -> anyhow::Result<Option<T>> {
    let raw: Option<Vec<u8>> = match overlay.and_then(|overlay| overlay.get(key)) {
        Some(Some(bytes)) => Some(bcs::from_bytes(bytes)?),
        Some(None) => return Ok(None),
        None => store.load::<Vec<u8>>(key)?,
    };
    match raw {
        Some(bytes) => Ok(Some(bcs::from_bytes(&bytes)?)),
        None => Ok(None),
    }
}

/// Host view of the persisted one-time-witness claim registry.
struct RuntimeWitnessClaimResolver {
    store: Arc<crate::storage::persistent_store::PersistentStore>,
    overlay: Option<crate::StateOverlay>,
}

impl WitnessClaimResolver for RuntimeWitnessClaimResolver {
    fn claimed_witnesses(&self) -> Result<Vec<String>, String> {
        load_metadata::<Vec<String>>(&self.store, self.overlay.as_deref(), CLAIMED_WITNESSES_KEY)
            .map(|claimed| claimed.unwrap_or_default())
            .map_err(|error| format!("persistent witness-claim read failed: {error}"))
    }
}

/// Host view of the persisted child-object link index.
struct RuntimeChildIndexResolver {
    store: Arc<crate::storage::persistent_store::PersistentStore>,
    overlay: Option<crate::StateOverlay>,
}

impl ChildIndexResolver for RuntimeChildIndexResolver {
    fn child_links(&self, parent_hex: &str) -> Result<ChildLinks, String> {
        let key = children_key(parent_hex);
        load_metadata::<ChildLinks>(&self.store, self.overlay.as_deref(), &key)
            .map(|links| links.unwrap_or_default())
            .map_err(|error| format!("persistent child-index read failed: {error}"))
    }
}

/// Size of a Move object UID in bytes (address)
const UID_SIZE: usize = 32;
/// Size of a u64 field in bytes
const U64_SIZE: usize = 8;

impl super::MoveRuntime {
    pub(crate) fn dynamic_field_storage_ext(
        &self,
        overlay: Option<crate::StateOverlay>,
    ) -> DynamicFieldStorageExt {
        DynamicFieldStorageExt::new(Arc::new(RuntimeDynamicFieldResolver {
            store: self.state.store(),
            overlay,
        }))
    }

    /// Extension giving `kanari_system::types` natives read access to the
    /// durable witness-claim registry.
    pub(crate) fn claim_witness_ext(
        &self,
        overlay: Option<crate::StateOverlay>,
    ) -> ClaimedWitnessesExt {
        ClaimedWitnessesExt::new(Arc::new(RuntimeWitnessClaimResolver {
            store: self.state.store(),
            overlay,
        }))
    }

    /// Extension giving `kanari_system::dynamic_object_field` natives read
    /// access to the durable child-object link index.
    pub(crate) fn child_index_ext(&self, overlay: Option<crate::StateOverlay>) -> ChildIndexExt {
        ChildIndexExt::new(Arc::new(RuntimeChildIndexResolver {
            store: self.state.store(),
            overlay,
        }))
    }

    /// Translate this transaction's drained witness-claim and child-link
    /// mutations into canonical storage writes (`ChangeSet::move_writes`), so
    /// they commit with the rest of the changeset exactly once. Base state is
    /// read from the overlay (shadowing the store) so a replayed transaction
    /// merges against the state it observed.
    pub(crate) fn record_session_metadata_writes(
        &self,
        cs: &mut crate::changeset::ChangeSet,
        claimed_witnesses: BTreeSet<String>,
        child_links: Vec<(String, Vec<u8>, Option<Vec<u8>>)>,
        overlay: Option<&BTreeMap<Vec<u8>, Option<Vec<u8>>>>,
    ) -> anyhow::Result<()> {
        let store = self.state.store();

        if !claimed_witnesses.is_empty() {
            let mut claimed: BTreeSet<String> =
                load_metadata::<Vec<String>>(&store, overlay, CLAIMED_WITNESSES_KEY)?
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
            claimed.extend(claimed_witnesses);
            let rendered: Vec<String> = claimed.into_iter().collect();
            cs.record_move_write(
                CLAIMED_WITNESSES_KEY.to_vec(),
                Some(bcs::to_bytes(&rendered)?),
            );
        }

        // Apply each parent's ops in call order over its durable base, then
        // write the merged list once per parent.
        let mut parents: Vec<String> = Vec::new();
        for (parent, _, _) in &child_links {
            if !parents.iter().any(|existing| existing == parent) {
                parents.push(parent.clone());
            }
        }
        for parent in parents {
            let key = children_key(&parent);
            let mut links: ChildLinks =
                load_metadata::<ChildLinks>(&store, overlay, &key)?.unwrap_or_default();
            for (_, name, child) in child_links
                .iter()
                .filter(|(op_parent, _, _)| op_parent == &parent)
            {
                links.retain(|(existing, _)| existing != name);
                if let Some(child) = child {
                    links.push((name.clone(), child.clone()));
                }
            }
            cs.record_move_write(key, Some(bcs::to_bytes(&links)?));
        }

        Ok(())
    }

    pub(crate) fn get_object_for_execution(
        &self,
        object_id: &str,
        overlay: Option<&BTreeMap<Vec<u8>, Option<Vec<u8>>>>,
    ) -> anyhow::Result<Option<crate::storage::object_storage::StoredObject>> {
        let key = crate::common::keys::object_key(object_id);
        if let Some(value) = overlay.and_then(|overlay| overlay.get(&key)) {
            return match value {
                Some(bytes) => Ok(Some(bcs::from_bytes(bytes)?)),
                None => Ok(None),
            };
        }
        if let Some(object) = self.object_storage.get_object(object_id)? {
            return Ok(Some(object));
        }

        let object_addr =
            match move_core_types::account_address::AccountAddress::from_hex_literal(object_id) {
                Ok(addr) => addr,
                Err(_) => return Ok(None),
            };
        let Some(object) = self.state.try_get_stored_object(&object_addr)? else {
            return Ok(None);
        };

        // Hydrate the runtime object cache so subsequent object-native borrows
        // in this process do not miss objects that already exist in state.
        let _ = self.object_storage.store_object(object.clone());
        Ok(Some(object))
    }

    /// Preload potential object arguments into LoadedObjectsExt before execution
    /// This enables native_borrow_global and borrow_global_mut to resolve objects during VM execution
    pub(crate) fn preload_objects_for_execution(
        &self,
        session: &mut move_vm_runtime::session::Session<
            crate::storage::resolver::KanariMoveResolver,
        >,
        object_inputs: &[ObjectInput],
        sender: Option<AccountAddress>,
        overlay: Option<&BTreeMap<Vec<u8>, Option<Vec<u8>>>>,
    ) -> anyhow::Result<()> {
        use kanari_system_natives::object::LoadedObjectsExt;

        let exts = session.get_native_extensions();
        let loaded_ext = exts.get_mut::<LoadedObjectsExt>();

        for input in object_inputs {
            if let Some(stored_obj) =
                self.get_object_for_execution(&input.object_ref.object_id, overlay)?
            {
                let can_mutably_borrow = Self::can_mutably_borrow_preloaded_object(
                    input.mutable,
                    &stored_obj.type_name,
                    &stored_obj.owner_kind,
                    stored_obj.owner,
                    sender,
                    true,
                );
                loaded_ext.insert(
                    input.object_ref.object_id.clone(),
                    stored_obj.type_name,
                    stored_obj.data,
                    can_mutably_borrow,
                );
                log::debug!(
                    "[RUNTIME] Preloaded explicit object input {} into LoadedObjectsExt",
                    input.object_ref.object_id
                );
            }
        }

        Ok(())
    }

    /// Preload object IDs passed as raw address arguments. Raw address args are
    /// allowed to mutably borrow only sender-owned or shared objects; cross-owner
    /// mutable app-object access must be declared through explicit object_inputs
    /// so validation and scheduling can observe the dependency.
    pub(crate) fn preload_object_ids_from_args(
        &self,
        session: &mut move_vm_runtime::session::Session<
            crate::storage::resolver::KanariMoveResolver,
        >,
        args: &[Vec<u8>],
        sender: Option<AccountAddress>,
        overlay: Option<&BTreeMap<Vec<u8>, Option<Vec<u8>>>>,
    ) -> anyhow::Result<()> {
        use kanari_system_natives::object::LoadedObjectsExt;

        let loaded_ext = session
            .get_native_extensions()
            .get_mut::<LoadedObjectsExt>();
        for arg in args {
            if arg.is_empty() || arg.len() > 32 {
                continue;
            }
            // Raw id bytes may carry stripped leading zeros after a display
            // round-trip; pad them back so they still resolve. Unresolvable
            // ids keep prior behavior (skipped below).
            let mut id_bytes = arg.clone();
            if id_bytes.len() < 32 {
                let mut padded = vec![0u8; 32 - id_bytes.len()];
                padded.append(&mut id_bytes);
                id_bytes = padded;
            }

            let object_id = format!("0x{}", hex::encode(&id_bytes));
            if loaded_ext.get(&object_id).is_some() {
                continue;
            }

            let Some(stored_obj) = self.get_object_for_execution(&object_id, overlay)? else {
                continue;
            };

            let can_mutably_borrow = Self::can_mutably_borrow_preloaded_object(
                true,
                &stored_obj.type_name,
                &stored_obj.owner_kind,
                stored_obj.owner,
                sender,
                false,
            );
            loaded_ext.insert(
                object_id.clone(),
                stored_obj.type_name,
                stored_obj.data,
                can_mutably_borrow,
            );
            log::debug!(
                "[RUNTIME] Preloaded address-based object argument {} into LoadedObjectsExt",
                object_id
            );
        }

        Ok(())
    }

    pub(crate) fn can_mutably_borrow_preloaded_object(
        requested_mutable: bool,
        object_type: &str,
        owner_kind: &ObjectOwnerKind,
        owner: AccountAddress,
        sender: Option<AccountAddress>,
        allow_cross_owner_non_coin: bool,
    ) -> bool {
        if !requested_mutable || matches!(owner_kind, ObjectOwnerKind::Immutable) {
            return false;
        }
        if matches!(owner_kind, ObjectOwnerKind::Shared) {
            return true;
        }
        if sender == Some(owner) {
            return true;
        }

        // Cross-owner mutable access to app objects must be declared as an
        // explicit object input so the transaction exposes its dependency to
        // validation/scheduling. Raw address arguments are intentionally scoped
        // to the sender's own owned objects.
        allow_cross_owner_non_coin && !Self::is_coin_object_type(object_type)
    }

    pub(crate) fn is_coin_object_type(object_type: &str) -> bool {
        object_type.contains("::coin::Coin<") || object_type.contains("::coin::coin::Coin<")
    }

    /// Check if struct tag represents a treasury resource
    pub(crate) fn is_treasury_resource(&self, struct_tag: &StructTag) -> bool {
        struct_tag.name.as_str() == CoinModule::TREASURY_CAP_STRUCT
    }

    /// Extract total supply from TreasuryCap bytes
    pub(crate) fn extract_treasury_total_from_bytes(&self, bytes: &[u8]) -> Option<u64> {
        // TreasuryCap: [32-byte address][8-byte total_supply]
        if bytes.len() >= (UID_SIZE + U64_SIZE) {
            let supply_bytes: [u8; U64_SIZE] =
                bytes[UID_SIZE..(UID_SIZE + U64_SIZE)].try_into().ok()?;
            Some(u64::from_le_bytes(supply_bytes))
        } else {
            None
        }
    }
}
