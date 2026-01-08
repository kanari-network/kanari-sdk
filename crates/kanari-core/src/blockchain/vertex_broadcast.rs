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
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

/// EWMA (Exponentially Weighted Moving Average) for RTT estimation
#[derive(Debug, Clone)]
pub struct EwmaEstimator {
    value: f64,
    alpha: f64, // Smoothing factor (0-1)
}

impl EwmaEstimator {
    pub fn new(alpha: f64) -> Self {
        Self {
            value: 0.0,
            alpha: alpha.clamp(0.0, 1.0),
        }
    }

    pub fn update(&mut self, sample: f64) {
        if self.value == 0.0 {
            self.value = sample;
        } else {
            self.value = self.alpha * sample + (1.0 - self.alpha) * self.value;
        }
    }

    pub fn get(&self) -> f64 {
        self.value
    }
}

/// Adaptive batching configuration
#[derive(Debug, Clone)]
pub struct AdaptiveBatchConfig {
    pub min_batch_size: usize,
    pub max_batch_size: usize,
    pub target_latency_ms: u64,
    pub adjustment_factor: f64, // How aggressively to adjust (0.1 = 10% per adjustment)
}

impl Default for AdaptiveBatchConfig {
    fn default() -> Self {
        Self {
            min_batch_size: 10,
            max_batch_size: 1000,
            target_latency_ms: 100, // 100ms target
            adjustment_factor: 0.1,
        }
    }
}

/// Adaptive batching manager
#[derive(Debug, Clone)]
pub struct AdaptiveBatcher {
    config: AdaptiveBatchConfig,
    current_batch_size: usize,
    network_rtt: EwmaEstimator,
    last_adjustment: Instant,
    adjustment_interval: Duration,
}

impl AdaptiveBatcher {
    pub fn new(config: AdaptiveBatchConfig) -> Self {
        let initial_batch_size = (config.min_batch_size + config.max_batch_size) / 2;
        Self {
            current_batch_size: initial_batch_size,
            config,
            network_rtt: EwmaEstimator::new(0.2), // 20% weight to new samples
            last_adjustment: Instant::now(),
            adjustment_interval: Duration::from_secs(5),
        }
    }

    /// Update with observed latency and adjust batch size
    pub fn observe_latency(&mut self, latency: Duration) {
        let latency_ms = latency.as_millis() as f64;
        self.network_rtt.update(latency_ms);

        // Only adjust periodically to avoid oscillation
        if self.last_adjustment.elapsed() < self.adjustment_interval {
            return;
        }

        self.adjust_batch_size();
        self.last_adjustment = Instant::now();
    }

    /// Adjust batch size based on network conditions
    fn adjust_batch_size(&mut self) {
        let current_latency = self.network_rtt.get();
        let target = self.config.target_latency_ms as f64;

        if current_latency > target * 1.2 {
            // Network is slow: increase batch size for better throughput
            let increase =
                (self.current_batch_size as f64 * self.config.adjustment_factor) as usize;
            self.current_batch_size =
                (self.current_batch_size + increase).min(self.config.max_batch_size);
        } else if current_latency < target * 0.8 {
            // Network is fast: decrease batch size for lower latency
            let decrease =
                (self.current_batch_size as f64 * self.config.adjustment_factor) as usize;
            self.current_batch_size =
                (self.current_batch_size.saturating_sub(decrease)).max(self.config.min_batch_size);
        }
        // Otherwise: latency is within acceptable range, keep current size
    }

    /// Get current recommended batch size
    pub fn get_batch_size(&self) -> usize {
        self.current_batch_size
    }

    /// Get current estimated network RTT
    pub fn get_rtt(&self) -> Duration {
        Duration::from_millis(self.network_rtt.get() as u64)
    }

