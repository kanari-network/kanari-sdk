// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Kanari DAG consensus with Mysticeti-inspired commit geometry.
//!
//! This module implements a Directed Acyclic Graph consensus mechanism that separates:
//! - Data Availability (DA): Broadcasting and storing transaction data
//! - Ordering: Determining the total order of transactions
//!
//! The committer uses Mysticeti-style strict quorum, three-round decision depth,
//! and deterministic multi-leader selection while retaining Kanari's local vertex,
//! checkpoint, execution, and networking types.
//!
//! This design enables:
//! - High throughput through parallel block production
//! - Low latency by decoupling DA from consensus
//! - Byzantine fault tolerance
//! - Efficient parallel execution (already supported in Kanari's produce_block.rs)

mod checkpointing;
mod store;
mod vertices;

use super::byzantine_detector::ByzantineDetector;
use super::cache::DagCaches;
use super::committee::{AdaptiveQuorumConfig, Committee, NetworkHealth, ValidatorInfo};
use super::crypto_signatures::Ed25519Keypair;
use super::metrics::DagMetrics;
use super::parallel_validator::{ParallelValidator, ParallelValidatorConfig};
use super::persistent_store::PersistentDagStore;
use super::protocol::{ConsensusProtocol, Protocol};
use super::pruning::{DagPruner, PruningConfig};
use super::state_sync::StateSynchronizer;
use super::vertex_broadcast::VertexBroadcaster;
use anyhow::Result;
use kanari_crypto::hash_data_blake3;
use kanari_types::transaction::SignedTransaction;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::mpsc;

pub use checkpointing::CheckpointStats;

/// Unique identifier for a DAG vertex (block)
/// Fixed-size [u8; 32] for zero heap allocations (500K TPS optimization)
/// Blake3 hash output is always 32 bytes
pub type VertexId = [u8; 32];

/// Round number in the DAG consensus protocol
pub type Round = u64;

/// Authority/validator identifier
pub type AuthorityId = String;

pub const DAG_VERTEX_SIGNATURE_VERSION: u16 = 1;
pub const DAG_VERTEX_SIGNATURE_DOMAIN: &[u8] = b"kanari-dag-vertex";

pub(crate) fn dag_vertex_signature_payload(id: &VertexId) -> Vec<u8> {
    let mut payload = Vec::with_capacity(
        DAG_VERTEX_SIGNATURE_DOMAIN.len() + std::mem::size_of::<u16>() + id.len(),
    );
    payload.extend_from_slice(DAG_VERTEX_SIGNATURE_DOMAIN);
    payload.extend_from_slice(&DAG_VERTEX_SIGNATURE_VERSION.to_le_bytes());
    payload.extend_from_slice(id);
    payload
}

fn vertex_id_from_hash_bytes(bytes: &[u8]) -> VertexId {
    let mut result = [0u8; 32];
    result.copy_from_slice(&bytes[..32]);
    result
}

fn logical_tx_hash(tx: &SignedTransaction) -> Vec<u8> {
    tx.transaction_hash().to_vec()
}

fn timestamp_bounds(parent_timestamps: &[u64]) -> Option<(u64, u64)> {
    const MAX_TIMESTAMP_DRIFT_SECS: u64 = 300;

    if parent_timestamps.is_empty() {
        return None;
    }

    let mut sorted = parent_timestamps.to_vec();
    sorted.sort_unstable();

    let median_timestamp = sorted[sorted.len() / 2];
    let max_parent_timestamp = sorted.last().copied().unwrap_or(median_timestamp);
    let min_allowed = median_timestamp.max(max_parent_timestamp);
    let max_allowed = median_timestamp.saturating_add(MAX_TIMESTAMP_DRIFT_SECS);

    Some(if min_allowed > max_allowed {
        (min_allowed, min_allowed)
    } else {
        (min_allowed, max_allowed)
    })
}

/// DAG Vertex - represents a batch of transactions in the DAG
/// Each vertex can reference multiple parent vertices, forming a DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagVertex {
    /// Unique identifier (hash of the vertex) - fixed [u8; 32]
    pub id: VertexId,

    /// Round number in the consensus protocol
    pub round: Round,

    /// Authority that created this vertex
    pub author: AuthorityId,

    /// Chain ID for cross-chain replay protection
    pub chain_id: String,

    /// References to parent vertices (from previous round)
    /// Requires 2f+1 parents for quorum (where f is max faulty nodes)
    pub parents: Vec<VertexId>,

    /// Transactions included in this vertex
    pub transactions: Vec<SignedTransaction>,

    /// Timestamp when vertex was created
    pub timestamp: u64,

    /// Signature from the authority
    pub signature: Vec<u8>,

    /// Metadata for consensus
    pub metadata: VertexMetadata,

    /// Cached serialized data (500K TPS optimization - avoid repeated serialization)
    #[serde(skip)]
    pub cached_serialized_data: Option<Vec<u8>>,

    /// Cached hash for quick lookups (computed once)
    #[serde(skip)]
    pub cached_hash: Option<Vec<u8>>,
}

/// Metadata for DAG vertex
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexMetadata {
    /// Total number of transactions in this vertex
    pub tx_count: usize,

    /// Gas used by transactions
    pub total_gas_used: u64,

    /// State root after executing transactions
    pub state_root: Vec<u8>,

    /// Whether this vertex is a checkpoint (committed)
    pub is_checkpoint: bool,

    /// Checkpoint sequence number (if committed)
    pub checkpoint_seq: Option<u64>,
}

impl DagVertex {
    fn hash_material(&self) -> Result<Vec<u8>> {
        let tx_hashes: Vec<Vec<u8>> = self.transactions.iter().map(logical_tx_hash).collect();
        let bytes = bcs::to_bytes(&(
            &self.chain_id,
            self.round,
            &self.author,
            &self.parents,
            tx_hashes,
            self.timestamp,
            &self.metadata.state_root,
        ))
        .map_err(|e| anyhow::anyhow!("Failed to serialize vertex for hashing: {}", e))?;

        Ok(bytes)
    }

    /// Create a new DAG vertex (panics on serialization failure - use try_new() for production)
    pub fn new(
        round: Round,
        author: AuthorityId,
        chain_id: String,
        parents: Vec<VertexId>,
        transactions: Vec<SignedTransaction>,
        state_root: Vec<u8>,
        timestamp: u64,
    ) -> Self {
        Self::try_new(
            round,
            author,
            chain_id,
            parents,
            transactions,
            state_root,
            timestamp,
        )
        .expect("DagVertex::new failed - this should never happen with valid inputs")
    }

    /// Try to create a new DAG vertex (returns Result for safe error handling)
    pub fn try_new(
        round: Round,
        author: AuthorityId,
        chain_id: String,
        parents: Vec<VertexId>,
        transactions: Vec<SignedTransaction>,
        state_root: Vec<u8>,
        timestamp: u64,
    ) -> Result<Self> {
        let tx_count = transactions.len();

        let metadata = VertexMetadata {
            tx_count,
            total_gas_used: 0,
            state_root,
            is_checkpoint: false,
            checkpoint_seq: None,
        };

        let mut vertex = Self {
            id: [0u8; 32],
            round,
            author,
            chain_id,
            parents,
            transactions,
            timestamp,
            signature: Vec::new(),
            metadata,
            cached_serialized_data: None,
            cached_hash: None,
        };

        let hash = vertex.compute_hash()?;
        vertex.cached_hash = Some(hash.to_vec());
        vertex.id = hash;
        Ok(vertex)
    }

    /// Create a new DAG vertex with default chain_id (for testing)
    #[cfg(test)]
    pub fn new_for_test(
        round: Round,
        author: AuthorityId,
        parents: Vec<VertexId>,
        transactions: Vec<SignedTransaction>,
        state_root: Vec<u8>,
        timestamp: u64,
    ) -> Self {
        Self::new(
            round,
            author,
            "test-chain".to_string(),
            parents,
            transactions,
            state_root,
            timestamp,
        )
    }

