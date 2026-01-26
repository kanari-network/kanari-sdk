// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! RocksDB-based Persistent Storage for DAG Consensus
//!
//! This module provides durable storage for DAG vertices, checkpoints, and state
//! with Write-Ahead Log (WAL) support for crash recovery.

use anyhow::Result;
use rocksdb::{ColumnFamilyDescriptor, DB, Options};
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
}

impl PersistentDagStore {
    /// Open or create a new persistent DAG store
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // Enable WAL for crash recovery
        opts.set_use_fsync(false);
        opts.set_wal_recovery_mode(rocksdb::DBRecoveryMode::PointInTime);

        // Column family descriptors
        let cfs = vec![
            ColumnFamilyDescriptor::new(CF_VERTICES, Options::default()),
            ColumnFamilyDescriptor::new(CF_CHECKPOINTS, Options::default()),
            ColumnFamilyDescriptor::new(CF_ROUNDS, Options::default()),
            ColumnFamilyDescriptor::new(CF_STATE, Options::default()),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cfs)
            .map_err(|e| anyhow::anyhow!("Failed to open RocksDB: {}", e))?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Store a DAG vertex
    pub fn put_vertex(&self, vertex: &DagVertex) -> Result<()> {
        let cf = self
            .db
            .cf_handle(CF_VERTICES)
            .ok_or_else(|| anyhow::anyhow!("Vertices CF not found"))?;

        let key = &vertex.id;
        let value = bcs::to_bytes(vertex)
            .map_err(|e| anyhow::anyhow!("Failed to serialize vertex: {}", e))?;

        self.db
            .put_cf(&cf, key, value)
            .map_err(|e| anyhow::anyhow!("Failed to write vertex: {}", e))?;

        // Index by round
        self.index_vertex_by_round(&vertex.id, vertex.round)?;

        Ok(())
    }

    /// Retrieve a DAG vertex by ID
    pub fn get_vertex(&self, id: &VertexId) -> Result<Option<DagVertex>> {
        let cf = self
            .db
            .cf_handle(CF_VERTICES)
            .ok_or_else(|| anyhow::anyhow!("Vertices CF not found"))?;

        match self.db.get_cf(&cf, id)? {
            Some(bytes) => {
                let vertex = bcs::from_bytes(&bytes)
                    .map_err(|e| anyhow::anyhow!("Failed to deserialize vertex: {}", e))?;
                Ok(Some(vertex))
            }
            None => Ok(None),
        }
    }

    /// Delete a vertex
    pub fn delete_vertex(&self, id: &VertexId) -> Result<()> {
        // First get the vertex to know its round (for index cleanup)
        let vertex = self.get_vertex(id)?;

        let cf = self
            .db
            .cf_handle(CF_VERTICES)
            .ok_or_else(|| anyhow::anyhow!("Vertices CF not found"))?;

        self.db
            .delete_cf(&cf, id)
            .map_err(|e| anyhow::anyhow!("Failed to delete vertex: {}", e))?;

        // Also remove from round index if vertex was found
        if let Some(v) = vertex {
            self.remove_vertex_from_round_index(id, v.round)?;
        }

        Ok(())
    }

    /// Store a checkpoint
    pub fn put_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()> {
        let cf = self
            .db
            .cf_handle(CF_CHECKPOINTS)
            .ok_or_else(|| anyhow::anyhow!("Checkpoints CF not found"))?;

        let key = checkpoint.sequence.to_le_bytes();
        let value = bcs::to_bytes(checkpoint)
            .map_err(|e| anyhow::anyhow!("Failed to serialize checkpoint: {}", e))?;

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

        let key = sequence.to_le_bytes();
        match self.db.get_cf(&cf, key)? {
            Some(bytes) => {
                let checkpoint = bcs::from_bytes(&bytes)
                    .map_err(|e| anyhow::anyhow!("Failed to deserialize checkpoint: {}", e))?;
                Ok(Some(checkpoint))
            }
            None => Ok(None),
        }
    }

    /// Delete a checkpoint
    pub fn delete_checkpoint(&self, sequence: u64) -> Result<()> {
        let cf = self
            .db
            .cf_handle(CF_CHECKPOINTS)
            .ok_or_else(|| anyhow::anyhow!("Checkpoints CF not found"))?;

        let key = sequence.to_le_bytes();
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

        let key = round.to_le_bytes();
        match self.db.get_cf(&cf, key)? {
            Some(bytes) => {
                let vertex_ids: Vec<VertexId> = bcs::from_bytes(&bytes)
                    .map_err(|e| anyhow::anyhow!("Failed to deserialize vertex IDs: {}", e))?;
                Ok(vertex_ids)
            }
            None => Ok(Vec::new()),
        }
    }