    /// Reset to default batch size (useful after network changes)
    pub fn reset(&mut self) {
        self.current_batch_size = (self.config.min_batch_size + self.config.max_batch_size) / 2;
        self.network_rtt = EwmaEstimator::new(0.2);
        self.last_adjustment = Instant::now();
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

    /// Adaptive batching
    adaptive_batcher: AdaptiveBatcher,
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
            adaptive_batcher: AdaptiveBatcher::new(AdaptiveBatchConfig::default()),
        }
    }

    /// Create broadcaster with custom adaptive config
    pub fn with_adaptive_config(
        max_batch_size: usize,
        max_batch_delay: Duration,
        adaptive_config: AdaptiveBatchConfig,
    ) -> Self {
        Self {
            pending: VecDeque::new(),
            broadcasted_filter: VertexBloomFilter::new(10000, 0.01),
            seen_vertices: HashSet::new(),
            max_batch_size,
            max_batch_delay,
            compression_enabled: true,
            priority_queue: VecDeque::new(),
            adaptive_batcher: AdaptiveBatcher::new(adaptive_config),
        }
    }

    /// Observe network latency for adaptive batching
    pub fn observe_latency(&mut self, latency: Duration) {
        self.adaptive_batcher.observe_latency(latency);
    }

    /// Get current adaptive batch size
    pub fn get_adaptive_batch_size(&self) -> usize {
        self.adaptive_batcher.get_batch_size()
    }

    /// Get estimated network RTT
    pub fn get_estimated_rtt(&self) -> Duration {
        self.adaptive_batcher.get_rtt()
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
        // Use adaptive batch size instead of fixed max_batch_size
        let adaptive_size = self.adaptive_batcher.get_batch_size();
        let batch_limit = adaptive_size.min(self.max_batch_size);

        let vertices: Vec<DagVertex> = self
            .priority_queue
            .drain(..)
            .chain(self.pending.drain(..).take(batch_limit))
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

    /// Compress batch using zstd
    pub fn compress_batch(&self, batch: &VertexBatch) -> Result<CompressedBatch> {
        // Serialize batch using BCS (Binary Canonical Serialization)
        let serialized = bcs::to_bytes(batch)
            .map_err(|e| anyhow::anyhow!("Failed to serialize batch: {}", e))?;
        let original_size = serialized.len();

        // Use compression_enabled flag to decide whether to compress
        let compressed = if self.compression_enabled {
            // Use zstd with compression level 3 (balanced speed/ratio)
            zstd::encode_all(&serialized[..], 3).unwrap_or_else(|_| serialized.clone())
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
        let decompressed = if self.compression_enabled {
            zstd::decode_all(&compressed.data[..])
                .map_err(|e| anyhow::anyhow!("Failed to decompress: {}", e))?
        } else {
            compressed.data.clone()
        };

        // Deserialize using BCS
        let batch: VertexBatch = bcs::from_bytes(&decompressed)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize batch: {}", e))?;

        Ok(batch)
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

    #[test]
    fn test_zstd_compression() {
        use crate::blockchain::VertexMetadata;

        let broadcaster = VertexBroadcaster::new(10, Duration::from_secs(1));

        let batch = VertexBatch {
            vertices: vec![DagVertex {
                id: vec![1, 2, 3],
                round: 1,
                author: "test_author".to_string(),
                parents: vec![],
                transactions: vec![],
                timestamp: 12345,
                signature: vec![0; 64],
                metadata: VertexMetadata {
                    tx_count: 0,
                    total_gas_used: 0,
                    state_root: vec![0; 32],
                    is_checkpoint: false,
                    checkpoint_seq: None,
                },
            }],
            round_range: (1, 1),
            size_bytes: 200,
            created_at: 1000,
        };

        let compressed = broadcaster.compress_batch(&batch).unwrap();
        assert!(compressed.data.len() > 0);
        assert!(compressed.compression_ratio < 1.0); // Should be compressed

        let decompressed = broadcaster.decompress_batch(&compressed).unwrap();
        assert_eq!(decompressed.vertices.len(), 1);
        assert_eq!(decompressed.vertices[0].id, vec![1, 2, 3]);
    }

    #[test]
    fn test_adaptive_batching() {
        let config = AdaptiveBatchConfig {
            min_batch_size: 10,
            max_batch_size: 1000,
            target_latency_ms: 100,
            adjustment_factor: 0.2,
        };

        let mut batcher = AdaptiveBatcher::new(config);
        let initial_size = batcher.get_batch_size();

        // Simulate high latency (200ms) - should increase batch size
        batcher.observe_latency(Duration::from_millis(200));
        std::thread::sleep(Duration::from_millis(5100)); // Wait past adjustment interval
        batcher.observe_latency(Duration::from_millis(200));

        assert!(batcher.get_batch_size() > initial_size);
    }

    #[test]
    fn test_adaptive_batching_fast_network() {
        let config = AdaptiveBatchConfig {
            min_batch_size: 10,
            max_batch_size: 1000,
            target_latency_ms: 100,
            adjustment_factor: 0.2,
        };

        let mut batcher = AdaptiveBatcher::new(config);
        let initial_size = batcher.get_batch_size();

        // Simulate low latency (30ms) - should decrease batch size
        batcher.observe_latency(Duration::from_millis(30));
        std::thread::sleep(Duration::from_millis(5100));
        batcher.observe_latency(Duration::from_millis(30));

        assert!(batcher.get_batch_size() < initial_size);
    }

    #[test]
    fn test_adaptive_batching_bounds() {
        let config = AdaptiveBatchConfig {
            min_batch_size: 10,
            max_batch_size: 100,
            target_latency_ms: 50,
            adjustment_factor: 0.5, // Aggressive adjustment
        };

        let mut batcher = AdaptiveBatcher::new(config);

        // Extreme high latency - should cap at max
        for _ in 0..10 {
            batcher.observe_latency(Duration::from_millis(1000));
            std::thread::sleep(Duration::from_millis(5100));
        }

        assert_eq!(batcher.get_batch_size(), 100);

        // Reset and test minimum
        batcher.reset();

        // Extreme low latency - should cap at min
        for _ in 0..10 {
            batcher.observe_latency(Duration::from_millis(1));
            std::thread::sleep(Duration::from_millis(5100));
        }

        assert_eq!(batcher.get_batch_size(), 10);
    }

    #[test]
    fn test_ewma_estimator() {
        let mut ewma = EwmaEstimator::new(0.3);

        ewma.update(100.0);
        assert_eq!(ewma.get(), 100.0);

        ewma.update(200.0);
        // Should be weighted average: 0.3 * 200 + 0.7 * 100 = 130
        assert!((ewma.get() - 130.0).abs() < 1.0);
    }

    #[test]
    fn test_broadcaster_with_adaptive() {
        let config = AdaptiveBatchConfig {
            min_batch_size: 5,
            max_batch_size: 50,
            target_latency_ms: 100,
            adjustment_factor: 0.1,
        };

        let mut broadcaster =
            VertexBroadcaster::with_adaptive_config(100, Duration::from_secs(1), config);

        // Add some vertices
        for i in 0..20 {
            let vertex = DagVertex::new(i, format!("auth{}", i), vec![], vec![], vec![]);
            broadcaster.add_vertex(vertex, false);
        }

        // Observe latency
        broadcaster.observe_latency(Duration::from_millis(50));

        // Get adaptive batch size
        let batch_size = broadcaster.get_adaptive_batch_size();
        assert!(batch_size >= 5 && batch_size <= 50);

        // Create batch - should respect adaptive size
        let batch = broadcaster.create_batch().unwrap();
        assert!(batch.vertices.len() <= batch_size);
    }
}