    /// Compute hash of the vertex (excluding id and signature)
    /// 500K TPS optimization: caches result to avoid repeated serialization
    /// Returns fixed-size [u8; 32] array (no heap allocation)
    pub fn compute_hash(&self) -> Result<VertexId> {
        if let Some(ref cached) = self.cached_hash {
            return Ok(vertex_id_from_hash_bytes(cached));
        }

        let hash_vec = hash_data_blake3(&self.hash_material()?);
        Ok(vertex_id_from_hash_bytes(&hash_vec))
    }

    /// Verify vertex integrity
    pub fn verify(&self) -> Result<()> {
        let computed_hash = self.compute_hash()?;
        if self.id != computed_hash {
            anyhow::bail!("Vertex hash mismatch");
        }

        if self.transactions.len() != self.metadata.tx_count {
            anyhow::bail!("Transaction count mismatch");
        }

        Ok(())
    }

    /// Check if this vertex has quorum support from unique, trusted authors
    ///
    /// This validates that at least `2f+1` unique trusted authorities have referenced
    /// this vertex as a parent. Excludes banned/untrusted authorities to prevent
    /// Byzantine nodes from influencing consensus.
    ///
    /// # Arguments
    /// * `store` - DAG store with vertex data and trust information
    /// * `required_quorum` - Required validator threshold for this round
    ///
    /// # Returns
    /// `true` if quorum is reached from trusted authorities only
    pub fn has_quorum_unique_authors(&self, store: &DagStore, required_quorum: usize) -> bool {
        if required_quorum == 0 {
            return false;
        }

        let mut unique_authors = HashSet::new();
        for parent_id in &self.parents {
            if let Some(parent_vertex) = store.get_vertex(parent_id) {
                if store.is_authority_trusted(&parent_vertex.author) {
                    unique_authors.insert(parent_vertex.author.clone());
                } else {
                    tracing::warn!(
                        "[Security] Excluding untrusted authority {} from quorum calculation",
                        parent_vertex.author
                    );
                }
            }
        }

        unique_authors.len() >= required_quorum
    }
}

/// Checkpoint represents a committed sequence of vertices
/// This is the output of the consensus protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Checkpoint sequence number
    pub sequence: u64,

    /// Vertices included in this checkpoint (in total order)
    pub vertices: Vec<VertexId>,

    /// All transactions in order
    pub transactions: Vec<SignedTransaction>,

    /// State root after executing all transactions
    pub state_root: Vec<u8>,

    /// Timestamp of checkpoint creation
    pub timestamp: u64,

    /// Previous checkpoint hash
    pub prev_checkpoint_hash: Vec<u8>,
}

impl Checkpoint {
    pub fn new(
        sequence: u64,
        vertices: Vec<VertexId>,
        transactions: Vec<SignedTransaction>,
        state_root: Vec<u8>,
        timestamp: u64,
        prev_checkpoint_hash: Vec<u8>,
    ) -> Self {
        Self {
            sequence,
            vertices,
            transactions,
            state_root,
            timestamp,
            prev_checkpoint_hash,
        }
    }

    pub fn hash(&self) -> Result<Vec<u8>> {
        // Checkpoint identity must be stable across peers that reach the same
        // committed state even if their local DAG paths or leader timestamps differ.
        // Hash only canonical execution data, not transport/local scheduling metadata.
        let tx_hashes: Vec<Vec<u8>> = self.transactions.iter().map(logical_tx_hash).collect();
        let serialized = bcs::to_bytes(&(
            self.sequence,
            &tx_hashes,
            &self.state_root,
            &self.prev_checkpoint_hash,
        ))?;
        Ok(hash_data_blake3(&serialized))
    }

    /// Genesis checkpoint
    pub fn genesis() -> Self {
        let genesis_state_root = smt::default_hashes()[0].to_vec();
        Self {
            sequence: 0,
            vertices: Vec::new(),
            transactions: Vec::new(),
            state_root: genesis_state_root,
            timestamp: 0,
            prev_checkpoint_hash: vec![0u8; 32],
        }
    }
}

/// Checkpoint configuration
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    /// Minimum rounds between checkpoints
    pub min_rounds: u64,

    /// Maximum rounds between checkpoints (force checkpoint after this)
    pub max_rounds: u64,

    /// Minimum vertices to justify creating a checkpoint
    pub min_vertices: usize,

    /// Maximum pending vertices before forcing a checkpoint
    pub max_vertices: usize,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            min_rounds: 5,       // Faster checkpointing for 500K TPS
            max_rounds: 50,      // More frequent forced checkpoints
            min_vertices: 1000,  // Larger batches for efficiency
            max_vertices: 50000, // 50k pending before force (handle bursts)
        }
    }
}

impl CheckpointConfig {
    fn from_values(
        min_rounds: u64,
        max_rounds: u64,
        min_vertices: usize,
        max_vertices: usize,
    ) -> Self {
        Self {
            min_rounds,
            max_rounds,
            min_vertices,
            max_vertices,
        }
    }

    /// Create high-throughput config for 500K+ TPS
    pub fn high_throughput() -> Self {
        Self::from_values(2, 20, 5000, 100000)
    }

    /// Create conservative config (frequent checkpoints, low latency)
    pub fn conservative() -> Self {
        Self::from_values(5, 50, 50, 5000)
    }

    /// Create aggressive config (infrequent checkpoints, high throughput)
    pub fn aggressive() -> Self {
        Self::from_values(20, 200, 500, 50000)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.min_rounds >= self.max_rounds {
            anyhow::bail!("min_rounds must be less than max_rounds");
        }
        if self.min_vertices >= self.max_vertices {
            anyhow::bail!("min_vertices must be less than max_vertices");
        }
        if self.max_rounds == 0 {
            anyhow::bail!("max_rounds must be greater than 0");
        }
        Ok(())
    }
}

/// DAG Storage - maintains the DAG structure
#[derive(Clone)]
pub struct DagStore {
    /// All vertices indexed by their ID (Arc for zero-copy sharing - 500K TPS)
    /// Optimized: HashMap for O(1) lookup instead of BTreeMap O(log n)
    vertices: HashMap<VertexId, Arc<DagVertex>>,
    /// Vertices indexed by round number
    vertices_by_round: BTreeMap<Round, Vec<VertexId>>,
    /// Vertices indexed by authority
    vertices_by_authority: BTreeMap<AuthorityId, Vec<VertexId>>,
    /// Checkpoints (committed state)
    checkpoints: VecDeque<Checkpoint>,
    /// Pending vertices (not yet checkpointed)
    pending_vertices: VecDeque<VertexId>,
    /// Executed transaction hashes to prevent replay across checkpoints
    /// Optimized: HashSet for O(1) lookup instead of BTreeSet O(log n)
    executed_tx_hashes: HashSet<Vec<u8>>,
    /// Current round number
    current_round: Round,
    /// Set of authority IDs
    /// Optimized: HashSet for O(1) lookup instead of BTreeSet O(log n)
    authorities: HashSet<AuthorityId>,
    /// Map of vertex ID to its checkpoint sequence number (for GC)
    vertex_checkpoint_map: HashMap<VertexId, u64>,
    /// Checkpoint configuration
    checkpoint_config: CheckpointConfig,
    /// Round of last checkpoint
    last_checkpoint_round: Round,
    /// Backpressure limit for pending vertices (500K TPS protection)
    max_pending_vertices: usize,
    /// FIX #2 & #3: Track banned/untrusted authorities (reputation = 0)
    banned_authorities: HashSet<AuthorityId>,
}

/// Serializable state for persisting DAG data across restarts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentDagState {
    pub vertices: Vec<DagVertex>,
    pub checkpoints: Vec<Checkpoint>,
    pub current_round: Round,
    pub last_checkpoint_round: Round,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagProductionPolicy {
    pub current_round: Round,
    pub parent_round: Round,
    pub target_round: Round,
    pub parent_ids: Vec<VertexId>,
    pub parent_authors: Vec<AuthorityId>,
    pub missing_parent_authors: Vec<AuthorityId>,
    pub parent_author_count: usize,
    pub quorum_size: usize,
    pub local_has_vertex_in_current_round: bool,
    pub using_catch_up_round: bool,
}

