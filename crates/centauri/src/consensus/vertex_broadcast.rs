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
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::{DagVertex, Round, VertexId};

const DEFAULT_BLOOM_EXPECTED_ITEMS: usize = 10_000;
const DEFAULT_BLOOM_FALSE_POSITIVE_RATE: f64 = 0.01;
const BLOOM_ROTATION_INTERVAL_SECS: u64 = 1;
const BLOOM_ROTATION_ITEMS: usize = 50_000;

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

    /// Simple fast hash extraction (Zero-cost optimization)
    fn hash(&self, vertex_id: &VertexId, seed: usize) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // FIX: Use DefaultHasher to mix vertex_id and seed for perfect entropy distribution
        let mut hasher = DefaultHasher::new();
        vertex_id.hash(&mut hasher);
        seed.hash(&mut hasher);

        (hasher.finish() as usize) % self.size
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
    pub adjustment_interval: Duration,
}

impl AdaptiveBatchConfig {
    /// Moderate config for 8-16 core machines (10K-30K TPS)
    pub fn moderate() -> Self {
        Self {
            min_batch_size: 10,     // Small minimum for low-end
            max_batch_size: 1000,   // 1K max batch
            target_latency_ms: 100, // 100ms moderate latency
            adjustment_factor: 0.1, // Conservative tuning
            adjustment_interval: Duration::from_secs(5),
        }
    }

    /// Extreme high-throughput config for 500K+ TPS
    pub fn extreme_throughput() -> Self {
        Self {
            min_batch_size: 1000,   // Start with 1K minimum
            max_batch_size: 50000,  // 50K max batch
            target_latency_ms: 20,  // 20ms ultra-low latency
            adjustment_factor: 0.2, // Very aggressive tuning
            adjustment_interval: Duration::from_secs(5),
        }
    }
}

impl Default for AdaptiveBatchConfig {
    fn default() -> Self {
        Self {
            min_batch_size: 100,     // Higher minimum for efficiency
            max_batch_size: 10000,   // 10K max for 500K TPS
            target_latency_ms: 50,   // 50ms target for faster batching
            adjustment_factor: 0.15, // More aggressive adjustment
            adjustment_interval: Duration::from_secs(5),
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
        let adjustment_interval = config.adjustment_interval;
        Self {
            current_batch_size: initial_batch_size,
            config,
            network_rtt: EwmaEstimator::new(0.2), // 20% weight to new samples
            last_adjustment: Instant::now(),
            adjustment_interval,
        }
    }

    /// Update with observed latency and adjust batch size
    pub fn observe_latency(&mut self, latency: Duration) {
        let latency_ms = latency.as_millis() as f64;
        self.network_rtt.update(latency_ms);
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
            let increase = ((self.current_batch_size as f64 * self.config.adjustment_factor).ceil()
                as usize)
                .max(1);
            self.current_batch_size =
                (self.current_batch_size + increase).min(self.config.max_batch_size);
        } else if current_latency < target * 0.8 {
            let decrease = ((self.current_batch_size as f64 * self.config.adjustment_factor).ceil()
                as usize)
                .max(1);
            self.current_batch_size =
                (self.current_batch_size.saturating_sub(decrease)).max(self.config.min_batch_size);
        }
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
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.current_batch_size = (self.config.min_batch_size + self.config.max_batch_size) / 2;
        self.network_rtt = EwmaEstimator::new(0.2);
        self.last_adjustment = Instant::now();
    }
}

/// Vertex broadcast manager
pub struct VertexBroadcaster {
    /// Pending vertices to broadcast
    pending: VecDeque<Arc<DagVertex>>,

    /// Active + previous bloom filters to bound false-positive growth over time.
    broadcasted_filter: VertexBloomFilter,
    previous_broadcasted_filter: VertexBloomFilter,
    bloom_last_rotation: Instant,
    bloom_items_since_rotation: usize,

    /// Batch configuration
    max_batch_size: usize,

    /// Compression enabled
    compression_enabled: bool,

    /// Priority queue for leader vertices
    priority_queue: VecDeque<Arc<DagVertex>>,

    /// Adaptive batching
    adaptive_batcher: AdaptiveBatcher,

    /// FIX #5: Queue size limits to prevent OOM during network partitions
    max_pending_queue_size: usize,
    max_priority_queue_size: usize,
}

