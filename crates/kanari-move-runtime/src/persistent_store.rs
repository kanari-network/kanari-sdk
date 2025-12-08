use anyhow::{Context, Result};
use serde::{Serialize, de::DeserializeOwned};
use std::path::PathBuf;
use std::sync::Arc;

use crate::shared_db::get_or_open_db;
use rocksdb::DB;

/// Lightweight persistent JSON store for runtime state (blockchain, accounts, etc.)
pub struct PersistentStore {
    db: Arc<DB>,
}

impl PersistentStore {
    /// Open default DB at `~/.kari/kanari-db/state_db`.
    pub fn open_default() -> Result<Self> {
        // Use shared DB; allow override with KANARI_STATE_DB env var for backwards compat.
        let db_path = std::env::var("KANARI_STATE_DB").ok().map(PathBuf::from);
        let db = get_or_open_db(db_path)?;
        Ok(PersistentStore { db })
    }

    /// Save a serializable value under `key` as JSON.
    pub fn save_json<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let bytes =
            serde_json::to_vec(value).context("Failed to serialize value for PersistentStore")?;
        self.db
            .put(key.as_bytes(), &bytes)
            .context("Failed to write value into PersistentStore RocksDB")?;
        Ok(())
    }

    /// Load a JSON-serialized value from `key` if it exists.
    pub fn load_json<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self.db.get(key.as_bytes()) {
            Ok(Some(v)) => {
                let obj = serde_json::from_slice(&v)
                    .context("Failed to deserialize value from PersistentStore")?;
                Ok(Some(obj))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(anyhow::anyhow!(format!("RocksDB error: {}", e))),
        }
    }
}