impl DagProductionPolicy {
    pub fn should_wait_for_current_round_quorum(&self) -> bool {
        self.current_round > 0
            && self.parent_round == self.current_round
            && self.local_has_vertex_in_current_round
            && self.parent_author_count < self.quorum_size
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagProgressPolicy {
    pub current_round: Round,
    pub last_checkpoint_round: Round,
    pub latest_local_round: Round,
}

impl DagProgressPolicy {
    pub fn needs_progress(&self) -> bool {
        self.current_round > self.last_checkpoint_round
            || (self.current_round > 0 && self.latest_local_round < self.current_round)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagProductionPlan {
    pub policy: DagProductionPolicy,
    pub history_vertices: Vec<VertexId>,
    pub history_tx_hashes: BTreeSet<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct DagPendingSelection {
    pub included: Vec<SignedTransaction>,
    pub remove_hashes: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct DagExecutionPlan {
    pub transactions: Vec<SignedTransaction>,
    pub history_vertices: Vec<VertexId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DagNetworkVertexAction {
    Accept,
    IgnoreExisting,
    IgnoreFarFutureEmpty { current_round: Round },
}

/// Mysticeti DAG consensus protocol.
pub struct DagConsensus {
    /// DAG storage
    store: DagStore,

    /// This node's authority ID
    authority_id: AuthorityId,

    /// Chain ID for cross-chain replay protection
    chain_id: String,

    /// Byzantine fault detector
    byzantine_detector: ByzantineDetector,

    /// Caching layer for performance optimization
    caches: DagCaches,

    /// Validator committee used for membership and quorum checks
    committee: Committee,

    /// Concrete Mysticeti-style protocol parameters for commit and readiness rules.
    protocol: Protocol,

    /// Metrics collection for monitoring
    metrics: DagMetrics,

    /// State synchronization for new nodes
    state_sync: StateSynchronizer,

    /// Vertex broadcasting with compression
    broadcaster: VertexBroadcaster,

    /// Persistent storage backend for DAG vertices
    persistent_store: Option<PersistentDagStore>,

    /// Async channel for background disk writes (500K TPS optimization - replaced raw thread with tokio)
    disk_writer_tx: Option<mpsc::Sender<Arc<DagVertex>>>,

    /// DAG pruning to manage storage growth
    pruner: DagPruner,

    /// Parallel vertex validator for high throughput
    parallel_validator: ParallelValidator,

    /// Local keypair for signing vertices created by this authority (demo key management).
    local_signing_key: ed25519_dalek::SigningKey,
}

impl DagConsensus {
    pub fn protocol(&self) -> &Protocol {
        &self.protocol
    }

    pub fn quorum_threshold(&self) -> usize {
        self.committee
            .required_quorum()
            .max(self.protocol.direct_commit_quorum)
    }

    /// Internal helper: collect reachable, uncheckpointed vertices in post-order.
    /// This preserves deterministic traversal by sorting roots and parent edges.
    fn collect_vertices_post_order(
        &self,
        mut roots: Vec<VertexId>,
        include_vertex_in_cycle_error: bool,
    ) -> Result<Vec<VertexId>> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut in_progress = HashSet::new();
        let mut stack = Vec::new();

        roots.sort();
        for root in roots {
            stack.push((root, false));
        }

        while let Some((vertex_id, processed)) = stack.pop() {
            if processed {
                if !visited.contains(&vertex_id) {
                    visited.insert(vertex_id);
                    result.push(vertex_id);
                }
                in_progress.remove(&vertex_id);
                continue;
            }

            if visited.contains(&vertex_id) {
                continue;
            }

            if in_progress.contains(&vertex_id) {
                if include_vertex_in_cycle_error {
                    anyhow::bail!("Cycle detected in DAG at vertex {}", hex::encode(vertex_id));
                }
                anyhow::bail!("Cycle detected in DAG");
            }

            if let Some(vertex) = self.store.get_vertex(&vertex_id) {
                if self.store.is_vertex_checkpointed(&vertex_id) {
                    visited.insert(vertex_id);
                    continue;
                }

                in_progress.insert(vertex_id);
                stack.push((vertex_id, true));

                let mut sorted_parents = vertex.parents.clone();
                sorted_parents.sort();
                for parent_id in sorted_parents {
                    if !visited.contains(&parent_id) {
                        stack.push((parent_id, false));
                    }
                }
            }
        }

        Ok(result)
    }

    /// Collect all vertices that would be committed if a vertex with given parents became a leader
    pub fn collect_history_for_parents(&self, parents: &[VertexId]) -> Result<Vec<VertexId>> {
        self.collect_vertices_post_order(parents.to_vec(), false)
    }

    /// Collect all vertices that should be committed (topotracingical sort with cycle detection)
    fn collect_vertices_to_commit(&self, leader_vertex_id: VertexId) -> Result<Vec<VertexId>> {
        self.collect_vertices_post_order(vec![leader_vertex_id], true)
    }

    /// Get current DAG store (read-only)
    pub fn store(&self) -> &DagStore {
        &self.store
    }

    /// Get committee (read-only)
    pub fn committee(&self) -> &Committee {
        &self.committee
    }

    pub fn enable_adaptive_quorum(&mut self, config: AdaptiveQuorumConfig) {
        self.committee.enable_adaptive_quorum(config);
    }

    pub fn disable_adaptive_quorum(&mut self) {
        self.committee.disable_adaptive_quorum();
    }

    pub fn update_network_health(&mut self, health: NetworkHealth) {
        self.committee.update_network_health(health);
    }

    /// Get metrics collector (read-only)
    pub fn metrics(&self) -> &DagMetrics {
        &self.metrics
    }

    /// Check if vertex exists in DAG
    pub fn has_vertex(&self, vertex_id: &VertexId) -> bool {
        self.store.vertices.contains_key(vertex_id)
    }

    pub fn has_executed_transaction(&self, tx_hash: &[u8]) -> bool {
        self.store.executed_tx_hashes.contains(tx_hash)
    }

    /// Save the essential state of the DAG to a serializable struct
    pub fn save_state(&self) -> Result<PersistentDagState> {
        let mut vertices: Vec<_> = self
            .store
            .vertices
            .values()
            .map(|v| (**v).clone())
            .collect();
        vertices.sort_by_key(|vertex| vertex.id);
        let state = PersistentDagState {
            vertices,
            checkpoints: self.store.checkpoints.iter().cloned().collect(),
            current_round: self.store.current_round,
            last_checkpoint_round: self.store.last_checkpoint_round,
        };
        Ok(state)
    }

    /// Load the state of the DAG from a serializable struct
    pub fn load_state(&mut self, state: PersistentDagState) -> Result<()> {
        let mut new_store = DagStore::with_config(
            self.store.authorities.iter().cloned().collect(),
            self.store.checkpoint_config.clone(),
        );

        new_store.checkpoints = state.checkpoints.into_iter().collect();
        new_store.current_round = state.current_round;
        new_store.last_checkpoint_round = state.last_checkpoint_round;

        for vertex in state.vertices {
            let vertex_id = vertex.id;
            new_store.insert_vertex_arc(vertex_id, Arc::new(vertex));
        }

        // Reconstruct pending_vertices
        let committed_vertices: BTreeSet<VertexId> = new_store
            .checkpoints
            .iter()
            .flat_map(|cp| cp.vertices.iter().cloned())
            .collect();

        new_store.pending_vertices = new_store
            .vertices
            .keys()
            .filter(|&id| !committed_vertices.contains(id))
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .collect();
        new_store.pending_vertices.make_contiguous().sort();
        for checkpoint in &new_store.checkpoints {
            for tx in &checkpoint.transactions {
                new_store.executed_tx_hashes.insert(logical_tx_hash(tx));
            }
        }

        self.store = new_store;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use kanari_types::transaction::Transaction;

    use super::*;

    fn round0_vertex(i: u64) -> DagVertex {
        let transaction =
            Transaction::new_transfer(format!("sender{}", i), "receiver".to_string(), i, i);
        DagVertex::new_for_test(
            0,
            "auth1".to_string(),
            vec![],
            vec![SignedTransaction::new(transaction)],
            vec![i as u8; 32],
            0,
        )
    }

    fn add_round0_vertices(store: &mut DagStore, count: u64) {
        let total_auths = store.num_authorities();
        for i in 0..count {
            store.add_vertex(round0_vertex(i), total_auths).ok();
        }
    }

    #[test]
    fn test_dag_vertex_creation() {
        let parent = [0u8; 32];
        let vertex = DagVertex::new_for_test(
            1,
            "authority1".to_string(),
            vec![parent],
            vec![],
            vec![0u8; 32],
            0,
        );

        assert_eq!(vertex.round, 1);
        assert_eq!(vertex.author, "authority1");
        assert_ne!(vertex.id, [0u8; 32]); // ID should be computed hash
    }

    #[test]
    fn test_dag_store() {
        let authorities = vec![
            "auth1".to_string(),
            "auth2".to_string(),
            "auth3".to_string(),
            "auth4".to_string(),
        ];

        let mut store = DagStore::new(authorities);

        let vertex0 =
            DagVertex::new_for_test(0, "auth1".to_string(), vec![], vec![], vec![0u8; 32], 0);

        store
            .add_vertex(vertex0.clone(), store.num_authorities())
            .unwrap();

        assert_eq!(store.current_round(), 0);
        assert!(store.get_vertex(&vertex0.id).is_some());
    }

    #[test]
    fn test_reject_duplicate_transaction_across_checkpoints() {
        let mut store = DagStore::new(vec!["auth1".to_string()]);
        let tx = SignedTransaction::new(Transaction::new_transfer(
            "alice".to_string(),
            "bob".to_string(),
            1,
            1,
        ));
        let vertex = DagVertex::new_for_test(
            0,
            "auth1".to_string(),
            vec![],
            vec![tx.clone()],
            vec![1u8; 32],
            0,
        );
        let vertex_id = vertex.id;
        store.add_vertex(vertex, store.num_authorities()).unwrap();

        let latest = store.latest_checkpoint();
        let prev_hash = latest.hash().expect("Checkpoint hash should succeed");
        store
            .add_checkpoint(Checkpoint::new(
                latest.sequence + 1,
                vec![vertex_id],
                vec![tx.clone()],
                vec![1u8; 32],
                1,
                prev_hash,
            ))
            .unwrap();

        let replay =
            DagVertex::new_for_test(0, "auth1".to_string(), vec![], vec![tx], vec![3u8; 32], 1);
        assert!(store.add_vertex(replay, store.num_authorities()).is_err());
    }

    #[test]
    fn test_checkpoint_transaction_order_is_canonical_across_vertex_order() {
        let mut consensus = DagConsensus::new(
            "auth1".to_string(),
            vec!["auth1".to_string(), "auth2".to_string()],
        );
        let tx_a = SignedTransaction::new(Transaction::new_transfer(
            "0x2".to_string(),
            "0x3".to_string(),
            1,
            7,
        ));
        let tx_b = SignedTransaction::new(Transaction::new_transfer(
            "0x1".to_string(),
            "0x3".to_string(),
            1,
            7,
        ));

        let vertex_a = DagVertex::new_for_test(
            0,
            "auth1".to_string(),
            vec![],
            vec![tx_a.clone()],
            vec![1u8; 32],
            1,
        );
        let vertex_b = DagVertex::new_for_test(
            0,
            "auth2".to_string(),
            vec![],
            vec![tx_b.clone()],
            vec![2u8; 32],
            1,
        );
        let id_a = vertex_a.id;
        let id_b = vertex_b.id;
        consensus
            .store
            .add_vertex(vertex_a, consensus.store.num_authorities())
            .unwrap();
        consensus
            .store
            .add_vertex(vertex_b, consensus.store.num_authorities())
            .unwrap();

        let first = consensus.collect_checkpoint_transactions(&[id_a, id_b]);
        let second = consensus.collect_checkpoint_transactions(&[id_b, id_a]);
        let first_hashes: Vec<Vec<u8>> = first.iter().map(logical_tx_hash).collect();
        let second_hashes: Vec<Vec<u8>> = second.iter().map(logical_tx_hash).collect();

        assert_eq!(first_hashes, second_hashes);
        assert_eq!(first[0].transaction.sender_address(), "0x1");
        assert_eq!(first[1].transaction.sender_address(), "0x2");
    }

    #[test]
    fn test_pending_selection_removes_signed_transaction_already_committed_by_logical_hash() {
        let mut consensus = DagConsensus::new("auth1".to_string(), vec!["auth1".to_string()]);
        let mut tx = SignedTransaction::new(Transaction::new_transfer(
            "alice".to_string(),
            "bob".to_string(),
            1,
            1,
        ));
        tx.signature = vec![42];

        let vertex = DagVertex::new_for_test(
            0,
            "auth1".to_string(),
            vec![],
            vec![tx.clone()],
            vec![1u8; 32],
            0,
        );
        let vertex_id = vertex.id;
        consensus
            .store
            .add_vertex(vertex, consensus.store.num_authorities())
            .unwrap();

        let latest = consensus.store.latest_checkpoint();
        let prev_hash = latest.hash().expect("Checkpoint hash should succeed");
        consensus
            .store
            .add_checkpoint(Checkpoint::new(
                latest.sequence + 1,
                vec![vertex_id],
                vec![tx.clone()],
                vec![1u8; 32],
                1,
                prev_hash,
            ))
            .unwrap();

        let logical_hash = tx.transaction.hash();
        assert_ne!(logical_hash, tx.hash());
        assert!(consensus.has_executed_transaction(&logical_hash));

        let selection = consensus.select_pending_transactions(
            &DagProductionPlan {
                policy: consensus.production_policy(),
                history_vertices: vec![],
                history_tx_hashes: BTreeSet::new(),
            },
            &[tx],
            |_| false,
        );

        assert!(selection.included.is_empty());
        assert_eq!(selection.remove_hashes, vec![logical_hash]);
    }

    #[test]
    fn test_add_vertex_with_quorum_enforces_explicit_threshold() {
        let authorities = vec![
            "auth1".to_string(),
            "auth2".to_string(),
            "auth3".to_string(),
            "auth4".to_string(),
        ];
        let mut store = DagStore::new(authorities);

        let parent1 =
            DagVertex::new_for_test(0, "auth1".to_string(), vec![], vec![], vec![1u8; 32], 100);
        let parent2 =
            DagVertex::new_for_test(0, "auth2".to_string(), vec![], vec![], vec![2u8; 32], 100);
        let parent3 =
            DagVertex::new_for_test(0, "auth3".to_string(), vec![], vec![], vec![3u8; 32], 100);

        store
            .add_vertex(parent1.clone(), store.num_authorities())
            .unwrap();
        store
            .add_vertex(parent2.clone(), store.num_authorities())
            .unwrap();
        store
            .add_vertex(parent3.clone(), store.num_authorities())
            .unwrap();

        let child = DagVertex::new_for_test(
            1,
            "auth4".to_string(),
            vec![parent1.id, parent2.id, parent3.id],
            vec![],
            vec![4u8; 32],
            101,
        );

        assert!(store.add_vertex_with_quorum(child, 4).is_err());
    }

    #[test]
    fn test_add_vertex_arc_compatibility_path_uses_static_quorum() {
        let authorities = vec![
            "auth1".to_string(),
            "auth2".to_string(),
            "auth3".to_string(),
            "auth4".to_string(),
        ];
        let mut store = DagStore::new(authorities);

        let parent1 = Arc::new(DagVertex::new_for_test(
            0,
            "auth1".to_string(),
            vec![],
            vec![],
            vec![1u8; 32],
            100,
        ));
        let parent2 = Arc::new(DagVertex::new_for_test(
            0,
            "auth2".to_string(),
            vec![],
            vec![],
            vec![2u8; 32],
            100,
        ));
        let parent3 = Arc::new(DagVertex::new_for_test(
            0,
            "auth3".to_string(),
            vec![],
            vec![],
            vec![3u8; 32],
            100,
        ));

        store
            .add_vertex_arc(parent1.clone(), store.num_authorities())
            .unwrap();
        store
            .add_vertex_arc(parent2.clone(), store.num_authorities())
            .unwrap();
        store
            .add_vertex_arc(parent3.clone(), store.num_authorities())
            .unwrap();

        let child = Arc::new(DagVertex::new_for_test(
            1,
            "auth4".to_string(),
            vec![parent1.id, parent2.id, parent3.id],
            vec![],
            vec![4u8; 32],
            101,
        ));
        let child_id = child.id;

        store
            .add_vertex_arc(child, store.num_authorities())
            .unwrap();
        assert!(store.get_vertex(&child_id).is_some());
    }

    #[test]
    fn test_reject_future_timestamp_vertex() {
        let authorities = vec![
            "auth1".to_string(),
            "auth2".to_string(),
            "auth3".to_string(),
            "auth4".to_string(),
        ];
        let mut store = DagStore::new(authorities);

        // Add parent vertices with timestamp 100
        let parent1 =
            DagVertex::new_for_test(0, "auth1".to_string(), vec![], vec![], vec![1u8; 32], 100);
        let parent2 =
            DagVertex::new_for_test(0, "auth2".to_string(), vec![], vec![], vec![2u8; 32], 100);

        store.add_vertex(parent1, store.num_authorities()).unwrap();
        store.add_vertex(parent2, store.num_authorities()).unwrap();

        // Try to add child vertex with timestamp way too far in the future
        // Max allowed is median + 300 (MAX_TIMESTAMP_DRIFT_SECS), so 500 should be rejected
        let child_parents = store.get_vertex_ids_in_round(0);
        let future_vertex = DagVertex::new_for_test(
            1,
            "auth1".to_string(),
            child_parents,
            vec![],
            vec![3u8; 32],
            500, // Exceeds median (100) + max drift (300) = 400
        );

        assert!(
            store
                .add_vertex(future_vertex, store.num_authorities())
                .is_err()
        );
    }

    #[test]
    fn test_genesis_vertices_are_deterministic_across_nodes() {
        let authorities = vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()];

        let consensus_a = DagConsensus::new("0x1".to_string(), authorities.clone());
        let consensus_b = DagConsensus::new("0x2".to_string(), authorities);

        let mut round_zero_a: Vec<_> = consensus_a
            .store
            .get_vertex_ids_in_round(0)
            .into_iter()
            .map(hex::encode)
            .collect();
        let mut round_zero_b: Vec<_> = consensus_b
            .store
            .get_vertex_ids_in_round(0)
            .into_iter()
            .map(hex::encode)
            .collect();

        round_zero_a.sort();
        round_zero_b.sort();

        assert_eq!(round_zero_a, round_zero_b);
    }

    #[test]
    fn test_adaptive_quorum_blocks_vertex_creation_until_disabled_when_trusted_parents_drop() {
        let authorities = vec![
            "auth1".to_string(),
            "auth2".to_string(),
            "auth3".to_string(),
            "auth4".to_string(),
        ];

        let mut consensus = DagConsensus::new("auth1".to_string(), authorities);
        consensus.enable_adaptive_quorum(AdaptiveQuorumConfig::default());
        consensus.update_network_health(NetworkHealth {
            connectivity_ratio: 0.35,
            delivery_success_ratio: 0.40,
            timeout_ratio: 0.45,
            median_latency_ms: 3_500,
        });

        consensus.store.ban_authority(&"auth4".to_string());

        let create_err = consensus
            .create_vertex(vec![], vec![9u8; 32], 1)
            .unwrap_err();
        assert!(
            create_err
                .to_string()
                .contains("Not enough parents for quorum")
        );

        consensus.disable_adaptive_quorum();

        let vertex = consensus.create_vertex(vec![], vec![9u8; 32], 1).unwrap();
        assert_eq!(vertex.round, 1);
        assert_eq!(vertex.parents.len(), 4);
    }

    #[test]
    fn test_checkpoint_creation() {
        let checkpoint = Checkpoint::genesis();
        assert_eq!(checkpoint.sequence, 0);
        assert!(checkpoint.transactions.is_empty());
    }

    #[test]
    fn test_try_commit_produces_checkpoint_after_multi_round_progress() {
        let authorities = vec!["auth1".to_string()];
        let mut consensus = DagConsensus::new("auth1".to_string(), authorities);

        let round1 = consensus.create_vertex(vec![], vec![1u8; 32], 1).unwrap();
        let round1_id = round1.id;
        consensus.add_vertex(round1).unwrap();

        let round2 = consensus.create_vertex(vec![], vec![2u8; 32], 2).unwrap();
        consensus.add_vertex(round2).unwrap();

        let round3 = consensus.create_vertex(vec![], vec![3u8; 32], 3).unwrap();
        consensus.add_vertex(round3).unwrap();

        let checkpoint = consensus
            .try_commit()
            .unwrap()
            .expect("checkpoint should be produced after quorum-supported rounds");

        assert_eq!(checkpoint.sequence, 1);
        assert!(checkpoint.vertices.contains(&round1_id));

        consensus.add_checkpoint(checkpoint.clone()).unwrap();
        assert_eq!(consensus.latest_checkpoint().sequence, 1);
    }

    #[test]
    fn test_checkpoint_timestamp_is_canonical_by_sequence() {
        let authorities = vec!["auth1".to_string()];
        let mut consensus = DagConsensus::new("auth1".to_string(), authorities);

        let round1 = consensus
            .create_vertex(vec![], vec![1u8; 32], 1_700_000_000_000)
            .unwrap();
        consensus.add_vertex(round1).unwrap();

        let round2 = consensus
            .create_vertex(vec![], vec![2u8; 32], 1_800_000_000_000)
            .unwrap();
        consensus.add_vertex(round2).unwrap();

        let round3 = consensus
            .create_vertex(vec![], vec![3u8; 32], 1_750_000_000_000)
            .unwrap();
        consensus.add_vertex(round3).unwrap();

        let checkpoint = consensus
            .try_commit()
            .unwrap()
            .expect("checkpoint should be produced after quorum-supported rounds");

        assert_eq!(checkpoint.sequence, 1);
        assert_eq!(
            checkpoint.timestamp,
            checkpointing::canonical_checkpoint_timestamp(checkpoint.sequence)
        );
        assert_ne!(checkpoint.timestamp, 1_800_000_000_000);
    }

    #[test]
    fn test_try_commit_batches_mysticeti_multi_leaders_into_one_checkpoint() {
        let authorities = vec![
            "0x1".to_string(),
            "0x2".to_string(),
            "0x3".to_string(),
            "0x4".to_string(),
        ];

        let leader_two_tx = SignedTransaction::new(Transaction::new_transfer(
            "0x2".to_string(),
            "0x1".to_string(),
            1,
            0,
        ));
        let leader_three_tx = SignedTransaction::new(Transaction::new_transfer(
            "0x3".to_string(),
            "0x1".to_string(),
            1,
            0,
        ));

        let mut round_one_vertices = Vec::new();
        for authority in &authorities {
            let mut consensus = DagConsensus::new(authority.clone(), authorities.clone());
            let transactions = match authority.as_str() {
                "0x2" => vec![leader_two_tx.clone()],
                "0x3" => vec![leader_three_tx.clone()],
                _ => vec![],
            };
            round_one_vertices.push(
                consensus
                    .create_vertex(transactions, vec![1u8; 32], 1)
                    .unwrap(),
            );
        }

        let mut observer = DagConsensus::new("0x1".to_string(), authorities.clone());
        for vertex in &round_one_vertices {
            observer.add_vertex(vertex.clone()).unwrap();
        }

        let support_authors = ["0x1", "0x2", "0x3"];
        let mut round_two_vertices = Vec::new();
        for authority in support_authors {
            let mut supporter = DagConsensus::new(authority.to_string(), authorities.clone());
            for vertex in &round_one_vertices {
                supporter.add_vertex(vertex.clone()).unwrap();
            }
            let round_two = supporter.create_vertex(vec![], vec![2u8; 32], 2).unwrap();
            observer.add_vertex(round_two.clone()).unwrap();
            round_two_vertices.push(round_two);
        }

        for authority in ["0x1", "0x2", "0x3"] {
            let mut decision_author = DagConsensus::new(authority.to_string(), authorities.clone());
            for vertex in &round_one_vertices {
                decision_author.add_vertex(vertex.clone()).unwrap();
            }
            for vertex in &round_two_vertices {
                decision_author.add_vertex(vertex.clone()).unwrap();
            }
            let round_three = decision_author
                .create_vertex(vec![], vec![3u8; 32], 3)
                .unwrap();
            observer.add_vertex(round_three).unwrap();
        }

        let checkpoint = observer
            .try_commit()
            .unwrap()
            .expect("round one Mysticeti leaders should commit together");

        let leader_two_vertex = round_one_vertices
            .iter()
            .find(|vertex| vertex.author == "0x2")
            .unwrap();
        let leader_three_vertex = round_one_vertices
            .iter()
            .find(|vertex| vertex.author == "0x3")
            .unwrap();

        assert_eq!(checkpoint.sequence, 1);
        assert!(checkpoint.vertices.contains(&leader_two_vertex.id));
        assert!(checkpoint.vertices.contains(&leader_three_vertex.id));
        assert_eq!(checkpoint.transactions.len(), 2);
        assert!(
            checkpoint
                .transactions
                .iter()
                .any(|tx| tx.hash() == leader_two_tx.hash())
        );
        assert!(
            checkpoint
                .transactions
                .iter()
                .any(|tx| tx.hash() == leader_three_tx.hash())
        );
    }

    #[test]
    fn test_production_policy_uses_catch_up_round_for_partial_remote_round() {
        let authorities = vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()];
        let mut source = DagConsensus::new("0x1".to_string(), authorities.clone());
        let remote_round_one = source.create_vertex(vec![], vec![1u8; 32], 1).unwrap();

        let mut consensus = DagConsensus::new("0x2".to_string(), authorities);
        consensus.add_vertex(remote_round_one).unwrap();

        let policy = consensus.production_policy();
        assert_eq!(policy.current_round, 1);
        assert_eq!(policy.parent_round, 0);
        assert_eq!(policy.target_round, 1);
        assert!(policy.using_catch_up_round);
        assert!(!policy.local_has_vertex_in_current_round);
        assert!(!policy.should_wait_for_current_round_quorum());
    }

    #[test]
    fn test_production_policy_waits_when_local_vertex_already_exists_in_partial_round() {
        let authorities = vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()];
        let mut consensus = DagConsensus::new("0x1".to_string(), authorities);

        let round_one = consensus.create_vertex(vec![], vec![1u8; 32], 1).unwrap();
        consensus.add_vertex(round_one).unwrap();

        let policy = consensus.production_policy();
        assert_eq!(policy.current_round, 1);
        assert_eq!(policy.parent_round, 1);
        assert_eq!(policy.target_round, 2);
        assert!(policy.local_has_vertex_in_current_round);
        assert_eq!(policy.parent_authors, vec!["0x1".to_string()]);
        assert_eq!(
            policy.missing_parent_authors,
            vec!["0x2".to_string(), "0x3".to_string()]
        );
        assert!(policy.should_wait_for_current_round_quorum());
    }

    #[test]
    fn test_progress_policy_tracks_uncheckpointed_rounds() {
        let authorities = vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()];
        let mut consensus = DagConsensus::new("0x2".to_string(), authorities);

        let round_one = consensus.create_vertex(vec![], vec![1u8; 32], 1).unwrap();
        consensus.add_vertex(round_one).unwrap();

        let progress = consensus.progress_policy();
        assert_eq!(progress.current_round, 1);
        assert_eq!(progress.last_checkpoint_round, 0);
        assert_eq!(progress.latest_local_round, 1);
        assert!(progress.needs_progress());
        assert!(consensus.needs_progress());
    }

    #[test]
    fn test_plan_timestamp_uses_plan_parents_for_catch_up_rounds() {
        let authorities = vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()];
        let mut source = DagConsensus::new("0x1".to_string(), authorities.clone());
        let remote_round_one = source.create_vertex(vec![], vec![1u8; 32], 1).unwrap();

        let mut consensus = DagConsensus::new("0x2".to_string(), authorities);
        consensus.add_vertex(remote_round_one).unwrap();

        let plan = consensus.production_plan().unwrap();
        let catch_up_timestamp = consensus.suggest_vertex_timestamp_for_plan(&plan, 10_000);
        let current_round_timestamp = consensus.suggest_vertex_timestamp(10_000);

        assert_eq!(plan.policy.parent_round, 0);
        assert_eq!(catch_up_timestamp, 300);
        assert_eq!(current_round_timestamp, 301);
    }

    #[test]
    fn test_classify_network_vertex_ignores_existing_and_far_future_empty_vertices() {
        let authorities = vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()];
        let mut consensus = DagConsensus::new("0x1".to_string(), authorities.clone());

        let existing = consensus.create_vertex(vec![], vec![1u8; 32], 1).unwrap();
        let existing_id = existing.id;
        consensus.add_vertex(existing).unwrap();

        let existing_vertex = consensus.store.get_vertex(&existing_id).unwrap().clone();
        assert_eq!(
            consensus.classify_network_vertex(&existing_vertex, 20),
            DagNetworkVertexAction::IgnoreExisting
        );

        let far_future_empty = DagVertex::new_for_test(
            50,
            "0x2".to_string(),
            consensus.store.get_vertex_ids_in_round(1),
            vec![],
            vec![2u8; 32],
            2,
        );
        assert_eq!(
            consensus.classify_network_vertex(&far_future_empty, 20),
            DagNetworkVertexAction::IgnoreFarFutureEmpty { current_round: 1 }
        );
    }

    #[test]
    fn test_select_commit_vertex_is_deterministic_across_arrival_order() {
        let authorities = vec![
            "0x1".to_string(),
            "0x2".to_string(),
            "0x3".to_string(),
            "0x4".to_string(),
        ];

        let mut round_one_vertices = Vec::new();
        for authority in &authorities {
            let mut consensus = DagConsensus::new(authority.clone(), authorities.clone());
            round_one_vertices.push(consensus.create_vertex(vec![], vec![1u8; 32], 1).unwrap());
        }

        let mut observer = DagConsensus::new("0x1".to_string(), authorities.clone());
        for vertex in round_one_vertices.iter().rev() {
            observer.add_vertex(vertex.clone()).unwrap();
        }

        let support_authors = vec!["0x1".to_string(), "0x2".to_string(), "0x4".to_string()];
        let mut round_two_vertices = Vec::new();
        for authority in support_authors {
            let mut supporter = DagConsensus::new(authority, authorities.clone());
            for vertex in &round_one_vertices {
                supporter.add_vertex(vertex.clone()).unwrap();
            }
            let round_two = supporter.create_vertex(vec![], vec![2u8; 32], 2).unwrap();
            observer.add_vertex(round_two.clone()).unwrap();
            round_two_vertices.push(round_two);
        }

        for authority in ["0x1", "0x2", "0x4"] {
            let mut decision_author = DagConsensus::new(authority.to_string(), authorities.clone());
            for vertex in &round_one_vertices {
                decision_author.add_vertex(vertex.clone()).unwrap();
            }
            for vertex in &round_two_vertices {
                decision_author.add_vertex(vertex.clone()).unwrap();
            }
            let round_three = decision_author
                .create_vertex(vec![], vec![3u8; 32], 3)
                .unwrap();
            observer.add_vertex(round_three).unwrap();
        }

        let preferred_leader = "0x1".to_string();
        let selected = observer
            .select_commit_vertex(1, &preferred_leader)
            .unwrap()
            .expect("commit candidate should exist");

        assert_eq!(selected.0.author, "0x1");
    }

    #[test]
    fn test_select_commit_vertex_does_not_fallback_to_non_leader() {
        let authorities = vec![
            "0x1".to_string(),
            "0x2".to_string(),
            "0x3".to_string(),
            "0x4".to_string(),
        ];

        let mut round_one_vertices = Vec::new();
        for authority in &authorities {
            let mut consensus = DagConsensus::new(authority.clone(), authorities.clone());
            round_one_vertices.push(consensus.create_vertex(vec![], vec![1u8; 32], 1).unwrap());
        }

        let mut observer = DagConsensus::new("0x1".to_string(), authorities.clone());
        for vertex in round_one_vertices
            .iter()
            .filter(|vertex| vertex.author != "0x4")
        {
            observer.add_vertex(vertex.clone()).unwrap();
        }

        for authority in ["0x1", "0x2", "0x3"] {
            let mut supporter = DagConsensus::new(authority.to_string(), authorities.clone());
            for vertex in round_one_vertices
                .iter()
                .filter(|vertex| vertex.author != "0x4")
            {
                supporter.add_vertex(vertex.clone()).unwrap();
            }
            let round_two = supporter.create_vertex(vec![], vec![2u8; 32], 2).unwrap();
            observer.add_vertex(round_two).unwrap();
        }

        let missing_leader = "0x4".to_string();
        let selected = observer.select_commit_vertex(1, &missing_leader).unwrap();

        assert!(selected.is_none());
    }

    #[test]
    fn test_checkpoint_hash_ignores_non_canonical_dag_metadata() {
        let tx = SignedTransaction::new(Transaction::new_transfer(
            "alice".to_string(),
            "bob".to_string(),
            1,
            7,
        ));

        let checkpoint_a = Checkpoint::new(
            1,
            vec![[1u8; 32]],
            vec![tx.clone()],
            vec![9u8; 32],
            123,
            vec![4u8; 32],
        );
        let checkpoint_b = Checkpoint::new(
            1,
            vec![[2u8; 32], [3u8; 32]],
            vec![tx],
            vec![9u8; 32],
            999,
            vec![4u8; 32],
        );

        assert_eq!(checkpoint_a.hash().unwrap(), checkpoint_b.hash().unwrap());
    }

    #[test]
    fn test_non_empty_checkpoint_root_is_provisional_until_engine_replay() {
        let authorities = vec!["0x1".to_string()];
        let mut consensus = DagConsensus::new("0x1".to_string(), authorities);

        let tx = SignedTransaction::new(Transaction::new_transfer(
            "0x1".to_string(),
            "0x2".to_string(),
            1,
            0,
        ));

        let vertex = consensus
            .create_vertex(vec![tx.clone()], vec![9u8; 32], 1)
            .unwrap();
        let vertex_id = vertex.id;
        consensus.add_vertex(vertex).unwrap();

        let vertices_to_commit = consensus.collect_vertices_to_commit(vertex_id).unwrap();
        let checkpoint_transactions =
            consensus.collect_checkpoint_transactions(&vertices_to_commit);
        let provisional_root = consensus
            .checkpoint_state_root(&vertices_to_commit, &checkpoint_transactions)
            .unwrap();

        assert_eq!(checkpoint_transactions.len(), 1);
        assert_eq!(provisional_root, Checkpoint::genesis().state_root);
        assert_ne!(provisional_root, vec![9u8; 32]);
    }

    #[test]
    fn test_checkpoint_store_allows_canonical_root_replacement() {
        let mut store = DagStore::new(vec!["0x1".to_string()]);
        let prev_hash = store.latest_checkpoint().hash().unwrap();

        let provisional = Checkpoint::new(1, vec![], vec![], vec![1u8; 32], 42, prev_hash.clone());
        store.add_checkpoint(provisional).unwrap();

        let canonical = Checkpoint::new(1, vec![], vec![], vec![2u8; 32], 42, prev_hash);
        store.add_checkpoint(canonical.clone()).unwrap();

        let latest = store.latest_checkpoint();
        assert_eq!(latest.sequence, 1);
        assert_eq!(latest.state_root, canonical.state_root);
    }

    #[test]
    fn test_reject_timestamp_far_ahead_of_old_parents() {
        let authorities = vec![
            "auth1".to_string(),
            "auth2".to_string(),
            "auth3".to_string(),
            "auth4".to_string(),
        ];
        let mut store = DagStore::new(authorities);

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let old_timestamp = current_time - 1200;

        let parent1 = DagVertex::new_for_test(
            0,
            "auth1".to_string(),
            vec![],
            vec![],
            vec![1u8; 32],
            old_timestamp,
        );
        let parent2 = DagVertex::new_for_test(
            0,
            "auth2".to_string(),
            vec![],
            vec![],
            vec![2u8; 32],
            old_timestamp,
        );
        let parent3 = DagVertex::new_for_test(
            0,
            "auth3".to_string(),
            vec![],
            vec![],
            vec![3u8; 32],
            old_timestamp,
        );

        store.add_vertex(parent1, store.num_authorities()).unwrap();
        store.add_vertex(parent2, store.num_authorities()).unwrap();
        store.add_vertex(parent3, store.num_authorities()).unwrap();

        let child_parents = store.get_vertex_ids_in_round(0);
        let restart_vertex = DagVertex::new_for_test(
            1,
            "auth1".to_string(),
            child_parents.clone(),
            vec![],
            vec![4u8; 32],
            current_time,
        );

        assert!(
            store
                .add_vertex(restart_vertex, store.num_authorities())
                .is_err(),
            "Validation must reject timestamps that exceed parent-derived bounds"
        );
    }

    #[test]
    fn test_accept_timestamp_within_parent_derived_window() {
        let authorities = vec![
            "auth1".to_string(),
            "auth2".to_string(),
            "auth3".to_string(),
            "auth4".to_string(),
        ];
        let mut store = DagStore::new(authorities);

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let old_timestamp = current_time - 7200;

        let parent1 = DagVertex::new_for_test(
            0,
            "auth1".to_string(),
            vec![],
            vec![],
            vec![1u8; 32],
            old_timestamp,
        );
        let parent2 = DagVertex::new_for_test(
            0,
            "auth2".to_string(),
            vec![],
            vec![],
            vec![2u8; 32],
            old_timestamp,
        );
        let parent3 = DagVertex::new_for_test(
            0,
            "auth3".to_string(),
            vec![],
            vec![],
            vec![3u8; 32],
            old_timestamp,
        );

        store.add_vertex(parent1, store.num_authorities()).unwrap();
        store.add_vertex(parent2, store.num_authorities()).unwrap();
        store.add_vertex(parent3, store.num_authorities()).unwrap();

        let child_parents = store.get_vertex_ids_in_round(0);
        let acceptable_vertex = DagVertex::new_for_test(
            1,
            "auth1".to_string(),
            child_parents.clone(),
            vec![],
            vec![4u8; 32],
            old_timestamp + 300,
        );

        assert!(
            store
                .add_vertex(acceptable_vertex, store.num_authorities())
                .is_ok(),
            "Vertex at the parent-derived upper bound should be accepted"
        );
    }

    #[test]
    fn test_normal_operation_timestamp_validation() {
        let authorities = vec![
            "auth1".to_string(),
            "auth2".to_string(),
            "auth3".to_string(),
            "auth4".to_string(),
        ];
        let mut store = DagStore::new(authorities);

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let recent_timestamp = current_time - 30;

        let parent1 = DagVertex::new_for_test(
            0,
            "auth1".to_string(),
            vec![],
            vec![],
            vec![1u8; 32],
            recent_timestamp,
        );
        let parent2 = DagVertex::new_for_test(
            0,
            "auth2".to_string(),
            vec![],
            vec![],
            vec![2u8; 32],
            recent_timestamp,
        );

        store.add_vertex(parent1, store.num_authorities()).unwrap();
        store.add_vertex(parent2, store.num_authorities()).unwrap();

        let child_parents = store.get_vertex_ids_in_round(0);
        let future_vertex = DagVertex::new_for_test(
            1,
            "auth1".to_string(),
            child_parents,
            vec![],
            vec![3u8; 32],
            recent_timestamp + 400,
        );

        assert!(
            store
                .add_vertex(future_vertex, store.num_authorities())
                .is_err(),
            "Normal operation should reject vertices with > 300s timestamp drift"
        );
    }

    #[test]
    fn test_checkpoint_config_default() {
        let config = CheckpointConfig::default();
        assert_eq!(config.min_rounds, 5);
        assert_eq!(config.max_rounds, 50);
        assert_eq!(config.min_vertices, 1000);
        assert_eq!(config.max_vertices, 50000);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_checkpoint_config_conservative() {
        let config = CheckpointConfig::conservative();
        // Conservative should still have smaller values than default
        assert!(config.min_rounds <= CheckpointConfig::default().min_rounds);
        assert!(config.max_rounds <= CheckpointConfig::default().max_rounds);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_checkpoint_config_aggressive() {
        let config = CheckpointConfig::aggressive();
        assert!(config.min_rounds > CheckpointConfig::default().min_rounds);
        assert!(config.max_rounds > CheckpointConfig::default().max_rounds);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_checkpoint_config_validation() {
        // Invalid: min >= max rounds
        let config = CheckpointConfig {
            min_rounds: 100,
            max_rounds: 50,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        // Invalid: min >= max vertices
        let config = CheckpointConfig {
            min_rounds: 10,
            max_rounds: 100,
            min_vertices: 5000,
            max_vertices: 1000,
        };
        assert!(config.validate().is_err());

        // Invalid: max_rounds = 0
        let config = CheckpointConfig {
            min_vertices: 100,
            max_vertices: 1000,
            max_rounds: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_should_create_checkpoint_min_rounds() {
        let config = CheckpointConfig {
            min_rounds: 10,
            max_rounds: 100,
            min_vertices: 100,
            max_vertices: 1000,
        };

        let store = DagStore::with_config(vec!["auth1".to_string()], config);

        // Too early - should not checkpoint
        assert!(!store.should_create_checkpoint());
    }

    #[test]
    fn test_should_create_checkpoint_max_rounds() {
        let config = CheckpointConfig {
            min_rounds: 5,
            max_rounds: 10,
            min_vertices: 100,
            max_vertices: 1000,
        };

        let mut store = DagStore::with_config(vec!["auth1".to_string()], config);

        // Add enough rounds to trigger max_rounds
        // Note: Using round 0 for all vertices to avoid quorum requirements
        store.current_round = 10; // Simulate 10 rounds passing
        add_round0_vertices(&mut store, 11);

        // Should force checkpoint due to max_rounds
        assert!(store.should_create_checkpoint());
    }

    #[test]
    fn test_should_create_checkpoint_min_vertices() {
        let config = CheckpointConfig {
            min_rounds: 1,
            max_rounds: 100,
            min_vertices: 5,
            max_vertices: 1000,
        };

        let mut store = DagStore::with_config(vec!["auth1".to_string()], config);

        // Add vertices at round 0 to avoid quorum requirements
        store.current_round = 1; // Simulate passing minimum rounds
        add_round0_vertices(&mut store, 10);

        // Should checkpoint due to min_vertices reached
        assert!(store.should_create_checkpoint());
    }

    #[test]
    fn test_should_create_checkpoint_max_vertices() {
        let config = CheckpointConfig {
            min_rounds: 1,
            max_rounds: 100,
            min_vertices: 100,
            max_vertices: 10, // Force at 10 vertices
        };

        let mut store = DagStore::with_config(vec!["auth1".to_string()], config);

        // Add many vertices at round 0
        store.current_round = 1; // Simulate passing minimum rounds
        add_round0_vertices(&mut store, 15);

        // Should force checkpoint due to max_vertices
        assert!(store.should_create_checkpoint());
    }

    #[test]
    fn test_checkpoint_stats() {
        let config = CheckpointConfig::default();
        let mut store = DagStore::with_config(vec!["auth1".to_string()], config);

        // Get initial stats
        let initial_stats = store.get_checkpoint_stats();
        assert_eq!(initial_stats.total_checkpoints, 1); // Genesis
        assert_eq!(initial_stats.pending_vertices, 0);

        // Add some vertices at round 0
        store.current_round = 1; // Simulate passing minimum rounds
        add_round0_vertices(&mut store, 5);

        let stats = store.get_checkpoint_stats();
        assert!(stats.pending_vertices > 0);
        assert_eq!(stats.total_checkpoints, 1); // Still just genesis
        assert!(stats.rounds_since_last > 0);
    }

    #[test]
    fn test_set_checkpoint_config() {
        let mut store = DagStore::new(vec!["auth1".to_string()]);

        let new_config = CheckpointConfig::aggressive();
        assert!(store.set_checkpoint_config(new_config.clone()).is_ok());

        assert_eq!(
            store.get_checkpoint_config().min_rounds,
            new_config.min_rounds
        );

        let invalid_config = CheckpointConfig {
            min_rounds: 100,
            max_rounds: 50,
            min_vertices: 100,
            max_vertices: 1000,
        };
        assert!(store.set_checkpoint_config(invalid_config).is_err());
    }

    #[test]
    fn test_add_vertex_rejects_missing_signature() {
        let authorities = vec![
            "auth1".to_string(),
            "auth2".to_string(),
            "auth3".to_string(),
            "auth4".to_string(),
        ];
        let mut consensus = DagConsensus::new("auth1".to_string(), authorities.clone());

        let vertex = DagVertex::new_for_test(
            1,
            "auth2".to_string(),
            Vec::new(),
            Vec::new(),
            vec![7u8; 32],
            1,
        );
        assert!(consensus.add_vertex(vertex).is_err());
    }

    #[test]
    fn test_with_chain_id_secure_accepts_explicit_keys() {
        let authorities = vec!["auth1".to_string(), "auth2".to_string()];
        let sk1 = ed25519_dalek::SigningKey::from_bytes(&[11u8; 32]);
        let sk2 = ed25519_dalek::SigningKey::from_bytes(&[22u8; 32]);
        let mut public_keys = std::collections::BTreeMap::new();
        public_keys.insert("auth1".to_string(), sk1.verifying_key().to_bytes().to_vec());
        public_keys.insert("auth2".to_string(), sk2.verifying_key().to_bytes().to_vec());

        let consensus = DagConsensus::with_chain_id_secure(
            "auth1".to_string(),
            authorities,
            "chain-secure".to_string(),
            sk1,
            public_keys,
        )
        .unwrap();
        assert!(consensus.committee().contains("auth1"));
        assert!(consensus.committee().contains("auth2"));
    }

    #[test]
    fn test_with_chain_id_secure_rejects_mismatched_local_key() {
        let authorities = vec!["auth1".to_string()];
        let expected = ed25519_dalek::SigningKey::from_bytes(&[33u8; 32]);
        let wrong = ed25519_dalek::SigningKey::from_bytes(&[44u8; 32]);
        let mut public_keys = std::collections::BTreeMap::new();
        public_keys.insert(
            "auth1".to_string(),
            expected.verifying_key().to_bytes().to_vec(),
        );

        let result = DagConsensus::with_chain_id_secure(
            "auth1".to_string(),
            authorities,
            "chain-secure".to_string(),
            wrong,
            public_keys,
        );
        assert!(result.is_err());
    }
}
