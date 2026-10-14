// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;

/// Mirror of `kanari_system::deny_list` storage structs for genesis
/// construction. Field order must match the Move declarations.
#[derive(serde::Serialize)]
struct GenesisBag {
    id: UIDRecord,
    size: u64,
}

#[derive(serde::Serialize)]
struct GenesisTable {
    id: UIDRecord,
    size: u64,
}

#[derive(serde::Serialize)]
struct GenesisPerTypeList {
    id: UIDRecord,
    denied_count: GenesisTable,
}

#[derive(serde::Serialize)]
struct GenesisDenyList {
    id: UIDRecord,
    lists: GenesisBag,
}

impl MoveRuntime {
    /// Convert an object-id string into a `UIDRecord` when possible.
    fn uid_from_object_id(object_id: &str) -> Option<kanari_types::object::UIDRecord> {
        AccountAddress::from_hex_literal(object_id)
            .ok()
            .map(kanari_types::object::UIDRecord::new)
    }

    /// Convert an object-id string into an `IDRecord` when possible.
    fn id_from_object_id(object_id: &str) -> Option<kanari_types::object::IDRecord> {
        AccountAddress::from_hex_literal(object_id)
            .ok()
            .map(kanari_types::object::IDRecord::new)
    }

    fn maybe_add_token_balance(
        &self,
        cs: &mut ChangeSet,
        owner: AccountAddress,
        type_name: &str,
        data: &[u8],
        object_id: &str,
        source: &str,
    ) {
        if let Ok(struct_tag) = type_name.parse::<move_core_types::language_storage::StructTag>()
            && crate::common::balance::is_balance_struct(&struct_tag)
            && let Some(amount) =
                crate::common::balance::extract_balance_from_object_bytes(data, &struct_tag)
            && let Some(token_type) =
                crate::common::balance::token_type_from_struct_tag(&struct_tag)
        {
            cs.add_token_balance_set(owner, token_type.clone(), amount);
            debug!(
                "[RUNTIME] Extracted balance from {} object {}: {} = {}",
                source, object_id, token_type, amount
            );
        }
    }

    /// Resolve ownership for one `save`/`borrow_global_mut` record and upsert it.
    fn upsert_native_written_object(
        &self,
        cs: &mut ChangeSet,
        loaded_mutable_objects: &[LoadedMutableObject],
        object_id: &str,
        object_type: &str,
        data: Vec<u8>,
        source: &str,
    ) -> Result<()> {
        let Some((owner, owner_kind, version)) =
            self.resolve_saved_owner_metadata(loaded_mutable_objects, object_id)?
        else {
            // Unknown (embedded) value: intentionally not persisted. See
            // `resolve_saved_owner_metadata`.
            return Ok(());
        };
        self.upsert_created_object(
            cs,
            owner,
            owner_kind,
            object_id,
            object_type,
            data,
            version,
            source,
        );
        Ok(())
    }

    /// Upsert every object recorded by `save` and `borrow_global_mut`.
    ///
    /// Both extensions carry the same shape (id, type, data) and need the same
    /// ownership/version resolution, so the two loops live side by side here.
    /// `processed_ids` is threaded through so a caller can skip objects already
    /// accounted for by a mutable-reference writeback, and so the ids handled
    /// here are excluded from later passes; pass an empty set to keep all.
    pub(super) fn upsert_saved_and_borrowed_objects(
        &self,
        cs: &mut ChangeSet,
        loaded_mutable_objects: &[LoadedMutableObject],
        saved: Vec<SavedObject>,
        borrowed: Vec<BorrowedObject>,
        saved_source: &str,
        borrowed_source: &str,
        processed_ids: &mut HashSet<String>,
    ) -> Result<()> {
        for saved in saved {
            if !processed_ids.insert(saved.object_id.clone()) {
                continue;
            }
            self.upsert_native_written_object(
                cs,
                loaded_mutable_objects,
                &saved.object_id,
                &saved.object_type,
                saved.data,
                saved_source,
            )?;
        }
        for borrowed in borrowed {
            if !processed_ids.insert(borrowed.object_id.clone()) {
                continue;
            }
            self.upsert_native_written_object(
                cs,
                loaded_mutable_objects,
                &borrowed.object_id,
                &borrowed.object_type,
                borrowed.data,
                borrowed_source,
            )?;
        }
        Ok(())
    }

