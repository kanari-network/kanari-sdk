// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Internal helper functions for Move execution.

use kanari_system_natives::child::{
    ChildIndexExt, ChildIndexResolver, ChildLinkOps, ChildLinks, children_key,
};
use kanari_system_natives::dynamic_field::{DynamicFieldResolver, DynamicFieldStorageExt};
use kanari_system_natives::types::{
    CLAIMED_WITNESSES_KEY, ClaimedWitnessesExt, WitnessClaimResolver,
};
use kanari_types::coin::CoinModule;
use kanari_types::transaction::{ObjectInput, ObjectOwnerKind};

use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::StructTag;
use std::collections::BTreeMap;
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

struct RuntimeWitnessClaimResolver {
    store: Arc<crate::storage::persistent_store::PersistentStore>,
    overlay: Option<crate::StateOverlay>,
}

impl WitnessClaimResolver for RuntimeWitnessClaimResolver {
    fn claimed_witnesses(&self) -> Result<Vec<String>, String> {
        if let Some(value) = self
            .overlay
            .as_ref()
            .and_then(|overlay| overlay.get(CLAIMED_WITNESSES_KEY))
        {
            return match value {
                Some(bytes) => bcs::from_bytes(bytes)
                    .map_err(|error| format!("overlay witness-claim decode failed: {error}")),
                None => Ok(Vec::new()),
            };
        }
        self.store
            .load::<Vec<String>>(CLAIMED_WITNESSES_KEY)
            .map(|opt| opt.unwrap_or_default())
            .map_err(|error| format!("persistent witness-claim read failed: {error}"))
    }
}

/// Size of a Move object UID in bytes (address)
const UID_SIZE: usize = 32;
/// Size of a u64 field in bytes
const U64_SIZE: usize = 8;

struct RuntimeChildIndexResolver {
    store: Arc<crate::storage::persistent_store::PersistentStore>,
    overlay: Option<crate::StateOverlay>,
}

impl ChildIndexResolver for RuntimeChildIndexResolver {
    fn child_links(&self, parent_hex: &str) -> Result<ChildLinks, String> {
        let key = children_key(parent_hex);
        if let Some(value) = self.overlay.as_ref().and_then(|overlay| overlay.get(&key)) {
            return match value {
                Some(bytes) => bcs::from_bytes(bytes)
                    .map_err(|error| format!("overlay child-index decode failed: {error}")),
                None => Ok(Vec::new()),
            };
        }
        self.store
            .load::<ChildLinks>(&key)
            .map(|opt| opt.unwrap_or_default())
            .map_err(|error| format!("persistent child-index read failed: {error}"))
    }
}

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

    pub(crate) fn witness_claims_ext(
        &self,
        overlay: Option<crate::StateOverlay>,
    ) -> ClaimedWitnessesExt {
        ClaimedWitnessesExt::new(Arc::new(RuntimeWitnessClaimResolver {
            store: self.state.store(),
            overlay,
        }))
    }

    /// Merge freshly claimed witness types into the durable registry.
    pub(crate) fn persist_witness_claims(
        &self,
        pending: std::collections::BTreeSet<String>,
    ) -> anyhow::Result<()> {
        if pending.is_empty() {
            return Ok(());
        }
        let store = self.state.store();
        let guard = store.transaction_guard();
        let mut claimed: std::collections::BTreeSet<String> = store
            .load::<Vec<String>>(CLAIMED_WITNESSES_KEY)
            .map_err(|error| anyhow::anyhow!("witness claim registry read failed: {error}"))?
            .unwrap_or_default()
            .into_iter()
            .collect();
        claimed.extend(pending);
        let vec: Vec<String> = claimed.into_iter().collect();
        store
            .apply_raw_changes(
                &[(
                    CLAIMED_WITNESSES_KEY.to_vec(),
                    bcs::to_bytes(&vec)
                        .map_err(|error| anyhow::anyhow!("witness claim encode failed: {error}"))?,
                )],
                &[],
            )
            .map_err(|error| anyhow::anyhow!("witness claim persist failed: {error}"))?;
        drop(guard);
        Ok(())
    }

    pub(crate) fn child_index_ext(&self, overlay: Option<crate::StateOverlay>) -> ChildIndexExt {
        ChildIndexExt::new(Arc::new(RuntimeChildIndexResolver {
            store: self.state.store(),
            overlay,
        }))
    }

    /// Merge freshly recorded child links into the durable per-parent index.
    pub(crate) fn persist_child_links(
        &self,
        pending: Vec<(String, Vec<u8>, Option<Vec<u8>>)>,
    ) -> anyhow::Result<()> {
        if pending.is_empty() {
            return Ok(());
        }
        let mut by_parent: std::collections::BTreeMap<String, ChildLinkOps> =
            std::collections::BTreeMap::new();
        for (parent, name, child) in pending {
            by_parent.entry(parent).or_default().push((name, child));
        }
        let store = self.state.store();
        let guard = store.transaction_guard();
        let mut updates = Vec::with_capacity(by_parent.len());
        for (parent, ops) in by_parent {
            let key = children_key(&parent);
            let mut pairs: ChildLinks = store
                .load::<ChildLinks>(&key)
                .map_err(|error| anyhow::anyhow!("child index read failed: {error}"))?
                .unwrap_or_default();
            for (name, child) in ops {
                pairs.retain(|(existing, _)| existing != &name);
                if let Some(child) = child {
                    pairs.push((name, child));
                }
            }
            updates.push((
                key,
                bcs::to_bytes(&pairs)
                    .map_err(|error| anyhow::anyhow!("child index encode failed: {error}"))?,
            ));
        }
        store
            .apply_raw_changes(&updates, &[])
            .map_err(|error| anyhow::anyhow!("child index persist failed: {error}"))?;
        drop(guard);
        Ok(())
    }

    pub(crate) fn get_object_for_execution(
        &self,
        object_id: &str,
        overlay: Option<&BTreeMap<Vec<u8>, Option<Vec<u8>>>>,
    ) -> anyhow::Result<Option<crate::storage::object_storage::StoredObject>> {
        // Resolve via the canonical full form first, then the raw input,
        // so short (`0x2`) and full (`0x00..02`) spellings both hit.
        let canonical = crate::common::ids::canonical_object_id(object_id)
            .unwrap_or_else(|| object_id.to_string());
        let mut candidates = Vec::with_capacity(2);
        candidates.push(canonical.as_str());
        if canonical.as_str() != object_id {
            candidates.push(object_id);
        }
        for candidate in candidates {
            let key = crate::common::keys::object_key(candidate);
            if let Some(value) = overlay.and_then(|overlay| overlay.get(&key)) {
                return match value {
                    Some(bytes) => Ok(Some(bcs::from_bytes(bytes)?)),
                    None => Ok(None),
                };
            }
            if let Some(object) = self.object_storage.get_object(candidate)? {
                return Ok(Some(object));
            }
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
                // Natives resolve via full-form `0x`+64 hex, so preload
                // under the canonical id to avoid short/long mismatches.
                let canonical_id =
                    crate::common::ids::canonical_object_id(&input.object_ref.object_id)
                        .unwrap_or_else(|| input.object_ref.object_id.clone());
                loaded_ext.insert(
                    canonical_id,
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
