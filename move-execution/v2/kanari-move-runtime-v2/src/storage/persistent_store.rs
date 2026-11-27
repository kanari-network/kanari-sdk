// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex, MutexGuard, OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard, Weak,
};

use rocksdb::{ColumnFamily, DB, IteratorMode, WriteBatch};

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

/// Type alias for in-memory store to reduce complexity
type MemoryStore = Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>;
type RawKeyValue = (Vec<u8>, Vec<u8>);

/// One genesis lock per shared RocksDB handle. `open_or_get_db` returns one
/// `Arc<DB>` for repeated opens of a path, so all wrappers around that DB share
/// this lock without serializing unrelated databases.
static DB_GENESIS_LOCKS: OnceLock<Mutex<HashMap<usize, Weak<Mutex<()>>>>> = OnceLock::new();

fn genesis_lock_for_db(db: &Arc<DB>) -> Arc<Mutex<()>> {
    let key = Arc::as_ptr(db) as usize;
    let registry = DB_GENESIS_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut registry = kanari_types::sync::mutex_lock_or_recover(registry);
    registry.retain(|_, lock| lock.strong_count() > 0);
    if let Some(lock) = registry.get(&key).and_then(Weak::upgrade) {
        return lock;
    }

    let lock = Arc::new(Mutex::new(()));
    registry.insert(key, Arc::downgrade(&lock));
    lock
}

/// Lightweight persistent BCS-backed store for runtime state using RocksDB.
#[derive(Debug)]
#[must_use]
pub struct PersistentStore {
    db: Option<Arc<DB>>,
    // Active state column families. Their count is persisted in the DB so a
    // later process cannot route keys differently after a config change.
    state_cf_names: Vec<String>,
    // In-memory store for when RocksDB is not used (e.g. tests, Miri)
    memory_store: Option<MemoryStore>,
    transaction_lock: Mutex<()>,
    // Serializes first-time genesis check, initialization, and commit for the
    // underlying database without blocking unrelated databases in this process.
    genesis_init_lock: Arc<Mutex<()>>,
}