    fn resolve_saved_owner_metadata(
        &self,
        loaded_mutable_objects: &[LoadedMutableObject],
        object_id: &str,
    ) -> anyhow::Result<
        Option<(
            AccountAddress,
            kanari_types::transaction::ObjectOwnerKind,
            u64,
        )>,
    > {
        if let Some((_, _, owner, owner_kind, _, version)) = loaded_mutable_objects
            .iter()
            .find(|(_, id, _, _, _, _)| id == object_id)
        {
            return Ok(Some((*owner, owner_kind.clone(), *version + 1)));
        }
        if let Some(stored) = self.object_storage.get_object(object_id)? {
            return Ok(Some((stored.owner, stored.owner_kind, stored.version + 1)));
        }
        // Unknown id: this is a `save_object` on an embedded (non-standalone)
        // value, e.g. a coin nested inside a wallet. Persisting it would
        // fabricate a ZERO-owned ghost duplicate alongside the real parent
        // object (which carries the embedded bytes itself), so skip it.
        // Genuine top-level objects always resolve through a declared input
        // (transfer/save paths) or existing storage.
        log::debug!(
            "[RUNTIME] Skipping save of unknown object id {} (embedded value?)",
            object_id
        );
        Ok(None)
    }

    pub(super) fn build_created_object(
        owner: AccountAddress,
        owner_kind: kanari_types::transaction::ObjectOwnerKind,
        object_id: &str,
        type_name: &str,
        data: Vec<u8>,
        version: u64,
    ) -> crate::changeset::CreatedObject {
        crate::changeset::CreatedObject {
            owner,
            owner_kind,
            uid: Self::uid_from_object_id(object_id),
            id: Self::id_from_object_id(object_id),
            type_: type_name.to_string(),
            data,
            version,
        }
    }

    pub(super) fn upsert_created_object(
        &self,
        cs: &mut ChangeSet,
        owner: AccountAddress,
        owner_kind: kanari_types::transaction::ObjectOwnerKind,
        object_id: &str,
        type_name: &str,
        data: Vec<u8>,
        version: u64,
        source: &str,
    ) {
        self.maybe_add_token_balance(cs, owner, type_name, &data, object_id, source);
        let updated_obj =
            Self::build_created_object(owner, owner_kind, object_id, type_name, data, version);
        cs.created_objects.retain(|(k, _)| k != object_id);
        cs.created_objects
            .push((object_id.to_string(), updated_obj));
    }

    /// Persist created objects from a changeset to the object store.
    pub fn persist_created_objects(&self, cs: &ChangeSet) -> Result<()> {
        for (id, created) in &cs.created_objects {
            let stored = StoredObject {
                id: id.clone(),
                owner: created.owner,
                owner_kind: created.owner_kind.clone(),
                type_name: created.type_.clone(),
                data: created.data.clone(),
                version: created.version,
            };
            self.object_storage
                .store_object(stored)
                .map_err(anyhow::Error::new)?;
        }
        Ok(())
    }

    /// Persist deleted objects from a changeset to the object store.
    pub fn persist_deleted_objects(&self, cs: &ChangeSet) -> Result<()> {
        for obj_id in &cs.deleted_objects {
            self.object_storage
                .delete_object(obj_id)
                .map_err(anyhow::Error::new)?;
        }
        // A deleted parent must not leave its child-index entry behind:
        // drop the whole per-parent link list (children themselves are
        // unaffected — only the index entry goes).
        let mut deletes = Vec::with_capacity(cs.deleted_objects.len());
        for obj_id in &cs.deleted_objects {
            let full =
                crate::common::ids::canonical_object_id(obj_id).unwrap_or_else(|| obj_id.clone());
            deletes.push(kanari_system_natives::child::children_key(&full));
        }
        if !deletes.is_empty() {
            self.state
                .store()
                .apply_raw_changes(&[], &deletes)
                .map_err(anyhow::Error::new)?;
        }
        Ok(())
    }

