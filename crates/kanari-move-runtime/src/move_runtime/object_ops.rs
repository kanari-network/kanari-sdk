// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Object storage operations
use crate::{changeset::ChangeSet, storage::object_storage::StoredObject};
use anyhow::Result;
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

    /// Delete object from storage
    pub fn delete_object(&mut self, object_id: &str) -> Result<()> {
        self.object_storage
            .delete_object(object_id)
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Get object storage statistics
    pub fn get_object_stats(&self) -> (usize, usize) {
        let total_objects = self.object_storage.count();
        let total_owners = 0; // Could add owner count tracking if needed
        (total_objects, total_owners)
    }

    /// Add transferred objects from native function tracking to changeset
    /// Also persists objects to ObjectStorage for later retrieval
    pub(crate) fn add_transferred_objects(&mut self, cs: &mut ChangeSet) {
        let transferred = kanari_types::transfer_natives::take_transferred_objects();

        let count = transferred.len();
        eprintln!("[DEBUG] Processing {} transferred objects", count);

        for obj in transferred {
            eprintln!(
                "[DEBUG] Adding transferred object: id={}, type={}, owner={}, data_len={}",
                obj.object_id,
                obj.object_type,
                obj.recipient,
                obj.data.len()
            );

            // Add to created_objects in changeset (for immediate response)
            cs.add_created_object(
                obj.object_id.clone(),
                obj.recipient,
                obj.object_type.clone(),
                obj.data.clone(),
                0,
            );

            // Persist to ObjectStorage if flagged
            if obj.should_persist {
                let stored_obj = StoredObject {
                    id: obj.object_id.clone(),
                    owner: obj.recipient,
                    type_name: obj.object_type.clone(),
                    data: obj.data,
                    version: 0,
                };

                match self.object_storage.store_object(stored_obj) {
                    Ok(_) => {
                        eprintln!(
                            "[DEBUG] ✓ Object {} persisted to ObjectStorage",
                            obj.object_id
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "[DEBUG] ✗ Failed to persist object {}: {}",
                            obj.object_id, e
                        );
                    }
                }
            }
        }

        if count > 0 {
            eprintln!("[DEBUG] Total {} objects added to changeset", count);
        }
    }
}
