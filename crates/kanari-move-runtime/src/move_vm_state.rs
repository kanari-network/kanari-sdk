use anyhow::{Context, Result};
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::ModuleId;
use move_vm_test_utils::InMemoryStorage;
use rocksdb::Direction;
use rocksdb::IteratorMode;
use std::path::PathBuf;
use std::sync::Arc;

use crate::shared_db::get_or_open_db;
use rocksdb::DB;

/// Simple persistent store for published modules and small runtime state.
pub struct MoveVMState {
    db: Arc<DB>,
}

impl MoveVMState {
    /// Create an in-memory MoveVMState for testing (uses temp directory)
    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self> {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Create unique temp directory for this test
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_path = std::env::temp_dir().join(format!("kanari_test_{}", timestamp));

        std::fs::create_dir_all(&temp_path)
            .context("Failed to create temp MoveVMState directory")?;

        // For test-only in-memory/temp DB we open a fresh RocksDB instance and wrap in Arc.
        let db = get_or_open_db(Some(temp_path))?;
        Ok(MoveVMState { db })
    }

    /// Open default DB at `~/.kari/kanari-db/move_vm_db`.
    pub fn open_default() -> Result<Self> {
        // Use shared DB (single RocksDB instance for the process). The shared DB path
        // can be overridden via `KANARI_DB` env var; legacy `KANARI_MOVE_VM_DB` is also supported
        // for backward compatibility (mapped into the shared DB).
        let db_path = if let Ok(dir) = std::env::var("KANARI_MOVE_VM_DB") {
            Some(PathBuf::from(dir))
        } else {
            None
        };

        let db = get_or_open_db(db_path)?;
        Ok(MoveVMState { db })
    }

    /// Save a module blob keyed by module id.
    pub fn save_module(&self, module_id: &ModuleId, blob: &[u8]) -> Result<()> {
        // NOTE: We use a string key for now. A binary serialization of ModuleId
        // would be more efficient; consider migrating to that format later.
        let key = format!(
            "module:{}:{}",
            module_id.address().to_hex_literal(),
            module_id.name().as_str()
        );
        self.db
            .put(key.as_bytes(), blob)
            .context("Failed to write module blob into MoveVMState RocksDB")?;
        Ok(())
    }

    /// Load persisted modules into an `InMemoryStorage` instance.
    pub fn load_into_storage(&self, storage: &mut InMemoryStorage) -> Result<()> {
        // Start iteration from the module prefix to avoid scanning unrelated keys.
        let prefix = b"module:";
        let iter = self
            .db
            .iterator(IteratorMode::From(prefix, Direction::Forward));

        for item in iter {
            let (key, value) = item.context("Error iterating MoveVMState RocksDB")?;

            // Convert key bytes to string once and fail fast on invalid UTF-8.
            let s =
                String::from_utf8(key.to_vec()).context("MoveVMState DB contains non-UTF8 key")?;

            // Ensure key starts with expected prefix (safety for IteratorMode::From)
            if !s.starts_with("module:") {
                // Reached keys beyond the module prefix - stop iteration.
                break;
            }

            // Expected format: module:{address}:{name}
            let parts: Vec<&str> = s.splitn(3, ':').collect();
            if parts.len() != 3 {
                anyhow::bail!("Malformed module key found in MoveVMState DB: {}", s);
            }

            let addr_str = parts[1];
            let name = parts[2];

            let addr = AccountAddress::from_hex_literal(addr_str).context(format!(
                "Invalid AccountAddress in module key: {}",
                addr_str
            ))?;

            let ident = Identifier::from_utf8(name.as_bytes().to_vec())
                .context(format!("Invalid module name in module key: {}", name))?;

            let module_id = ModuleId::new(addr, ident);
            storage.publish_or_overwrite_module(module_id, value.to_vec());
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
        self.db
            .get(key.as_bytes())
            .ok()
            .flatten()
            .map(|v| v.to_vec())
    }
}
