use anyhow::{Context, Result};
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::ModuleId;
use move_vm_test_utils::InMemoryStorage;
use rocksdb::Direction;
use rocksdb::{DB, IteratorMode, Options};
use std::path::PathBuf;

/// Simple persistent store for published modules and small runtime state.
pub struct MoveVMState {
    db: DB,
}

impl MoveVMState {
    /// Open default DB at `~/.kari/kanari-db/move_vm_db`.
    pub fn open_default() -> Result<Self> {
        // Allow overriding the DB directory via env var for tests or custom setups.
        if let Ok(dir) = std::env::var("KANARI_MOVE_VM_DB") {
            let mut path = PathBuf::from(dir);
            std::fs::create_dir_all(&path).context("Failed to create MoveVMState DB directory")?;
            // Use given path directly (can be a file path or a directory). If it's a directory,
            // use a default DB name inside it.
            if path.is_dir() {
                path.push("move_vm_db");
            }
            let mut opts = Options::default();
            opts.create_if_missing(true);
            let db = DB::open(&opts, path).context("Failed to open RocksDB for MoveVMState")?;
            return Ok(MoveVMState { db });
        }

        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".kari");
        path.push("kanari-db");
        std::fs::create_dir_all(&path).context("Failed to create MoveVMState DB directory")?;
        path.push("move_vm_db");

        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path).context("Failed to open RocksDB for MoveVMState")?;
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
