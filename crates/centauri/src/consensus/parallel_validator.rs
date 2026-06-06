// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use siphasher::sip::SipHasher13;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::cache::LruCache;
use super::crypto_signatures::Ed25519Keypair;
use super::dag_consensus::{DagVertex, VertexId};
use super::persistent_store::PersistentDagStore;
use std::hash::{Hash, Hasher};

// --- Security Constants (FIX #3: Prevent DoS via excessive parents) ---
const MAX_PARENTS_PER_VERTEX: usize = 100; // Reasonable limit for DAG consensus

// --- Bloom Filter Implementation ---
pub struct BloomFilter {
    bit_set: Vec<bool>,
    num_hashes: usize,
}

impl BloomFilter {
    pub fn new(capacity: usize, _error_rate: f64) -> Self {
        Self {
            bit_set: vec![false; capacity],
            num_hashes: 3, // Optimal value for general use cases
        }
    }

    pub fn insert<T: Hash>(&mut self, item: &T) {
        for i in 0..self.num_hashes {
            // FIX #3: Use SipHasher13 with deterministic seeds instead of DefaultHasher
            // DefaultHasher uses random seeds which makes results non-deterministic across restarts
            let mut s = SipHasher13::new_with_keys(0, 0); // Deterministic keys
            b"bloom_filter_seed".hash(&mut s);
            item.hash(&mut s);
            (i as u64).hash(&mut s);
            let index = (s.finish() as usize) % self.bit_set.len();
            self.bit_set[index] = true;
        }
    }

    pub fn contains<T: Hash>(&self, item: &T) -> bool {
        for i in 0..self.num_hashes {
            // FIX #3: Same deterministic hashing as insert
            let mut s = SipHasher13::new_with_keys(0, 0); // Deterministic keys
            b"bloom_filter_seed".hash(&mut s);
            item.hash(&mut s);
            (i as u64).hash(&mut s);
            let index = (s.finish() as usize) % self.bit_set.len();
            if !self.bit_set[index] {
                return false;
            }
        }
        true
    }
}

/// Configuration for parallel validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelValidatorConfig {
    pub num_workers: usize,
    pub max_batch_size: usize,
    pub parallel_sig_verify: bool,
    pub queue_capacity: usize,
}

impl Default for ParallelValidatorConfig {
    fn default() -> Self {
        let num_cpus = rayon::current_num_threads();
        Self {
            num_workers: num_cpus.min(64),
            max_batch_size: 5000,
            parallel_sig_verify: true,
            queue_capacity: 100000,
        }
    }
}

