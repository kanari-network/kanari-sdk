// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Object storage operations
use crate::{changeset::ChangeSet, storage::object_storage::StoredObject};
use anyhow::Result;
use kanari_system_natives::transfer_natives::TransferredObject;
use log::debug;
use move_core_types::account_address::AccountAddress;

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
        &mut self,
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
        &mut self,
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

            // Use the object ID provided by the native function (which is the real UID or a hash)
            // Do NOT recompute it here, as that would break the link to the on-chain UID.
            let canonical_id = id.clone();

            // Persist to ObjectStorage first if flagged (before changeset)
            if should_persist {
                let stored_obj = StoredObject {
                    id: canonical_id.clone(),
                    owner,
                    type_name: obj_type.clone(),
                    data: data.clone(),
                    version: 1,
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
            }

            // Add to created_objects in changeset (after storage to avoid double clone)
            // Pass the explicit ID to ensure ChangeSet uses the same ID as ObjectStorage.
            cs.add_created_object(owner, obj_type, data, 1, None, Some(canonical_id));
        }

        if count > 0 {
            debug!("Total {} objects added to changeset", count);
        }
    }
}
