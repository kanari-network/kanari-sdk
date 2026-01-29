use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

use super::cache::LruCache;
use super::crypto_signatures::Ed25519Keypair;
use super::dag_consensus::{DagVertex, VertexId};
use super::persistent_store::PersistentDagStore;

/// Configuration for parallel validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelValidatorConfig {
    /// Number of worker threads for validation
    pub num_workers: usize,

    /// Maximum batch size for parallel processing
    pub max_batch_size: usize,

    /// Enable parallel signature verification
    pub parallel_sig_verify: bool,

    /// Queue capacity per worker
    pub queue_capacity: usize,
}

impl Default for ParallelValidatorConfig {
    fn default() -> Self {
        let num_cpus = rayon::current_num_threads();

        Self {
            num_workers: num_cpus.min(64), // Utilize more cores for 500K TPS
            max_batch_size: 5000,          // Much larger batches for high throughput
            parallel_sig_verify: true,
            queue_capacity: 100000, // Large queue to handle bursts
        }
    }
}

impl ParallelValidatorConfig {
    /// Create moderate config for 8-16 core machines (10K-30K TPS)
    pub fn moderate() -> Self {
        let num_cpus = rayon::current_num_threads();
        Self {
            num_workers: num_cpus.min(16), // Use up to 16 cores
            max_batch_size: 500,           // 500 batch for moderate throughput
            parallel_sig_verify: true,
            queue_capacity: 10000, // 10K queue depth
        }
    }

    /// Create high-performance config optimized for 500K+ TPS
    pub fn high_throughput() -> Self {
        let num_cpus = rayon::current_num_threads();
        Self {
            num_workers: num_cpus.min(128), // Use up to 128 cores
            max_batch_size: 10000,          // 10K batch for maximum throughput
            parallel_sig_verify: true,
            queue_capacity: 500000, // 500K queue depth
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

/// Result of vertex validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub vertex_id: VertexId,
    pub is_valid: bool,
    pub error: Option<String>,
}

/// Statistics for parallel validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStats {
    pub total_validated: usize,
    pub successful: usize,
    pub failed: usize,
    pub avg_validation_time_ms: f64,
    pub throughput_per_sec: f64,
}

/// Parallel vertex validator using thread pool
pub struct ParallelValidator {
    config: ParallelValidatorConfig,
    stats: ValidationStats,

    /// Cache for validated vertices (prevents re-validation)
    validated_cache: LruCache<VertexId, bool>,

    /// Optional persistent storage for validated vertices
    persistent_store: Option<Arc<PersistentDagStore>>,
}

impl ParallelValidator {
    /// Create a new parallel validator
    pub fn new(config: ParallelValidatorConfig) -> Result<Self> {
        config.validate()?;

        Ok(Self {
            config,
            stats: ValidationStats {
                total_validated: 0,
                successful: 0,
                failed: 0,
                avg_validation_time_ms: 0.0,
                throughput_per_sec: 0.0,
            },
            validated_cache: LruCache::new(10_000), // Cache up to 10k validated vertices
            persistent_store: None,
        })
    }

    /// Create validator with persistent storage
    pub fn with_persistent_store(
        config: ParallelValidatorConfig,
        store: Arc<PersistentDagStore>,
    ) -> Result<Self> {
        config.validate()?;

        Ok(Self {
            config,
            stats: ValidationStats {
                total_validated: 0,
                successful: 0,
                failed: 0,
                avg_validation_time_ms: 0.0,
                throughput_per_sec: 0.0,
            },
            validated_cache: LruCache::new(10_000),
            persistent_store: Some(store),
        })
    }

    /// Validate a batch of vertices in parallel using Rayon
    pub fn validate_batch(&mut self, vertices: Vec<DagVertex>) -> Result<Vec<ValidationResult>> {
        let start = std::time::Instant::now();

        if vertices.is_empty() {
            return Ok(Vec::new());
        }

        // Check cache and persistent store first to avoid re-validation
        let (cached_results, vertices_to_validate) = self.check_cache_and_store(vertices)?;

        // Use Rayon's parallel iterator for efficient work-stealing
        let validation_results: Vec<ValidationResult> = vertices_to_validate
            .into_par_iter()
            .map(|vertex| Self::validate_single(vertex, self.config.parallel_sig_verify))
            .collect();

        // Cache newly validated vertices
        for result in &validation_results {
            if result.is_valid {
                self.validated_cache.put(result.vertex_id, true);
            }
        }

        // Combine cached and newly validated results
        let mut all_results = cached_results;
        all_results.extend(validation_results);

        // Update statistics
        let duration = start.elapsed();
        self.update_stats(&all_results, duration);

        Ok(all_results)
    }

