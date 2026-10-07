// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Phases extracted from `apply_changeset_with_options`. Each phase is a
//! contiguous block of the original flow with explicit inputs and outputs;
//! the main flow keeps orchestration, timing markers, and shared state.

use super::*;

impl StateManager {
    /// Snapshot native (ledger, object) balances for every owner the
    /// changeset touches, before any mutation. Debit validation and
    /// post-object recomputation both need this snapshot; independently
    /// resolving the same owner here used to scan every owned Coin object
    /// twice before the write pass.
    pub(crate) fn snapshot_native_balances(
        &self,
        changeset: &ChangeSet,
    ) -> Result<BTreeMap<AccountAddress, (u64, u64)>> {
        let mut native_snapshot_owners = changeset
            .owner_deltas
            .keys()
            .copied()
            .collect::<BTreeSet<_>>();
        for (_, created) in &changeset.created_objects {
            if Self::object_balance_token_for_type_and_data(&created.type_, &created.data)
                .is_some_and(|token_type| token_type == GAS_COIN)
            {
                native_snapshot_owners.insert(created.owner);
            }
        }
        for object_id in &changeset.deleted_objects {
            if let Some((_, existing)) = self.load_stored_object_by_any_id(object_id)?
                && Self::object_balance_token_for_type_and_data(&existing.type_name, &existing.data)
                    .is_some_and(|token_type| token_type == GAS_COIN)
            {
                native_snapshot_owners.insert(existing.owner);
            }
        }
        let mut native_balances_before = BTreeMap::new();
        for owner in native_snapshot_owners {
            let ledger_balance = self
                .load_owner_state(&owner)?
                .map(|state| state.native_balance())
                .unwrap_or(0);
            let native_debit = changeset
                .owner_deltas
                .get(&owner)
                .map(|change| change.balance_delta)
                .filter(|delta| *delta < 0)
                .map(|delta| {
                    u64::try_from(delta.unsigned_abs())
                        .require("Native debit overflowed u64 owner balance")
                })
                .transpose()?
                .unwrap_or(0);
            let object_balance = if native_debit > 0 && ledger_balance >= native_debit {
                0
            } else {
                self.compute_owned_token_balances(owner, None)?
                    .get(GAS_COIN)
                    .copied()
                    .unwrap_or(0)
            };
            native_balances_before.insert(owner, (ledger_balance, object_balance));
        }
        Ok(native_balances_before)
    }

    /// Reject debits no owner can cover from ledger or object balances.
    pub(crate) fn validate_owner_debits(
        &self,
        changeset: &ChangeSet,
        native_balances_before: &BTreeMap<AccountAddress, (u64, u64)>,
    ) -> Result<()> {
        for (address, change) in &changeset.owner_deltas {
            if change.balance_delta >= 0 {
                continue;
            }
            let debit = u64::try_from(change.balance_delta.unsigned_abs())
                .require("Native debit overflowed u64 owner balance")?;
            let (ledger_balance, object_balance) = native_balances_before
                .get(address)
                .copied()
                .unwrap_or((0, 0));
            let balance = ledger_balance.max(object_balance);
            ensure!(
                balance >= debit,
                "Insufficient native balance for {}: need {}, have {}",
                address.to_hex_literal(),
                debit,
                balance
            );
        }
        Ok(())
    }

    /// Apply treasury records, NFT caps, and token-balance hints. Only marks
    /// owners for recompute; balances themselves come from objects later.
    pub(crate) fn apply_treasury_nft_balances(
        &mut self,
        changeset: &ChangeSet,
        owners_to_recompute: &mut BTreeSet<AccountAddress>,
    ) -> Result<()> {
        // Apply treasury creations/updates (canonical spelling going forward;
        // readers fall back to raw keys for legacy DBs).
        for (owner, token_type, total_supply) in &changeset.treasuries {
            let normalized = Self::normalize_token_type(token_type);
            let key = Self::supply_key(&normalized);
            self.save_internal(&key, total_supply)?;

            let mut key_owner = b"treasury:".to_vec();
            key_owner.extend_from_slice(normalized.as_bytes());
            self.save_internal(&key_owner, owner)?;

            self.add_to_index_list(b"treasury_index", format!("treasury:{}", normalized))?;

            if Self::normalize_token_type(token_type) == GAS_COIN {
                self.save_native_total_supply(total_supply.total_supply)?;
            }
        }

        // Apply NFT capability creations/updates
        for (_, token_type, nft_cap) in &changeset.nft_caps {
            let mut key = b"nft:".to_vec();
            key.extend_from_slice(token_type.as_bytes());
            self.save_internal(&key, nft_cap)?;
        }

        // Record Global Token Supplies to database only once after all processing
        // Treat token balance hints as recompute triggers, not as canonical balance writes.
        // Non-native balances must come from owned objects so queries always reflect object
        // inventory instead of a parallel per-owner cache.
        for (owner, token_type, _) in &changeset.token_balance_sets {
            let normalized_token_type = Self::normalize_token_type(token_type);
            if normalized_token_type == GAS_COIN {
                // Native KANARI balance is applied from owner deltas so gas and
                // transfers remain exact. Object-derived hints must not double count it.
                continue;
            }

            let object_backed = self.changeset_touches_object_backed_token_for_owner(
                changeset,
                *owner,
                &normalized_token_type,
            )?;
            if object_backed {
                owners_to_recompute.insert(*owner);
            }
        }
        Ok(())
    }

