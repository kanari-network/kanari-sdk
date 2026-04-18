// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! RocksDB-based Persistent Storage for DAG Consensus
//!
//! This module provides durable storage for DAG vertices, checkpoints, and state
//! with Write-Ahead Log (WAL) support for crash recovery.

use anyhow::Result;
use rocksdb::{ColumnFamilyDescriptor, DB, Options, WriteBatch};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

use super::{Checkpoint, DagVertex, Round, VertexId};

/// Column family names
const CF_VERTICES: &str = "vertices";
const CF_CHECKPOINTS: &str = "checkpoints";
const CF_ROUNDS: &str = "rounds";
const CF_STATE: &str = "state";

/// Persistent DAG storage backed by RocksDB
#[derive(Clone)]
pub struct PersistentDagStore {
    db: Arc<DB>,
    // FIX #3: Removed write_lock - RocksDB is already thread-safe at C++ level
    // No need for application-level mutex that causes thread starvation
}

impl PersistentDagStore {
    fn round_key(round: Round) -> [u8; 8] {
        round.to_le_bytes()
    }

    fn checkpoint_key(sequence: u64) -> [u8; 8] {
        sequence.to_le_bytes()
    }

    fn serialize<T: Serialize>(value: &T, what: &str) -> Result<Vec<u8>> {
        bcs::to_bytes(value).map_err(|e| anyhow::anyhow!("Failed to serialize {}: {}", what, e))
    }

    fn deserialize<T: for<'de> Deserialize<'de>>(bytes: &[u8], what: &str) -> Result<T> {
        bcs::from_bytes(bytes).map_err(|e| anyhow::anyhow!("Failed to deserialize {}: {}", what, e))
    }

    fn count_cf_entries(&self, cf: &impl rocksdb::AsColumnFamilyRef) -> usize {
        // FIX #3: Use RocksDB property estimate instead of full table scan
        // This avoids O(N) iteration that freezes the node on large databases
        self.db
            .property_int_value_cf(cf, "rocksdb.estimate-num-keys")
            .unwrap_or(Some(0))
            .unwrap_or(0) as usize
    }

    /// Open or create a new persistent DAG store
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // Performance and longevity optimizations for 10-year operation
        opts.set_use_fsync(false); // Async writes for performance (WAL provides crash safety)
        opts.set_wal_recovery_mode(rocksdb::DBRecoveryMode::PointInTime);

        // Enable Zstd compression to reduce disk usage by ~60% long-term
        opts.set_compression_type(rocksdb::DBCompressionType::Zstd);

        // Optimize for SSD with parallelism
        opts.increase_parallelism(num_cpus::get() as i32);
        opts.optimize_level_style_compaction(512 * 1024 * 1024); // 512MB memtable

        // Set reasonable limits for long-running nodes
        opts.set_max_open_files(10000);
        opts.set_max_background_jobs(4);

        let mut cf_opts = Options::default();
        cf_opts.set_compression_type(rocksdb::DBCompressionType::Zstd);

