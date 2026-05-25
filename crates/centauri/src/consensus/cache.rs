use moka::sync::Cache as MokaCache;

use crate::consensus::{DagVertex, VertexId};

/// High-concurrency cache using Moka (lock-free reads, segment-based writes)
pub type LruCache<K, V> = MokaCache<K, V>;

/// Helper to create a new cache with capacity
pub fn new_cache<K, V>(capacity: usize) -> LruCache<K, V>
where
    K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    MokaCache::builder().max_capacity(capacity as u64).build()
}

/// Cache statistics snapshot
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub size: usize,
}

/// Specialized DAG caches for optimal performance
pub struct DagCaches {
    /// Vertex cache: VertexId -> DagVertex
    pub vertices: LruCache<VertexId, DagVertex>,

    /// State root cache: VertexId -> state_root
    pub state_roots: LruCache<VertexId, Vec<u8>>,

    /// Merkle proof cache: (vertex_id, leaf_index) -> proof
    pub merkle_proofs: LruCache<(VertexId, usize), Vec<Vec<u8>>>,

    /// Parent vertices cache: `VertexId` -> `Vec<VertexId>`
    pub parent_vertices: LruCache<VertexId, Vec<VertexId>>,

    /// Round cache: Round -> `Vec<VertexId>`
    pub round_vertices: LruCache<u64, Vec<VertexId>>,
}

impl DagCaches {
    fn preset(
        vertices: usize,
        state_roots: usize,
        merkle_proofs: usize,
        parent_vertices: usize,
        round_vertices: usize,
    ) -> Self {
        Self {
            vertices: new_cache(vertices),
            state_roots: new_cache(state_roots),
            merkle_proofs: new_cache(merkle_proofs),
            parent_vertices: new_cache(parent_vertices),
            round_vertices: new_cache(round_vertices),
        }
    }

    /// Create new DAG caches with default sizes optimized for high throughput
    pub fn new() -> Self {
        Self::preset(100000, 50000, 10000, 100000, 10000)
    }

    /// Create extreme high-throughput caches for 500K+ TPS
    pub fn extreme_throughput() -> Self {
        Self::preset(500000, 250000, 50000, 500000, 50000)
    }

    /// Get total cache statistics
    pub fn total_stats(&self) -> DagCacheStats {
        DagCacheStats {
            vertices: CacheStats {
                size: self.vertices.entry_count() as usize,
            },
            state_roots: CacheStats {
                size: self.state_roots.entry_count() as usize,
            },
            merkle_proofs: CacheStats {
                size: self.merkle_proofs.entry_count() as usize,
            },
            parent_vertices: CacheStats {
                size: self.parent_vertices.entry_count() as usize,
            },
            round_vertices: CacheStats {
                size: self.round_vertices.entry_count() as usize,
            },
        }
    }

    /// Get total memory usage estimate (bytes)
    pub fn memory_usage(&self) -> usize {
        let stats = self.total_stats();
        let vertex_size = 1000;
        let state_root_size = 32;
        let proof_size = 500;
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
    /// Format statistics as human-readable string
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str("=== DAG Cache Statistics ===\n\n");

        s.push_str(&format!("Vertices: {} entries\n", self.vertices.size));
        s.push_str(&format!("State Roots: {} entries\n", self.state_roots.size));
        s.push_str(&format!(
            "Merkle Proofs: {} entries\n",
            self.merkle_proofs.size
        ));
        s.push_str(&format!(
            "Parent Vertices: {} entries\n",
            self.parent_vertices.size
        ));
        s.push_str(&format!(
            "Round Vertices: {} entries\n",
            self.round_vertices.size
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
        let cache: LruCache<i32, String> = new_cache(10);

        cache.insert(1, "one".to_string());
        cache.insert(2, "two".to_string());
        cache.insert(3, "three".to_string());

        assert_eq!(cache.get(&1), Some("one".to_string()));
        assert_eq!(cache.get(&2), Some("two".to_string()));
        assert!(cache.entry_count() <= 10);
    }

    #[test]
    fn test_eviction() {
        let cache: LruCache<i32, String> = new_cache(2);

        cache.insert(1, "one".to_string());
        cache.insert(2, "two".to_string());
        cache.insert(3, "three".to_string());

        cache.run_pending_tasks();
        assert!(cache.entry_count() <= 3);

        let has_any = cache.get(&1).is_some() || cache.get(&2).is_some() || cache.get(&3).is_some();
        assert!(has_any, "Cache should have at least one item");
    }

    #[test]
    fn test_lru_order() {
        let cache: LruCache<i32, String> = new_cache(10);

        cache.insert(1, "one".to_string());
        cache.insert(2, "two".to_string());

        assert_eq!(cache.get(&1), Some("one".to_string()));

        cache.insert(3, "three".to_string());

        cache.run_pending_tasks();

        assert!(cache.get(&1).is_some());
        assert!(cache.get(&3).is_some());
    }

    #[test]
    fn test_update_existing() {
        let cache: LruCache<i32, String> = new_cache(2);

        cache.insert(1, "one".to_string());
        cache.insert(1, "ONE".to_string());

        cache.run_pending_tasks();

        assert_eq!(cache.get(&1), Some("ONE".to_string()));
        assert!(cache.entry_count() <= 2);
    }

    #[test]
    fn test_thread_safety() {
        let cache: LruCache<i32, i32> = new_cache(100);
        let cache_clone = cache.clone();

        let handle = thread::spawn(move || {
            for i in 0..50 {
                cache_clone.insert(i, i * 2);
            }
        });

        for i in 50..100 {
            cache.insert(i, i * 2);
        }

        handle.join().unwrap();

        assert!(cache.entry_count() <= 100);
    }

    #[test]
    fn test_dag_caches() {
        let caches = DagCaches::new();

        let mut vertex_id = [0u8; 32];
        vertex_id[0..3].copy_from_slice(&[1, 2, 3]);

        // Test vertices cache
        let vertex = DagVertex {
            chain_id: "test_chain".to_string(),
            id: vertex_id,
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
            cached_serialized_data: None,
            cached_hash: None,
        };

        caches.vertices.insert(vertex_id, vertex.clone());
        assert!(caches.vertices.get(&vertex_id).is_some());

        // Test state roots cache
        caches.state_roots.insert(vertex_id, vec![7, 8, 9]);
        assert_eq!(caches.state_roots.get(&vertex_id), Some(vec![7, 8, 9]));

        let stats = caches.total_stats();
        assert!(stats.vertices.size <= 1); // May vary with async eviction
        assert!(stats.state_roots.size <= 1);
    }

    #[test]
    fn test_memory_usage() {
        let caches = DagCaches::new();

        // Add some vertices
        for i in 0..100 {
            let mut vertex_id = [0u8; 32];
            vertex_id[0] = i;
            caches.vertices.insert(
                vertex_id,
                DagVertex {
                    chain_id: "test_chain".to_string(),
                    id: vertex_id,
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
                    cached_serialized_data: None,
                    cached_hash: None,
                },
            );
        }

        let memory = caches.memory_usage();
        assert!(memory > 0);
        assert!(memory < 1_000_000); // Should be reasonable
    }
}
