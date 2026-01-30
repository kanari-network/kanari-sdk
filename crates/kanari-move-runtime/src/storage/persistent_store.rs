// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};
use std::path::PathBuf;
use std::sync::Arc;

use crate::storage::shared_db::get_or_open_db;
use rocksdb::DB;

/// Custom error type for PersistentStore operations
#[derive(Debug)]
pub enum PersistentStoreError {
    RocksDB(rocksdb::Error),
    Serialization(bcs::Error),
    Internal(String),
    Io(std::io::Error),
}

impl std::fmt::Display for PersistentStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistentStoreError::RocksDB(e) => write!(f, "RocksDB error: {}", e),
            PersistentStoreError::Serialization(e) => write!(f, "Serialization error: {}", e),
            PersistentStoreError::Internal(e) => write!(f, "Internal error: {}", e),
            PersistentStoreError::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for PersistentStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PersistentStoreError::RocksDB(e) => Some(e),
            PersistentStoreError::Serialization(e) => Some(e),
            PersistentStoreError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<rocksdb::Error> for PersistentStoreError {
    fn from(e: rocksdb::Error) -> Self {
        PersistentStoreError::RocksDB(e)
    }
}

impl From<bcs::Error> for PersistentStoreError {
    fn from(e: bcs::Error) -> Self {
        PersistentStoreError::Serialization(e)
    }
}

impl From<std::io::Error> for PersistentStoreError {
    fn from(e: std::io::Error) -> Self {
        PersistentStoreError::Io(e)
    }
}

/// Lightweight persistent BCS-backed store for runtime state using RocksDB.
pub struct PersistentStore {
    db: Option<Arc<DB>>,
}

impl PersistentStore {
    /// Open default store path (same behavior as previous implementation).
    pub fn open_default() -> Result<Self> {
        let db_path = std::env::var("KANARI_STATE_DB").ok().map(PathBuf::from);
        Self::open_with_path(db_path)
    }

    /// Open store using an explicit path (None -> default).
    pub fn open_with_path(path_opt: Option<PathBuf>) -> Result<Self> {
        let db = get_or_open_db(path_opt)?;
        Ok(PersistentStore { db: Some(db) })
    }

    /// Open an in-memory-only store that performs no filesystem operations.
    /// Useful for tests and Miri where disk APIs may be unavailable.
    pub fn open_in_memory() -> Result<Self> {
        Ok(PersistentStore { db: None })
    }

    /// Save a serializable value under `key`.
    pub fn save<T: Serialize + ?Sized>(
        &self,
        key: &str,
        value: &T,
    ) -> std::result::Result<(), PersistentStoreError> {
        let bytes = bcs::to_bytes(value)?;

        if let Some(db) = &self.db {
            db.put(key.as_bytes(), &bytes)?;
        }

        Ok(())
    }

    /// Flush all pending writes to the backing store synchronously.
    pub fn flush(&self) -> std::result::Result<(), PersistentStoreError> {
        // RocksDB writes are synchronous in this implementation
        Ok(())
    }

    /// Load a value encoded with BCS from `key` if it exists.
    pub fn load<T: DeserializeOwned>(
        &self,
        key: &str,
    ) -> std::result::Result<Option<T>, PersistentStoreError> {
        if let Some(db) = &self.db {
            match db.get(key.as_bytes()) {
                Ok(Some(v)) => {
                    let obj = bcs::from_bytes(&v)?;
                    Ok(Some(obj))
                }
                Ok(None) => Ok(None),
                Err(e) => Err(PersistentStoreError::RocksDB(e)),
            }
        } else {
            Ok(None)
        }
    }

    /// Delete a key from the store.
    pub fn delete(&self, key: &str) -> std::result::Result<(), PersistentStoreError> {
        if let Some(db) = &self.db {
            db.delete(key.as_bytes())?;
        }
        Ok(())
    }
}
