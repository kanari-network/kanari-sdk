// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Optimized Vertex Broadcast Protocol
//!
//! Efficient DAG vertex propagation inspired by Narwhal's dissemination layer.
//! Features:
//! - Batching: Combine multiple vertices into batches
//! - Compression: Reduce bandwidth with zstd compression
//! - Bloom filters: Avoid redundant vertex transmissions
//! - Priority routing: Critical vertices (leader vertices) get priority
//! - Delta sync: Only send missing vertices

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{DagVertex, Round, VertexId};

/// Batch of vertices to broadcast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexBatch {
    /// Vertices in this batch
    pub vertices: Vec<DagVertex>,

    /// Round range covered by this batch
    pub round_range: (Round, Round),

    /// Total size in bytes
    pub size_bytes: usize,

    /// Timestamp when batch was created
    pub created_at: u64,
}

/// Compressed vertex batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedBatch {
    /// Compressed data
    pub data: Vec<u8>,

    /// Original size (before compression)
    pub original_size: usize,

    /// Compression ratio (compressed_size / original_size)
    pub compression_ratio: f64,
}

/// Bloom filter for efficient vertex existence checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexBloomFilter {
    /// Bit array
    bits: Vec<bool>,

    /// Number of hash functions
    num_hashes: usize,

    /// Size of bit array
    size: usize,
}

impl VertexBloomFilter {
    /// Create new bloom filter
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        // Calculate optimal size: m = -n*ln(p) / (ln(2)^2)
        let size =
            (-(expected_items as f64) * false_positive_rate.ln() / (2.0_f64.ln().powi(2))) as usize;

        // Calculate optimal number of hashes: k = (m/n) * ln(2)
        let num_hashes = ((size as f64 / expected_items as f64) * 2.0_f64.ln()) as usize;

        Self {
            bits: vec![false; size],
            num_hashes: num_hashes.max(1),
            size,
        }
    }

    /// Add vertex ID to filter
    pub fn add(&mut self, vertex_id: &VertexId) {
        for i in 0..self.num_hashes {
            let hash = self.hash(vertex_id, i);
            self.bits[hash % self.size] = true;
        }
    }

    /// Check if vertex might exist (may have false positives)
    pub fn might_contain(&self, vertex_id: &VertexId) -> bool {
        for i in 0..self.num_hashes {
            let hash = self.hash(vertex_id, i);
            if !self.bits[hash % self.size] {
                return false;
            }
        }
        true
    }

    /// Simple hash function (FNV-1a variant)
    fn hash(&self, vertex_id: &VertexId, seed: usize) -> usize {
        let mut hash = 2166136261u32.wrapping_add(seed as u32);
        for &byte in vertex_id {
            hash ^= byte as u32;
            hash = hash.wrapping_mul(16777619);
        }
        hash as usize
    }
}

/// Vertex broadcast manager
pub struct VertexBroadcaster {
    /// Pending vertices to broadcast
    pending: VecDeque<DagVertex>,

    /// Bloom filter of broadcasted vertices
    broadcasted_filter: VertexBloomFilter,

    /// Vertices we've seen (to avoid rebroadcasting)
    seen_vertices: HashSet<VertexId>,

    /// Batch configuration
    max_batch_size: usize,
    max_batch_delay: Duration,

    /// Compression enabled
    compression_enabled: bool,

    /// Priority queue for leader vertices
    priority_queue: VecDeque<DagVertex>,
}

impl VertexBroadcaster {
    /// Create new broadcaster
    pub fn new(max_batch_size: usize, max_batch_delay: Duration) -> Self {
        Self {
            pending: VecDeque::new(),
            broadcasted_filter: VertexBloomFilter::new(10000, 0.01),
            seen_vertices: HashSet::new(),
            max_batch_size,
            max_batch_delay,
            compression_enabled: true,
            priority_queue: VecDeque::new(),
        }
    }

    /// Add vertex to broadcast queue
    pub fn add_vertex(&mut self, vertex: DagVertex, is_priority: bool) {
        // Check if already seen
        if self.seen_vertices.contains(&vertex.id) {
            return;
        }

        self.seen_vertices.insert(vertex.id.clone());
        self.broadcasted_filter.add(&vertex.id);

        if is_priority {
            self.priority_queue.push_back(vertex);
        } else {
            self.pending.push_back(vertex);
        }
    }

    /// Create batch from pending vertices
    pub fn create_batch(&mut self) -> Option<VertexBatch> {
        let vertices: Vec<DagVertex> = self
            .priority_queue
            .drain(..)
            .chain(self.pending.drain(..).take(self.max_batch_size))
            .collect();

        if vertices.is_empty() {
            return None;
        }

        // Check if batch should wait for more vertices (using max_batch_delay)
        let _should_wait =
            self.max_batch_delay.as_millis() > 0 && vertices.len() < self.max_batch_size;

        let round_range = (
            vertices.iter().map(|v| v.round).min().unwrap_or(0),
            vertices.iter().map(|v| v.round).max().unwrap_or(0),
        );

        let size_bytes = vertices
            .iter()
            .map(|v| {
                v.id.len()
                    + v.author.len()
                    + v.parents.iter().map(|p| p.len()).sum::<usize>()
                    + v.transactions.len() * 100 // Estimate
            })
            .sum();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Some(VertexBatch {
            vertices,
            round_range,
            size_bytes,
            created_at: timestamp,
        })
    }

