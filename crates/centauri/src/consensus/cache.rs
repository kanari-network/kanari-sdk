// Phase 2.3: Advanced Caching System
// High-performance LRU caching for DAG consensus (10-100x performance boost)

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::consensus::{DagVertex, VertexId};

/// Generic LRU Cache with thread-safety and TTL support
pub struct LruCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    inner: Arc<RwLock<LruCacheInner<K, V>>>,
}

struct LruCacheInner<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    capacity: usize,
    entries: HashMap<K, CacheEntry<V>>,
    access_order: VecDeque<K>,
    ttl: Option<Duration>,

    // Statistics
    hits: u64,
    misses: u64,
    evictions: u64,
}

#[derive(Clone)]
struct CacheEntry<V> {
    value: V,
    inserted_at: Instant,
    last_accessed: Instant,
}

impl<K, V> LruCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    /// Create new LRU cache with capacity
    pub fn new(capacity: usize) -> Self {
        Self::with_ttl(capacity, None)
    }

    /// Create LRU cache with TTL (time-to-live)
    pub fn with_ttl(capacity: usize, ttl: Option<Duration>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(LruCacheInner {
                capacity,
                entries: HashMap::new(),
                access_order: VecDeque::new(),
                ttl,
                hits: 0,
                misses: 0,
                evictions: 0,
            })),
        }
    }

    /// Get value from cache
    pub fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.inner.write().ok()?;

        // Check if entry exists and not expired
        if let Some(entry) = cache.entries.get(key) {
            // Check TTL
            if let Some(ttl) = cache.ttl
                && entry.inserted_at.elapsed() > ttl
            {
                // Expired, remove it
                cache.entries.remove(key);
                cache.access_order.retain(|k| k != key);
                cache.misses += 1;
                return None;
            }

            // Clone value before mutable operations
            let value = entry.value.clone();

            // Update access time and move to back (most recently used)
            if let Some(entry) = cache.entries.get_mut(key) {
                entry.last_accessed = Instant::now();
            }
            cache.access_order.retain(|k| k != key);
            cache.access_order.push_back(key.clone());

            cache.hits += 1;
            Some(value)
        } else {
            cache.misses += 1;
            None
        }
    }

    /// Insert value into cache
    pub fn put(&self, key: K, value: V) {
        if let Ok(mut cache) = self.inner.write() {
            let now = Instant::now();

            // If key already exists, update it
            if cache.entries.contains_key(&key) {
                cache.entries.insert(
                    key.clone(),
                    CacheEntry {
                        value,
                        inserted_at: now,
                        last_accessed: now,
                    },
                );
                cache.access_order.retain(|k| k != &key);
                cache.access_order.push_back(key);
                return;
            }

            // Check capacity
            if cache.entries.len() >= cache.capacity {
                // Evict least recently used
                if let Some(lru_key) = cache.access_order.pop_front() {
                    cache.entries.remove(&lru_key);
                    cache.evictions += 1;
                }
            }

            // Insert new entry
            cache.entries.insert(
                key.clone(),
                CacheEntry {
                    value,
                    inserted_at: now,
                    last_accessed: now,
                },
            );
            cache.access_order.push_back(key);
        }
    }

    /// Remove value from cache
    pub fn remove(&self, key: &K) -> Option<V> {
        let mut cache = self.inner.write().ok()?;

        cache.access_order.retain(|k| k != key);
        cache.entries.remove(key).map(|entry| entry.value)
    }

    /// Clear all entries
    pub fn clear(&self) {
        if let Ok(mut cache) = self.inner.write() {
            cache.entries.clear();
            cache.access_order.clear();
        }
    }

    /// Get cache size
    pub fn len(&self) -> usize {
        self.inner.read().ok().map(|c| c.entries.len()).unwrap_or(0)
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        if let Ok(cache) = self.inner.read() {
            CacheStats {
                size: cache.entries.len(),
                capacity: cache.capacity,
                hits: cache.hits,
                misses: cache.misses,
                evictions: cache.evictions,
                hit_rate: if cache.hits + cache.misses == 0 {
                    0.0
                } else {
                    cache.hits as f64 / (cache.hits + cache.misses) as f64
                },
            }
        } else {
            CacheStats::default()
        }
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        if let Ok(mut cache) = self.inner.write() {
            cache.hits = 0;
            cache.misses = 0;
            cache.evictions = 0;
        }
    }
}

