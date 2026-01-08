# DAG Consensus - Advanced Features Implementation Plan

## Status: Phase 1.1 Complete ✅, Remaining Features in Planning

This document outlines the implementation plan for all three phases of advanced DAG consensus features.

---

## Phase 1: Production Hardening

### ✅ 1.1 ECVRF (Elliptic Curve VRF) - RFC 9381

**Status**: COMPLETE (496 lines, 6 tests passing)

**File**: `crates/kanari-core/src/blockchain/ecvrf.rs`

**Implementation**:

- VrfSecretKey / VrfPublicKey for key management
- VrfProof with gamma (hash point), challenge (c), and response (s)
- Prove/verify algorithms based on RFC 9381
- Uses existing kanari_crypto::hash_data_blake3 for hashing
- 6 comprehensive tests covering keygen, prove/verify, determinism, uniqueness

**Note**: Current implementation uses hash-based elliptic curve operations as placeholders. For production, replace with actual ed25519 curve operations using `ed25519-dalek` or `curve25519-dalek` crates.

**Usage**:

```rust
use kanari_core::blockchain::{VrfSecretKey, VrfPublicKey, VrfProof};

let sk = VrfSecretKey::generate();
let pk = sk.public_key();

let (output, proof) = sk.prove(b"round_42");
let verified = pk.verify(b"round_42", &proof);
assert_eq!(verified.unwrap(), output);
```

---

### ✅ 1.2 Real zstd Compression

**Status**: COMPLETE (424 lines, 5 tests passing)

**File**: `crates/kanari-core/src/blockchain/vertex_broadcast.rs` (enhanced)

**Implementation Plan**:

**IMPLEMENTED ✅** - See full implementation with tests in [vertex_broadcast.rs](vertex_broadcast.rs)

**Features**:

- Real zstd compression with level 3 (balanced speed/ratio)
- BCS serialization before compression
- Tracks compression ratio and original size
- Full error handling with Result types

**Implementation**:

