// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Internal helper functions for Move execution.

use kanari_system_natives::child::{ChildIndexExt, ChildIndexResolver, ChildLinks, children_key};
use kanari_system_natives::dynamic_field::{DynamicFieldResolver, DynamicFieldStorageExt};
use kanari_system_natives::object::LoadedObjectsExt;
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
        let object_ids = crate::common::ids::object_id_variants(object_id);

        for candidate_id in &object_ids {
            let key = crate::common::keys::object_key(candidate_id);
            if let Some(value) = overlay.and_then(|overlay| overlay.get(&key)) {
                return match value {
                    Some(bytes) => Ok(Some(bcs::from_bytes(bytes)?)),
                    None => Ok(None),
                };
            }
        }

        for candidate_id in &object_ids {
            if let Some(object) = self.object_storage.get_object(candidate_id)? {
                return Ok(Some(object));
            }
        }

        let store = self.state.store();
        for candidate_id in &object_ids {
            let key = crate::common::keys::object_key(candidate_id);
            if let Some(object) =
                store.load::<crate::storage::object_storage::StoredObject>(&key)?
            {
                let _ = self.object_storage.store_object(object.clone());
                return Ok(Some(object));
            }
        }

        let canonical_id = crate::common::ids::canonical_object_id_or_raw(object_id);
        if let Ok(object_addr) =
            move_core_types::account_address::AccountAddress::from_hex_literal(&canonical_id)
            && let Some(object) = self.state.try_get_stored_object(&object_addr)?
        {
            let _ = self.object_storage.store_object(object.clone());
            return Ok(Some(object));
        }

        Ok(None)
    }

    /// Resolve one object id and preload it into the session's
    /// `LoadedObjectsExt`, so `borrow_global` natives can see the object
    /// without the transaction declaring it as an object input.
    ///
    /// `object_id` is used verbatim as the extension's lookup key, so each
    /// caller keeps the form its later lookups expect (`input.object_ref.object_id`,
    /// zero-padded full hex, ...). Ids the resolver cannot find are skipped —
    /// the borrow then fails in-VM against undeclared state, as before
    /// preloading existed.
    fn preload_object_entry(
        &self,
        loaded: &mut LoadedObjectsExt,
        object_id: &str,
        requested_mutable: bool,
        sender: Option<AccountAddress>,
        allow_cross_owner_non_coin: bool,
        overlay: Option<&BTreeMap<Vec<u8>, Option<Vec<u8>>>>,
        origin: &str,
    ) -> anyhow::Result<()> {
        let Some(stored_obj) = self.get_object_for_execution(object_id, overlay)? else {
            return Ok(());
        };
        let can_mutably_borrow = Self::can_mutably_borrow_preloaded_object(
            requested_mutable,
            &stored_obj.type_name,
            &stored_obj.owner_kind,
            stored_obj.owner,
            sender,
            allow_cross_owner_non_coin,
        );
        loaded.insert(
            object_id.to_string(),
            stored_obj.type_name,
            stored_obj.data,
            can_mutably_borrow,
        );
        log::debug!("[RUNTIME] Preloaded {origin} {object_id} into LoadedObjectsExt");
        Ok(())
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
        let loaded = session
            .get_native_extensions()
            .get_mut::<LoadedObjectsExt>();

        // Runs first, on the freshly created session, so declared object
        // inputs always take precedence over the raw-address and well-known
        // singleton preloads that follow.
        for input in object_inputs {
            self.preload_object_entry(
                loaded,
                &input.object_ref.object_id,
                input.mutable,
                sender,
                true,
                overlay,
                "explicit object input",
            )?;
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
        let loaded = session
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
            if loaded.get(&object_id).is_some() {
                continue;
            }

            self.preload_object_entry(
                loaded,
                &object_id,
                true,
                sender,
                false,
                overlay,
                "address-based object argument",
            )?;
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

    /// Canonical full-hex object id of the deny-list singleton, matching
    /// native borrow key construction (`0x` + 64 hex digits).
    pub(crate) fn deny_list_singleton_full_id() -> String {
        let addr = AccountAddress::from_hex_literal(crate::common::ids::DENY_LIST_SINGLETON_ID)
            .expect("hardcoded deny-list singleton id is valid");
        format!("0x{}", hex::encode(addr.to_vec()))
    }

    /// Preload well-known system singletons so framework code can borrow them
    /// without the transaction declaring them (e.g.
    /// `kanari_system::deny_list::borrow_singleton()`).
    ///
    /// Read-only: entries are inserted with `requested_mutable = false` and
    /// never overwrite an explicitly declared input, so writes still require
    /// a declared object input. Missing singletons (stores predating genesis
    /// support) are skipped silently; the borrow then fails in-VM as before.
    pub(crate) fn preload_well_known_singletons(
        &self,
        session: &mut move_vm_runtime::session::Session<
            crate::storage::resolver::KanariMoveResolver,
        >,
        overlay: Option<&BTreeMap<Vec<u8>, Option<Vec<u8>>>>,
    ) -> anyhow::Result<()> {
        let object_id = Self::deny_list_singleton_full_id();
        let loaded = session
            .get_native_extensions()
            .get_mut::<LoadedObjectsExt>();
        if loaded.get(&object_id).is_some() {
            return Ok(());
        }
        self.preload_object_entry(
            loaded,
            &object_id,
            false,
            None,
            false,
            overlay,
            "well-known singleton",
        )
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