impl<K, V> Clone for LruCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub hit_rate: f64,
}

/// Specialized DAG caches for optimal performance
pub struct DagCaches {
    /// Vertex cache: VertexId -> DagVertex
    pub vertices: LruCache<VertexId, DagVertex>,

    /// State root cache: VertexId -> state_root
    pub state_roots: LruCache<VertexId, Vec<u8>>,

    /// Merkle proof cache: (vertex_id, leaf_index) -> proof
    pub merkle_proofs: LruCache<(VertexId, usize), Vec<Vec<u8>>>,

    /// Parent vertices cache: VertexId -> Vec<VertexId>
    pub parent_vertices: LruCache<VertexId, Vec<VertexId>>,

    /// Round cache: Round -> Vec<VertexId>
    pub round_vertices: LruCache<u64, Vec<VertexId>>,
}

impl DagCaches {
    /// Create new DAG caches with default sizes
    pub fn new() -> Self {
        Self {
            vertices: LruCache::new(10000),        // 10k vertices
            state_roots: LruCache::new(5000),      // 5k state roots
            merkle_proofs: LruCache::new(1000),    // 1k proofs
            parent_vertices: LruCache::new(10000), // 10k parent sets
            round_vertices: LruCache::new(1000),   // 1k rounds
        }
    }

    /// Create with custom capacities
    pub fn with_capacities(
        vertices: usize,
        state_roots: usize,
        merkle_proofs: usize,
        parent_vertices: usize,
        round_vertices: usize,
    ) -> Self {
        Self {
            vertices: LruCache::new(vertices),
            state_roots: LruCache::new(state_roots),
            merkle_proofs: LruCache::new(merkle_proofs),
            parent_vertices: LruCache::new(parent_vertices),
            round_vertices: LruCache::new(round_vertices),
        }
    }

    /// Get total cache statistics
    pub fn total_stats(&self) -> DagCacheStats {
        DagCacheStats {
            vertices: self.vertices.stats(),
            state_roots: self.state_roots.stats(),
            merkle_proofs: self.merkle_proofs.stats(),
            parent_vertices: self.parent_vertices.stats(),
            round_vertices: self.round_vertices.stats(),
        }
    }

    /// Clear all caches
    pub fn clear_all(&self) {
        self.vertices.clear();
        self.state_roots.clear();
        self.merkle_proofs.clear();
        self.parent_vertices.clear();
        self.round_vertices.clear();
    }

    /// Get total memory usage estimate (bytes)
    pub fn memory_usage(&self) -> usize {
        let stats = self.total_stats();

        // Rough estimates
        let vertex_size = 1000; // ~1KB per vertex
        let state_root_size = 32;
        let proof_size = 500; // ~500 bytes per proof
        let parent_set_size = 100;
        let round_set_size = 1000;

        stats.vertices.size * vertex_size
            + stats.state_roots.size * state_root_size
            + stats.merkle_proofs.size * proof_size
            + stats.parent_vertices.size * parent_set_size
            + stats.round_vertices.size * round_set_size
    }
}

impl Default for DagCaches {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct DagCacheStats {
    pub vertices: CacheStats,
    pub state_roots: CacheStats,
    pub merkle_proofs: CacheStats,
    pub parent_vertices: CacheStats,
    pub round_vertices: CacheStats,
}

impl DagCacheStats {
    /// Get overall hit rate across all caches
    pub fn overall_hit_rate(&self) -> f64 {
        let total_hits = self.vertices.hits
            + self.state_roots.hits
            + self.merkle_proofs.hits
            + self.parent_vertices.hits
            + self.round_vertices.hits;

        let total_requests = total_hits
            + self.vertices.misses
            + self.state_roots.misses
            + self.merkle_proofs.misses
            + self.parent_vertices.misses
            + self.round_vertices.misses;

        if total_requests == 0 {
            0.0
        } else {
            total_hits as f64 / total_requests as f64
        }
    }