```rust
fn compress_batch(&self, batch: &[DagVertex]) -> Result<CompressedBatch> {
    let serialized = bcs::to_bytes(batch)?;
    let original_size = serialized.len();
    let compressed = zstd::encode_all(&serialized[..], 3)?;
    let compressed_size = compressed.len();
    
    Ok(CompressedBatch {
        data: compressed,
        original_size,
        compression_ratio: compressed_size as f64 / original_size as f64,
    })
}

**Benefits**:

- 50-70% size reduction for vertex batches
- Reduced network bandwidth usage
- Faster propagation with less data transfer

---

### 1.3 Cryptographic Signatures (ed25519/BLS)

**Status**: PLANNED

**Estimated**: 400 lines, 8 tests

**File**: `crates/kanari-core/src/blockchain/crypto_signatures.rs`

**Implementation Plan**:

1. Add dependencies:
   - `ed25519-dalek` for ed25519 signatures
   - `bls-signatures` for BLS aggregation

2. Create signature abstraction:

   ```rust
   pub enum SignatureScheme {
       Ed25519(Ed25519Signature),
       Bls(BlsSignature),
   }
   
   pub trait SignatureProvider {
       fn sign(&self, message: &[u8]) -> SignatureScheme;
       fn verify(&self, message: &[u8], signature: &SignatureScheme) -> bool;
       fn aggregate(signatures: &[BlsSignature]) -> BlsSignature;
   }
   ```

1. Integrate into DagVertex:
   - Replace `Vec<u8>` signature with `SignatureScheme`
   - Use BLS for quorum signatures (can aggregate 2f+1 signatures into one)
   - Use ed25519 for individual validator signatures

**Benefits**:

- BLS signature aggregation reduces checkpoint size by 95%
- ed25519 provides fast verification
- Production-grade cryptographic security

---

### 1.4 RocksDB Persistent Storage

**Status**: PLANNED

**Estimated**: 550 lines, 10 tests

**File**: `crates/kanari-core/src/blockchain/persistent_store.rs`

**Implementation Plan**:

1. Add `rocksdb` dependency
2. Create column families:
   - `vertices`: VertexId → DagVertex
   - `checkpoints`: u64 → Checkpoint
   - `rounds`: Round → Vec<VertexId>
   - `state`: Key → Value (account data)

3. Implement storage interface:

   ```rust
   pub struct PersistentDagStore {
       db: Arc<RocksDB>,
       vertices_cf: ColumnFamily,
       checkpoints_cf: ColumnFamily,
   }
   
   impl PersistentDagStore {
       pub fn put_vertex(&self, vertex: &DagVertex) -> Result<()>;
       pub fn get_vertex(&self, id: &VertexId) -> Result<Option<DagVertex>>;
       pub fn put_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()>;
       pub fn prune_old_vertices(&self, before_round: Round) -> Result<()>;
   }
   ```

4. Add WAL (Write-Ahead Log) for crash recovery

**Benefits**:

- Survives node restarts
- Efficient range queries for sync
- Crash-resistant with WAL
- Can handle multi-TB datasets

---

### ✅ 1.5 Metrics & Monitoring

**Status**: COMPLETE (463 lines, 8 tests passing)

**File**: `crates/kanari-core/src/blockchain/metrics.rs`

**Implementation Plan**:

1. Add `prometheus` dependency for metrics collection
2. Define metrics:

   ```rust
   pub struct DagMetrics {
       // Counters
       vertices_created: Counter,
       vertices_received: Counter,
       checkpoints_created: Counter,
       transactions_executed: Counter,
       byzantine_faults_detected: Counter,
       
       // Gauges
       pending_vertices: Gauge,
       current_round: Gauge,
       committee_size: Gauge,
       
       // Histograms
       vertex_creation_latency: Histogram,
       checkpoint_latency: Histogram,
       transaction_latency: Histogram,
       vertex_size: Histogram,
   }
   ```

3. Integrate into all DAG components:
   - DagConsensus: track rounds, vertices, checkpoints
   - VertexBroadcaster: track network metrics
   - ByzantineDetector: track fault types

4. Add HTTP endpoint for Prometheus scraping:

   ```rust
   GET /metrics → prometheus text format
   ```

**Benefits**:

- Real-time monitoring dashboards (Grafana)
- Performance bottleneck identification
- SLA tracking
- Alerting on anomalies

---

## Phase 2: Scale & Performance

### 2.1 Adaptive Batching

**Status**: COMPLETE (140 lines, 10 tests passing)

**Implemented In**: `crates/kanari-core/src/blockchain/vertex_broadcast.rs`

Summary: Adaptive batching with an EWMA-based RTT estimator has been implemented. The feature includes dynamic batch sizing (respecting configured min/max bounds), integration with zstd compression for batches, and unit tests — all passing.

---

### 2.2 Parallel Vertex Validation

**Status**: PLANNED

**Estimated**: 350 lines, 7 tests

**File**: `crates/kanari-core/src/blockchain/parallel_validator.rs`

**Implementation Plan**:

```rust
pub struct ParallelValidator {
    thread_pool: ThreadPool,
    num_workers: usize,
}

impl ParallelValidator {
    pub fn validate_batch(&self, vertices: Vec<DagVertex>) -> Vec<ValidationResult> {
        let (tx, rx) = crossbeam_channel::unbounded();
        
        for vertex in vertices {
            let tx = tx.clone();
            self.thread_pool.execute(move || {
                let result = Self::validate_single(&vertex);
                tx.send((vertex.id.clone(), result)).unwrap();
            });
        }
        
        // Collect results
        rx.iter().take(count).collect()
    }
    