impl PersistentStore {
    /// Serialize read-modify-write transactions that maintain secondary indexes.
    pub(crate) fn transaction_guard(&self) -> MutexGuard<'_, ()> {
        kanari_types::sync::mutex_lock_or_recover(&self.transaction_lock)
    }

    /// Serialize first-time genesis initialization for this store's database.
    pub(crate) fn genesis_init_guard(&self) -> MutexGuard<'_, ()> {
        kanari_types::sync::mutex_lock_or_recover(&self.genesis_init_lock)
    }

    /// Open default store path (same behavior as previous implementation).
    pub fn open_default() -> Result<Self> {
        let db_path = std::env::var("KANARI_STATE_DB").ok().map(PathBuf::from);
        Self::open_with_path(db_path)
    }

    /// Open store using an explicit path (None -> default).
    pub fn open_with_path(path_opt: Option<PathBuf>) -> Result<Self> {
        let (db, shard_count) = kanari_db_common::open_or_get_db_with_layout(path_opt)?;
        let state_cf_names = (0..shard_count).map(kanari_db_common::state_cf_name).collect();
        let genesis_init_lock = genesis_lock_for_db(&db);
        Ok(PersistentStore {
            db: Some(db),
            state_cf_names,
            memory_store: None,
            transaction_lock: Mutex::new(()),
            genesis_init_lock,
        })
    }

    /// Open an in-memory-only store that performs no filesystem operations.
    /// Useful for tests and Miri where disk APIs may be unavailable.
    pub fn open_in_memory() -> Result<Self> {
        Ok(PersistentStore {
            db: None,
            state_cf_names: Vec::new(),
            memory_store: Some(Arc::new(RwLock::new(HashMap::new()))),
            transaction_lock: Mutex::new(()),
            genesis_init_lock: Arc::new(Mutex::new(())),
        })
    }

    fn cf_index_for_key(&self, key: &[u8]) -> usize {
        let shard_count = self.state_cf_names.len().max(1);
        // SMT nodes/index metadata and the persisted CF-layout marker must stay
        // in default so `get_db()` consumers (SMT diagnostics/maintenance)
        // still see them. `meta:` is not part of the logical-state view, so it
        // is intentionally not included in `is_internal_key` below.
        if Self::is_internal_key(key) || key.starts_with(b"meta:") {
            return shard_count;
        }
        kanari_db_common::shard_index_for_key(key, shard_count)
    }

    fn cf_handle_for_index<'a>(
        &self,
        db: &'a DB,
        index: usize,
    ) -> std::result::Result<&'a ColumnFamily, PersistentStoreError> {
        let name = if index == self.state_cf_names.len().max(1) {
            "default"
        } else {
            self.state_cf_names.get(index).map(String::as_str).unwrap_or("state_0")
        };
        db.cf_handle(name).ok_or_else(|| {
            PersistentStoreError::Internal(format!("Missing RocksDB column family {name}"))
        })
    }

    fn cf_for_key<'a>(
        &self,
        db: &'a DB,
        key: &[u8],
    ) -> std::result::Result<&'a ColumnFamily, PersistentStoreError> {
        self.cf_handle_for_index(db, self.cf_index_for_key(key))
    }

    fn scan_cf_names(&self) -> Vec<&str> {
        let mut names = Vec::with_capacity(self.state_cf_names.len() + 1);
        names.push("default");
        names.extend(self.state_cf_names.iter().map(String::as_str));
        names
    }

    fn read_memory_store(store: &MemoryStore) -> RwLockReadGuard<'_, HashMap<Vec<u8>, Vec<u8>>> {
        kanari_types::sync::rw_read_or_recover(store)
    }

    fn write_memory_store(store: &MemoryStore) -> RwLockWriteGuard<'_, HashMap<Vec<u8>, Vec<u8>>> {
        kanari_types::sync::rw_write_or_recover(store)
    }

    fn read_raw(&self, key: &[u8]) -> std::result::Result<Option<Vec<u8>>, PersistentStoreError> {
        if let Some(db) = &self.db {
            let cf = self.cf_for_key(db.as_ref(), key)?;
            return Ok(db.get_cf(cf, key)?);
        }

        if let Some(store) = &self.memory_store {
            return Ok(Self::read_memory_store(store).get(key).cloned());
        }

        Ok(None)
    }

    fn write_raw(
        &self,
        key: &[u8],
        value: Vec<u8>,
    ) -> std::result::Result<(), PersistentStoreError> {
        if let Some(db) = &self.db {
            let cf = self.cf_for_key(db.as_ref(), key)?;
            db.put_cf(cf, key, value)?;
        } else if let Some(store) = &self.memory_store {
            Self::write_memory_store(store).insert(key.to_vec(), value);
        }
        Ok(())
    }

    /// Save a serializable value under `key`.
    pub fn save<T: Serialize + ?Sized>(
        &self,
        key: &[u8],
        value: &T,
    ) -> std::result::Result<(), PersistentStoreError> {
        let bytes = bcs::to_bytes(value)?;
        self.write_raw(key, bytes)
    }

    /// Load a deserializable value from `key`.
    pub fn load<T: DeserializeOwned>(
        &self,
        key: &[u8],
    ) -> std::result::Result<Option<T>, PersistentStoreError> {
        match self.read_raw(key)? {
            Some(bytes) => Ok(Some(bcs::from_bytes(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Batch form of [`Self::load`]. Results keep the exact input order, and
    /// each slot carries its own backend/deserialization error so callers can
    /// attribute a failure to the same key they passed in. The RocksDB backend
    /// uses one `multi_get` for the whole batch instead of one `get` per key;
    /// the single-key behavior (including fail-closed errors) is unchanged.
    pub fn load_many<T: DeserializeOwned>(
        &self,
        keys: &[Vec<u8>],
    ) -> Vec<std::result::Result<Option<T>, PersistentStoreError>> {
        if keys.is_empty() {
            return Vec::new();
        }
        if let Some(db) = &self.db {
            // Resolve a CF handle once per shard instead of once per key while
            // keeping the exact input order expected by callers.
            let mut handles: Vec<Option<&ColumnFamily>> =
                vec![None; self.state_cf_names.len().max(1) + 1];
            let mut cf_keys = Vec::with_capacity(keys.len());
            let mut routing_error = None;
            for key in keys {
                let index = self.cf_index_for_key(key);
                let cf = match handles[index] {
                    Some(cf) => cf,
                    None => match self.cf_handle_for_index(db.as_ref(), index) {
                        Ok(cf) => {
                            handles[index] = Some(cf);
                            cf
                        }
                        Err(error) => {
                            if routing_error.is_none() {
                                routing_error = Some(error.to_string());
                            }
                            continue;
                        }
                    },
                };
                cf_keys.push((cf, key));
            }
            if let Some(message) = routing_error {
                return (0..keys.len())
                    .map(|_| Err(PersistentStoreError::Internal(message.clone())))
                    .collect();
            }
            return db
                .multi_get_cf(cf_keys)
                .into_iter()
                .map(|result| match result? {
                    Some(bytes) => Ok(Some(bcs::from_bytes::<T>(&bytes)?)),
                    None => Ok(None),
                })
                .collect();
        }
        if let Some(store) = &self.memory_store {
            let guard = Self::read_memory_store(store);
            return keys
                .iter()
                .map(|key| match guard.get(key) {
                    Some(bytes) => Ok(Some(bcs::from_bytes::<T>(bytes)?)),
                    None => Ok(None),
                })
                .collect();
        }
        (0..keys.len()).map(|_| Ok(None)).collect()
    }

    /// Delete a key from the store.
    pub fn delete(&self, key: &[u8]) -> std::result::Result<(), PersistentStoreError> {
        if let Some(db) = &self.db {
            let cf = self.cf_for_key(db.as_ref(), key)?;
            db.delete_cf(cf, key)?;
        } else if let Some(store) = &self.memory_store {
            Self::write_memory_store(store).remove(key);
        }
        Ok(())
    }

    /// Return a stable snapshot of logical state entries, excluding SMT
    /// internals and the persisted CF-layout marker. Merging through a
    /// `BTreeMap` keeps the byte-sorted order used by canonical root recompute.
    pub fn logical_entries(&self) -> std::result::Result<Vec<RawKeyValue>, PersistentStoreError> {
        let mut entries = Vec::new();

        if let Some(db) = &self.db {
            // Default CF first: a legacy key still present there must lose to
            // the routed copy in a state CF if a migration was interrupted.
            let mut merged: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
            for cf_name in self.scan_cf_names() {
                let Some(cf) = db.cf_handle(cf_name) else {
                    continue;
                };
                for item in db.iterator_cf(cf, IteratorMode::Start) {
                    let (key, value) = item?;
                    if Self::is_internal_key(&key) {
                        continue;
                    }
                    merged.insert(key.to_vec(), value.to_vec());
                }
            }
            entries.extend(merged);
        } else if let Some(store) = &self.memory_store {
            entries.extend(
                Self::read_memory_store(store)
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone())),
            );
        }

        entries.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(entries)
    }

    /// Check whether any key exists with the given byte prefix, without
    /// materializing all matching entries. Every CF is probed because keys are
    /// routed by hash rather than by prefix.
    pub fn has_prefix(&self, prefix: &[u8]) -> std::result::Result<bool, PersistentStoreError> {
        if let Some(db) = &self.db {
            for cf_name in self.scan_cf_names() {
                let Some(cf) = db.cf_handle(cf_name) else {
                    continue;
                };
                let mut iter =
                    db.iterator_cf(cf, IteratorMode::From(prefix, rocksdb::Direction::Forward));
                if let Some(Ok((key, _))) = iter.next()
                    && key.starts_with(prefix)
                {
                    return Ok(true);
                }
            }
            Ok(false)
        } else if let Some(store) = &self.memory_store {
            let guard = Self::read_memory_store(store);
            Ok(guard.keys().any(|k| k.starts_with(prefix)))
        } else {
            Ok(false)
        }
    }

    /// Returns `true` if any logical (non-internal) entry exists, without
    /// dumping the entire store. Stops at the first qualifying key.
    pub fn has_any_logical_entry(&self) -> std::result::Result<bool, PersistentStoreError> {
        if let Some(db) = &self.db {
            for cf_name in self.scan_cf_names() {
                let Some(cf) = db.cf_handle(cf_name) else {
                    continue;
                };
                for item in db.iterator_cf(cf, IteratorMode::Start) {
                    let (key, _) = item?;
                    if !Self::is_internal_key(&key) {
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        } else if let Some(store) = &self.memory_store {
            let guard = Self::read_memory_store(store);
            Ok(guard.keys().any(|k| !k.starts_with(b"n:") && !k.starts_with(b"d:")))
        } else {
            Ok(false)
        }
    }

    /// Apply owned raw state changes, moving values into the in-memory backend
    /// instead of cloning the whole checkpoint overlay.
    pub fn apply_raw_changes_owned(
        &self,
        updates: Vec<(Vec<u8>, Vec<u8>)>,
        deletes: Vec<Vec<u8>>,
    ) -> std::result::Result<(), PersistentStoreError> {
        if self.db.is_some() {
            return self.apply_raw_changes(&updates, &deletes);
        }
        if let Some(store) = &self.memory_store {
            let mut guard = Self::write_memory_store(store);
            for (key, value) in updates {
                guard.insert(key, value);
            }
            for key in deletes {
                guard.remove(&key);
            }
        }
        Ok(())
    }

    /// Apply raw state key updates/deletes atomically where the backend supports it.
    pub fn apply_raw_changes(
        &self,
        updates: &[(Vec<u8>, Vec<u8>)],
        deletes: &[Vec<u8>],
    ) -> std::result::Result<(), PersistentStoreError> {
        if let Some(db) = &self.db {
            let mut batch = WriteBatch::default();
            self.stage_raw_changes_in_batch(&mut batch, updates, deletes)?;
            db.write(batch)?;
        } else if let Some(store) = &self.memory_store {
            let mut guard = Self::write_memory_store(store);
            for (key, value) in updates {
                guard.insert(key.clone(), value.clone());
            }
            for key in deletes {
                guard.remove(key);
            }
        }
        Ok(())
    }

    /// Stage routed key updates/deletes into a caller-owned batch. This lets
    /// `commit.rs` keep state keys and SMT nodes in the same atomic RocksDB
    /// write while each state key goes to its own column family.
    pub(crate) fn stage_raw_changes_in_batch(
        &self,
        batch: &mut WriteBatch,
        updates: &[(Vec<u8>, Vec<u8>)],
        deletes: &[Vec<u8>],
    ) -> std::result::Result<(), PersistentStoreError> {
        let Some(db) = &self.db else {
            return Ok(());
        };
        // Resolve each CF handle once instead of once per key. Input order is
        // preserved, so repeated keys keep the same last-write-wins result and
        // the batch contains byte-identical entries.
        let mut handles: Vec<Option<&ColumnFamily>> =
            vec![None; self.state_cf_names.len().max(1) + 1];
        for (key, value) in updates {
            let index = self.cf_index_for_key(key);
            let cf = match handles[index] {
                Some(cf) => cf,
                None => {
                    let cf = self.cf_handle_for_index(db.as_ref(), index)?;
                    handles[index] = Some(cf);
                    cf
                }
            };
            batch.put_cf(cf, key, value);
        }
        for key in deletes {
            let index = self.cf_index_for_key(key);
            let cf = match handles[index] {
                Some(cf) => cf,
                None => {
                    let cf = self.cf_handle_for_index(db.as_ref(), index)?;
                    handles[index] = Some(cf);
                    cf
                }
            };
            batch.delete_cf(cf, key);
        }
        Ok(())
    }

    /// Expose underlying RocksDB instance for other components (e.g. SMT)
    pub fn get_db(&self) -> Option<Arc<DB>> {
        self.db.clone()
    }

    /// Flush pending RocksDB writes before a node snapshot or clean shutdown.
    pub fn flush(&self) -> std::result::Result<(), PersistentStoreError> {
        if let Some(db) = &self.db {
            for cf_name in self.scan_cf_names() {
                if let Some(cf) = db.cf_handle(cf_name) {
                    db.flush_cf(cf)?;
                }
            }
        }
        Ok(())
    }

    /// Request a full-range compaction. This only reclaims obsolete RocksDB
    /// versions; it never prunes canonical blockchain keys.
    pub fn compact(&self) {
        if let Some(db) = &self.db {
            for cf_name in self.scan_cf_names() {
                if let Some(cf) = db.cf_handle(cf_name) {
                    db.compact_range_cf::<&[u8], &[u8]>(cf, None, None);
                }
            }
        }
    }

    /// Read a RocksDB string property for benchmark/operational diagnostics.
    /// The default CF report is kept as the representative string snapshot;
    /// per-CF counters are exposed through `rocksdb_int_property`.
    pub fn rocksdb_property(
        &self,
        name: &str,
    ) -> std::result::Result<Option<String>, PersistentStoreError> {
        let Some(db) = &self.db else {
            return Ok(None);
        };
        Ok(db.property_value(name)?)
    }

    /// Read a RocksDB integer property for benchmark/operational diagnostics.
    /// Counters are summed across the active CFs so sharding does not hide
    /// memtable/compaction pressure in non-default column families.
    pub fn rocksdb_int_property(
        &self,
        name: &str,
    ) -> std::result::Result<Option<u64>, PersistentStoreError> {
        let Some(db) = &self.db else {
            return Ok(None);
        };
        let mut total: Option<u64> = None;
        for cf_name in self.scan_cf_names() {
            let Some(cf) = db.cf_handle(cf_name) else {
                continue;
            };
            if let Some(value) = db.property_int_value_cf(cf, name)? {
                let slot = total.get_or_insert(0);
                *slot = (*slot).saturating_add(value);
            }
        }
        Ok(total)
    }

    fn is_internal_key(key: &[u8]) -> bool {
        Self::is_internal_smt_key(key) || key == kanari_db_common::STATE_CF_SHARD_COUNT_KEY
    }

    fn is_internal_smt_key(key: &[u8]) -> bool {
        key.starts_with(b"n:") || key.starts_with(b"d:")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_many_preserves_order_and_missing_slots() {
        let store = PersistentStore::open_in_memory().unwrap();
        let keys = vec![b"k0".to_vec(), b"k1".to_vec(), b"k2".to_vec()];
        store.save(&keys[0], &10u64).unwrap();
        store.save(&keys[2], &30u64).unwrap();

        let loaded = store.load_many::<u64>(&keys);
        assert_eq!(loaded.len(), keys.len());
        assert_eq!(loaded[0].as_ref().unwrap(), &Some(10));
        assert!(loaded[1].as_ref().unwrap().is_none());
        assert_eq!(loaded[2].as_ref().unwrap(), &Some(30));
    }

    #[test]
    fn load_many_keeps_error_on_matching_slot() {
        let store = PersistentStore::open_in_memory().unwrap();
        let good = b"good".to_vec();
        let bad = b"bad".to_vec();
        store.save(&good, &7u64).unwrap();
        // A single byte cannot decode as BCS u64 (needs eight bytes).
        store.write_raw(&bad, vec![0xff]).unwrap();

        let loaded = store.load_many::<u64>(&[good, bad]);
        assert_eq!(loaded[0].as_ref().unwrap(), &Some(7));
        assert!(loaded[1].is_err());
    }

    #[test]
    fn load_many_empty_input_is_empty() {
        let store = PersistentStore::open_in_memory().unwrap();
        assert!(store.load_many::<u64>(&[]).is_empty());
    }

    #[test]
    fn rocksdb_state_keys_are_routed_to_column_families() {
        let nanos =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir =
            std::env::temp_dir().join(format!("kanari-cf-store-{}-{nanos}", std::process::id()));
        let store = PersistentStore::open_with_path(Some(dir.clone())).unwrap();
        let key = b"object:cf-routing-test".to_vec();
        store.save(&key, &123u64).unwrap();
        store.write_raw(b"n:diagnostic", vec![1, 2, 3]).unwrap();
        store.write_raw(b"meta:node_index_version", b"1".to_vec()).unwrap();

        let db = store.get_db().unwrap();
        let index = kanari_db_common::shard_index_for_key(&key, store.state_cf_names.len());
        let cf_name = &store.state_cf_names[index];
        let cf = db.cf_handle(cf_name).unwrap();
        assert!(db.get_cf(cf, &key).unwrap().is_some());
        assert!(db.get(&key).unwrap().is_none());
        assert!(db.get(b"n:diagnostic").unwrap().is_some());
        assert!(db.get(b"meta:node_index_version").unwrap().is_some());

        let entries = store.logical_entries().unwrap();
        assert_eq!(
            entries,
            vec![
                (b"meta:node_index_version".to_vec(), b"1".to_vec()),
                (key.clone(), bcs::to_bytes(&123u64).unwrap()),
            ]
        );
        assert!(store.has_any_logical_entry().unwrap());

        drop(db);
        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_default_cf_state_keys_migrate_without_touching_smt_internals() {
        let nanos =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir =
            std::env::temp_dir().join(format!("kanari-cf-migrate-{}-{nanos}", std::process::id()));

        // Simulate a database written by the former single-CF layout.
        {
            let mut opts = rocksdb::Options::default();
            opts.create_if_missing(true);
            let db = rocksdb::DB::open(&opts, &dir).unwrap();
            db.put(b"object:legacy", vec![9, 8, 7]).unwrap();
            db.put(b"n:legacy-node", vec![6]).unwrap();
            db.put(b"meta:node_index_version", b"1".to_vec()).unwrap();
        }

        let store = PersistentStore::open_with_path(Some(dir.clone())).unwrap();
        let db = store.get_db().unwrap();
        assert!(db.get(b"object:legacy").unwrap().is_none());
        assert_eq!(db.get(b"n:legacy-node").unwrap(), Some(vec![6]));
        assert_eq!(db.get(b"meta:node_index_version").unwrap(), Some(b"1".to_vec()));
        assert!(db.get(kanari_db_common::STATE_CF_SHARD_COUNT_KEY).unwrap().is_some());

        let cf = store.cf_for_key(db.as_ref(), b"object:legacy").unwrap();
        assert_eq!(db.get_cf(cf, b"object:legacy").unwrap(), Some(vec![9, 8, 7]));
        assert_eq!(
            store.logical_entries().unwrap(),
            vec![
                (b"meta:node_index_version".to_vec(), b"1".to_vec()),
                (b"object:legacy".to_vec(), vec![9, 8, 7]),
            ]
        );

        drop(db);
        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
