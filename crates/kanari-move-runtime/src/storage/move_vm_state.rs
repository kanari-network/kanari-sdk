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
        self.store.save(&key, blob)?;

        // Update module index so `load_into_storage()` can discover modules
        let mut index = self
            .store
            .load::<Vec<String>>("module_index")?
            .unwrap_or_default();
        if !index.iter().any(|x| x == &key) {
            index.push(key);
            self.store.save("module_index", &index)?;
        }

        Ok(())
    }

    /// Load persisted modules into an `InMemoryStorage` instance.
    /// Returns a list of loaded ModuleIds.
    pub fn load_into_storage(&self, storage: &mut InMemoryStorage) -> Result<Vec<ModuleId>> {
        // Prefix scan is not directly supported by SMT shim; fallback to RocksDB
        // behavior by attempting to iterate using underlying RocksDB if available.
        // For simplicity, attempt to load by trying keys stored in an index key
        // `module_index` if present, otherwise return Ok(()) to avoid blocking.

        let mut loaded_modules = Vec::new();

        if let Ok(Some(bytes)) = self.store.load::<Vec<String>>("module_index") {
            for s in bytes.into_iter() {
                // expected module keys in the index are full keys: module:addr:name
                if let Ok(Some(blob)) = self.store.load::<Vec<u8>>(&s) {
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

    /// Get module bytecode from persistent storage
    pub fn get_module(&self, module_id: &ModuleId) -> Option<Vec<u8>> {
        let key = format!(
            "module:{}:{}",
            module_id.address().to_hex_literal(),
            module_id.name().as_str()
        );
        self.store.load::<Vec<u8>>(&key).ok().flatten()
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
        self.store.save(&key, &(owner, cap))?;

        // Update index
        let mut index = self
            .store
            .load::<Vec<String>>("treasury_index")?
            .unwrap_or_default();
        if !index.iter().any(|x| x == &key) {
            index.push(key);
            self.store.save("treasury_index", &index)?;
        }

        Ok(())
    }

    /// Load persisted treasuries as a vector of tuples (owner, token_type, TreasuryCap)
    pub fn load_treasuries(&self) -> Result<Vec<(AccountAddress, String, TreasuryCap)>> {
        let mut out = Vec::new();
        if let Ok(Some(keys)) = self.store.load::<Vec<String>>("treasury_index") {
            for key in keys.into_iter() {
                if let Ok(Some((owner_addr, cap))) =
                    self.store.load::<(AccountAddress, TreasuryCap)>(&key)
                {
                    let token_type = key.strip_prefix("treasury:").unwrap_or(&key).to_string();
                    out.push((owner_addr, token_type, cap));
                }
            }
        }
        Ok(out)
    }
}