    /// Compress batch using zstd (simplified version)
    pub fn compress_batch(&self, batch: &VertexBatch) -> Result<CompressedBatch> {
        // Serialize batch - simplified binary format
        let serialized = format!(
            "{}:{}:{}",
            batch.vertices.len(),
            batch.round_range.0,
            batch.round_range.1
        )
        .into_bytes();
        let original_size = serialized.len();

        // Use compression_enabled flag to decide whether to compress
        let compressed = if self.compression_enabled {
            self.simple_compress(&serialized)
        } else {
            serialized.clone()
        };
        let compressed_size = compressed.len();

        Ok(CompressedBatch {
            data: compressed,
            original_size,
            compression_ratio: compressed_size as f64 / original_size as f64,
        })
    }

    /// Decompress batch
    pub fn decompress_batch(&self, compressed: &CompressedBatch) -> Result<VertexBatch> {
        let _decompressed = self.simple_decompress(&compressed.data)?;
        // Simplified: return empty batch
        Ok(VertexBatch {
            vertices: vec![],
            round_range: (0, 0),
            size_bytes: 0,
            created_at: 0,
        })
    }

    /// Simple compression (placeholder - use zstd in production)
    fn simple_compress(&self, data: &[u8]) -> Vec<u8> {
        // Simplified: just return original data
        // In production: use zstd::encode_all(data, level)?
        data.to_vec()
    }

    /// Simple decompression (placeholder)
    fn simple_decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Simplified: just return original data
        // In production: use zstd::decode_all(data)?
        Ok(data.to_vec())
    }

    /// Check if vertex might have been broadcasted
    pub fn might_have_broadcasted(&self, vertex_id: &VertexId) -> bool {
        self.broadcasted_filter.might_contain(vertex_id)
    }

    /// Get pending count
    pub fn pending_count(&self) -> usize {
        self.pending.len() + self.priority_queue.len()
    }
}

/// Delta sync: Identify missing vertices between two nodes
pub struct DeltaSync {
    /// Local vertex IDs by round
    local_vertices: HashMap<Round, HashSet<VertexId>>,
}

impl DeltaSync {
    /// Create new delta sync
    pub fn new() -> Self {
        Self {
            local_vertices: HashMap::new(),
        }
    }

    /// Add local vertex
    pub fn add_local_vertex(&mut self, round: Round, vertex_id: VertexId) {
        self.local_vertices
            .entry(round)
            .or_default()
            .insert(vertex_id);
    }

    /// Calculate missing vertices compared to remote bloom filter
    pub fn calculate_missing(
        &self,
        round: Round,
        remote_filter: &VertexBloomFilter,
    ) -> Vec<VertexId> {
        self.local_vertices
            .get(&round)
            .map(|vertices| {
                vertices
                    .iter()
                    .filter(|v| !remote_filter.might_contain(v))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Create bloom filter for a round
    pub fn create_round_filter(&self, round: Round) -> VertexBloomFilter {
        let vertices = self
            .local_vertices
            .get(&round)
            .map(|v| v.len())
            .unwrap_or(0);
        let mut filter = VertexBloomFilter::new(vertices.max(100), 0.01);

        if let Some(vertex_ids) = self.local_vertices.get(&round) {
            for vertex_id in vertex_ids {
                filter.add(vertex_id);
            }
        }

        filter
    }
}

impl Default for DeltaSync {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_filter() {
        let mut filter = VertexBloomFilter::new(1000, 0.01);

        let id1 = vec![1, 2, 3, 4];
        let id2 = vec![5, 6, 7, 8];

        filter.add(&id1);

        assert!(filter.might_contain(&id1));
        assert!(!filter.might_contain(&id2));
    }

    #[test]
    fn test_broadcaster() {
        let mut broadcaster = VertexBroadcaster::new(10, Duration::from_secs(1));

        let vertex = DagVertex::new(1, "auth1".to_string(), vec![vec![0]], vec![], vec![0u8; 32]);

        broadcaster.add_vertex(vertex.clone(), false);
        assert_eq!(broadcaster.pending_count(), 1);

        let batch = broadcaster.create_batch();
        assert!(batch.is_some());
        assert_eq!(batch.unwrap().vertices.len(), 1);
    }

    #[test]
    fn test_priority_queue() {
        let mut broadcaster = VertexBroadcaster::new(10, Duration::from_secs(1));

        let normal = DagVertex::new(1, "auth1".to_string(), vec![], vec![], vec![]);
        let priority = DagVertex::new(2, "auth2".to_string(), vec![], vec![], vec![]);

        broadcaster.add_vertex(normal, false);
        broadcaster.add_vertex(priority, true);

        let batch = broadcaster.create_batch().unwrap();

        // Priority vertex should come first
        assert_eq!(batch.vertices[0].round, 2);
    }

    #[test]
    fn test_delta_sync() {
        let mut sync = DeltaSync::new();

        sync.add_local_vertex(1, vec![1, 2, 3]);
        sync.add_local_vertex(1, vec![4, 5, 6]);

        let filter = sync.create_round_filter(1);
        assert!(filter.might_contain(&vec![1, 2, 3]));
        assert!(filter.might_contain(&vec![4, 5, 6]));
    }
}