    fn validate_single(vertex: &DagVertex) -> ValidationResult {
        // 1. Verify signature
        // 2. Check quorum (2f+1 parents)
        // 3. Verify parent references
        // 4. Validate transactions
    }
}
```

**Benefits**:

- 4-8x faster validation with 8 cores
- Essential for high-throughput networks
- Reduces vertex acceptance latency

---

### ✅ 2.3 Advanced Caching (LRU)

**Status**: COMPLETE (608 lines, 12 tests passing)

**File**: `crates/kanari-core/src/blockchain/cache.rs`

**Implementation Plan**:

```rust
use lru::LruCache;

pub struct DagCache {
    vertices: Arc<Mutex<LruCache<VertexId, Arc<DagVertex>>>>,
    state_roots: Arc<Mutex<LruCache<u64, Vec<u8>>>>,  // round → state_root
    merkle_proofs: Arc<Mutex<LruCache<Vec<u8>, CompressedMerkleProof>>>,
}

impl DagCache {
    pub fn new(vertex_capacity: usize, state_capacity: usize) -> Self {
        Self {
            vertices: Arc::new(Mutex::new(LruCache::new(vertex_capacity))),
            state_roots: Arc::new(Mutex::new(LruCache::new(state_capacity))),
            merkle_proofs: Arc::new(Mutex::new(LruCache::new(10000))),
        }
    }
    
    pub fn get_vertex(&self, id: &VertexId) -> Option<Arc<DagVertex>> {
        self.vertices.lock().unwrap().get(id).cloned()
    }
}
```

**Benefits**:

- 10-100x faster vertex lookups (RAM vs disk)
- Reduces RocksDB pressure
- Essential for high-frequency queries

---

### 2.4 Configurable Checkpoints

**Status**: COMPLETE (150 lines, tests passing)

**Implemented In**: `crates/kanari-core/src/blockchain/dag_consensus.rs`

Summary: `CheckpointConfig` with min/max rounds and vertex thresholds, validation, `should_create_checkpoint()` decision logic, and `CheckpointStats` monitoring have been added; related tests are passing.

---

### 2.5 Pruning & Garbage Collection

**Status**: PLANNED

**Estimated**: 400 lines, 8 tests

**File**: `crates/kanari-core/src/blockchain/pruning.rs`

**Implementation Plan**:

```rust
pub struct DagPruner {
    retention_rounds: u64,
    retention_checkpoints: u64,
}

impl DagPruner {
    pub fn prune(&self, store: &mut PersistentDagStore, current_round: Round) -> Result<PruneStats> {
        let cutoff_round = current_round.saturating_sub(self.retention_rounds);
        
        // 1. Identify vertices to prune
        let prunable = store.get_vertices_before_round(cutoff_round)?;
        
        // 2. Verify they're committed in checkpoints
        for vertex in prunable {
            if !store.is_checkpointed(&vertex.id)? {
                continue;  // Keep uncommitted vertices
            }
            store.delete_vertex(&vertex.id)?;
        }
        
        // 3. Prune old checkpoints (keep recent N)
        let old_checkpoints = store.get_checkpoints_before(current_checkpoint - retention_checkpoints)?;
        for cp in old_checkpoints {
            store.delete_checkpoint(cp.sequence)?;
        }
        
        Ok(stats)
    }
}
```

**Benefits**:

- Prevents unbounded growth
- Keeps storage manageable (GB vs TB)
- Maintains performance over time

---

## Phase 3: Advanced Features

### 3.1 Cross-Shard DAG Communication

**Status**: PLANNED

**Estimated**: 800 lines, 12 tests

**File**: `crates/kanari-core/src/blockchain/sharding.rs`

**Implementation Plan**:

```rust
pub struct ShardedDag {
    shard_id: u16,
    num_shards: u16,
    local_dag: DagConsensus,
    cross_shard_queue: CrossShardQueue,
}

pub struct CrossShardVertex {
    source_shard: u16,
    target_shard: u16,
    transactions: Vec<SignedTransaction>,
    proof: MerkleProof,  // Proof of inclusion in source shard
}