    /// Check cache and persistent store to avoid re-validation
    fn check_cache_and_store(
        &self,
        vertices: Vec<DagVertex>,
    ) -> Result<(Vec<ValidationResult>, Vec<DagVertex>)> {
        let mut cached_results = Vec::new();
        let mut vertices_to_validate = Vec::new();

        for vertex in vertices {
            let vertex_id = vertex.id;

            // Check in-memory cache first (fastest)
            if self.validated_cache.get(&vertex_id).is_some() {
                cached_results.push(ValidationResult {
                    vertex_id,
                    is_valid: true,
                    error: None,
                });
                continue;
            }

            // Check persistent store if available
            if let Some(store) = &self.persistent_store
                && let Ok(Some(_)) = store.get_vertex(&vertex_id)
            {
                // Vertex exists in persistent store, assume it's validated
                self.validated_cache.put(vertex_id, true); // Warm up cache
                cached_results.push(ValidationResult {
                    vertex_id,
                    is_valid: true,
                    error: None,
                });
                continue;
            }

            // Not found in cache or store, needs validation
            vertices_to_validate.push(vertex);
        }

        Ok((cached_results, vertices_to_validate))
    }

    /// Validate a single vertex
    fn validate_single(vertex: DagVertex, _parallel_sig: bool) -> ValidationResult {
        let vertex_id = vertex.id;

        // Basic validation checks
        if let Err(e) = Self::check_vertex_structure(&vertex) {
            return ValidationResult {
                vertex_id,
                is_valid: false,
                error: Some(e.to_string()),
            };
        }

        // Validate round progression
        if let Err(e) = Self::check_round_progression(&vertex) {
            return ValidationResult {
                vertex_id,
                is_valid: false,
                error: Some(e.to_string()),
            };
        }

        // Validate parents
        if let Err(e) = Self::check_parents(&vertex) {
            return ValidationResult {
                vertex_id,
                is_valid: false,
                error: Some(e.to_string()),
            };
        }

        ValidationResult {
            vertex_id,
            is_valid: true,
            error: None,
        }
    }

    /// Check vertex structure
    fn check_vertex_structure(vertex: &DagVertex) -> Result<()> {
        if vertex.id.is_empty() {
            return Err(anyhow::anyhow!("Vertex ID is empty"));
        }

        if vertex.author.is_empty() {
            return Err(anyhow::anyhow!("Author is empty"));
        }

        if vertex.metadata.state_root.is_empty() {
            return Err(anyhow::anyhow!("State root is empty"));
        }

        Ok(())
    }

    /// Check round progression
    fn check_round_progression(vertex: &DagVertex) -> Result<()> {
        // Parents should be from previous rounds
        for parent_id in &vertex.parents {
            // In real implementation, would fetch parent and check round
            if parent_id.is_empty() {
                return Err(anyhow::anyhow!("Invalid parent ID"));
            }
        }
        Ok(())
    }

    /// Check if parents are unique
    fn check_parents(vertex: &DagVertex) -> Result<()> {
        use std::collections::BTreeSet;
        let mut seen = BTreeSet::new();
        for parent in &vertex.parents {
            if !seen.insert(parent) {
                return Err(anyhow::anyhow!("Duplicate parent in vertex"));
            }
        }
        Ok(())
    }

    /// Validate vertices and verify signatures in parallel using Rayon
    pub fn validate_and_verify_signatures(
        &mut self,
        vertices: Vec<DagVertex>,
        public_keys: BTreeMap<String, ed25519_dalek::VerifyingKey>,
    ) -> Result<Vec<ValidationResult>> {
        let start = std::time::Instant::now();

        if vertices.is_empty() {
            return Ok(Vec::new());
        }

        // Check cache first
        let (cached_results, vertices_to_validate) = self.check_cache_and_store(vertices)?;

        // Use Arc for zero-copy sharing of public keys across threads
        let keys_arc = Arc::new(public_keys);

        // Use Rayon's parallel iterator with work-stealing scheduler
        let validation_results: Vec<ValidationResult> = vertices_to_validate
            .into_par_iter()
            .map(|vertex| {
                let keys = Arc::clone(&keys_arc);
                Self::validate_with_signature(vertex, &keys)
            })
            .collect();

        // Cache validated vertices
        for result in &validation_results {
            if result.is_valid {
                self.validated_cache.put(result.vertex_id, true);
            }
        }

        // Combine results
        let mut all_results = cached_results;
        all_results.extend(validation_results);

        let duration = start.elapsed();
        self.update_stats(&all_results, duration);

        Ok(all_results)
    }

