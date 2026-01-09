use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use super::crypto_signatures::Ed25519Keypair;
use super::dag_consensus::{DagVertex, VertexId};

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
        let num_cpus = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        Self {
            num_workers: num_cpus.min(16), // Cap at 16 threads
            max_batch_size: 100,
            parallel_sig_verify: true,
            queue_capacity: 1000,
        }
    }
}

impl ParallelValidatorConfig {
    pub fn validate(&self) -> Result<()> {
        if self.num_workers == 0 {
            return Err(anyhow::anyhow!("num_workers must be > 0"));
        }
        if self.num_workers > 64 {
            return Err(anyhow::anyhow!("num_workers must be <= 64"));
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
        })
    }

    /// Validate a batch of vertices in parallel
    pub fn validate_batch(&mut self, vertices: Vec<DagVertex>) -> Result<Vec<ValidationResult>> {
        let start = std::time::Instant::now();

        if vertices.is_empty() {
            return Ok(Vec::new());
        }

        // Split vertices into chunks for parallel processing
        let chunk_size = vertices.len().div_ceil(self.config.num_workers);
        let chunks: Vec<Vec<DagVertex>> = vertices
            .chunks(chunk_size.max(1))
            .map(|chunk| chunk.to_vec())
            .collect();

        // Create channel for results
        let (tx, rx): (
            Sender<Vec<ValidationResult>>,
            Receiver<Vec<ValidationResult>>,
        ) = channel();

        // Spawn worker threads
        let mut handles = Vec::new();
        for chunk in chunks {
            let tx_clone = tx.clone();
            let parallel_sig = self.config.parallel_sig_verify;

            let handle = thread::spawn(move || {
                let results = Self::validate_chunk(chunk, parallel_sig);
                let _ = tx_clone.send(results);
            });

            handles.push(handle);
        }

        // Drop original sender so rx knows when all workers are done
        drop(tx);

        // Collect results from all workers
        let mut all_results = Vec::new();
        while let Ok(results) = rx.recv() {
            all_results.extend(results);
        }

        // Wait for all threads to complete
        for handle in handles {
            let _ = handle.join();
        }

        // Update statistics
        let duration = start.elapsed();
        self.update_stats(&all_results, duration);

        Ok(all_results)
    }

    /// Validate a chunk of vertices (runs in worker thread)
    fn validate_chunk(vertices: Vec<DagVertex>, parallel_sig: bool) -> Vec<ValidationResult> {
        vertices
            .into_iter()
            .map(|vertex| Self::validate_single(vertex, parallel_sig))
            .collect()
    }

    /// Validate a single vertex
    fn validate_single(vertex: DagVertex, _parallel_sig: bool) -> ValidationResult {
        let vertex_id = vertex.id.clone();

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

    /// Check parent references
    fn check_parents(vertex: &DagVertex) -> Result<()> {
        // Check for duplicate parents
        let mut seen = std::collections::HashSet::new();
        for parent in &vertex.parents {
            if !seen.insert(parent) {
                return Err(anyhow::anyhow!("Duplicate parent found"));
            }
        }
        Ok(())
    }

    /// Validate vertices and verify signatures in parallel
    pub fn validate_and_verify_signatures(
        &mut self,
        vertices: Vec<DagVertex>,
        public_keys: HashMap<String, ed25519_dalek::VerifyingKey>,
    ) -> Result<Vec<ValidationResult>> {
        let start = std::time::Instant::now();

        if vertices.is_empty() {
            return Ok(Vec::new());
        }

        // Split into chunks
        let chunk_size = vertices.len().div_ceil(self.config.num_workers);
        let chunks: Vec<Vec<DagVertex>> = vertices
            .chunks(chunk_size.max(1))
            .map(|chunk| chunk.to_vec())
            .collect();

        let (tx, rx) = channel();
        let mut handles = Vec::new();

        for chunk in chunks {
            let tx_clone = tx.clone();
            let keys = public_keys.clone();

            let handle = thread::spawn(move || {
                let results: Vec<ValidationResult> = chunk
                    .into_iter()
                    .map(|vertex| Self::validate_with_signature(vertex, &keys))
                    .collect();
                let _ = tx_clone.send(results);
            });

            handles.push(handle);
        }

        drop(tx);

        let mut all_results = Vec::new();
        while let Ok(results) = rx.recv() {
            all_results.extend(results);
        }

        for handle in handles {
            let _ = handle.join();
        }

        let duration = start.elapsed();
        self.update_stats(&all_results, duration);

        Ok(all_results)
    }

    /// Validate vertex and verify signature
    fn validate_with_signature(
        vertex: DagVertex,
        public_keys: &HashMap<String, ed25519_dalek::VerifyingKey>,
    ) -> ValidationResult {
        let vertex_id = vertex.id.clone();

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

        let mut invalid_config = ParallelValidatorConfig::default();
        invalid_config.num_workers = 0;
        assert!(invalid_config.validate().is_err());

        invalid_config = ParallelValidatorConfig::default();
        invalid_config.num_workers = 100;
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
        assert!(stats.throughput_per_sec > 0.0);

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
        let mut public_keys = HashMap::new();
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
        let mut public_keys = HashMap::new();
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
        assert!(stats.throughput_per_sec > 0.0);
        assert!(stats.avg_validation_time_ms > 0.0);
        println!("Throughput: {:.2} vertices/sec", stats.throughput_per_sec);
        println!(
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
}