    /// Index a vertex by its round (internal helper)
    fn index_vertex_by_round(&self, vertex_id: &VertexId, round: Round) -> Result<()> {
        let cf = self
            .db
            .cf_handle(CF_ROUNDS)
            .ok_or_else(|| anyhow::anyhow!("Rounds CF not found"))?;

        let key = round.to_le_bytes();
        let mut vertex_ids = self.get_vertices_by_round(round)?;
        vertex_ids.push(*vertex_id);

        let value = bcs::to_bytes(&vertex_ids)
            .map_err(|e| anyhow::anyhow!("Failed to serialize vertex IDs: {}", e))?;

        self.db
            .put_cf(&cf, key, value)
            .map_err(|e| anyhow::anyhow!("Failed to index vertex by round: {}", e))?;

        Ok(())
    }

    /// Remove a vertex from the round index (internal helper)
    fn remove_vertex_from_round_index(&self, vertex_id: &VertexId, round: Round) -> Result<()> {
        let cf = self
            .db
            .cf_handle(CF_ROUNDS)
            .ok_or_else(|| anyhow::anyhow!("Rounds CF not found"))?;

        let key = round.to_le_bytes();
        let mut vertex_ids = self.get_vertices_by_round(round)?;
        vertex_ids.retain(|id| id != vertex_id);

        if vertex_ids.is_empty() {
            // Remove the round entry entirely if no vertices left
            self.db
                .delete_cf(&cf, key)
                .map_err(|e| anyhow::anyhow!("Failed to delete round index: {}", e))?;
        } else {
            let value = bcs::to_bytes(&vertex_ids)
                .map_err(|e| anyhow::anyhow!("Failed to serialize vertex IDs: {}", e))?;

            self.db
                .put_cf(&cf, key, value)
                .map_err(|e| anyhow::anyhow!("Failed to update round index: {}", e))?;
        }

        Ok(())
    }

    /// Prune vertices before a certain round (must be checkpointed)
    pub fn prune_old_vertices(&self, before_round: Round) -> Result<usize> {
        let mut pruned_count = 0;

        for round in 0..before_round {
            let vertex_ids = self.get_vertices_by_round(round)?;

            for vertex_id in vertex_ids {
                self.delete_vertex(&vertex_id)?;
                pruned_count += 1;
            }

            // Clear round index
            let cf = self
                .db
                .cf_handle(CF_ROUNDS)
                .ok_or_else(|| anyhow::anyhow!("Rounds CF not found"))?;
            self.db.delete_cf(&cf, round.to_le_bytes())?;
        }

        Ok(pruned_count)
    }

    /// Check if a vertex is checkpointed (stub - needs integration with checkpoint logic)
    pub fn is_checkpointed(&self, _vertex_id: &VertexId) -> Result<bool> {
        // Placeholder: In production, track which vertices are included in checkpoints
        Ok(true)
    }

    /// Compact the database to reclaim space
    pub fn compact(&self) -> Result<()> {
        self.db.compact_range::<&[u8], &[u8]>(None, None);
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

        // Count entries (rough estimate)
        let mut vertex_count = 0;
        let mut checkpoint_count = 0;

        let iter = self
            .db
            .iterator_cf(&vertices_cf, rocksdb::IteratorMode::Start);
        for _ in iter {
            vertex_count += 1;
        }

        let iter = self
            .db
            .iterator_cf(&checkpoints_cf, rocksdb::IteratorMode::Start);
        for _ in iter {
            checkpoint_count += 1;
        }

        Ok(StorageStats {
            vertex_count,
            checkpoint_count,
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
        DagVertex::new(round, author, vec![], vec![], vec![round as u8; 32])
    }

    fn create_test_checkpoint(sequence: u64) -> Checkpoint {
        let mut vertex_id = [0u8; 32];
        for i in 0..32 {
            vertex_id[i] = sequence as u8;
        }

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
        let vertex_id = vertex.id.clone();

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
        let vertex_id = vertex.id.clone();

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

        // Prune rounds < 2
        let pruned = store.prune_old_vertices(2)?;
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