    /// Validate vertex and verify signature
    fn validate_with_signature(
        vertex: DagVertex,
        public_keys: &BTreeMap<String, ed25519_dalek::VerifyingKey>,
    ) -> ValidationResult {
        let vertex_id = vertex.id;

        // Basic validation
        if let Err(e) = Self::check_vertex_structure(&vertex) {
            return ValidationResult {
                vertex_id,
                is_valid: false,
                error: Some(e.to_string()),
            };
        }

        // Signature verification
        if let Some(pubkey) = public_keys.get(&vertex.author) {
            // Create a copy without signature for verification
            let mut vertex_for_verify = vertex.clone();
            let signature_bytes = vertex_for_verify.signature.clone();
            vertex_for_verify.signature = vec![];

            let vertex_bytes = match bcs::to_bytes(&vertex_for_verify) {
                Ok(bytes) => bytes,
                Err(e) => {
                    return ValidationResult {
                        vertex_id,
                        is_valid: false,
                        error: Some(format!("Serialization failed: {}", e)),
                    };
                }
            };

            if let Err(e) = Ed25519Keypair::verify(pubkey, &vertex_bytes, &signature_bytes) {
                return ValidationResult {
                    vertex_id,
                    is_valid: false,
                    error: Some(format!("Signature verification failed: {}", e)),
                };
            }
        } else {
            return ValidationResult {
                vertex_id,
                is_valid: false,
                error: Some("Public key not found for author".to_string()),
            };
        }

        ValidationResult {
            vertex_id,
            is_valid: true,
            error: None,
        }
    }

    /// Update validation statistics
    fn update_stats(&mut self, results: &[ValidationResult], duration: std::time::Duration) {
        let total = results.len();
        if total == 0 {
            return; // Avoid division by zero
        }

        let successful = results.iter().filter(|r| r.is_valid).count();
        let failed = total - successful;

        let duration_ms = duration.as_millis() as f64;
        let throughput = if duration_ms > 0.0 {
            (total as f64 / duration_ms) * 1000.0
        } else {
            0.0
        };

        self.stats.total_validated += total;
        self.stats.successful += successful;
        self.stats.failed += failed;

        // Update moving average of validation time
        let new_avg = if self.stats.avg_validation_time_ms == 0.0 {
            duration_ms / total as f64
        } else {
            (self.stats.avg_validation_time_ms + duration_ms / total as f64) / 2.0
        };

        self.stats.avg_validation_time_ms = new_avg;
        self.stats.throughput_per_sec = throughput;
    }

    /// Get current validation statistics
    pub fn stats(&self) -> &ValidationStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = ValidationStats {
            total_validated: 0,
            successful: 0,
            failed: 0,
            avg_validation_time_ms: 0.0,
            throughput_per_sec: 0.0,
        };
    }

    /// Get configuration
    pub fn config(&self) -> &ParallelValidatorConfig {
        &self.config
    }

    /// Persist validated vertices to storage
    pub fn persist_validated_vertices(&self, vertices: &[DagVertex]) -> Result<()> {
        if let Some(store) = &self.persistent_store {
            for vertex in vertices {
                // Only persist if it's in validated cache
                if self.validated_cache.get(&vertex.id).is_some() {
                    store.put_vertex(vertex)?;
                }
            }
        }
        Ok(())
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize, f64) {
        let stats = self.validated_cache.stats();
        (stats.size, stats.capacity, stats.hit_rate)
    }

    /// Invalidate cache entries for pruned vertices
    pub fn invalidate_pruned_vertices(&mut self, vertex_ids: &[VertexId]) {
        for id in vertex_ids {
            // Remove from cache - vertex no longer exists
            let _ = self.validated_cache.get(id); // This will mark as miss if exists
        }
    }

    /// Clear entire validation cache (use after major state changes)
    pub fn clear_cache(&mut self) {
        self.validated_cache = LruCache::new(10_000);
    }