    /// Process deleted objects: snapshot native balances, drop index entries,
    /// and tombstone the overlay keys.
    pub(crate) fn apply_deleted_objects(
        &mut self,
        changeset: &ChangeSet,
        native_object_before_from_changes: &mut BTreeMap<AccountAddress, u64>,
        native_object_after_from_changes: &mut BTreeMap<AccountAddress, u64>,
        native_object_owner_changed_owners: &mut BTreeSet<AccountAddress>,
        owners_to_recompute: &mut BTreeSet<AccountAddress>,
        native_object_changed_owners: &mut BTreeSet<AccountAddress>,
        non_native_object_changed_owners: &mut BTreeSet<AccountAddress>,
    ) -> Result<()> {
        for obj_id in &changeset.deleted_objects {
            let (obj_key, stored_id) = if let Some((stored_id, existing)) =
                self.load_stored_object_by_any_id(obj_id)?
            {
                let obj_key = object_key(&stored_id);
                Self::record_object_balance_owner_if_needed(
                    owners_to_recompute,
                    native_object_changed_owners,
                    non_native_object_changed_owners,
                    existing.owner,
                    &existing.type_name,
                    &existing.data,
                );
                if Self::object_balance_token_for_type_and_data(&existing.type_name, &existing.data)
                    .is_some_and(|token_type| token_type == GAS_COIN)
                {
                    if let Some((_, amount)) =
                        Self::balance_token_amount(&existing.type_name, &existing.data)
                    {
                        let owner_balance = native_object_before_from_changes
                            .entry(existing.owner)
                            .or_insert(0);
                        *owner_balance = owner_balance
                            .checked_add(amount)
                            .require("Native object balance snapshot overflow")?;
                        native_object_after_from_changes
                            .entry(existing.owner)
                            .or_insert(0);
                    }
                    native_object_owner_changed_owners.insert(existing.owner);
                }
                if matches!(existing.owner_kind, ObjectOwnerKind::AddressOwner(_)) {
                    self.remove_owned_object_index_variants(
                        existing.owner,
                        obj_id,
                        Some(&stored_id),
                    )?;
                }
                (obj_key, Some(stored_id))
            } else {
                (object_key(obj_id), None)
            };

            self.overlay.insert(obj_key, None);
            if let Some(stored_id) = &stored_id {
                self.remove_from_index_list(b"object_index", stored_id)?;
            }
            self.remove_from_index_list(b"object_index", &Self::canonical_owned_object_id(obj_id))?;
        }
        Ok(())
    }

    /// Index collections and NFT collection memberships from created objects
    /// and MintLog events.
    pub(crate) fn index_collections(&mut self, changeset: &ChangeSet) -> Result<()> {
        // 1. Check for newly created Objects to index Collections
        for (obj_id, created) in &changeset.created_objects {
            // If this is a Collection type Object, record it in the global index
            if created.type_.contains("::collection::Collection") {
                self.add_to_index_list(b"nft_collection_index", obj_id.clone())?;
            }
        }

        // 2. Check Events to index NFT <-> Collection relationships
        for event in &changeset.events {
            // Check if this is a MintLog from james::nft
            if event.type_tag.to_string().contains("::nft::MintLog") {
                // MintLog data in nft.move contains: object_id(32), creator(32), collection_id(32)
                if event.event_data.len() >= 96 {
                    let nft_id_bytes = &event.event_data[0..32];
                    let coll_id_bytes = &event.event_data[64..96];

                    let (Ok(nft_id), Ok(coll_id)) = (
                        AccountAddress::from_bytes(nft_id_bytes).map(|addr| addr.to_hex_literal()),
                        AccountAddress::from_bytes(coll_id_bytes).map(|addr| addr.to_hex_literal()),
                    ) else {
                        log::warn!(
                            "Skipping malformed MintLog object ids while indexing collection members"
                        );
                        continue;
                    };

                    // Record in Collection member index (O(1) Access)
                    let key = metadata_key(b"collection_members:", &coll_id);
                    self.add_to_index_list(&key, nft_id)?;
                }
            }
        }
        Ok(())
    }

    /// Apply canonical Move module/resource writes to the overlay, keeping the
    /// module index in step.
    pub(crate) fn apply_move_writes(&mut self, changeset: &ChangeSet) -> Result<()> {
        // Move modules/resources are part of the same canonical overlay as objects and
        // balances. Keeping them here prevents speculative VM execution from mutating the
        // shared database before the checkpoint is validated and committed.
        for (key, value) in &changeset.move_writes {
            match value {
                Some(bytes) => self.save_internal(key, bytes)?,
                None => {
                    self.overlay.insert(key.clone(), None);
                }
            }

            if key.starts_with(b"module:") {
                let module_key =
                    String::from_utf8(key.clone()).context("Move module key is not valid UTF-8")?;
                if value.is_some() {
                    self.add_to_index_list(b"module_index", module_key)?;
                } else {
                    self.remove_from_index_list(b"module_index", &module_key)?;
                }
            }
        }
        Ok(())
    }

    /// Apply dynamic-field additions and removals to the overlay.
    pub(crate) fn apply_dynamic_fields(&mut self, changeset: &ChangeSet) -> Result<()> {
        // =====================================================================
        // Process Dynamic Fields into State Overlay.
        // =====================================================================
        for (object_id, name_bytes, value_bytes) in &changeset.added_dynamic_fields {
            let df_key = dynamic_field_key(object_id, name_bytes);
            self.save_internal(&df_key, value_bytes)?;
        }

        for (object_id, name_bytes) in &changeset.removed_dynamic_fields {
            let df_key = dynamic_field_key(object_id, name_bytes);
            // Record as None so commit() will delete it from RocksDB
            self.overlay.insert(df_key, None);
        }
        Ok(())
    }
}