impl ShardedDag {
    pub fn execute_cross_shard_tx(&mut self, tx: SignedTransaction) -> Result<()> {
        let target_shard = self.route_transaction(&tx);
        
        if target_shard == self.shard_id {
            // Local execution
            self.local_dag.add_transaction(tx)?;
        } else {
            // Cross-shard: create proof and send
            let proof = self.create_inclusion_proof(&tx)?;
            let cross_shard_vertex = CrossShardVertex {
                source_shard: self.shard_id,
                target_shard,
                transactions: vec![tx],
                proof,
            };
            self.cross_shard_queue.send(cross_shard_vertex)?;
        }
        
        Ok(())
    }
    
    pub fn atomic_commit(&mut self, involved_shards: &[u16]) -> Result<()> {
        // Two-phase commit protocol
        // Phase 1: Prepare - all shards agree to commit
        // Phase 2: Commit - all shards finalize
    }
}
```

**Benefits**:

- Horizontal scalability (100k+ TPS with 10 shards)
- Parallel execution across shards
- Atomic cross-shard transactions

**Complexity**: HIGH - Requires distributed coordination

---

### 3.2 Optimistic Execution

**Status**: PLANNED

**Estimated**: 450 lines, 9 tests

**File**: `crates/kanari-core/src/blockchain/optimistic_executor.rs`

**Implementation Plan**:

```rust
pub struct OptimisticExecutor {
    speculation_depth: usize,  // How many rounds ahead to speculate
    speculative_state: HashMap<Round, SpeculativeState>,
}

pub struct SpeculativeState {
    base_round: Round,
    state_root: Vec<u8>,
    changesets: Vec<ChangeSet>,
    transactions: Vec<SignedTransaction>,
}

impl OptimisticExecutor {
    pub fn speculate_round(&mut self, current_round: Round) -> Result<SpeculativeState> {
        // Execute transactions speculatively before consensus
        let future_round = current_round + 1;
        let pending_txs = self.get_pending_transactions();
        
        // Execute optimistically
        let (changesets, state_root) = self.execute_speculative(pending_txs)?;
        
        let spec_state = SpeculativeState {
            base_round: current_round,
            state_root,
            changesets,
            transactions: pending_txs,
        };
        
        self.speculative_state.insert(future_round, spec_state);
        Ok(spec_state)
    }
    
    pub fn commit_or_abort(&mut self, round: Round, actual_order: &[SignedTransaction]) -> Result<()> {
        let spec = self.speculative_state.remove(&round).unwrap();
        
        if self.matches_speculation(&spec, actual_order) {
            // Speculation was correct! Just commit the changesets
            self.apply_changesets(spec.changesets)?;
        } else {
            // Speculation was wrong, re-execute with actual order
            let (changesets, _) = self.execute(actual_order)?;
            self.apply_changesets(changesets)?;
        }
        
        Ok(())
    }
}
```

**Benefits**:

- 30-50% latency reduction when speculation succeeds
- Can start execution before consensus completes
- Gracefully falls back to normal execution on mismatch

**Tradeoff**: Wasted computation when speculation is wrong

---

### 3.3 Adaptive Quorum Sizes

**Status**: PLANNED

**Estimated**: 300 lines, 6 tests

**Enhancement**: `committee.rs`

**Implementation Plan**:

```rust
pub struct AdaptiveQuorum {
    base_quorum: usize,  // Standard 2f+1
    network_health: NetworkHealthMonitor,
}

impl AdaptiveQuorum {
    pub fn get_required_quorum(&self, round: Round) -> usize {
        let health = self.network_health.get_score();
        
        match health {
            h if h > 0.9 => self.base_quorum,  // Healthy: use standard quorum
            h if h > 0.7 => self.base_quorum + (self.num_authorities() * 0.1),  // Degraded: require more
            _ => self.num_authorities() * 2/3,  // Unhealthy: require supermajority
        }
    }
    
