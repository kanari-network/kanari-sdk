// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
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
    /// Create an in-memory MoveVMState for testing (uses temp directory)
    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self> {
        use std::time::{SystemTime, UNIX_EPOCH};

        use anyhow::Context;

        // Create unique temp directory for this test
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_path = std::env::temp_dir().join(format!("kanari_test_{}", timestamp));

        std::fs::create_dir_all(&temp_path)
            .context("Failed to create temp MoveVMState directory")?;

        let store = PersistentStore::open_with_path(Some(temp_path))?;
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
    pub fn load_into_storage(&self, storage: &mut InMemoryStorage) -> Result<()> {
        // Prefix scan is not directly supported by SMT shim; fallback to RocksDB
        // behavior by attempting to iterate using underlying RocksDB if available.
        // For simplicity, attempt to load by trying keys stored in an index key
        // `module_index` if present, otherwise return Ok(()) to avoid blocking.

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
                    let ident = Identifier::from_utf8(parts[2].as_bytes().to_vec()).ok();
                    if let (Some(a), Some(id)) = (addr, ident) {
                        let module_id = ModuleId::new(a, id);
                        storage.publish_or_overwrite_module(module_id, blob);
                    }
                }
            }
        }

        Ok(())
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
}