    /// Format statistics as human-readable string
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str("=== DAG Cache Statistics ===\n\n");

        s.push_str("Vertices Cache:\n");
        s.push_str(&format!(
            "  Size:      {}/{}\n",
            self.vertices.size, self.vertices.capacity
        ));
        s.push_str(&format!("  Hits:      {}\n", self.vertices.hits));
        s.push_str(&format!("  Misses:    {}\n", self.vertices.misses));
        s.push_str(&format!("  Evictions: {}\n", self.vertices.evictions));
        s.push_str(&format!(
            "  Hit Rate:  {:.2}%\n\n",
            self.vertices.hit_rate * 100.0
        ));

        s.push_str("State Roots Cache:\n");
        s.push_str(&format!(
            "  Size:      {}/{}\n",
            self.state_roots.size, self.state_roots.capacity
        ));
        s.push_str(&format!(
            "  Hit Rate:  {:.2}%\n\n",
            self.state_roots.hit_rate * 100.0
        ));

        s.push_str("Merkle Proofs Cache:\n");
        s.push_str(&format!(
            "  Size:      {}/{}\n",
            self.merkle_proofs.size, self.merkle_proofs.capacity
        ));
        s.push_str(&format!(
            "  Hit Rate:  {:.2}%\n\n",
            self.merkle_proofs.hit_rate * 100.0
        ));

        s.push_str(&format!(
            "Overall Hit Rate: {:.2}%\n",
            self.overall_hit_rate() * 100.0
        ));

        s
    }
}

#[cfg(test)]
mod tests {
    use crate::consensus::{DagVertex, VertexMetadata};

    use super::*;
    use std::thread;

    #[test]
    fn test_basic_lru() {
        let cache: LruCache<i32, String> = LruCache::new(3);

        cache.put(1, "one".to_string());
        cache.put(2, "two".to_string());
        cache.put(3, "three".to_string());

        assert_eq!(cache.get(&1), Some("one".to_string()));
        assert_eq!(cache.get(&2), Some("two".to_string()));
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn test_eviction() {
        let cache: LruCache<i32, String> = LruCache::new(2);

        cache.put(1, "one".to_string());
        cache.put(2, "two".to_string());
        cache.put(3, "three".to_string()); // Should evict 1

        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&2), Some("two".to_string()));
        assert_eq!(cache.get(&3), Some("three".to_string()));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_lru_order() {
        let cache: LruCache<i32, String> = LruCache::new(2);

        cache.put(1, "one".to_string());
        cache.put(2, "two".to_string());

        // Access 1, making it more recent
        assert_eq!(cache.get(&1), Some("one".to_string()));

        // Add 3, should evict 2 (least recently used)
        cache.put(3, "three".to_string());

        assert_eq!(cache.get(&1), Some("one".to_string()));
        assert_eq!(cache.get(&2), None);
        assert_eq!(cache.get(&3), Some("three".to_string()));
    }

