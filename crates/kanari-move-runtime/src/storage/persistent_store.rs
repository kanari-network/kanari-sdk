// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::storage::shared_db::get_or_open_db;
use rocksdb::DB;
use smt::SparseMerkleTree;
use zstd;

/// Custom error type for PersistentStore operations
#[derive(Debug)]
pub enum PersistentStoreError {
    RocksDB(rocksdb::Error),
    Serialization(bcs::Error),
    Compression(std::io::Error),
    Smt(anyhow::Error),
    Channel(String),
    Internal(String),
    Io(std::io::Error),
}

impl std::fmt::Display for PersistentStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistentStoreError::RocksDB(e) => write!(f, "RocksDB error: {}", e),
            PersistentStoreError::Serialization(e) => write!(f, "Serialization error: {}", e),
            PersistentStoreError::Compression(e) => write!(f, "Compression error: {}", e),
            PersistentStoreError::Smt(e) => write!(f, "SMT error: {}", e),
            PersistentStoreError::Channel(e) => write!(f, "Channel error: {}", e),
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
            PersistentStoreError::Compression(e) => Some(e),
            PersistentStoreError::Smt(e) => e.source(),
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

/// Aliases to simplify complex SMT-related return types
pub type SmtProof = (bool, [u8; 32], Vec<[u8; 32]>);
pub type SmtSnapshot = Vec<(Vec<u8>, Vec<u8>)>;

/// Background write operation for batched persistence.
enum WriteOp {
    Put(Vec<u8>, Vec<u8>),
    Delete(Vec<u8>),
    Flush(mpsc::Sender<()>),
}

/// Lightweight persistent BCS-backed store for runtime state. Internally it
/// prefers an SMT backend (fast batched writes) and falls back to RocksDB for
/// compatibility. Writes are enqueued and flushed in a background thread in
/// batches to maximize throughput. Set `KANARI_PERSIST_SYNC=1` to force
/// synchronous writes for callers that require durability immediately.
pub struct PersistentStore {
    db: Option<Arc<DB>>,
    smt: Option<Arc<SparseMerkleTree>>,
    sender: Option<mpsc::Sender<WriteOp>>,
    worker: Option<JoinHandle<()>>,
}

impl PersistentStore {
    /// Open default store path (same behavior as previous implementation).
    pub fn open_default() -> Result<Self> {
        let db_path = std::env::var("KANARI_STATE_DB").ok().map(PathBuf::from);
        Self::open_with_path(db_path)
    }

    /// Open store using an explicit path (None -> default). This will open
    /// both RocksDB (for compatibility) and SMT (used for fast commits).
    pub fn open_with_path(path_opt: Option<PathBuf>) -> Result<Self> {
        // RocksDB (legacy)
        let db = get_or_open_db(path_opt.clone())?;

        // SMT (prefer opening at same parent directory if provided)
        let smt = SparseMerkleTree::open(path_opt).ok().map(Arc::new);

        // Start background writer if SMT available
        let (sender, worker) = if let Some(smt_arc) = smt.clone() {
            let (tx, rx) = mpsc::channel::<WriteOp>();
            let worker_smt = smt_arc.clone();
            let worker_db = db.clone(); // Capture DB for legacy cleanup
            let handle = thread::spawn(move || {
                // batch loop
                let mut puts: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
                let mut deletes: Vec<Vec<u8>> = Vec::new();

                // Helper to execute flush
                let do_flush = |w_smt: &Arc<SparseMerkleTree>,
                                w_db: &Arc<DB>,
                                p: &mut Vec<(Vec<u8>, Vec<u8>)>,
                                d: &mut Vec<Vec<u8>>| {
                    if !p.is_empty() {
                        let _ = w_smt.insert(p);
                        p.clear();
                    }
                    if !d.is_empty() {
                        // Delete from SMT
                        let _ = w_smt.delete(d);

                        // Also delete from legacy RocksDB to prevent "zombie" data
                        // reappearing if SMT entry is removed but legacy entry remains.
                        let mut batch = rocksdb::WriteBatch::default();
                        for k in d.iter() {
                            batch.delete(k);
                        }
                        let _ = w_db.write(batch);

                        d.clear();
                    }
                };

                loop {
                    // Wait for first op with timeout to allow graceful exit
                    match rx.recv_timeout(Duration::from_millis(50)) {
                        Ok(op) => match op {
                            WriteOp::Put(k, v) => puts.push((k, v)),
                            WriteOp::Delete(k) => deletes.push(k),
                            WriteOp::Flush(ack) => {
                                do_flush(&worker_smt, &worker_db, &mut puts, &mut deletes);
                                // Signal completion
                                let _ = ack.send(());
                            }
                        },
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            // timeout - continue to flushing logic
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            // channel closed - flush remaining and exit
                            do_flush(&worker_smt, &worker_db, &mut puts, &mut deletes);
                            break;
                        }
                    }

                    // Drain any additional queued ops without blocking
                    while let Ok(op) = rx.try_recv() {
                        match op {
                            WriteOp::Put(k, v) => puts.push((k, v)),
                            WriteOp::Delete(k) => deletes.push(k),
                            WriteOp::Flush(ack) => {
                                do_flush(&worker_smt, &worker_db, &mut puts, &mut deletes);
                                // Signal completion
                                let _ = ack.send(());
                            }
                        }
                        if puts.len() + deletes.len() >= 5000 {
                            break;
                        }
                    }

                    // Flush buffered ops if present
                    do_flush(&worker_smt, &worker_db, &mut puts, &mut deletes);
                }
            });
            (Some(tx), Some(handle))
        } else {
            (None, None)
        };