    /// Preload an object snapshot into the object cache before execution.
    pub fn preload_object_snapshot(
        &self,
        object_id: &str,
        owner: AccountAddress,
        type_name: &str,
        data: Vec<u8>,
        version: u64,
    ) -> Result<()> {
        let stored = StoredObject {
            id: object_id.to_string(),
            owner,
            owner_kind: default_owner_kind_for_type(type_name, owner),
            type_name: type_name.to_string(),
            data,
            version,
        };
        self.object_storage
            .store_object(stored)
            .require("Object storage operation failed")
    }

    /// Ensure the system clock object exists, creating it via VM or native path as needed.
    pub fn ensure_system_clock(&self, state: &mut StateManager) -> Result<AccountAddress> {
        if let Some(id) = state.get_system_clock_object_id()? {
            return Ok(id);
        }

        if !std::env::var("KANARI_CLOCK_CREATE_VM")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false)
        {
            return self.ensure_system_clock_native(state);
        }

        let module_id = ClockModule::get_module_id()?;
        let func_name = ClockModule::function_names().create;

        let system_sender = AccountAddress::ZERO;
        let genesis_tx_hash = hash_data_blake3(b"KANARI::GENESIS::CLOCK").to_vec();

        let cs = self.execute_system_function_with_tx_hash_and_persistence(
            &module_id,
            func_name,
            vec![],
            vec![],
            Some(system_sender),
            None,
            Some(0),
            Some(genesis_tx_hash),
            false,
        )?;

        state.apply_changeset_without_supply_validation(&cs)?;
        self.persist_created_objects(&cs)?;
        self.persist_deleted_objects(&cs)?;

        let (object_id, _) = cs
            .created_objects
            .iter()
            .find(|(_, created)| created.type_.contains("::clock::Clock"))
            .require("Clock object was not created by clock::create")?;