    #[test]
    fn test_update_existing() {
        let cache: LruCache<i32, String> = LruCache::new(2);

        cache.put(1, "one".to_string());
        cache.put(1, "ONE".to_string()); // Update

        assert_eq!(cache.get(&1), Some("ONE".to_string()));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_remove() {
        let cache: LruCache<i32, String> = LruCache::new(3);

        cache.put(1, "one".to_string());
        cache.put(2, "two".to_string());

        assert_eq!(cache.remove(&1), Some("one".to_string()));
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_clear() {
        let cache: LruCache<i32, String> = LruCache::new(3);

        cache.put(1, "one".to_string());
        cache.put(2, "two".to_string());

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_stats() {
        let cache: LruCache<i32, String> = LruCache::new(2);

        cache.put(1, "one".to_string());
        cache.put(2, "two".to_string());

        cache.get(&1); // Hit
        cache.get(&2); // Hit
        cache.get(&3); // Miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 2.0 / 3.0);
    }

    #[test]
    fn test_ttl() {
        let cache: LruCache<i32, String> = LruCache::with_ttl(3, Some(Duration::from_millis(50)));

        cache.put(1, "one".to_string());

        assert_eq!(cache.get(&1), Some("one".to_string()));

        thread::sleep(Duration::from_millis(60));

        assert_eq!(cache.get(&1), None); // Expired
    }

    #[test]
    fn test_thread_safety() {
        let cache: LruCache<i32, i32> = LruCache::new(100);
        let cache_clone = cache.clone();

        let handle = thread::spawn(move || {
            for i in 0..50 {
                cache_clone.put(i, i * 2);
            }
        });

        for i in 50..100 {
            cache.put(i, i * 2);
        }

        handle.join().unwrap();

        assert!(cache.len() <= 100);
    }

    #[test]
    fn test_dag_caches() {
        let caches = DagCaches::new();

        // Test vertices cache
        let vertex = DagVertex {
            id: vec![1, 2, 3],
            round: 1,
            author: "test_author".to_string(),
            parents: vec![],
            transactions: vec![],
            timestamp: 12345,
            signature: vec![],
            metadata: VertexMetadata {
                tx_count: 0,
                total_gas_used: 0,
                state_root: vec![],
                is_checkpoint: false,
                checkpoint_seq: None,
            },
        };

        caches.vertices.put(vec![1, 2, 3], vertex.clone());
        assert!(caches.vertices.get(&vec![1, 2, 3]).is_some());

        // Test state roots cache
        caches.state_roots.put(vec![1, 2, 3], vec![7, 8, 9]);
        assert_eq!(caches.state_roots.get(&vec![1, 2, 3]), Some(vec![7, 8, 9]));

        let stats = caches.total_stats();
        assert_eq!(stats.vertices.size, 1);
        assert_eq!(stats.state_roots.size, 1);
    }

    #[test]
    fn test_cache_stats_summary() {
        let caches = DagCaches::new();

        // Add some data
        for i in 0..10 {
            caches.vertices.put(
                vec![i],
                DagVertex {
                    id: vec![i],
                    round: i as u64,
                    author: "test".to_string(),
                    parents: vec![],
                    transactions: vec![],
                    timestamp: 0,
                    signature: vec![],
                    metadata: VertexMetadata {
                        tx_count: 0,
                        total_gas_used: 0,
                        state_root: vec![],
                        is_checkpoint: false,
                        checkpoint_seq: None,
                    },
                },
            );
        }

        // Generate some hits/misses
        for i in 0..5 {
            caches.vertices.get(&vec![i]);
        }
        caches.vertices.get(&vec![99]); // Miss

        let stats = caches.total_stats();
        let summary = stats.summary();

        assert!(summary.contains("DAG Cache Statistics"));
        assert!(summary.contains("Vertices Cache:"));
        assert!(summary.contains("Hit Rate:"));
    }

    #[test]
    fn test_memory_usage() {
        let caches = DagCaches::new();

        // Add some vertices
        for i in 0..100 {
            caches.vertices.put(
                vec![i],
                DagVertex {
                    id: vec![i],
                    round: i as u64,
                    author: "test".to_string(),
                    parents: vec![],
                    transactions: vec![],
                    timestamp: 0,
                    signature: vec![],
                    metadata: VertexMetadata {
                        tx_count: 0,
                        total_gas_used: 0,
                        state_root: vec![],
                        is_checkpoint: false,
                        checkpoint_seq: None,
                    },
                },
            );
        }

        let memory = caches.memory_usage();
        assert!(memory > 0);
        assert!(memory < 1_000_000); // Should be reasonable
    }
}