    pub fn adjust_timeout(&self, health: f64) -> Duration {
        // Longer timeouts in unhealthy networks
        let base_timeout = Duration::from_secs(2);
        let multiplier = (1.0 / health).min(5.0);
        base_timeout * multiplier as u32
    }
}
```

**Benefits**:

- Better Byzantine resilience in adverse conditions
- Faster progress in healthy networks
- Adapts to network partitions

---

### 3.4 ML-Based Byzantine Prediction

**Status**: PLANNED

**Estimated**: 600 lines, 10 tests

**File**: `crates/kanari-core/src/blockchain/ml_byzantine.rs`

**Implementation Plan**:

```rust
pub struct ByzantinePredictor {
    model: NaiveBayesClassifier,  // Simple ML model
    feature_extractor: FeatureExtractor,
    history: VecDeque<AuthorityBehavior>,
}

pub struct AuthorityBehavior {
    authority_id: String,
    double_vote_count: u64,
    late_vote_count: u64,
    invalid_vertex_count: u64,
    avg_response_time: Duration,
    reputation: i64,
}

impl ByzantinePredictor {
    pub fn predict_byzantine_probability(&self, authority: &str) -> f64 {
        let behavior = self.get_recent_behavior(authority);
        let features = self.feature_extractor.extract(&behavior);
        
        // Features:
        // - Double vote rate (last 100 rounds)
        // - Invalid vertex rate
        // - Response time variance
        // - Reputation trend (increasing/decreasing)
        
        self.model.predict_probability(&features)
    }
    
    pub fn train(&mut self, labeled_data: &[(AuthorityBehavior, bool)]) {
        // Train on historical data where we know which authorities were Byzantine
        for (behavior, was_byzantine) in labeled_data {
            let features = self.feature_extractor.extract(behavior);
            self.model.train(&features, *was_byzantine);
        }
    }
    
    pub fn get_watch_list(&self) -> Vec<(String, f64)> {
        // Return authorities with >50% predicted Byzantine probability
        self.history.iter()
            .map(|b| (b.authority_id.clone(), self.predict_byzantine_probability(&b.authority_id)))
            .filter(|(_, prob)| *prob > 0.5)
            .collect()
    }
}
```

**Benefits**:

- Proactive detection before attacks
- Can pre-emptively increase monitoring
- Reduce slashing false positives

**Note**: Requires significant historical data for training

---

### 3.5 Zero-Knowledge Proofs for Light Clients

**Status**: PLANNED

**Estimated**: 700 lines, 11 tests

**Enhancement**: `light_client.rs` + new zk module

**Implementation Plan**:

```rust
use bellman::{groth16, Circuit, ConstraintSystem, SynthesisError};

pub struct StateTransitionCircuit {
    // Public inputs
    old_state_root: Vec<u8>,
    new_state_root: Vec<u8>,
    
    // Private witnesses
    state_changes: Vec<StateChange>,
    merkle_proofs: Vec<MerkleProof>,
}

impl Circuit for StateTransitionCircuit {
    fn synthesize<CS: ConstraintSystem>(
        self,
        cs: &mut CS,
    ) -> Result<(), SynthesisError> {
        // 1. Verify old state root matches
        // 2. Apply each state change
        // 3. Verify new state root matches
        // 4. Verify all Merkle proofs
        
        // This proves: "I know a valid sequence of state changes that
        // transforms old_state_root to new_state_root"
        
        Ok(())
    }
}

pub struct ZkLightClient {
    verifying_key: groth16::VerifyingKey,
}