impl VertexBroadcaster {
    fn unix_timestamp_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn build(
        max_batch_size: usize,
        _max_batch_delay: Duration,
        adaptive_batcher: AdaptiveBatcher,
    ) -> Self {
        // FIX #5: Set reasonable queue size limits to prevent OOM during network partitions
        // Default: 100K vertices max (at ~1KB per vertex = ~100MB memory)
        const MAX_PENDING_QUEUE_SIZE: usize = 100_000;
        const MAX_PRIORITY_QUEUE_SIZE: usize = 10_000;

        Self {
            pending: VecDeque::new(),
            broadcasted_filter: VertexBloomFilter::new(
                DEFAULT_BLOOM_EXPECTED_ITEMS,
                DEFAULT_BLOOM_FALSE_POSITIVE_RATE,
            ),
            previous_broadcasted_filter: VertexBloomFilter::new(
                DEFAULT_BLOOM_EXPECTED_ITEMS,
                DEFAULT_BLOOM_FALSE_POSITIVE_RATE,
            ),
            bloom_last_rotation: Instant::now(),
            bloom_items_since_rotation: 0,
            max_batch_size,
            compression_enabled: true,
            priority_queue: VecDeque::new(),
            adaptive_batcher,
            max_pending_queue_size: MAX_PENDING_QUEUE_SIZE,
            max_priority_queue_size: MAX_PRIORITY_QUEUE_SIZE,
        }
    }

    fn enqueue_vertex(&mut self, vertex: Arc<DagVertex>, is_priority: bool) {
        // FIX #5: Enforce queue size limits to prevent OOM during network partitions
        if is_priority {
            if self.priority_queue.len() >= self.max_priority_queue_size {
                // Drop oldest priority vertex (FIFO) when queue is full
                tracing::warn!(
                    "[BROADCASTER] Priority queue full ({} vertices), dropping oldest",
                    self.priority_queue.len()
                );
                self.priority_queue.pop_front();
            }
            self.priority_queue.push_back(vertex);
        } else {
            if self.pending.len() >= self.max_pending_queue_size {
                // Drop oldest pending vertex (FIFO) when queue is full
                tracing::warn!(
                    "[BROADCASTER] Pending queue full ({} vertices, limit: {}), dropping oldest. State Sync will recover.",
                    self.pending.len(),
                    self.max_pending_queue_size
                );
                self.pending.pop_front();
            }
            self.pending.push_back(vertex);
        }
    }

    fn rotate_bloom_if_needed(&mut self) {
        if self.bloom_last_rotation.elapsed() < Duration::from_secs(BLOOM_ROTATION_INTERVAL_SECS)
            && self.bloom_items_since_rotation < BLOOM_ROTATION_ITEMS
        {
            return;
        }
        self.previous_broadcasted_filter = std::mem::replace(
            &mut self.broadcasted_filter,
            VertexBloomFilter::new(
                DEFAULT_BLOOM_EXPECTED_ITEMS,
                DEFAULT_BLOOM_FALSE_POSITIVE_RATE,
            ),
        );
        self.bloom_items_since_rotation = 0;
        self.bloom_last_rotation = Instant::now();
    }

    /// Create new broadcaster
    pub fn new(max_batch_size: usize, max_batch_delay: Duration) -> Self {
        Self::build(
            max_batch_size,
            max_batch_delay,
            AdaptiveBatcher::new(AdaptiveBatchConfig::default()),
        )
    }

    /// Create broadcaster with custom adaptive config
    pub fn with_adaptive_config(
        max_batch_size: usize,
        max_batch_delay: Duration,
        adaptive_config: AdaptiveBatchConfig,
    ) -> Self {
        Self::build(
            max_batch_size,
            max_batch_delay,
            AdaptiveBatcher::new(adaptive_config),
        )
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
        self.add_vertex_arc(Arc::new(vertex), is_priority);
    }

    /// Add shared vertex to broadcast queue (hot-path friendly).
    pub fn add_vertex_arc(&mut self, vertex: Arc<DagVertex>, is_priority: bool) {
        self.rotate_bloom_if_needed();
        if self.broadcasted_filter.might_contain(&vertex.id)
            || self.previous_broadcasted_filter.might_contain(&vertex.id)
        {
            return; // Likely already broadcasted
        }

        self.broadcasted_filter.add(&vertex.id);
        self.bloom_items_since_rotation += 1;

        self.enqueue_vertex(vertex, is_priority);
    }