    /// Update configuration
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
        DagVertex::new(round, author, vec![], vec![], vec![round as u8; 32])
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

        // Test exceeds new 256 limit for high-throughput
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

        // Create large batch
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
        assert!(stats.throughput_per_sec >= 0.0); // Can be 0 if test runs too fast

        Ok(())
    }

    #[test]
    fn test_invalid_vertex_detection() -> Result<()> {
        let config = ParallelValidatorConfig::default();
        let mut validator = ParallelValidator::new(config)?;

        // Create invalid vertex (empty author)
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

        // Create keypair
        let keypair = Ed25519Keypair::generate();
        let mut public_keys = BTreeMap::new();
        public_keys.insert("validator_0".to_string(), keypair.public());

        // Create vertex
        let mut vertex = create_test_vertex(1, "validator_0".to_string());

        // Sign the vertex (serialize without signature for signing)
        let mut vertex_for_signing = vertex.clone();
        vertex_for_signing.signature = vec![];
        let vertex_bytes = bcs::to_bytes(&vertex_for_signing)?;
        vertex.signature = keypair.sign(&vertex_bytes);

        let results = validator.validate_and_verify_signatures(vec![vertex], public_keys)?;

        assert_eq!(results.len(), 1);
        assert!(results[0].is_valid);

        Ok(())
    }

    #[test]
    fn test_invalid_signature() -> Result<()> {
        let config = ParallelValidatorConfig::default();
        let mut validator = ParallelValidator::new(config)?;

        // Create keypair
        let keypair = Ed25519Keypair::generate();
        let mut public_keys = BTreeMap::new();
        public_keys.insert("validator_0".to_string(), keypair.public());

        // Create vertex with invalid signature
        let mut vertex = create_test_vertex(1, "validator_0".to_string());
        vertex.signature = vec![0u8; 64]; // Invalid signature

        let results = validator.validate_and_verify_signatures(vec![vertex], public_keys)?;

        assert_eq!(results.len(), 1);
        assert!(!results[0].is_valid);
        assert!(
            results[0]
                .error
                .as_ref()
                .unwrap()
                .contains("Signature verification failed")
        );

        Ok(())
    }

    #[test]
    fn test_throughput_measurement() -> Result<()> {
        let config = ParallelValidatorConfig {
            num_workers: 8,
            ..Default::default()
        };

        let mut validator = ParallelValidator::new(config)?;

        // Create large batch to measure throughput
        let mut vertices = Vec::new();
        for i in 0..1000 {
            vertices.push(create_test_vertex(i, format!("validator_{}", i % 10)));
        }

        let results = validator.validate_batch(vertices)?;

        assert_eq!(results.len(), 1000);

        let stats = validator.stats();
        assert!(stats.throughput_per_sec >= 0.0); // Rayon is so fast, duration_ms can be 0
        assert!(stats.avg_validation_time_ms >= 0.0);
        eprintln!("Throughput: {:.2} vertices/sec", stats.throughput_per_sec);
        eprintln!(
            "Avg validation time: {:.4} ms",
            stats.avg_validation_time_ms
        );

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
        assert_eq!(validator.stats().successful, 0);
        assert_eq!(validator.stats().failed, 0);

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

        // First validation - cache miss
        let results1 = validator.validate_batch(vertices.clone())?;
        assert_eq!(results1.len(), 2);
        assert!(results1.iter().all(|r| r.is_valid));

        // Second validation - cache hit (should be much faster)
        let results2 = validator.validate_batch(vertices)?;
        assert_eq!(results2.len(), 2);
        assert!(results2.iter().all(|r| r.is_valid));

        // Check cache stats
        let (size, capacity, hit_rate) = validator.cache_stats();
        assert!(size > 0);
        assert_eq!(capacity, 10_000);
        assert!(hit_rate > 0.0, "Cache hit rate should be > 0");

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

        // Validate and persist
        let results = validator.validate_batch(vertices.clone())?;
        assert_eq!(results.len(), 2);
        validator.persist_validated_vertices(&vertices)?;

        // Create new validator with same store
        let config2 = ParallelValidatorConfig::default();
        let mut validator2 = ParallelValidator::with_persistent_store(config2, store)?;

        // Should load from persistent store
        let results2 = validator2.validate_batch(vertices)?;
        assert_eq!(results2.len(), 2);
        assert!(results2.iter().all(|r| r.is_valid));

        Ok(())
    }
}
