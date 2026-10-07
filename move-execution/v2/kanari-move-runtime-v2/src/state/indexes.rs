// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Derived index lists (owners, owned objects, collections).

use super::*;

impl StateManager {
    pub(crate) fn collect_derived_indexes(&self) -> Result<DerivedIndexes> {
        let entries = self
            .store
            .logical_entries()
            .context("Failed to read state entries for derived index rebuild")?;

        let mut owner_ids = BTreeSet::new();
        let mut owned_objects: BTreeMap<AccountAddress, BTreeSet<String>> = BTreeMap::new();
        let mut object_ids = BTreeSet::new();

        for (key, value) in entries {
            if let Some(owner_bytes) = key.strip_prefix(b"account:") {
                if let Ok(owner) = AccountAddress::from_bytes(owner_bytes) {
                    owner_ids.insert(owner.to_hex_literal());
                }
                continue;
            }

            let Some(object_id_bytes) = key.strip_prefix(b"object:") else {
                continue;
            };
            let Ok(mut object_id) = String::from_utf8(object_id_bytes.to_vec()) else {
                continue;
            };
            if let Some(canonical_id) = canonical_object_id(&object_id) {
                object_id = canonical_id;
            }
            object_ids.insert(object_id.clone());

            let Ok(stored) = bcs::from_bytes::<StoredObject>(&value) else {
                continue;
            };
            if matches!(stored.owner_kind, ObjectOwnerKind::AddressOwner(_)) {
                owner_ids.insert(stored.owner.to_hex_literal());
                owned_objects
                    .entry(stored.owner)
                    .or_default()
                    .insert(object_id);
            }
        }

        Ok((
            owner_ids.into_iter().collect(),
            owned_objects
                .into_iter()
                .map(|(owner, ids)| (owner, ids.into_iter().collect()))
                .collect(),
            object_ids.into_iter().collect(),
        ))
    }

    pub(crate) fn load_index_list(&self, key: &[u8]) -> Result<Vec<String>> {
        Ok(self.load_internal(key)?.unwrap_or_default())
    }

    pub(crate) fn save_index_list(&mut self, key: &[u8], ids: &[String]) -> Result<()> {
        self.save_internal(key, ids)
    }

    pub(crate) fn add_to_index_list(&mut self, key: &[u8], value: String) -> Result<()> {
        let mut index = self.load_index_list(key)?;
        let Err(pos) = index.binary_search(&value) else {
            return Ok(());
        };

        index.insert(pos, value);
        self.save_index_list(key, &index)?;
        Ok(())
    }

    pub(crate) fn remove_from_index_list(&mut self, key: &[u8], value: &str) -> Result<()> {
        let mut index = self.load_index_list(key)?;
        let Ok(pos) = index.binary_search_by(|entry| entry.as_str().cmp(value)) else {
            return Ok(());
        };

        index.remove(pos);
        self.save_index_list(key, &index)?;
        Ok(())
    }

    /// Retrieves all Collection IDs from the index
    pub fn try_get_all_collection_ids(&self) -> Result<Vec<String>> {
        self.load_index_list(b"nft_collection_index")
    }

    /// Retrieve all collection IDs from the index.
    pub fn get_all_collection_ids(&self) -> Vec<String> {
        self.try_get_all_collection_ids()
            .unwrap_or_else(|error| Self::log_index_fallback("nft_collection_index", error))
    }

    /// Retrieves NFT IDs for the specified collection from the index
    pub fn try_get_collection_nft_ids(&self, collection_id: &str) -> Result<Vec<String>> {
        self.load_index_list(&metadata_key(b"collection_members:", collection_id))
    }

    /// Retrieve NFT IDs for the specified collection.
    pub fn get_collection_nft_ids(&self, collection_id: &str) -> Vec<String> {
        self.try_get_collection_nft_ids(collection_id)
            .unwrap_or_else(|error| {
                Self::log_index_fallback(&format!("collection_members:{collection_id}"), error)
            })
    }

    pub(crate) fn log_index_fallback(index_name: &str, error: anyhow::Error) -> Vec<String> {
        log::error!("[StateManager] Failed to load index {index_name}: {error:#}");
        Vec::new()
    }

    pub(crate) fn repair_derived_indexes_on_startup(&mut self) -> Result<bool> {
        let (expected_owner_ids, expected_owned_objects, expected_object_ids) =
            self.collect_derived_indexes()?;
        let mut changed = false;

        let current_owner_ids = self.load_index_list(OWNER_INDEX_KEY)?;
        if current_owner_ids != expected_owner_ids {
            self.save_index_list(OWNER_INDEX_KEY, &expected_owner_ids)?;
            changed = true;
        }

        let legacy_owner_ids = self.load_index_list(LEGACY_ACCOUNT_INDEX_KEY)?;
        if !legacy_owner_ids.is_empty() {
            self.overlay.insert(LEGACY_ACCOUNT_INDEX_KEY.to_vec(), None);
            changed = true;
        }

        let current_object_ids = self.load_index_list(b"object_index")?;
        if current_object_ids != expected_object_ids {
            self.save_index_list(b"object_index", &expected_object_ids)?;
            changed = true;
        }

        let indexed_owners = current_owner_ids
            .into_iter()
            .chain(expected_owner_ids.iter().cloned())
            .filter_map(|id| AccountAddress::from_hex_literal(&id).ok())
            .collect::<BTreeSet<_>>();

        for owner in indexed_owners {
            let expected_ids = expected_owned_objects
                .get(&owner)
                .cloned()
                .unwrap_or_default();
            let key = owned_objects_key(&owner);
            let current_ids = self.load_index_list(&key)?;
            if current_ids != expected_ids {
                self.save_index_list(&key, &expected_ids)?;
                changed = true;
            }
        }

        Ok(changed)
    }
}