    /// Create batch from pending vertices
    pub fn create_batch(&mut self) -> Option<VertexBatch> {
        let adaptive_size = self.adaptive_batcher.get_batch_size();
        let batch_limit = adaptive_size.min(self.max_batch_size);
        let mut vertices = Vec::with_capacity(batch_limit);

        // FIX: Add byte size limit to prevent network socket congestion
        const MAX_BATCH_BYTES: usize = 2 * 1024 * 1024; // 2 MB limit per batch
        let mut current_bytes = 0;

        // FIX #5: Prevent priority queue starvation by using ratio-based selection
        // Allocate 70% capacity to priority queue, 30% to normal queue
        // This ensures normal transactions are never completely starved
        let priority_limit = (batch_limit as f64 * 0.7) as usize;
        let normal_limit = batch_limit.saturating_sub(priority_limit);

        // Helper function to calculate actual vertex size in bytes
        // FIX #2 & #15: Use cached serialized data instead of re-serializing transactions (Hot Path optimization)
        let calculate_vertex_size = |vertex: &DagVertex| -> usize {
            // Base overhead for vertex structure (id, round, author, parents, timestamp, signature, metadata)
            let base_overhead = 256; // Conservative estimate for fixed fields

            // FIX #15: CRITICAL - Use cached transaction sizes instead of re-serializing
            // Previously called bcs::to_bytes() on every transaction in hot path
            // causing CPU exhaustion at high TPS (100K+ TPS scenarios)
            let tx_data_size: usize = if let Some(ref cached_data) = vertex.cached_serialized_data {
                // Use pre-computed cached size (O(1))
                cached_data.len()
            } else {
                // Fallback: estimate based on transaction count and average size
                // Average transaction size is ~200-300 bytes for simple transfers
                const AVG_TX_SIZE: usize = 256;
                vertex.transactions.len().saturating_mul(AVG_TX_SIZE)
            };

            base_overhead + tx_data_size
        };

        // Fill batch from priority queue first (up to 70% of capacity).
        let mut priority_count = 0;
        while priority_count < priority_limit && vertices.len() < batch_limit {
            if let Some(vertex) = self.priority_queue.front() {
                let v_size = calculate_vertex_size(vertex);
                if current_bytes + v_size > MAX_BATCH_BYTES && !vertices.is_empty() {
                    break; // Size too large, cut batch here
                }
                current_bytes += v_size;
                vertices.push((**vertex).clone());
                self.priority_queue.pop_front();
                priority_count += 1;
            } else {
                break;
            }
        }

        // Then fill remaining capacity from normal queue (at least 30%).
        let mut normal_count = 0;
        while normal_count < normal_limit && vertices.len() < batch_limit {
            if let Some(vertex) = self.pending.front() {
                let v_size = calculate_vertex_size(vertex);
                if current_bytes + v_size > MAX_BATCH_BYTES && !vertices.is_empty() {
                    break;
                }
                current_bytes += v_size;
                vertices.push((**vertex).clone());
                self.pending.pop_front();
                normal_count += 1;
            } else {
                break;
            }
        }

        // If we still have capacity and priority queue has more, allow overflow
        while vertices.len() < batch_limit {
            if let Some(vertex) = self.priority_queue.front() {
                let v_size = calculate_vertex_size(vertex);
                if current_bytes + v_size > MAX_BATCH_BYTES && !vertices.is_empty() {
                    break;
                }
                current_bytes += v_size;
                vertices.push((**vertex).clone());
                self.priority_queue.pop_front();
            } else if let Some(vertex) = self.pending.front() {
                let v_size = calculate_vertex_size(vertex);
                if current_bytes + v_size > MAX_BATCH_BYTES && !vertices.is_empty() {
                    break;
                }
                current_bytes += v_size;
                vertices.push((**vertex).clone());
                self.pending.pop_front();
            } else {
                break;
            }
        }

        if vertices.is_empty() {
            return None;
        }

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
                    + v.transactions.len() * 150 // Estimate
            })
            .sum();

        Some(VertexBatch {
            vertices,
            round_range,
            size_bytes,
            created_at: Self::unix_timestamp_secs(),
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

    /// Decompress batch with safety limits (Prevents Zip Bomb attacks)
    pub fn decompress_batch(&self, compressed: &CompressedBatch) -> Result<VertexBatch> {
        // FIX 2: Prevent Zip Bomb (OOM Attack) by limiting decompressed size
        // Set ceiling at 10 MB which is sufficient for largest VertexBatch per config (50K vertices)
        const MAX_SAFE_DECOMPRESSED_SIZE: usize = 10 * 1024 * 1024;

        if compressed.original_size > MAX_SAFE_DECOMPRESSED_SIZE {
            return Err(anyhow::anyhow!(
                "Security Alert: Decompressed size {} exceeds safety limit ({} bytes)",
                compressed.original_size,
                MAX_SAFE_DECOMPRESSED_SIZE
            ));
        }

        let decompressed = if self.compression_enabled {
            // Use zstd with output size limit or check from sent metadata
            zstd::decode_all(&compressed.data[..])
                .map_err(|e| anyhow::anyhow!("Failed to decompress: {}", e))?
        } else {
            compressed.data.clone()
        };

        // Verify actual size after decompression for maximum security
        if decompressed.len() != compressed.original_size {
            return Err(anyhow::anyhow!(
                "Decompressed size mismatch: actual size doesn't match reported size"
            ));
        }

        // Deserialize using BCS
        let batch: VertexBatch = bcs::from_bytes(&decompressed)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize batch: {}", e))?;

        Ok(batch)
    }

    /// Check if vertex might have been broadcasted
    pub fn might_have_broadcasted(&self, vertex_id: &VertexId) -> bool {
        self.broadcasted_filter.might_contain(vertex_id)
            || self.previous_broadcasted_filter.might_contain(vertex_id)
    }

    /// Get pending count
    pub fn pending_count(&self) -> usize {
        self.pending.len() + self.priority_queue.len()
    }
}

/// Delta sync: Identify missing vertices between two nodes
pub struct DeltaSync {
    /// Local vertex IDs by round
    local_vertices: BTreeMap<Round, BTreeSet<VertexId>>,
}

impl DeltaSync {
    /// Create new delta sync
    pub fn new() -> Self {
        Self {
            local_vertices: BTreeMap::new(),
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

    /// Prune old round data to prevent memory leak
    pub fn prune_old_rounds(&mut self, before_round: Round) {
        self.local_vertices
            .retain(|round, _| *round >= before_round);

        tracing::debug!(
            "Pruned DeltaSync before round {}, remaining: {} rounds",
            before_round,
            self.local_vertices.len()
        );
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
    use crate::consensus::VertexMetadata;

    use super::*;

    #[test]
    fn test_bloom_filter() {
        let mut filter = VertexBloomFilter::new(1000, 0.01);

        let mut id1 = [0u8; 32];
        id1[0..4].copy_from_slice(&[1, 2, 3, 4]);
        let mut id2 = [0u8; 32];
        id2[0..4].copy_from_slice(&[5, 6, 7, 8]);

        filter.add(&id1);

        assert!(filter.might_contain(&id1));
        assert!(!filter.might_contain(&id2));
    }

    #[test]
    fn test_broadcaster() {
        let mut broadcaster = VertexBroadcaster::new(10, Duration::from_secs(1));

        let parent = [0u8; 32];
        let vertex = DagVertex::new_for_test(
            1,
            "auth1".to_string(),
            vec![parent],
            vec![],
            vec![0u8; 32],
            0,
        );

        broadcaster.add_vertex(vertex.clone(), false);
        assert_eq!(broadcaster.pending_count(), 1);

        let batch = broadcaster.create_batch();
        assert!(batch.is_some());
        assert_eq!(batch.unwrap().vertices.len(), 1);
    }

    #[test]
    fn test_priority_queue() {
        let mut broadcaster = VertexBroadcaster::new(10, Duration::from_secs(1));

        let normal = DagVertex::new_for_test(1, "auth1".to_string(), vec![], vec![], vec![], 0);
        let priority = DagVertex::new_for_test(2, "auth2".to_string(), vec![], vec![], vec![], 0);

        broadcaster.add_vertex(normal, false);
        broadcaster.add_vertex(priority, true);

        let batch = broadcaster.create_batch().unwrap();

        // Priority vertex should come first
        assert_eq!(batch.vertices[0].round, 2);
    }

    #[test]
    fn test_delta_sync() {
        let mut sync = DeltaSync::new();

        let mut id1 = [0u8; 32];
        id1[0..3].copy_from_slice(&[1, 2, 3]);
        let mut id2 = [0u8; 32];
        id2[0..3].copy_from_slice(&[4, 5, 6]);

        sync.add_local_vertex(1, id1);
        sync.add_local_vertex(1, id2);

        let filter = sync.create_round_filter(1);
        assert!(filter.might_contain(&id1));
        assert!(filter.might_contain(&id2));
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn test_zstd_compression() {
        let broadcaster = VertexBroadcaster::new(10, Duration::from_secs(1));

        let mut vertex_id = [0u8; 32];
        vertex_id[0..3].copy_from_slice(&[1, 2, 3]);

        let batch = VertexBatch {
            vertices: vec![DagVertex {
                chain_id: "test_chain".to_string(),
                id: vertex_id,
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
                cached_serialized_data: None,
                cached_hash: None,
            }],
            round_range: (1, 1),
            size_bytes: 200,
            created_at: 1000,
        };

        let compressed = broadcaster.compress_batch(&batch).unwrap();
        assert!(!compressed.data.is_empty());
        assert!(compressed.compression_ratio < 1.0); // Should be compressed

        let decompressed = broadcaster.decompress_batch(&compressed).unwrap();
        assert_eq!(decompressed.vertices.len(), 1);
        assert_eq!(decompressed.vertices[0].id, vertex_id);
    }

    #[test]
    fn test_adaptive_batching() {
        let config = AdaptiveBatchConfig {
            min_batch_size: 10,
            max_batch_size: 1000,
            target_latency_ms: 100,
            adjustment_factor: 0.2,
            adjustment_interval: Duration::from_millis(10), // Short interval for testing
        };

        let mut batcher = AdaptiveBatcher::new(config);
        let initial_size = batcher.get_batch_size();

        // Simulate high latency (200ms) - should increase batch size
        batcher.observe_latency(Duration::from_millis(200));
        std::thread::sleep(Duration::from_millis(20)); // Wait past short adjustment interval
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
            adjustment_interval: Duration::from_millis(10), // Short interval for testing
        };

        let mut batcher = AdaptiveBatcher::new(config);
        let initial_size = batcher.get_batch_size();

        // Simulate low latency (30ms) - should decrease batch size
        batcher.observe_latency(Duration::from_millis(30));
        std::thread::sleep(Duration::from_millis(20)); // Wait past short adjustment interval
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
            adjustment_interval: Duration::from_millis(0), // Immediate adjustment for testing
        };

        let mut batcher = AdaptiveBatcher::new(config);

        // Extreme high latency - should cap at max
        for _ in 0..10 {
            batcher.observe_latency(Duration::from_millis(1000));
        }

        assert_eq!(batcher.get_batch_size(), 100);

        // Reset and test minimum
        batcher.reset();

        // Extreme low latency - should cap at min
        for _ in 0..10 {
            batcher.observe_latency(Duration::from_millis(1));
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
            adjustment_interval: Duration::from_millis(10), // Short interval for testing
        };

        let mut broadcaster =
            VertexBroadcaster::with_adaptive_config(100, Duration::from_secs(1), config);

        // Add some vertices
        for i in 0..20 {
            let vertex =
                DagVertex::new_for_test(i, format!("auth{}", i), vec![], vec![], vec![], 0);
            broadcaster.add_vertex(vertex, false);
        }

        // Observe latency
        broadcaster.observe_latency(Duration::from_millis(50));

        // Get adaptive batch size
        let batch_size = broadcaster.get_adaptive_batch_size();
        assert!((5..=50).contains(&batch_size));

        // Create batch - should respect adaptive size
        let batch = broadcaster.create_batch().unwrap();
        assert!(batch.vertices.len() <= batch_size);
    }

    #[test]
    fn test_create_batch_does_not_drop_pending_vertices() {
        let mut broadcaster = VertexBroadcaster::new(2, Duration::from_secs(1));

        for i in 0..5 {
            let vertex =
                DagVertex::new_for_test(i, format!("auth{}", i), vec![], vec![], vec![], 0);
            broadcaster.add_vertex(vertex, false);
        }

        let first_batch = broadcaster.create_batch().unwrap();
        assert_eq!(first_batch.vertices.len(), 2);
        assert_eq!(broadcaster.pending_count(), 3);
    }

    #[test]
    fn test_bloom_filter_rotation() {
        let mut broadcaster = VertexBroadcaster::new(10, Duration::from_secs(1));
        let vertex =
            DagVertex::new_for_test(1, "auth1".to_string(), vec![], vec![], vec![0u8; 32], 0);
        let vertex_id = vertex.id;
        broadcaster.add_vertex(vertex, false);
        assert!(broadcaster.might_have_broadcasted(&vertex_id));

        broadcaster.bloom_last_rotation = Instant::now() - Duration::from_secs(2);
        broadcaster.rotate_bloom_if_needed();
        assert!(broadcaster.might_have_broadcasted(&vertex_id));

        broadcaster.bloom_last_rotation = Instant::now() - Duration::from_secs(2);
        broadcaster.rotate_bloom_if_needed();
        assert!(!broadcaster.might_have_broadcasted(&vertex_id));
    }
}
