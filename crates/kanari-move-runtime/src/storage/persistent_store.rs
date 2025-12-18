// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use serde::{Serialize, de::DeserializeOwned};
use std::path::PathBuf;
use std::sync::Arc;

use crate::storage::shared_db::get_or_open_db;
use rocksdb::DB;

/// Lightweight persistent BCS-backed store for runtime state (blockchain, accounts, etc.)
///
/// Note: method names remain `save`/`load` for API compatibility,
/// but values are serialized using BCS (binary canonical serialization).
pub struct PersistentStore {
    db: Arc<DB>,
}

impl PersistentStore {
    /// Open default DB at `~/.kanari/kanari-db/state_db`.
    pub fn open_default() -> Result<Self> {
        // Use shared DB; allow override with KANARI_STATE_DB env var for backwards compat.
        let db_path = std::env::var("KANARI_STATE_DB").ok().map(PathBuf::from);
        let db = get_or_open_db(db_path)?;
        Ok(PersistentStore { db })
    }

    /// Save a serializable value under `key` using BCS encoding.
    pub fn save<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let bytes =
            bcs::to_bytes(value).context("Failed to serialize value for PersistentStore")?;
        self.db
            .put(key.as_bytes(), &bytes)
            .context("Failed to write value into PersistentStore RocksDB")?;
        Ok(())
    }

    /// Load a value encoded with BCS from `key` if it exists.
    pub fn load<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self.db.get(key.as_bytes()) {
            Ok(Some(v)) => {
                let obj = bcs::from_bytes(&v)
                    .context("Failed to deserialize value from PersistentStore")?;
                Ok(Some(obj))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(anyhow::anyhow!(format!("RocksDB error: {}", e))),
        }
    }

    /// Delete a key from the DB
    pub fn delete(&self, key: &str) -> Result<()> {
        self.db
            .delete(key.as_bytes())
            .context("Failed to delete key from PersistentStore RocksDB")?;
        Ok(())
    }
}
