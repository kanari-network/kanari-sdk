// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use kanari_types::coin::TreasuryCap;
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::ModuleId;
use move_vm_test_utils::InMemoryStorage;
use std::path::PathBuf;
use std::sync::Arc;

use crate::storage::persistent_store::PersistentStore;

/// Simple persistent store wrapper for published modules using `PersistentStore`.
#[derive(Clone)]
pub struct MoveVMState {
    store: Arc<PersistentStore>,
}

impl MoveVMState {
    /// Create an in-memory MoveVMState for testing or Miri (no filesystem ops).
    pub fn new_in_memory() -> Result<Self> {
        let store = PersistentStore::open_in_memory()?;
        Ok(MoveVMState {
            store: Arc::new(store),
        })
    }

    /// Open default store for Move VM state.
    pub fn open_default() -> Result<Self> {
        // Honor legacy env var for Move VM DB path
        let db_path = std::env::var("KANARI_MOVE_VM_DB").ok().map(PathBuf::from);
        let store = PersistentStore::open_with_path(db_path)?;
        Ok(MoveVMState {
            store: Arc::new(store),
        })
    }

    /// Save a module blob keyed by module id.
    pub fn save_module(&self, module_id: &ModuleId, blob: &[u8]) -> Result<()> {
        let key = format!(
            "module:{}:{}",
            module_id.address().to_hex_literal(),
            module_id.name().as_str()
        );
        // Persist module blob
        self.store.save(key.as_bytes(), blob)?;

        // Update module index so `load_into_storage()` can discover modules
        let mut index = self
            .store
            .load::<Vec<String>>(b"module_index")?
            .unwrap_or_default();
        if !index.iter().any(|x| x == &key) {
            index.push(key);
            self.store.save(b"module_index", &index)?;
        }

        Ok(())
    }

    /// Delete a module blob keyed by module id and remove it from the persistent index.
    pub fn delete_module(&self, module_id: &ModuleId) -> Result<()> {
        let key = format!(
            "module:{}:{}",
            module_id.address().to_hex_literal(),
            module_id.name().as_str()
        );
        self.store.delete(key.as_bytes())?;

        let mut index = self
            .store
            .load::<Vec<String>>(b"module_index")?
            .unwrap_or_default();
        let old_len = index.len();
        index.retain(|x| x != &key);
        if index.len() != old_len {
            self.store.save(b"module_index", &index)?;
        }

        Ok(())
    }

    /// Persist framework manifest + hash for operational safety / debugging.
    pub fn save_framework_manifest(
        &self,
        name: &str,
        manifest: &Vec<(String, String)>,
        hash_hex: &str,
    ) -> Result<()> {
        let manifest_key = format!("framework_manifest:{name}");
        let hash_key = format!("framework_hash:{name}");
        self.store.save(manifest_key.as_bytes(), manifest)?;
        self.store
            .save(hash_key.as_bytes(), &hash_hex.to_string())?;
        Ok(())
    }

    /// Load a previously persisted framework hash (if any).
    pub fn get_framework_hash(&self, name: &str) -> Option<String> {
        let hash_key = format!("framework_hash:{name}");
        self.store
            .load::<String>(hash_key.as_bytes())
            .ok()
            .flatten()
    }

    /// Load persisted modules into an `InMemoryStorage` instance.
    /// Returns a list of loaded ModuleIds.
    pub fn load_into_storage(&self, storage: &mut InMemoryStorage) -> Result<Vec<ModuleId>> {
        // Prefix scan is not directly supported by SMT shim; fallback to RocksDB
        // behavior by attempting to iterate using underlying RocksDB if available.
        // For simplicity, attempt to load by trying keys stored in an index key
        // `module_index` if present, otherwise return Ok(()) to avoid blocking.

        let mut loaded_modules = Vec::new();

        if let Ok(Some(bytes)) = self.store.load::<Vec<String>>(b"module_index") {
            for s in bytes.into_iter() {
                // expected module keys in the index are full keys: module:addr:name
                if let Ok(Some(blob)) = self.store.load::<Vec<u8>>(s.as_bytes()) {
                    // parse key to reconstruct ModuleId
                    let parts: Vec<&str> = s.splitn(3, ':').collect();
                    if parts.len() != 3 {
                        continue;
                    }
                    let addr = AccountAddress::from_hex_literal(parts[1]).ok();
                    let ident = Identifier::new(parts[2]).ok();
                    if let (Some(a), Some(id)) = (addr, ident) {
                        let module_id = ModuleId::new(a, id);
                        storage.publish_or_overwrite_module(module_id.clone(), blob);
                        loaded_modules.push(module_id);
                    }
                }
            }
        }

        Ok(loaded_modules)
    }

    /// Get all module IDs from the persistent index.
    pub fn get_all_module_ids(&self) -> Result<Vec<ModuleId>> {
        let mut modules = Vec::new();
        if let Ok(Some(index)) = self.store.load::<Vec<String>>(b"module_index") {
            for s in index {
                let parts: Vec<&str> = s.splitn(3, ':').collect();
                if let (3, Some(addr), Some(name)) = (
                    parts.len(),
                    AccountAddress::from_hex_literal(parts[1]).ok(),
                    Identifier::new(parts[2]).ok(),
                ) {
                    modules.push(ModuleId::new(addr, name));
                }
            }
        }
        Ok(modules)
    }