        let addr = AccountAddress::from_hex_literal(object_id)?;
        state.set_system_clock_object_id(addr)?;
        Ok(addr)
    }

    fn ensure_system_clock_native(&self, state: &mut StateManager) -> Result<AccountAddress> {
        let genesis_tx_hash = hash_data_blake3(b"KANARI::GENESIS::CLOCK");
        let mut hasher = Sha3_256::new();
        hasher.update(genesis_tx_hash);
        hasher.update(0u64.to_le_bytes());
        let digest = hasher.finalize();
        let addr = AccountAddress::from_bytes(&digest[..AccountAddress::LENGTH])
            .context("Failed to derive native system clock object id")?;
        let clock = Clock {
            id: UIDRecord::new(addr),
            timestamp_ms: 0,
        };
        let clock_type = {
            let module_id = ClockModule::get_module_id()?;
            format!(
                "{}::{}::{}",
                module_id.address().to_hex_literal(),
                module_id.name(),
                ClockModule::CLOCK_STRUCT
            )
        };
        let mut changeset = ChangeSet::new();
        changeset.created_objects.push((
            addr.to_hex_literal(),
            crate::changeset::CreatedObject {
                owner: AccountAddress::ZERO,
                owner_kind: kanari_types::transaction::ObjectOwnerKind::Shared,
                uid: Some(clock.id.clone()),
                id: None,
                type_: clock_type,
                data: bcs::to_bytes(&clock).context("Failed to encode native system clock")?,
                version: 1,
            },
        ));
        state.apply_changeset_without_supply_validation(&changeset)?;
        self.persist_created_objects(&changeset)?;
        self.persist_deleted_objects(&changeset)?;
        state.set_system_clock_object_id(addr)?;
        Ok(addr)
    }

    /// Well-known object id of the shared deny-list singleton (Sui parity:
    /// Sui uses `0x403`). Fixed so transfer-time enforcement can find it
    /// without discovery.
    pub const DENY_LIST_OBJECT_ID_SHORT: &str = "0x403";

    /// The fixed deny-list id in full 32-byte canonical form.
    pub fn deny_list_object_id() -> String {
        format!("0x{:0>64}", "403")
    }

    /// Derive a genesis child-object id the same way `tx_context::derive_id`
    /// does: `Sha3_256(blake3(seed) || counter_le)`.
    fn derive_genesis_object_id(seed: &[u8], counter: u64) -> Result<AccountAddress> {
        let genesis_tx_hash = hash_data_blake3(seed);
        let mut hasher = Sha3_256::new();
        hasher.update(genesis_tx_hash);
        hasher.update(counter.to_le_bytes());
        let digest = hasher.finalize();
        AccountAddress::from_bytes(&digest[..AccountAddress::LENGTH])
            .context("Failed to derive genesis object id")
    }

    /// Ensure the shared deny-list singleton exists at its fixed id,
    /// creating it via runtime insertion (Move `object::new` cannot target
    /// an address). Idempotent: an existing entry is left untouched.
    pub fn ensure_deny_list(&self, state: &mut StateManager) -> Result<AccountAddress> {
        let addr = AccountAddress::from_hex_literal(Self::DENY_LIST_OBJECT_ID_SHORT)
            .context("Invalid fixed deny-list object id")?;
        let id_str = Self::deny_list_object_id();
        if state.get_object(&id_str)?.is_some() {
            return Ok(addr);
        }

        let seed = b"KANARI::GENESIS::DENYLIST";
        let bag_id = Self::derive_genesis_object_id(seed, 1)?;
        let list_id = Self::derive_genesis_object_id(seed, 2)?;
        let table_id = Self::derive_genesis_object_id(seed, 3)?;

        let deny_list = GenesisDenyList {
            id: UIDRecord::new(addr),
            lists: GenesisBag {
                id: UIDRecord::new(bag_id),
                size: 1,
            },
        };
        // The bag entry (COIN_INDEX -> PerTypeList) lives as a dynamic
        // field, exactly as Move `bag::add` would record it: the count
        // table starts empty and entries arrive via `deny_list_add`.
        let per_type_list = GenesisPerTypeList {
            id: UIDRecord::new(list_id),
            denied_count: GenesisTable {
                id: UIDRecord::new(table_id),
                size: 0,
            },
        };

        let deny_type = format!(
            "{}::deny_list::DenyList",
            KanariAddress::kanari_system_account_address().to_hex_literal()
        );
        let mut changeset = ChangeSet::new();
        changeset.created_objects.push((
            id_str.clone(),
            crate::changeset::CreatedObject {
                owner: AccountAddress::ZERO,
                owner_kind: kanari_types::transaction::ObjectOwnerKind::Shared,
                uid: Some(deny_list.id.clone()),
                id: None,
                type_: deny_type,
                data: bcs::to_bytes(&deny_list).context("Failed to encode deny list")?,
                version: 1,
            },
        ));
        changeset.added_dynamic_fields.push((
            format!("0x{}", hex::encode(bag_id.as_ref())),
            0u64.to_le_bytes().to_vec(),
            bcs::to_bytes(&per_type_list).context("Failed to encode per-type list")?,
        ));
        state.apply_changeset_without_supply_validation(&changeset)?;
        self.persist_created_objects(&changeset)?;
        self.persist_deleted_objects(&changeset)?;
        Ok(addr)
    }

    /// Build a consensus commit prologue changeset for the native clock object.
    pub fn build_native_clock_consensus_commit_prologue(
        &self,
        state: &StateManager,
        clock_id: AccountAddress,
        timestamp_ms: u64,
    ) -> Result<ChangeSet> {
        let object_id = clock_id.to_hex_literal();
        let existing = state
            .get_object(&object_id)?
            .with_context(|| format!("System clock object {object_id} not found"))?;
        let current_clock: Clock = bcs::from_bytes(&existing.data)
            .context("Failed to decode system clock object for native prologue")?;
        ensure!(
            timestamp_ms >= current_clock.timestamp_ms,
            "Clock timestamp is not monotonic: new={} current={}",
            timestamp_ms,
            current_clock.timestamp_ms
        );

        let clock = Clock {
            id: UIDRecord::new(clock_id),
            timestamp_ms,
        };
        let data = bcs::to_bytes(&clock).context("Failed to encode native clock prologue")?;
        let version = if data == existing.data {
            existing.version
        } else {
            existing.version.saturating_add(1)
        };

        let mut changeset = ChangeSet::new();
        changeset.created_objects.push((
            object_id,
            crate::changeset::CreatedObject {
                owner: existing.owner,
                owner_kind: existing.owner_kind,
                uid: Some(clock.id),
                id: existing.id,
                type_: existing.type_,
                data,
                version,
            },
        ));
        Ok(changeset)
    }
}