        let cfs = vec![
            ColumnFamilyDescriptor::new(CF_VERTICES, cf_opts.clone()),
            ColumnFamilyDescriptor::new(CF_CHECKPOINTS, cf_opts.clone()),
            ColumnFamilyDescriptor::new(CF_ROUNDS, cf_opts.clone()),
            ColumnFamilyDescriptor::new(CF_STATE, cf_opts),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cfs)
            .map_err(|e| anyhow::anyhow!("Failed to open RocksDB: {}", e))?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Store a DAG vertex using composite key (FIX #2: O(1) write instead of O(N²))
    pub fn put_vertex(&self, vertex: &DagVertex) -> Result<()> {
        let vertices_cf = self
            .db
            .cf_handle(CF_VERTICES)
            .ok_or_else(|| anyhow::anyhow!("Vertices CF not found"))?;
        let rounds_cf = self
            .db
            .cf_handle(CF_ROUNDS)
            .ok_or_else(|| anyhow::anyhow!("Rounds CF not found"))?;

        // Store vertex by ID
        let vertex_key = &vertex.id;
        let vertex_value = Self::serialize(vertex, "vertex")?;

        // Store round->vertex mapping using composite key: [Round (8 bytes) + VertexId (32 bytes)]
        let mut composite_key = Vec::with_capacity(40);
        composite_key.extend_from_slice(&Self::round_key(vertex.round));
        composite_key.extend_from_slice(&vertex.id);

        let mut batch = WriteBatch::default();
        batch.put_cf(&vertices_cf, vertex_key, vertex_value);
        // Use empty value for composite key (we only care about existence)
        batch.put_cf(&rounds_cf, composite_key, vec![]);

        self.db
            .write(batch)
            .map_err(|e| anyhow::anyhow!("Failed to write vertex batch: {}", e))?;

        Ok(())
    }

    /// Retrieve a DAG vertex by ID
    pub fn get_vertex(&self, id: &VertexId) -> Result<Option<DagVertex>> {
        let cf = self
            .db
            .cf_handle(CF_VERTICES)
            .ok_or_else(|| anyhow::anyhow!("Vertices CF not found"))?;
        self.db
            .get_cf(&cf, id)?
            .map(|bytes| Self::deserialize(&bytes, "vertex"))
            .transpose()
    }

    /// Delete a vertex
    pub fn delete_vertex(&self, id: &VertexId) -> Result<()> {
        let vertex = self.get_vertex(id)?;

        let vertices_cf = self
            .db
            .cf_handle(CF_VERTICES)
            .ok_or_else(|| anyhow::anyhow!("Vertices CF not found"))?;
        let rounds_cf = self
            .db
            .cf_handle(CF_ROUNDS)
            .ok_or_else(|| anyhow::anyhow!("Rounds CF not found"))?;

        let mut batch = WriteBatch::default();
        batch.delete_cf(&vertices_cf, id);

        if let Some(v) = vertex {
            // Delete using composite key
            let mut composite_key = Vec::with_capacity(40);
            composite_key.extend_from_slice(&Self::round_key(v.round));
            composite_key.extend_from_slice(id);

            batch.delete_cf(&rounds_cf, composite_key);
        }
        self.db
            .write(batch)
            .map_err(|e| anyhow::anyhow!("Failed to delete vertex batch: {}", e))?;

        Ok(())
    }

    /// Prune a vertex for garbage collection WITHOUT write_lock (fast path)
    /// This is safe because we're only deleting old data that won't conflict with new writes
    pub fn prune_vertex_fast(&self, id: &VertexId) -> Result<()> {
        let vertices_cf = self
            .db
            .cf_handle(CF_VERTICES)
            .ok_or_else(|| anyhow::anyhow!("Vertices CF not found"))?;

        self.db
            .delete_cf(&vertices_cf, id)
            .map_err(|e| anyhow::anyhow!("Failed to prune vertex: {}", e))?;

        Ok(())
    }

    /// Prune an entire round without deserializing (FIX #2: Avoid O(N²) I/O)
    /// This is much more efficient than deleting vertices one-by-one
    pub fn prune_entire_round(&self, round: Round) -> Result<usize> {
        let vertices_cf = self
            .db
            .cf_handle(CF_VERTICES)
            .ok_or_else(|| anyhow::anyhow!("Vertices CF not found"))?;
        let rounds_cf = self
            .db
            .cf_handle(CF_ROUNDS)
            .ok_or_else(|| anyhow::anyhow!("Rounds CF not found"))?;

        // Get all vertex IDs in this round to delete from vertices CF and count them
        let vertex_ids = self.get_vertices_by_round(round)?;
        let count = vertex_ids.len();

        if count == 0 {
            return Ok(0);
        }

        // Batch delete all vertices and the round indices
        let mut batch = WriteBatch::default();

        for vertex_id in &vertex_ids {
            batch.delete_cf(&vertices_cf, vertex_id);
        }

        // Delete all composite keys for this round
        let prefix = Self::round_key(round);
        let iter = self.db.prefix_iterator_cf(&rounds_cf, prefix);
        for item in iter {
            let (key, _) =
                item.map_err(|e| anyhow::anyhow!("Failed to iterate rounds for deletion: {}", e))?;
            if key.len() == 40 && key.starts_with(&prefix) {
                batch.delete_cf(&rounds_cf, key);
            } else {
                break;
            }
        }

        self.db
            .write(batch)
            .map_err(|e| anyhow::anyhow!("Failed to prune round {}: {}", round, e))?;

        Ok(count)
    }

    /// FIX #16: Get minimum existing round in database
    /// Used during node restart to initialize pruning state correctly
    pub fn get_min_existing_round(&self) -> Result<Option<Round>> {
        let rounds_cf = self
            .db
            .cf_handle(CF_ROUNDS)
            .ok_or_else(|| anyhow::anyhow!("Rounds CF not found"))?;

        // Get the first key in the rounds column family (minimum round)
        let mut iter = self
            .db
            .iterator_cf(&rounds_cf, rocksdb::IteratorMode::Start);

        if let Some(Ok((key, _))) = iter.next() {
            // Key format: [8 bytes round][32 bytes vertex_id]
            if key.len() >= 8 {
                let round_bytes: [u8; 8] = key[..8].try_into().unwrap_or([0u8; 8]);
                let min_round = Round::from_le_bytes(round_bytes);
                return Ok(Some(min_round));
            }
        }

        Ok(None) // Database is empty
    }

    /// FIX #16: Get minimum existing checkpoint sequence in database
    /// Used during node restart to initialize pruning state correctly
    pub fn get_min_existing_checkpoint_sequence(&self) -> Result<Option<u64>> {
        let checkpoints_cf = self
            .db
            .cf_handle(CF_CHECKPOINTS)
            .ok_or_else(|| anyhow::anyhow!("Checkpoints CF not found"))?;

        // Get the first key in the checkpoints column family (minimum sequence)
        let mut iter = self
            .db
            .iterator_cf(&checkpoints_cf, rocksdb::IteratorMode::Start);

        if let Some(Ok((key, _))) = iter.next() {
            // Key format: 8 bytes sequence number
            if key.len() >= 8 {
                let seq_bytes: [u8; 8] = key[..8].try_into().unwrap_or([0u8; 8]);
                let min_seq = u64::from_le_bytes(seq_bytes);
                return Ok(Some(min_seq));
            }
        }

        Ok(None) // Database is empty
    }

    /// Store a checkpoint
    pub fn put_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()> {
        let cf = self
            .db
            .cf_handle(CF_CHECKPOINTS)
            .ok_or_else(|| anyhow::anyhow!("Checkpoints CF not found"))?;

        let key = Self::checkpoint_key(checkpoint.sequence);
        let value = Self::serialize(checkpoint, "checkpoint")?;

        self.db
            .put_cf(&cf, key, value)
            .map_err(|e| anyhow::anyhow!("Failed to write checkpoint: {}", e))?;

        Ok(())
    }

    /// Retrieve a checkpoint by sequence number
    pub fn get_checkpoint(&self, sequence: u64) -> Result<Option<Checkpoint>> {
        let cf = self
            .db
            .cf_handle(CF_CHECKPOINTS)
            .ok_or_else(|| anyhow::anyhow!("Checkpoints CF not found"))?;

        let key = Self::checkpoint_key(sequence);
        self.db
            .get_cf(&cf, key)?
            .map(|bytes| Self::deserialize(&bytes, "checkpoint"))
            .transpose()
    }

    /// Delete a checkpoint
    pub fn delete_checkpoint(&self, sequence: u64) -> Result<()> {
        let cf = self
            .db
            .cf_handle(CF_CHECKPOINTS)
            .ok_or_else(|| anyhow::anyhow!("Checkpoints CF not found"))?;

        let key = Self::checkpoint_key(sequence);
        self.db
            .delete_cf(&cf, key)
            .map_err(|e| anyhow::anyhow!("Failed to delete checkpoint: {}", e))?;

        Ok(())
    }

    /// Get all vertices in a specific round
    pub fn get_vertices_by_round(&self, round: Round) -> Result<Vec<VertexId>> {
        let cf = self
            .db
            .cf_handle(CF_ROUNDS)
            .ok_or_else(|| anyhow::anyhow!("Rounds CF not found"))?;

        let prefix = Self::round_key(round);
        let mut vertex_ids = Vec::new();

        // Iterate over keys with the given prefix
        let iter = self.db.prefix_iterator_cf(&cf, prefix);
        for item in iter {
            let (key, _) = item.map_err(|e| anyhow::anyhow!("Failed to iterate rounds: {}", e))?;

            // Ensure the key starts with our prefix and is exactly the right length for a composite key
            if key.len() == 40 && key.starts_with(&prefix) {
                // Extract VertexId (last 32 bytes)
                let mut id = [0u8; 32];
                id.copy_from_slice(&key[8..]);
                vertex_ids.push(id);
            } else {
                // Since we use prefix iterator, keys are sorted.
                // If key doesn't start with prefix anymore (due to how prefix_iterator works internally or extra data), break.
                // However, standard prefix_iterator handles the range correctly.
                // We just need to be careful that we don't pick up keys from next round if prefix matches partially (unlikely with fixed size u64).
                break;
            }
        }

        Ok(vertex_ids)
    }

    /// Prune vertices before a certain round (must be checkpointed)
    /// FIX #3: Removed write_lock to prevent global deadlock during long-running operations
    /// RocksDB operations are already thread-safe at the C++ level
    /// OPTIMIZATION: Accept start_round parameter for O(1) pruning instead of always starting from 0
    pub fn prune_old_vertices(&self, start_round: Round, before_round: Round) -> Result<usize> {
        // FIX #3: Don't hold write_lock for the entire pruning operation
        // Each individual operation has its own safety

        let mut pruned_count = 0;

        for round in start_round..before_round {
            // FIX #2: Use prune_entire_round instead of deleting vertices one-by-one
            // This avoids O(N²) disk I/O by batching all deletes in one WriteBatch
            let pruned_in_round = self.prune_entire_round(round)?;
            pruned_count += pruned_in_round;
        }

        Ok(pruned_count)
    }

    /// Check if a vertex is checkpointed (stub - needs integration with checkpoint logic)
    pub fn is_checkpointed(&self, _vertex_id: &VertexId) -> Result<bool> {
        // Placeholder: In production, track which vertices are included in checkpoints
        Ok(true)
    }

    /// Compact the database to reclaim disk space
    /// Recommended: Call this periodically (e.g., weekly during low-traffic periods)
    /// This forces RocksDB to physically delete old data and return space to the OS
    pub fn compact(&self) -> Result<()> {
        tracing::info!("Starting RocksDB compaction to reclaim disk space...");
        let start = std::time::Instant::now();

        self.db.compact_range::<&[u8], &[u8]>(None, None);

        let elapsed = start.elapsed();
        tracing::info!("RocksDB compaction completed in {:?}", elapsed);

        Ok(())
    }

    /// Compact a specific column family (more granular control)
    pub fn compact_cf(&self, cf_name: &str) -> Result<()> {
        let cf = self
            .db
            .cf_handle(cf_name)
            .ok_or_else(|| anyhow::anyhow!("Column family {} not found", cf_name))?;

        tracing::info!("Starting compaction for column family: {}", cf_name);
        let start = std::time::Instant::now();

        self.db.compact_range_cf(&cf, None::<&[u8]>, None::<&[u8]>);

        let elapsed = start.elapsed();
        tracing::info!("Compaction for {} completed in {:?}", cf_name, elapsed);

        Ok(())
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> Result<StorageStats> {
        let vertices_cf = self
            .db
            .cf_handle(CF_VERTICES)
            .ok_or_else(|| anyhow::anyhow!("Vertices CF not found"))?;
        let checkpoints_cf = self
            .db
            .cf_handle(CF_CHECKPOINTS)
            .ok_or_else(|| anyhow::anyhow!("Checkpoints CF not found"))?;

        Ok(StorageStats {
            vertex_count: self.count_cf_entries(&vertices_cf),
            checkpoint_count: self.count_cf_entries(&checkpoints_cf),
        })
    }
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub vertex_count: usize,
    pub checkpoint_count: usize,
}

#[cfg(test)]
mod tests {
    use super::super::AuthorityId;
    use super::*;
    use tempfile::TempDir;

    fn create_test_vertex(round: Round, author: AuthorityId) -> DagVertex {
        DagVertex::new_for_test(round, author, vec![], vec![], vec![round as u8; 32], 0)
    }

    fn create_test_checkpoint(sequence: u64) -> Checkpoint {
        let mut vertex_id = [0u8; 32];
        vertex_id.fill(sequence as u8);

        Checkpoint {
            sequence,
            vertices: vec![vertex_id],
            transactions: vec![],
            state_root: vec![sequence as u8; 32],
            timestamp: 12345,
            prev_checkpoint_hash: vec![0u8; 32],
        }
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn test_persistent_store_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = PersistentDagStore::new(temp_dir.path())?;

        let stats = store.get_stats()?;
        assert_eq!(stats.vertex_count, 0);
        assert_eq!(stats.checkpoint_count, 0);

        Ok(())
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn test_put_get_vertex() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = PersistentDagStore::new(temp_dir.path())?;

        let vertex = create_test_vertex(1, "auth1".to_string());
        let vertex_id = vertex.id;

        store.put_vertex(&vertex)?;

        let retrieved = store.get_vertex(&vertex_id)?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().round, 1);

        Ok(())
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn test_put_get_checkpoint() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = PersistentDagStore::new(temp_dir.path())?;

        let checkpoint = create_test_checkpoint(42);
        store.put_checkpoint(&checkpoint)?;

        let retrieved = store.get_checkpoint(42)?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().sequence, 42);

        Ok(())
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn test_delete_vertex() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = PersistentDagStore::new(temp_dir.path())?;

        let vertex = create_test_vertex(1, "auth1".to_string());
        let vertex_id = vertex.id;

        store.put_vertex(&vertex)?;
        assert!(store.get_vertex(&vertex_id)?.is_some());

        store.delete_vertex(&vertex_id)?;
        assert!(store.get_vertex(&vertex_id)?.is_none());

        Ok(())
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn test_vertices_by_round() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = PersistentDagStore::new(temp_dir.path())?;

        let v1 = create_test_vertex(5, "auth1".to_string());
        let v2 = create_test_vertex(5, "auth2".to_string());
        let v3 = create_test_vertex(6, "auth1".to_string());

        store.put_vertex(&v1)?;
        store.put_vertex(&v2)?;
        store.put_vertex(&v3)?;

        let round5_vertices = store.get_vertices_by_round(5)?;
        assert_eq!(round5_vertices.len(), 2);

        let round6_vertices = store.get_vertices_by_round(6)?;
        assert_eq!(round6_vertices.len(), 1);

        Ok(())
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn test_prune_old_vertices() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = PersistentDagStore::new(temp_dir.path())?;

        // Add vertices in rounds 0, 1, 2
        for round in 0..3 {
            let vertex = create_test_vertex(round, "auth1".to_string());
            store.put_vertex(&vertex)?;
        }

        let stats_before = store.get_stats()?;
        assert_eq!(stats_before.vertex_count, 3);

        // Prune rounds < 2 (start from 0)
        let pruned = store.prune_old_vertices(0, 2)?;
        assert_eq!(pruned, 2);

        let stats_after = store.get_stats()?;
        assert_eq!(stats_after.vertex_count, 1);

        Ok(())
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn test_persistence_across_reopens() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().to_path_buf();

        // First session: write data
        {
            let store = PersistentDagStore::new(&path)?;
            let vertex = create_test_vertex(10, "auth1".to_string());
            store.put_vertex(&vertex)?;
        }

        // Second session: read data
        {
            let store = PersistentDagStore::new(&path)?;
            let vertices = store.get_vertices_by_round(10)?;
            assert_eq!(vertices.len(), 1);
        }

        Ok(())
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn test_storage_stats() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = PersistentDagStore::new(temp_dir.path())?;

        // Add vertices and checkpoints
        for i in 0..5 {
            let vertex = create_test_vertex(i, "auth1".to_string());
            store.put_vertex(&vertex)?;

            let checkpoint = create_test_checkpoint(i);
            store.put_checkpoint(&checkpoint)?;
        }

        let stats = store.get_stats()?;
        assert_eq!(stats.vertex_count, 5);
        assert_eq!(stats.checkpoint_count, 5);

        Ok(())
    }
}