    /// Get module bytecode from persistent storage
    pub fn get_module(&self, module_id: &ModuleId) -> Option<Vec<u8>> {
        let key = format!(
            "module:{}:{}",
            module_id.address().to_hex_literal(),
            module_id.name().as_str()
        );
        self.store.load::<Vec<u8>>(key.as_bytes()).ok().flatten()
    }

    /// Save a resource blob keyed by address and struct tag.
    /// 🚨 UPDATED: Added logic to sync with Kanari Objects to prevent inflation.
    pub fn save_resource(
        &self,
        address: &AccountAddress,
        tag: &move_core_types::language_storage::StructTag,
        blob: &[u8],
    ) -> Result<()> {
        let key = format!("resource:{}:{}", address.to_hex_literal(), tag);

        // 1. Save raw data to Move Store as usual
        self.store
            .save(key.as_bytes(), blob)
            .map_err(|e| anyhow::anyhow!(e))?;

        // 2. 🚨 DEEP SYNC: If this Resource is a Coin (token)
        // We must force update data in Object Storage to ensure correct total balance
        if tag.module.as_str() == "coin" && tag.name.as_str() == "Coin" {
            let object_id = address.to_hex_literal();
            let obj_key = format!("object:{}", object_id);

            // Extract original Object data to update Data (new balance)
            if let Ok(Some(obj_bytes)) = self.store.load::<Vec<u8>>(obj_key.as_bytes()) {
                // Assuming CreatedObject structure in your DB uses BCS
                // We will overwrite only the Data part with new data from MoveVM
                if let Ok(mut created_obj) =
                    bcs::from_bytes::<crate::changeset::CreatedObject>(&obj_bytes)
                {
                    created_obj.data = blob.to_vec(); // Update balance after deduction
                    created_obj.version += 1;

                    let updated_bytes = bcs::to_bytes(&created_obj)?;
                    self.store.save(obj_key.as_bytes(), &updated_bytes)?;
                }
            }
        }

        Ok(())
    }

    /// Get resource blob from persistent storage
    pub fn get_resource(
        &self,
        address: &AccountAddress,
        tag: &move_core_types::language_storage::StructTag,
    ) -> Option<Vec<u8>> {
        let key = format!("resource:{}:{}", address.to_hex_literal(), tag);
        self.store.load::<Vec<u8>>(key.as_bytes()).ok().flatten()
    }

    /// Get object data from persistent storage (Sui-style objects)
    pub fn get_object(&self, object_id: &AccountAddress) -> Option<Vec<u8>> {
        let obj_key = format!("object:{}", object_id.to_hex_literal());
        if let Ok(Some(obj_bytes)) = self.store.load::<Vec<u8>>(obj_key.as_bytes()) {
            // Extract data from CreatedObject wrapper
            if let Ok(created_obj) = bcs::from_bytes::<crate::changeset::CreatedObject>(&obj_bytes)
            {
                return Some(created_obj.data);
            }
        }
        None
    }

    /// Delete a resource blob keyed by address and struct tag.
    pub fn delete_resource(
        &self,
        address: &AccountAddress,
        tag: &move_core_types::language_storage::StructTag,
    ) -> Result<()> {
        let key = format!("resource:{}:{}", address.to_hex_literal(), tag);
        self.store
            .delete(key.as_bytes())
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Persist a treasury record (owner + total_supply) for a token type so it
    /// survives node restarts. Uses `treasury_index` to track persisted keys.
    pub fn save_treasury(
        &self,
        token_type: &str,
        owner: &AccountAddress,
        total: u64,
    ) -> Result<()> {
        // Use kanari-types `TreasuryCap` to store the total supply
        let key = format!("treasury:{}", token_type);
        let cap = TreasuryCap {
            total_supply: total,
        };

        // Save tuple (owner, cap)
        self.store.save(key.as_bytes(), &(owner, cap))?;

        // Update index
        let mut index = self
            .store
            .load::<Vec<String>>(b"treasury_index")?
            .unwrap_or_default();
        if !index.iter().any(|x| x == &key) {
            index.push(key);
            self.store.save(b"treasury_index", &index)?;
        }

        Ok(())
    }

    /// Load persisted treasuries as a vector of tuples (owner, token_type, TreasuryCap)
    pub fn load_treasuries(&self) -> Result<Vec<(AccountAddress, String, TreasuryCap)>> {
        let mut out = Vec::new();
        if let Ok(Some(keys)) = self.store.load::<Vec<String>>(b"treasury_index") {
            for key in keys.into_iter() {
                if let Ok(Some((owner_addr, cap))) = self
                    .store
                    .load::<(AccountAddress, TreasuryCap)>(key.as_bytes())
                {
                    let token_type = key.strip_prefix("treasury:").unwrap_or(&key).to_string();
                    out.push((owner_addr, token_type, cap));
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::MoveVMState;
    use anyhow::Result;
    use move_core_types::account_address::AccountAddress;
    use move_core_types::language_storage::StructTag;
    use std::str::FromStr;

    #[test]
    fn delete_resource_removes_saved_value() -> Result<()> {
        let state = MoveVMState::new_in_memory()?;
        let owner = AccountAddress::from_hex_literal("0x1234")?;
        let tag = StructTag::from_str("0x2::coin::Coin<0x2::kanari::KANARI>")?;
        let bytes = vec![1u8, 2, 3, 4];

        state.save_resource(&owner, &tag, &bytes)?;
        assert_eq!(state.get_resource(&owner, &tag), Some(bytes));

        state.delete_resource(&owner, &tag)?;
        assert_eq!(state.get_resource(&owner, &tag), None);
        Ok(())
    }
}