        Ok(PersistentStore {
            db: Some(db),
            smt,
            sender,
            worker,
        })
    }

    /// Open an in-memory-only store that performs no filesystem operations.
    /// Useful for tests and Miri where disk APIs may be unavailable.
    pub fn open_in_memory() -> Result<Self> {
        Ok(PersistentStore {
            db: None,
            smt: None,
            sender: None,
            worker: None,
        })
    }

    /// Save a serializable value under `key`. By default this enqueues the
    /// write for asynchronous batched persistence to the SMT backend. If
    /// `KANARI_PERSIST_SYNC=1` is set, the write will be performed
    /// synchronously (SMT or RocksDB fallback).
    pub fn save<T: Serialize + ?Sized>(
        &self,
        key: &str,
        value: &T,
    ) -> std::result::Result<(), PersistentStoreError> {
        let bytes = bcs::to_bytes(value)?;

        let sync = std::env::var("KANARI_PERSIST_SYNC").ok().as_deref() == Some("1");

        if sync {
            if let Some(smt_store) = &self.smt {
                smt_store
                    .insert(&[(key.as_bytes().to_vec(), bytes)])
                    .map_err(PersistentStoreError::Smt)?;
                return Ok(());
            }

            if let Some(db) = &self.db {
                db.put(key.as_bytes(), &bytes)?;
                return Ok(());
            }
        }

        // Async path: enqueue to background worker if available
        if let Some(tx) = &self.sender {
            tx.send(WriteOp::Put(key.as_bytes().to_vec(), bytes))
                .map_err(|e| {
                    PersistentStoreError::Channel(format!("Failed to enqueue write: {}", e))
                })?;
            return Ok(());
        }

        // Fallback to synchronous RocksDB write
        if let Some(db) = &self.db {
            db.put(key.as_bytes(), &bytes)?;
            return Ok(());
        }

        Ok(())
    }

    /// Flush all pending writes to the backing store synchronously.
    /// This blocks until the background worker has processed all currently queued operations.
    pub fn flush(&self) -> std::result::Result<(), PersistentStoreError> {
        if let Some(tx) = &self.sender {
            let (ack_tx, ack_rx) = mpsc::channel();
            tx.send(WriteOp::Flush(ack_tx)).map_err(|e| {
                PersistentStoreError::Channel(format!("Failed to enqueue flush: {}", e))
            })?;
            ack_rx.recv().map_err(|e| {
                PersistentStoreError::Channel(format!("Failed to receive flush ack: {}", e))
            })?;
        }
        Ok(())
    }

    /// Load a value encoded with BCS from `key` if it exists.
    pub fn load<T: DeserializeOwned>(
        &self,
        key: &str,
    ) -> std::result::Result<Option<T>, PersistentStoreError> {
        // Try SMT first
        if let Some(smt_store) = &self.smt
            && let Some(b) = smt_store
                .get(key.as_bytes())
                .map_err(PersistentStoreError::Smt)?
        {
            let obj = bcs::from_bytes(&b)?;
            return Ok(Some(obj));
        }

        // Fallback to RocksDB
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

    /// Produce an SMT membership proof for a `key` if SMT backend is available.
    /// Returns Ok(None) when SMT isn't configured; otherwise Some((is_member, leaf_hash, siblings)).
    pub fn proof(&self, key: &str) -> std::result::Result<Option<SmtProof>, PersistentStoreError> {
        if let Some(smt_store) = &self.smt {
            let p = smt_store
                .proof(key.as_bytes())
                .map_err(PersistentStoreError::Smt)?;
            Ok(Some(p))
        } else {
            Ok(None)
        }
    }

    /// Delete a key from the store. Deletions are enqueued if SMT worker
    /// exists; otherwise executed synchronously on RocksDB.
    pub fn delete(&self, key: &str) -> std::result::Result<(), PersistentStoreError> {
        if let Some(tx) = &self.sender {
            tx.send(WriteOp::Delete(key.as_bytes().to_vec()))
                .map_err(|e| {
                    PersistentStoreError::Channel(format!("Failed to enqueue delete: {}", e))
                })?;
            return Ok(());
        }

        if let Some(db) = &self.db {
            db.delete(key.as_bytes())?;
        }
        Ok(())
    }

    /// Save an SMT snapshot for the given block height (serialized list of KV pairs).
    /// No-op if SMT backend is not configured.
    pub fn save_smt_snapshot(&self, height: u64) -> std::result::Result<(), PersistentStoreError> {
        if let Some(smt_arc) = &self.smt {
            let pairs = smt_arc
                .export_snapshot()
                .map_err(PersistentStoreError::Smt)?;
            let key = format!("smt_snapshot:{}", height);

            // Serialize snapshot and compress with zstd for space savings
            let raw = bcs::to_bytes(&pairs)?;
            let compressed = zstd::bulk::compress(&raw, 0)?;

            // Synchronously store compressed snapshot into RocksDB to ensure availability
            if let Some(db) = &self.db {
                db.put(key.as_bytes(), &compressed)?;

                // Update snapshot index and prune old snapshots according to retention
                let retention: usize = std::env::var("SMT_SNAPSHOT_RETENTION")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10);

                // load index
                let mut idx: Vec<u64> = if let Ok(Some(v)) = db.get(b"smt_snapshots_index") {
                    bcs::from_bytes(&v).unwrap_or_default()
                } else {
                    Vec::new()
                };

                idx.push(height);

                while idx.len() > retention {
                    let old = idx.remove(0);
                    let old_key = format!("smt_snapshot:{}", old);
                    let _ = db.delete(old_key.as_bytes());
                }

                let _ = db.put(b"smt_snapshots_index", bcs::to_bytes(&idx)?);
            } else {
                // As fallback, use existing async save (may enqueue)
                self.save(&key, &pairs)?;
            }
        }

        Ok(())
    }

    /// Load an SMT snapshot saved for a block height.
    pub fn load_smt_snapshot(
        &self,
        height: u64,
    ) -> std::result::Result<Option<SmtSnapshot>, PersistentStoreError> {
        let key = format!("smt_snapshot:{}", height);
        // Read raw compressed bytes directly from RocksDB
        if let Some(db) = &self.db {
            match db.get(key.as_bytes()) {
                Ok(Some(v)) => {
                    // decompress then deserialize
                    let decompressed = zstd::bulk::decompress(&v, 0)?;
                    let pairs: SmtSnapshot = bcs::from_bytes(&decompressed)?;
                    Ok(Some(pairs))
                }
                Ok(None) => Ok(None),
                Err(e) => Err(PersistentStoreError::RocksDB(e)),
            }
        } else {
            Ok(None)
        }
    }

    /// Prune snapshots to keep only the latest `retention` entries.
    pub fn prune_smt_snapshots(
        &self,
        retention: usize,
    ) -> std::result::Result<(), PersistentStoreError> {
        if let Some(db) = &self.db
            && let Ok(Some(v)) = db.get(b"smt_snapshots_index")
        {
            let mut idx: Vec<u64> = bcs::from_bytes(&v).unwrap_or_default();
            while idx.len() > retention {
                let old = idx.remove(0);
                let old_key = format!("smt_snapshot:{}", old);
                let _ = db.delete(old_key.as_bytes());
            }
            let _ = db.put(b"smt_snapshots_index", bcs::to_bytes(&idx)?);
        }
        Ok(())
    }
}

impl Drop for PersistentStore {
    fn drop(&mut self) {
        // Close sender to signal worker to flush and exit
        self.sender.take();
        if let Some(handle) = self.worker.take() {
            // Join worker thread; ignore errors
            let _ = handle.join();
        }
    }
}