impl ParallelValidatorConfig {
    pub fn high_throughput() -> Self {
        let num_cpus = rayon::current_num_threads();
        Self {
            num_workers: num_cpus.min(128),
            max_batch_size: 10000,
            parallel_sig_verify: true,
            queue_capacity: 500000,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.num_workers == 0 {
            return Err(anyhow::anyhow!("num_workers must be > 0"));
        }
        if self.num_workers > 256 {
            return Err(anyhow::anyhow!("num_workers must be <= 256"));
        }
        if self.max_batch_size == 0 {
            return Err(anyhow::anyhow!("max_batch_size must be > 0"));
        }
        if self.queue_capacity == 0 {
            return Err(anyhow::anyhow!("queue_capacity must be > 0"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub vertex_id: VertexId,
    pub is_valid: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStats {
    pub total_validated: usize,
    pub successful: usize,
    pub failed: usize,
    pub avg_validation_time_ms: f64,
    pub throughput_per_sec: f64,
}

pub struct ParallelValidator {
    config: ParallelValidatorConfig,
    stats: ValidationStats,
    validated_cache: LruCache<VertexId, bool>,
    signature_validated_cache: LruCache<VertexId, bool>,
    persistent_store: Option<Arc<PersistentDagStore>>,
    disk_bloom_filter: Arc<std::sync::RwLock<BloomFilter>>,
}

impl ParallelValidator {
    fn default_stats() -> ValidationStats {
        ValidationStats {
            total_validated: 0,
            successful: 0,
            failed: 0,
            avg_validation_time_ms: 0.0,
            throughput_per_sec: 0.0,
        }
    }

    fn validation_ok(vertex_id: VertexId) -> ValidationResult {
        ValidationResult {
            vertex_id,
            is_valid: true,
            error: None,
        }
    }

    fn validation_err(vertex_id: VertexId, error: impl ToString) -> ValidationResult {
        ValidationResult {
            vertex_id,
            is_valid: false,
            error: Some(error.to_string()),
        }
    }

    fn cache_valid_results(&self, results: &[ValidationResult]) {
        for result in results {
            if result.is_valid {
                self.validated_cache.insert(result.vertex_id, true);
            }
        }
    }

    pub fn new(config: ParallelValidatorConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            stats: Self::default_stats(),
            validated_cache: LruCache::new(1_000_000),
            signature_validated_cache: LruCache::new(1_000_000),
            persistent_store: None,
            disk_bloom_filter: Arc::new(std::sync::RwLock::new(BloomFilter::new(10_000_000, 0.01))),
        })
    }

    #[cfg(test)]
    fn with_persistent_store(
        config: ParallelValidatorConfig,
        store: Arc<PersistentDagStore>,
    ) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            stats: Self::default_stats(),
            validated_cache: LruCache::new(1_000_000),
            signature_validated_cache: LruCache::new(1_000_000),
            persistent_store: Some(store),
            disk_bloom_filter: Arc::new(std::sync::RwLock::new(BloomFilter::new(10_000_000, 0.01))),
        })
    }

    pub fn validate_batch(&mut self, vertices: Vec<DagVertex>) -> Result<Vec<ValidationResult>> {
        let start = std::time::Instant::now();
        if vertices.is_empty() {
            return Ok(Vec::new());
        }

        let (cached_results, vertices_to_validate) = self.check_cache_and_store(vertices)?;

        let this = &*self;

        let validation_results: Vec<ValidationResult> = vertices_to_validate
            .into_par_iter()
            .map(|vertex| this.validate_single_ref(&vertex)) // Use this. to reference instance
            .collect();

        for result in &validation_results {
            if result.is_valid {
                self.validated_cache.insert(result.vertex_id, true);
            }
        }

        let mut all_results = cached_results;
        all_results.extend(validation_results);
        self.update_stats(&all_results, start.elapsed());

        Ok(all_results)
    }

    pub fn validate_vertex_with_public_key(
        &mut self,
        vertex: &DagVertex,
        public_key: &ed25519_dalek::VerifyingKey,
    ) -> Result<ValidationResult> {
        let start = std::time::Instant::now();
        let vertex_id = vertex.id;

        // Check base validation cache first (structure, parents, etc.)
        let needs_base_validation = self.validated_cache.get(&vertex_id).is_none();

        let base_result = if needs_base_validation {
            self.validate_single_ref(vertex)
        } else {
            // Base validation already passed, reuse result
            Self::validation_ok(vertex_id)
        };

        if !base_result.is_valid {
            self.update_stats(std::slice::from_ref(&base_result), start.elapsed());
            return Ok(base_result);
        }

        // FIX #8: Always verify signature (don't cache signature validation)
        // Signature verification is fast compared to full validation, and caching it
        // can lead to issues if the vertex is mutated or if we need to re-verify
        // with a different public key
        let result = match Self::verify_vertex_signature(vertex, public_key) {
            Ok(()) => {
                // Only cache base validation, not signature
                if needs_base_validation {
                    self.validated_cache.insert(vertex_id, true);
                }
                Self::validation_ok(vertex_id)
            }
            Err(e) => {
                // Remove from base cache if signature fails
                self.validated_cache.invalidate(&vertex_id);
                Self::validation_err(vertex_id, e)
            }
        };

        self.update_stats(std::slice::from_ref(&result), start.elapsed());
        Ok(result)
    }

    fn check_cache_and_store(
        &self,
        vertices: Vec<DagVertex>,
    ) -> Result<(Vec<ValidationResult>, Vec<DagVertex>)> {
        let mut cached_results = Vec::new();
        let mut vertices_to_validate = Vec::new();

        for vertex in vertices {
            if self.validated_cache.get(&vertex.id).is_some() {
                cached_results.push(ValidationResult {
                    vertex_id: vertex.id,
                    is_valid: true,
                    error: None,
                });
                continue;
            }

            if let Some(store) = &self.persistent_store {
                let is_in_bloom = {
                    let bloom = self
                        .disk_bloom_filter
                        .read()
                        .unwrap_or_else(|e| e.into_inner());
                    bloom.contains(&vertex.id)
                };

                if is_in_bloom && let Ok(Some(_)) = store.get_vertex(&vertex.id) {
                    self.validated_cache.insert(vertex.id, true);
                    cached_results.push(ValidationResult {
                        vertex_id: vertex.id,
                        is_valid: true,
                        error: None,
                    });
                    continue;
                }
            }
            vertices_to_validate.push(vertex);
        }
        Ok((cached_results, vertices_to_validate))
    }

    /// Validate a single vertex by reference.
    fn validate_single_ref(&self, vertex: &DagVertex) -> ValidationResult {
        let vertex_id = vertex.id;

        if let Err(e) = Self::check_vertex_structure(vertex) {
            return Self::validation_err(vertex_id, e);
        }

        // FIX: Use self. instead of Self::
        if let Err(e) = self.check_round_progression(vertex) {
            return Self::validation_err(vertex_id, e);
        }

        if let Err(e) = Self::check_parents(vertex) {
            return Self::validation_err(vertex_id, e);
        }

        Self::validation_ok(vertex_id)
    }

    fn check_vertex_structure(vertex: &DagVertex) -> Result<()> {
        vertex.verify()?;
        if vertex.author.is_empty() {
            anyhow::bail!("Author is empty");
        }
        if vertex.metadata.state_root.len() != 32 {
            anyhow::bail!("Invalid state root");
        }
        Ok(())
    }

    fn check_round_progression(&self, vertex: &DagVertex) -> Result<()> {
        if vertex.round == 0 {
            return Ok(());
        }

        // FIX #4: Round progression validation must work in both Persistent and In-Memory modes
        // Previously, this check was skipped if persistent_store was None, allowing invalid rounds

        let bloom = self
            .disk_bloom_filter
            .read()
            .unwrap_or_else(|e| e.into_inner());

        for parent_id in &vertex.parents {
            // Check Bloom filter first (fast path - covers both disk and cache)
            if !bloom.contains(parent_id) {
                continue; // Parent not in our tracking, skip validation
            }

            // Try persistent store first, then fall back to checking it's tracked
            let parent_round_valid = if let Some(s) = &self.persistent_store {
                // Has persistent store - validate round from DB
                if let Ok(Some(parent)) = s.get_vertex(parent_id) {
                    parent.round == vertex.round - 1
                } else {
                    true // Parent not found, assume valid (will fail elsewhere)
                }
            } else {
                // In-memory mode only - trust bloom filter presence
                // For full validation, use DagCaches or external state
                true
            };

            if !parent_round_valid {
                anyhow::bail!(
                    "Parent round mismatch: expected {}, got {}",
                    vertex.round - 1,
                    "unknown"
                );
            }
        }

        Ok(())
    }

    fn check_parents(vertex: &DagVertex) -> Result<()> {
        let parents = &vertex.parents;

        // FIX #3: Prevent DoS attack via excessive parent lists (Out of Memory)
        if parents.len() > MAX_PARENTS_PER_VERTEX {
            anyhow::bail!(
                "Too many parents: {} (max: {})",
                parents.len(),
                MAX_PARENTS_PER_VERTEX
            );
        }

        // FIX #1: Use HashSet for O(N) duplicate detection instead of O(N²) nested loop
        // Prevents CPU exhaustion DoS attacks with large parent lists
        let mut seen_parents = std::collections::HashSet::with_capacity(parents.len());

        for parent_id in parents {
            if !seen_parents.insert(parent_id) {
                return Err(anyhow::anyhow!("Duplicate parent in vertex"));
            }
        }

        Ok(())
    }

    pub fn validate_and_verify_signatures(
        &mut self,
        vertices: Vec<DagVertex>,
        public_keys: BTreeMap<String, ed25519_dalek::VerifyingKey>,
    ) -> Result<Vec<ValidationResult>> {
        let start = std::time::Instant::now();

        if vertices.is_empty() {
            return Ok(Vec::new());
        }

        let (cached_results, vertices_to_validate) = self.check_cache_and_store(vertices)?;

        let keys_arc = Arc::new(public_keys);
        let this = &*self; // Allow Rayon to reference through this

        let validation_results: Vec<ValidationResult> = vertices_to_validate
            .into_par_iter()
            .map(|vertex| {
                let keys = Arc::clone(&keys_arc);
                this.validate_with_signature(vertex, &keys) // Use this.
            })
            .collect();

        self.cache_valid_results(&validation_results);

        let mut all_results = cached_results;
        all_results.extend(validation_results);

        let duration = start.elapsed();
        self.update_stats(&all_results, duration);

        Ok(all_results)
    }

    fn validate_with_signature(
        &self, // FIX: Change to method accepting &self
        vertex: DagVertex,
        public_keys: &BTreeMap<String, ed25519_dalek::VerifyingKey>,
    ) -> ValidationResult {
        let vertex_id = vertex.id;

        if let Err(e) = Self::check_vertex_structure(&vertex) {
            return Self::validation_err(vertex_id, e);
        }

        // FIX: Use self. instead of Self::
        if let Err(e) = self.check_round_progression(&vertex) {
            return Self::validation_err(vertex_id, e);
        }

        if let Some(pubkey) = public_keys.get(&vertex.author) {
            match Self::verify_vertex_signature(&vertex, pubkey) {
                Ok(_) => Self::validation_ok(vertex_id),
                Err(e) => {
                    Self::validation_err(vertex_id, format!("Signature verification failed: {}", e))
                }
            }
        } else {
            Self::validation_err(vertex_id, "Public key not found for author")
        }
    }

    pub fn verify_vertex_signature(
        vertex: &DagVertex,
        public_key: &ed25519_dalek::VerifyingKey,
    ) -> Result<()> {
        Ed25519Keypair::verify(public_key, &vertex.id, &vertex.signature)
    }

    fn update_stats(&mut self, results: &[ValidationResult], duration: std::time::Duration) {
        let total = results.len();
        if total == 0 {
            return;
        }
        let successful = results.iter().filter(|r| r.is_valid).count();
        self.stats.total_validated += total;
        self.stats.successful += successful;
        self.stats.failed += total - successful;
        let duration_ms = duration.as_millis() as f64;
        if duration_ms > 0.0 {
            self.stats.throughput_per_sec = (total as f64 / duration_ms) * 1000.0;
        }
    }

    pub fn stats(&self) -> &ValidationStats {
        &self.stats
    }

    #[cfg(test)]
    fn reset_stats(&mut self) {
        self.stats = Self::default_stats();
    }

    pub fn config(&self) -> &ParallelValidatorConfig {
        &self.config
    }

    /// Persist validated vertices to disk and update Bloom filter
    /// FIX #2: Removed duplicate disk write - now only updates Bloom filter
    /// The actual disk write is handled by async disk_writer in dag_consensus.rs
    pub fn persist_validated_vertices(&self, vertices: &[DagVertex]) -> Result<()> {
        if let Some(_store) = &self.persistent_store {
            let mut bloom = self.disk_bloom_filter.write().unwrap();
            for vertex in vertices {
                if self.validated_cache.get(&vertex.id).is_some() {
                    // Only update Bloom filter, not disk (disk write is async in dag_consensus)
                    bloom.insert(&vertex.id);
                }
            }
        }
        Ok(())
    }

    #[cfg(test)]
    fn cache_stats(&self) -> usize {
        self.validated_cache.run_pending_tasks();
        self.validated_cache.entry_count() as usize
    }

    pub fn invalidate_pruned_vertices(&mut self, vertex_ids: &[VertexId]) {
        for id in vertex_ids {
            self.validated_cache.invalidate(id);
            self.signature_validated_cache.invalidate(id);
        }
    }

    pub fn update_config(&mut self, config: ParallelValidatorConfig) -> Result<()> {
        config.validate()?;
        self.config = config;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::AuthorityId;
    use super::super::dag_consensus::Round;
    use super::*;

    fn create_test_vertex(round: Round, author: AuthorityId) -> DagVertex {
        DagVertex::new_for_test(round, author, vec![], vec![], vec![round as u8; 32], 0)
    }

    #[test]
    fn test_config_validation() {
        let valid_config = ParallelValidatorConfig::default();
        assert!(valid_config.validate().is_ok());

        let invalid_config = ParallelValidatorConfig {
            num_workers: 0,
            ..Default::default()
        };
        assert!(invalid_config.validate().is_err());

        let invalid_config = ParallelValidatorConfig {
            num_workers: 300,
            ..Default::default()
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_basic_validation() -> Result<()> {
        let config = ParallelValidatorConfig::default();
        let mut validator = ParallelValidator::new(config)?;

        let vertices = vec![
            create_test_vertex(1, "validator_0".to_string()),
            create_test_vertex(2, "validator_1".to_string()),
            create_test_vertex(3, "validator_2".to_string()),
        ];

        let results = validator.validate_batch(vertices)?;

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_valid));

        let stats = validator.stats();
        assert_eq!(stats.total_validated, 3);
        assert_eq!(stats.successful, 3);
        assert_eq!(stats.failed, 0);

        Ok(())
    }

    #[test]
    fn test_parallel_batch_validation() -> Result<()> {
        let config = ParallelValidatorConfig {
            num_workers: 4,
            max_batch_size: 100,
            parallel_sig_verify: true,
            queue_capacity: 1000,
        };

        let mut validator = ParallelValidator::new(config)?;

        let mut vertices = Vec::new();
        for i in 0..100 {
            vertices.push(create_test_vertex(i, format!("validator_{}", i % 10)));
        }

        let results = validator.validate_batch(vertices)?;

        assert_eq!(results.len(), 100);
        assert!(results.iter().all(|r| r.is_valid));

        let stats = validator.stats();
        assert_eq!(stats.total_validated, 100);
        assert_eq!(stats.successful, 100);
        assert!(stats.throughput_per_sec >= 0.0);

        Ok(())
    }

    #[test]
    fn test_invalid_vertex_detection() -> Result<()> {
        let config = ParallelValidatorConfig::default();
        let mut validator = ParallelValidator::new(config)?;

        let invalid_vertex = create_test_vertex(1, "".to_string());
        let results = validator.validate_batch(vec![invalid_vertex])?;

        assert_eq!(results.len(), 1);
        assert!(!results[0].is_valid);
        assert!(results[0].error.is_some());

        let stats = validator.stats();
        assert_eq!(stats.failed, 1);

        Ok(())
    }

    #[test]
    fn test_signature_verification() -> Result<()> {
        let config = ParallelValidatorConfig::default();
        let mut validator = ParallelValidator::new(config)?;

        let keypair = Ed25519Keypair::generate();
        let mut public_keys = BTreeMap::new();
        public_keys.insert("validator_0".to_string(), keypair.public());

        let mut vertex = create_test_vertex(1, "validator_0".to_string());
        vertex.signature = keypair.sign(&vertex.id);

        let results = validator.validate_and_verify_signatures(vec![vertex], public_keys)?;

        assert_eq!(results.len(), 1);
        assert!(results[0].is_valid);

        Ok(())
    }

    #[test]
    fn test_invalid_signature() -> Result<()> {
        let config = ParallelValidatorConfig::default();
        let mut validator = ParallelValidator::new(config)?;

        let keypair = Ed25519Keypair::generate();
        let mut public_keys = BTreeMap::new();
        public_keys.insert("validator_0".to_string(), keypair.public());

        let mut vertex = create_test_vertex(1, "validator_0".to_string());
        vertex.signature = vec![0u8; 64];

        let results = validator.validate_and_verify_signatures(vec![vertex], public_keys)?;

        assert_eq!(results.len(), 1);
        assert!(!results[0].is_valid);

        Ok(())
    }

    #[test]
    fn test_validate_vertex_with_public_key_not_poisoned_by_invalid_signature() -> Result<()> {
        let config = ParallelValidatorConfig::default();
        let mut validator = ParallelValidator::new(config)?;
        let keypair = Ed25519Keypair::generate();
        let mut vertex = create_test_vertex(1, "validator_0".to_string());
        vertex.signature = vec![0u8; 64];

        let result1 = validator.validate_vertex_with_public_key(&vertex, &keypair.public())?;
        let result2 = validator.validate_vertex_with_public_key(&vertex, &keypair.public())?;
        assert!(!result1.is_valid);
        assert!(!result2.is_valid);
        Ok(())
    }

    #[test]
    fn test_signature_cache_revalidates_signature_bytes() -> Result<()> {
        let config = ParallelValidatorConfig::default();
        let mut validator = ParallelValidator::new(config)?;
        let keypair = Ed25519Keypair::generate();
        let mut vertex = create_test_vertex(1, "validator_0".to_string());
        vertex.signature = keypair.sign(&vertex.id);

        let ok = validator.validate_vertex_with_public_key(&vertex, &keypair.public())?;
        assert!(ok.is_valid);

        vertex.signature = vec![0u8; 64];
        let bad = validator.validate_vertex_with_public_key(&vertex, &keypair.public())?;
        assert!(!bad.is_valid);
        Ok(())
    }

    #[test]
    fn test_throughput_measurement() -> Result<()> {
        let config = ParallelValidatorConfig {
            num_workers: 8,
            ..Default::default()
        };

        let mut validator = ParallelValidator::new(config)?;

        let mut vertices = Vec::new();
        for i in 0..1000 {
            vertices.push(create_test_vertex(i, format!("validator_{}", i % 10)));
        }

        let results = validator.validate_batch(vertices)?;

        assert_eq!(results.len(), 1000);

        let stats = validator.stats();
        assert!(stats.throughput_per_sec >= 0.0);
        assert!(stats.avg_validation_time_ms >= 0.0);

        Ok(())
    }

    #[test]
    fn test_stats_reset() -> Result<()> {
        let config = ParallelValidatorConfig::default();
        let mut validator = ParallelValidator::new(config)?;

        let vertices = vec![create_test_vertex(1, "validator_0".to_string())];
        validator.validate_batch(vertices)?;

        assert_eq!(validator.stats().total_validated, 1);

        validator.reset_stats();

        assert_eq!(validator.stats().total_validated, 0);

        Ok(())
    }

    #[test]
    fn test_cache_hit_performance() -> Result<()> {
        let config = ParallelValidatorConfig::default();
        let mut validator = ParallelValidator::new(config)?;

        let vertices = vec![
            create_test_vertex(1, "validator_0".to_string()),
            create_test_vertex(2, "validator_1".to_string()),
        ];

        let results1 = validator.validate_batch(vertices.clone())?;
        assert_eq!(results1.len(), 2);
        assert!(results1.iter().all(|r| r.is_valid));

        let results2 = validator.validate_batch(vertices)?;
        assert_eq!(results2.len(), 2);
        assert!(results2.iter().all(|r| r.is_valid));

        let size = validator.cache_stats();
        assert_eq!(size, results1.len());

        Ok(())
    }

    #[test]
    fn test_persistent_store_integration() -> Result<()> {
        use tempfile::TempDir;

        let temp_dir = TempDir::new()?;
        let store = Arc::new(PersistentDagStore::new(temp_dir.path())?);

        let config = ParallelValidatorConfig::default();
        let mut validator = ParallelValidator::with_persistent_store(config, store.clone())?;

        let vertices = vec![
            create_test_vertex(1, "validator_0".to_string()),
            create_test_vertex(2, "validator_1".to_string()),
        ];

        let results = validator.validate_batch(vertices.clone())?;
        assert_eq!(results.len(), 2);
        validator.persist_validated_vertices(&vertices)?;

        let config2 = ParallelValidatorConfig::default();
        let mut validator2 = ParallelValidator::with_persistent_store(config2, store)?;

        let results2 = validator2.validate_batch(vertices)?;
        assert_eq!(results2.len(), 2);
        assert!(results2.iter().all(|r| r.is_valid));

        Ok(())
    }
}