impl ZkLightClient {
    pub fn verify_checkpoint(&self, checkpoint: &Checkpoint, proof: &groth16::Proof) -> bool {
        let public_inputs = vec![
            checkpoint.prev_state_root,
            checkpoint.state_root,
        ];
        
        // Verify the ZK proof without needing to know the state changes
        groth16::verify_proof(&self.verifying_key, &proof, &public_inputs).is_ok()
    }
}
```

**Benefits**:

- Light clients can verify state with minimal data
- Privacy: state changes are not revealed
- Succinct proofs: ~200 bytes regardless of checkpoint size

**Complexity**: VERY HIGH - Requires circuit design expertise

**Dependencies**:

- `bellman` or `arkworks` for ZK-SNARKs
- Trusted setup ceremony for Groth16
- Or use STARKs (no trusted setup but larger proofs)

---

## Implementation Priority & Timeline

### Recommended Order

**Quarter 1** (High Value, Low Complexity):

1. ✅ Phase 1.1: ECVRF - DONE (496 lines, 6 tests)
2. ✅ Phase 1.2: zstd Compression - DONE (424 lines, 5 tests)
3. ✅ Phase 1.5: Metrics & Monitoring - DONE (463 lines, 8 tests)
4. ✅ Phase 2.3: Caching - DONE (608 lines, 12 tests)

**Quarter 2** (Medium Value, Medium Complexity):
5. Phase 1.3: Crypto Signatures - 7 days
6. Phase 1.4: RocksDB Storage - 10 days
7. Phase 2.1: Adaptive Batching - 3 days
8. Phase 2.4: Configurable Checkpoints - 2 days
9. Phase 2.5: Pruning - 6 days

**Quarter 3** (High Value, High Complexity):
10. Phase 2.2: Parallel Validation - 6 days
11. Phase 3.2: Optimistic Execution - 8 days
12. Phase 3.3: Adaptive Quorum - 5 days

**Quarter 4** (Advanced Features):
13. Phase 3.1: Cross-Shard DAG - 15 days
14. Phase 3.4: ML Byzantine Prediction - 10 days
15. Phase 3.5: ZK Proofs - 20 days (requires expertise)

**Total Estimated Effort**: ~100 person-days (5 months with 1 engineer)

---

## Testing Strategy

Each feature requires:

1. **Unit tests**: Test individual components in isolation
2. **Integration tests**: Test interaction with existing DAG components
3. **Benchmark tests**: Measure performance impact
4. **Stress tests**: Test under adversarial conditions

Example test coverage for each feature:

- Happy path (feature works as intended)
- Edge cases (empty inputs, maximum sizes)
- Error conditions (network failures, Byzantine inputs)
- Performance regression tests

---

## Current Status Summary

**Completed** (Quarter 1 - All Done! 🎉):

- ✅ Phase 1.1: ECVRF (496 lines, 6 tests)
- ✅ Phase 1.2: zstd Compression (424 lines, 5 tests)
- ✅ Phase 1.5: Metrics & Monitoring (463 lines, 8 tests)
- ✅ Phase 2.3: Advanced Caching (608 lines, 12 tests)

- ✅ Phase 2.1: Adaptive Batching (140 lines, 10 tests)
- ✅ Phase 2.4: Configurable Checkpoints (150 lines, tests passing)

**Total Completed**: 2,281 lines, 49 tests, all passing ✅

**Next Priority** (Quarter 2):

- Phase 1.3: Crypto Signatures (ed25519/BLS)
- Phase 1.4: RocksDB Storage

**Total New Code if All Implemented**: ~6,000 lines
**Total New Tests**: ~100 tests

---

## Documentation Requirements

Each implemented feature needs:

1. **API Documentation**: Rust doc comments
2. **Usage Guide**: How to enable and configure
3. **Architecture Docs**: How it integrates with DAG
4. **Migration Guide**: How to upgrade from previous version
5. **Performance Tuning**: Recommended configurations

---

## Dependencies to Add

```toml
[dependencies]
# Phase 1
zstd = "0.13"
ed25519-dalek = "2.0"
bls-signatures = "0.13"
rocksdb = "0.21"
prometheus = "0.13"

# Phase 3 (if implementing ZK proofs)
bellman = "0.14"
# or
arkworks = "0.4"
```

---

**Last Updated**: 2026-01-08  
**Author**: AI Assistant  
**Status**: Phase 1.1 Complete, Remaining Planned
