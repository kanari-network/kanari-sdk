// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Object storage operations
use crate::{changeset::ChangeSet, storage::object_storage::StoredObject};
use anyhow::Result;
use kanari_system_natives::transfer_natives::TransferredObject;
use log::debug;
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::StructTag;
use std::str::FromStr;

impl super::MoveRuntime {
    /// Get object by ID from ObjectStorage
    pub fn get_object(&self, object_id: &str) -> Option<StoredObject> {
        self.object_storage.get_object(object_id)
    }

    /// Get all objects owned by an address
    pub fn get_objects_by_owner(&self, owner: &AccountAddress) -> Vec<StoredObject> {
        self.object_storage.get_objects_by_owner(owner)
    }

    /// Transfer object ownership
    pub fn transfer_object_ownership(
        &self,
        object_id: &str,
        new_owner: AccountAddress,
    ) -> Result<()> {
        self.object_storage
            .transfer_object(object_id, new_owner)
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Get object storage count
    pub fn get_object_count(&self) -> usize {
        self.object_storage.count()
    }

    /// Add transferred objects from native function tracking to changeset
    /// Also persists objects to ObjectStorage for later retrieval
    pub(crate) fn add_transferred_objects_to_changeset(
        &self,
        cs: &mut ChangeSet,
        transferred: Vec<TransferredObject>,
    ) {
        let count = transferred.len();
        debug!("Processing {} transferred objects", count);

        for obj in transferred {
            // take ownership of fields to avoid unnecessary clones where possible
            let id = obj.object_id;
            let obj_type = obj.object_type;
            let owner = obj.recipient;
            let data = obj.data;
            let should_persist = obj.should_persist;

            debug!(
                "Adding transferred object: id={} type={} owner={} data_len={}",
                id,
                obj_type,
                owner,
                data.len()
            );

            if !should_persist {
                debug!(
                    "Skipping non-persistable transferred object {} (fallback payload)",
                    id
                );
                continue;
            }

            // Use the object ID provided by the native function (which is the real UID or a hash)
            // Do NOT recompute it here, as that would break the link to the on-chain UID.
            let canonical_id = id.clone();
            let next_version = self
                .object_storage
                .get_object(&canonical_id)
                .map(|existing| existing.version.saturating_add(1))
                .unwrap_or(1);

            // Persist to ObjectStorage first (before changeset)
            let stored_obj = StoredObject {
                id: canonical_id.clone(),
                owner,
                type_name: obj_type.clone(),
                data: data.clone(),
                version: next_version,
            };

            match self.object_storage.store_object(stored_obj) {
                Ok(_) => debug!("Object {} persisted to ObjectStorage", canonical_id),
                Err(e) => {
                    debug!(
                        "WARNING: Failed to persist object {} to storage: {}. Object remains in changeset.",
                        canonical_id, e
                    );
                }
            }

            // Upsert by object_id: a transferred object should represent the final owner/state.
            // This avoids duplicate entries for the same id from mixed paths
            // (e.g. mutable writeback + native transfer capture in the same tx).
            cs.created_objects
                .retain(|(existing_id, _)| existing_id != &canonical_id);

            // Add to created_objects in changeset (after storage to avoid double clone).
            // Pass the explicit ID to ensure ChangeSet uses the same ID as ObjectStorage.
            let uid = if let Ok(addr) = AccountAddress::from_hex_literal(&canonical_id) {
                Some(kanari_types::object::UIDRecord::new(addr))
            } else {
                None
            };

            // Detect special objects (TreasuryCap, Coin) and add them to changeset
            if let Ok(struct_tag) = StructTag::from_str(&obj_type) {
                // Parse TreasuryCap resources
                if self.is_treasury_resource(&struct_tag)
                    && let Some(total) = self.extract_treasury_total_from_bytes(&data)
                    && let Some(token_type) = self.token_type_from_struct_tag(&struct_tag)
                {
                    cs.add_treasury(owner, token_type, total);
                    debug!("Detected TreasuryCap object: supply={}", total);
                }

                // Parse Coin resources (Balance)
                if self.is_balance_resource(&struct_tag)
                    && let Some(amount) = self.extract_balance_from_bytes(&data, &struct_tag)
                    && let Some(token_type) = self.token_type_from_struct_tag(&struct_tag)
                {
                    cs.add_token_balance_set(owner, token_type, amount);
                    debug!("Detected Coin object: amount={}", amount);
                }
            }

            cs.add_created_object(owner, obj_type, data, next_version, uid, Some(canonical_id));
        }

        if count > 0 {
            debug!("Total {} objects added to changeset", count);
        }
    }
}
