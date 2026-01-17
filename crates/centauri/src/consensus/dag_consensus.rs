// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! DAG-based Consensus Implementation (Narwhal & Tusk / Bullshark style)
//!
//! This module implements a Directed Acyclic Graph consensus mechanism that separates:
//! - Data Availability (DA): Broadcasting and storing transaction data
//! - Ordering: Determining the total order of transactions
//!
//! Inspired by Sui's Narwhal & Bullshark consensus, this design enables:
//! - High throughput through parallel block production
//! - Low latency by decoupling DA from consensus
//! - Byzantine fault tolerance
//! - Efficient parallel execution (already supported in Kanari's produce_block.rs)

use super::byzantine_detector::ByzantineDetector;
use super::cache::DagCaches;
use super::committee::{Committee, ValidatorInfo};
use super::metrics::DagMetrics;
use super::parallel_validator::{ParallelValidator, ParallelValidatorConfig};
use super::persistent_store::PersistentDagStore;
use super::pruning::{DagPruner, PruningConfig};
use super::state_sync::StateSynchronizer;
use super::vertex_broadcast::VertexBroadcaster;
use super::vrf_leader::{VrfLeaderElection, VrfOutput};
use anyhow::Result;
use kanari_crypto::hash_data_blake3;
use kanari_types::transaction::SignedTransaction;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
// Use fully-qualified time APIs where needed to avoid unused-import warnings

/// Unique identifier for a DAG vertex (block)
pub type VertexId = Vec<u8>;

/// Round number in the DAG consensus protocol
pub type Round = u64;

/// Authority/validator identifier
pub type AuthorityId = String;

/// DAG Vertex (equivalent to a block in traditional blockchain)
/// Each vertex can reference multiple parent vertices, forming a DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagVertex {
    /// Unique identifier (hash of the vertex)
    pub id: VertexId,

    /// Round number in the consensus protocol
    pub round: Round,

    /// Authority that created this vertex
    pub author: AuthorityId,

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
    /// Create a new DAG vertex
    pub fn new(
        round: Round,
        author: AuthorityId,
        parents: Vec<VertexId>,
        transactions: Vec<SignedTransaction>,
        state_root: Vec<u8>,
    ) -> Self {
        #[cfg(miri)]
        let timestamp: u64 = 0;

        #[cfg(not(miri))]
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let tx_count = transactions.len();

        let metadata = VertexMetadata {
            tx_count,
            total_gas_used: 0,
            state_root,
            is_checkpoint: false,
            checkpoint_seq: None,
        };

        let mut vertex = Self {
            id: Vec::new(), // Will be computed
            round,
            author,
            parents,
            transactions,
            timestamp,
            signature: Vec::new(),
            metadata,
        };

        // Compute vertex ID (hash)
        vertex.id = vertex.compute_hash();
        vertex
    }

    /// Compute hash of the vertex (excluding id and signature)
    pub fn compute_hash(&self) -> Vec<u8> {
        // Serialize vertex data for hashing
        let mut data = Vec::new();
        data.extend_from_slice(&self.round.to_le_bytes());
        data.extend_from_slice(self.author.as_bytes());

        for parent in &self.parents {
            data.extend_from_slice(parent);
        }

        for tx in &self.transactions {
            data.extend_from_slice(&tx.hash());
        }

        data.extend_from_slice(&self.timestamp.to_le_bytes());
        data.extend_from_slice(&self.metadata.state_root);

        hash_data_blake3(&data)
    }

    /// Verify vertex integrity
    pub fn verify(&self) -> Result<()> {
        // Verify hash matches
        let computed_hash = self.compute_hash();
        if self.id != computed_hash {
            anyhow::bail!("Vertex hash mismatch");
        }

        // Verify transaction count
        if self.transactions.len() != self.metadata.tx_count {
            anyhow::bail!("Transaction count mismatch");
        }

        Ok(())
    }

    /// Check if this vertex forms a quorum with its parents
    /// Requires 2f+1 parents (Byzantine fault tolerance)
    pub fn has_quorum(&self, total_authorities: usize) -> bool {
        let f = (total_authorities - 1) / 3; // Max faulty nodes
        let quorum_size = 2 * f + 1;
        self.parents.len() >= quorum_size
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
        prev_checkpoint_hash: Vec<u8>,
    ) -> Self {
        #[cfg(miri)]
        let timestamp: u64 = 0;

        #[cfg(not(miri))]
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            sequence,
            vertices,
            transactions,
            state_root,
            timestamp,
            prev_checkpoint_hash,
        }
    }

    pub fn hash(&self) -> Vec<u8> {
        let serialized = bcs::to_bytes(self).unwrap_or_default();
        hash_data_blake3(&serialized)
    }

    /// Genesis checkpoint
    pub fn genesis() -> Self {
        Self {
            sequence: 0,
            vertices: Vec::new(),
            transactions: Vec::new(),
            state_root: vec![0u8; 32],
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
            min_rounds: 10,      // At least 10 rounds between checkpoints
            max_rounds: 100,     // Force checkpoint after 100 rounds
            min_vertices: 100,   // Need at least 100 vertices
            max_vertices: 10000, // Force checkpoint if 10k pending vertices
        }
    }
}

impl CheckpointConfig {
    /// Create conservative config (frequent checkpoints, low latency)
    pub fn conservative() -> Self {
        Self {
            min_rounds: 5,
            max_rounds: 50,
            min_vertices: 50,
            max_vertices: 5000,
        }
    }

    /// Create aggressive config (infrequent checkpoints, high throughput)
    pub fn aggressive() -> Self {
        Self {
            min_rounds: 20,
            max_rounds: 200,
            min_vertices: 500,
            max_vertices: 50000,
        }
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
#[derive(Debug, Clone)]
pub struct DagStore {
    /// All vertices indexed by their ID
    vertices: HashMap<VertexId, DagVertex>,

    /// Vertices indexed by round number
    vertices_by_round: HashMap<Round, Vec<VertexId>>,

    /// Vertices indexed by authority
    vertices_by_authority: HashMap<AuthorityId, Vec<VertexId>>,

    /// Checkpoints (committed state)
    checkpoints: Vec<Checkpoint>,

    /// Pending vertices (not yet checkpointed)
    pending_vertices: VecDeque<VertexId>,

    /// Current round number
    current_round: Round,

    /// Set of authority IDs
    authorities: HashSet<AuthorityId>,

    /// Checkpoint configuration
    checkpoint_config: CheckpointConfig,

    /// Round of last checkpoint
    last_checkpoint_round: Round,
}

impl DagStore {
    pub fn new(authorities: Vec<AuthorityId>) -> Self {
        Self::with_config(authorities, CheckpointConfig::default())
    }

    pub fn with_config(authorities: Vec<AuthorityId>, config: CheckpointConfig) -> Self {
        let genesis_checkpoint = Checkpoint::genesis();

        Self {
            vertices: HashMap::new(),
            vertices_by_round: HashMap::new(),
            vertices_by_authority: HashMap::new(),
            checkpoints: vec![genesis_checkpoint],
            pending_vertices: VecDeque::new(),
            current_round: 0,
            authorities: authorities.into_iter().collect(),
            checkpoint_config: config,
            last_checkpoint_round: 0,
        }
    }

    /// Check if a checkpoint should be created based on configuration
    pub fn should_create_checkpoint(&self) -> bool {
        let rounds_since_last = self
            .current_round
            .saturating_sub(self.last_checkpoint_round);
        let pending_count = self.pending_vertices.len();

        // Force checkpoint if max rounds reached
        if rounds_since_last >= self.checkpoint_config.max_rounds {
            return true;
        }

        // Too soon for checkpoint
        if rounds_since_last < self.checkpoint_config.min_rounds {
            return false;
        }

        // Force checkpoint if too many pending vertices
        if pending_count >= self.checkpoint_config.max_vertices {
            return true;
        }

        // Create checkpoint if enough vertices accumulated
        pending_count >= self.checkpoint_config.min_vertices
    }

    /// Get checkpoint configuration
    pub fn get_checkpoint_config(&self) -> &CheckpointConfig {
        &self.checkpoint_config
    }

    /// Update checkpoint configuration
    pub fn set_checkpoint_config(&mut self, config: CheckpointConfig) -> Result<()> {
        config.validate()?;
        self.checkpoint_config = config;
        Ok(())
    }

    /// Add a new vertex to the DAG
    pub fn add_vertex(&mut self, vertex: DagVertex) -> Result<()> {
        // Verify vertex
        vertex.verify()?;

        // Check if vertex already exists
        if self.vertices.contains_key(&vertex.id) {
            anyhow::bail!("Vertex already exists");
        }

        // Verify all parents exist (except for round 0)
        if vertex.round > 0 {
            for parent_id in &vertex.parents {
                if !self.vertices.contains_key(parent_id) {
                    anyhow::bail!("Parent vertex not found");
                }
            }

            // Verify quorum
            if !vertex.has_quorum(self.authorities.len()) {
                anyhow::bail!("Vertex does not have quorum of parents");
            }

            // Verify parents are from previous round
            for parent_id in &vertex.parents {
                if let Some(parent) = self.vertices.get(parent_id)
                    && parent.round != vertex.round - 1
                {
                    anyhow::bail!("Parent from wrong round");
                }
            }
        }

        // Update round number
        if vertex.round > self.current_round {
            self.current_round = vertex.round;
        }

        // Store vertex
        let vertex_id = vertex.id.clone();
        let round = vertex.round;
        let author = vertex.author.clone();

        self.vertices.insert(vertex_id.clone(), vertex);

        // Index by round
        self.vertices_by_round
            .entry(round)
            .or_default()
            .push(vertex_id.clone());

        // Index by authority
        self.vertices_by_authority
            .entry(author)
            .or_default()
            .push(vertex_id.clone());

        // Add to pending
        self.pending_vertices.push_back(vertex_id);

        Ok(())
    }

    /// Get vertex by ID
    pub fn get_vertex(&self, id: &VertexId) -> Option<&DagVertex> {
        self.vertices.get(id)
    }

    /// Get vertex by ID (mutable)
    pub fn get_vertex_mut(&mut self, id: &VertexId) -> Option<&mut DagVertex> {
        self.vertices.get_mut(id)
    }

    /// Get all vertices in a round
    pub fn get_vertices_in_round(&self, round: Round) -> Vec<&DagVertex> {
        self.vertices_by_round
            .get(&round)
            .map(|ids| ids.iter().filter_map(|id| self.vertices.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get vertices by authority
    pub fn get_vertices_by_authority(&self, authority: &AuthorityId) -> Vec<&DagVertex> {
        self.vertices_by_authority
            .get(authority)
            .map(|ids| ids.iter().filter_map(|id| self.vertices.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get latest checkpoint
    pub fn latest_checkpoint(&self) -> Checkpoint {
        self.checkpoints
            .last()
            .cloned()
            .unwrap_or_else(Checkpoint::genesis)
    }

    /// Get checkpoint by sequence number
    pub fn get_checkpoint(&self, sequence: u64) -> Option<&Checkpoint> {
        self.checkpoints.iter().find(|cp| cp.sequence == sequence)
    }

    /// Add a new checkpoint (commits vertices)
    pub fn add_checkpoint(&mut self, checkpoint: Checkpoint) -> Result<()> {
        // Verify checkpoint sequence
        let expected_seq = self.latest_checkpoint().sequence + 1;
        if checkpoint.sequence != expected_seq {
            anyhow::bail!("Invalid checkpoint sequence");
        }

        // Verify previous checkpoint hash
        let prev_hash = self.latest_checkpoint().hash();
        if checkpoint.prev_checkpoint_hash != prev_hash {
            anyhow::bail!("Invalid previous checkpoint hash");
        }

        // Mark vertices as checkpointed
        for vertex_id in &checkpoint.vertices {
            if let Some(vertex) = self.vertices.get_mut(vertex_id) {
                vertex.metadata.is_checkpoint = true;
                vertex.metadata.checkpoint_seq = Some(checkpoint.sequence);
            }
        }

        // Remove from pending
        self.pending_vertices
            .retain(|id| !checkpoint.vertices.contains(id));

        // Update last checkpoint round
        self.last_checkpoint_round = self.current_round;

        self.checkpoints.push(checkpoint);
        Ok(())
    }

    /// Get statistics about pending vertices
    pub fn get_checkpoint_stats(&self) -> CheckpointStats {
        CheckpointStats {
            pending_vertices: self.pending_vertices.len(),
            rounds_since_last: self
                .current_round
                .saturating_sub(self.last_checkpoint_round),
            total_checkpoints: self.checkpoints.len(),
            should_checkpoint: self.should_create_checkpoint(),
        }
    }

    /// Current round
    pub fn current_round(&self) -> Round {
        self.current_round
    }

    /// Number of authorities
    pub fn num_authorities(&self) -> usize {
        self.authorities.len()
    }
}

/// Statistics for checkpoint creation
#[derive(Debug, Clone)]
pub struct CheckpointStats {
    pub pending_vertices: usize,
    pub rounds_since_last: u64,
    pub total_checkpoints: usize,
    pub should_checkpoint: bool,
}

/// DAG Consensus Protocol (Bullshark-style with VRF leader election)
pub struct DagConsensus {
    /// DAG storage
    store: DagStore,

    /// This node's authority ID
    authority_id: AuthorityId,

    /// VRF-based leader election (replaces round-robin)
    vrf_election: VrfLeaderElection,

    /// Byzantine fault detector
    byzantine_detector: ByzantineDetector,

    /// Caching layer for performance optimization
    caches: DagCaches,

    /// Committee management for dynamic validator sets
    committee: Committee,

    /// Metrics collection for monitoring
    metrics: DagMetrics,

    /// State synchronization for new nodes
    state_sync: StateSynchronizer,

    /// Vertex broadcasting with compression
    broadcaster: VertexBroadcaster,

    /// Persistent storage backend for DAG vertices
    persistent_store: Option<PersistentDagStore>,

    /// DAG pruning to manage storage growth
    pruner: DagPruner,

    /// Parallel vertex validator for high throughput
    parallel_validator: ParallelValidator,

    /// Fallback: Simple round-robin leader schedule
    /// Used when VRF is not available
    leader_schedule: HashMap<Round, AuthorityId>,
}

impl DagConsensus {
    pub fn new(authority_id: AuthorityId, authorities: Vec<AuthorityId>) -> Self {
        let mut store = DagStore::new(authorities.clone());

        // Initialize VRF-based leader election
        let mut vrf_election = VrfLeaderElection::new();

        // Register all authorities with deterministic VRF keys (for demo)
        // In production, use proper key management (HSM, encrypted storage)
        for (i, auth) in authorities.iter().enumerate() {
            // Create deterministic secret key from index
            let mut secret = [0u8; 32];
            secret[0] = i as u8;
            secret[1] = (i >> 8) as u8;
            vrf_election.register_authority_bytes(auth.clone(), &secret);
        }

        // Fallback: Create simple round-robin leader schedule
        let mut leader_schedule = HashMap::new();
        for round in 0..1000 {
            let leader_idx = (round as usize) % authorities.len();
            leader_schedule.insert(round, authorities[leader_idx].clone());
        }

        // Create genesis vertices (round 0) for all authorities
        for authority in &authorities {
            let genesis_vertex = DagVertex::new(
                0,
                authority.clone(),
                vec![],        // No parents for genesis
                vec![],        // No transactions in genesis
                vec![0u8; 32], // Genesis state root
            );
            // Add genesis vertices (will succeed because round 0 has no parent requirements)
            let _ = store.add_vertex(genesis_vertex);
        }

        // Initialize Byzantine detector
        let mut byzantine_detector = ByzantineDetector::new();
        for authority in &authorities {
            byzantine_detector.init_authority(authority.clone());
        }

        // Initialize caches for performance
        let caches = DagCaches::new();

        // Initialize committee with all authorities
        let validator_infos: Vec<ValidatorInfo> = authorities
            .iter()
            .enumerate()
            .map(|(i, auth)| ValidatorInfo {
                authority_id: auth.clone(),
                stake: 100, // Equal stake for demo
                public_key: vec![i as u8; 32],
                network_address: format!("validator-{}", i),
                active: true,
            })
            .collect();
        let committee = Committee::new(0, validator_infos);

        // Initialize metrics
        let metrics = DagMetrics::new();

        // Initialize state synchronizer
        let state_sync = StateSynchronizer::new();

        // Initialize broadcaster with reasonable defaults
        let broadcaster = VertexBroadcaster::new(
            1000,                                  // max_batch_size
            std::time::Duration::from_millis(100), // max_batch_delay
        );

        // Initialize persistent store (optional - for production deployment)
        // In memory-only mode for testing, set to None
        let persistent_store = None;

        // Initialize pruner with conservative defaults
        let pruner = DagPruner::new(PruningConfig::default())
            .expect("Failed to create pruner with default config");

        // Initialize parallel validator with reasonable thread count
        let parallel_validator = ParallelValidator::new(ParallelValidatorConfig {
            num_workers: num_cpus::get().min(8),
            max_batch_size: 1000,
            parallel_sig_verify: true,
            queue_capacity: 10000,
        })
        .expect("Failed to create parallel validator");

        Self {
            store,
            authority_id,
            vrf_election,
            byzantine_detector,
            caches,
            committee,
            metrics,
            state_sync,
            broadcaster,
            persistent_store,
            pruner,
            parallel_validator,
            leader_schedule,
        }
    }

    /// Create a new vertex for current round
    pub fn create_vertex(
        &mut self,
        transactions: Vec<SignedTransaction>,
        state_root: Vec<u8>,
    ) -> Result<DagVertex> {
        let current_round = self.store.current_round();
        let next_round = current_round + 1;

        // Get parent vertices from current round
        let parents: Vec<VertexId> = self
            .store
            .get_vertices_in_round(current_round)
            .into_iter()
            .map(|v| v.id.clone())
            .collect();

        // Create vertex
        let vertex = DagVertex::new(
            next_round,
            self.authority_id.clone(),
            parents,
            transactions,
            state_root,
        );

        Ok(vertex)
    }

    /// Add vertex to the DAG
    pub fn add_vertex(&mut self, vertex: DagVertex) -> Result<()> {
        let vertex_id = vertex.id.clone();
        let author = vertex.author.clone();

        // 1. Verify author is in current committee
        if !self.committee.contains(&author) {
            anyhow::bail!("Vertex author '{}' is not in current committee", author);
        }

        // 1.5. Parallel validation check (structure, parents, etc.)
        let validation_results = self
            .parallel_validator
            .validate_batch(vec![vertex.clone()])?;

        if let Some(result) = validation_results.first()
            && !result.is_valid
        {
            anyhow::bail!(
                "Vertex validation failed: {}",
                result.error.as_deref().unwrap_or("unknown error")
            );
        }

        // 2. Check for Byzantine faults before adding
        let total_authorities = self.store.num_authorities();
        self.byzantine_detector.check_double_voting(&vertex)?;
        self.byzantine_detector
            .check_vertex_validity(&vertex, total_authorities)?;

        // 3. Add to store (takes ownership)
        self.store.add_vertex(vertex.clone())?;

        // 3.5. Persist vertex to disk if persistent store exists
        if let Some(persistent) = &self.persistent_store {
            let _ = persistent.put_vertex(&vertex);
        }

        // 4. Cache vertex for faster lookups (use Arc to avoid clone)
        self.caches.vertices.put(vertex_id.clone(), vertex.clone());

        // 5. Determine if this is a priority vertex
        let is_priority = self.vrf_election.is_leader(vertex.round, &author);
        
        // 6. Add to broadcaster and state sync (final clone for both)
        self.broadcaster.add_vertex(vertex.clone(), is_priority);
        self.state_sync.add_vertex(vertex);

        // 7. Check if pruning should run
        let current_round = self.store.current_round();
        if self.pruner.should_prune(current_round) {
            // Prune in background if persistent store exists
            if let Some(persistent) = &self.persistent_store {
                let latest_checkpoint = self.store.latest_checkpoint();
                if let Ok(prune_stats) =
                    self.pruner
                        .prune(persistent, current_round, Some(latest_checkpoint.sequence))
                {
                    // Invalidate cache entries for pruned vertices
                    self.parallel_validator
                        .invalidate_pruned_vertices(&prune_stats.pruned_vertex_ids);

                    // Also invalidate DAG cache for pruned vertices
                    for vertex_id in &prune_stats.pruned_vertex_ids {
                        self.caches.vertices.remove(vertex_id);
                    }

                    // Prune Byzantine detector old round data to prevent memory leak
                    self.byzantine_detector
                        .prune_old_rounds(prune_stats.cutoff_round);

                    // Prune VRF leader election cache to prevent memory leak
                    self.vrf_election.prune_old_rounds(prune_stats.cutoff_round);

                    // Prune state synchronizer old data
                    // Keep last 100 checkpoints and rounds matching retention policy
                    let keep_checkpoints = latest_checkpoint.sequence.saturating_sub(100);
                    self.state_sync
                        .prune_old_data(keep_checkpoints, prune_stats.cutoff_round);

                    tracing::debug!(
                        "Pruning completed: {} vertices, {} checkpoints pruned at round {}",
                        prune_stats.vertices_pruned,
                        prune_stats.checkpoints_pruned,
                        current_round
                    );
                }
            }
        }

        Ok(())
    }

    /// Try to commit vertices to a checkpoint
    /// Uses Bullshark-style leader-based ordering with VRF
    pub fn try_commit(&mut self) -> Result<Option<Checkpoint>> {
        let current_round = self.store.current_round();

        // Need at least 3 rounds to commit (leader round + 2 acknowledgment rounds)
        if current_round < 3 {
            return Ok(None);
        }

        let commit_round = current_round - 2;

        // Try VRF-based leader election first
        let leader_id = if let Some(vrf_leader) = self.vrf_election.elect_leader(commit_round) {
            vrf_leader
        } else {
            // Fallback to round-robin if VRF not available
            self.leader_schedule
                .get(&commit_round)
                .ok_or_else(|| anyhow::anyhow!("No leader for round"))?
                .clone()
        };

        // Find leader's vertex in commit round
        let leader_vertex = self
            .store
            .get_vertices_in_round(commit_round)
            .into_iter()
            .find(|v| v.author == *leader_id);

        if let Some(leader_vertex) = leader_vertex {
            // Check if leader vertex has enough support (2f+1 vertices in next round reference it)
            let next_round_vertices = self.store.get_vertices_in_round(commit_round + 1);
            let support_count = next_round_vertices
                .iter()
                .filter(|v| v.parents.contains(&leader_vertex.id))
                .count();

            let f = (self.store.num_authorities() - 1) / 3;
            let quorum = 2 * f + 1;

            if support_count >= quorum {
                // Commit! Collect all uncommitted vertices up to and including leader vertex
                let vertices_to_commit =
                    self.collect_vertices_to_commit(leader_vertex.id.clone())?;

                // Order transactions from vertices
                let mut all_transactions = Vec::new();
                for vertex_id in &vertices_to_commit {
                    if let Some(vertex) = self.store.get_vertex(vertex_id) {
                        all_transactions.extend(vertex.transactions.clone());
                    }
                }

                // Create checkpoint
                let latest = self.store.latest_checkpoint();
                let checkpoint = Checkpoint::new(
                    latest.sequence + 1,
                    vertices_to_commit.clone(),
                    all_transactions,
                    leader_vertex.metadata.state_root.clone(),
                    latest.hash(),
                );

                // Add checkpoint to store
                self.store.add_checkpoint(checkpoint.clone())?;

                // Persist checkpoint to disk if persistent store exists
                if let Some(persistent) = &self.persistent_store {
                    let _ = persistent.put_checkpoint(&checkpoint);

                    // Persist committed vertices
                    for vertex_id in &vertices_to_commit {
                        if let Some(vertex) = self.store.get_vertex(vertex_id) {
                            let _ = persistent.put_vertex(vertex);
                        }
                    }
                }

                // Update state synchronizer with new checkpoint
                self.state_sync.add_checkpoint(checkpoint.clone());

                return Ok(Some(checkpoint));
            }
        }

        Ok(None)
    }

    /// Collect all vertices that should be committed (topological sort)
    fn collect_vertices_to_commit(&self, leader_vertex_id: VertexId) -> Result<Vec<VertexId>> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = vec![leader_vertex_id];

        while let Some(vertex_id) = stack.pop() {
            if visited.contains(&vertex_id) {
                continue;
            }

            if let Some(vertex) = self.store.get_vertex(&vertex_id) {
                // Skip if already checkpointed
                if vertex.metadata.is_checkpoint {
                    continue;
                }

                // Add parents to stack first (depth-first)
                for parent_id in &vertex.parents {
                    if !visited.contains(parent_id) {
                        stack.push(parent_id.clone());
                    }
                }

                visited.insert(vertex_id.clone());
                result.push(vertex_id);
            }
        }

        Ok(result)
    }

    /// Get current DAG store (read-only)
    pub fn store(&self) -> &DagStore {
        &self.store
    }

    /// Get mutable DAG store
    pub fn store_mut(&mut self) -> &mut DagStore {
        &mut self.store
    }

    /// Generate VRF output for a round
    pub fn generate_vrf(&self, round: Round) -> Result<VrfOutput> {
        self.vrf_election.generate_vrf(round, &self.authority_id)
    }

    /// Submit VRF output from another authority
    pub fn submit_vrf(&mut self, vrf: VrfOutput) -> Result<()> {
        // Get public key for verification (in production, verify properly)
        if let Some(pk) = self.vrf_election.get_public_key(&vrf.authority) {
            if !vrf.verify(pk) {
                anyhow::bail!("Invalid VRF proof");
            }
        } else {
            anyhow::bail!("Unknown authority: {}", vrf.authority);
        }

        self.vrf_election.add_vrf(vrf);
        Ok(())
    }

    /// Check if this authority is the leader for a round (using VRF)
    pub fn is_vrf_leader(&self, round: Round) -> bool {
        self.vrf_election.is_leader(round, &self.authority_id)
    }

    /// Get elected leader for a round (VRF-based)
    pub fn get_vrf_leader(&self, round: Round) -> Option<AuthorityId> {
        self.vrf_election.elect_leader(round)
    }

    /// Get Byzantine detector (read-only access)
    pub fn byzantine_detector(&self) -> &ByzantineDetector {
        &self.byzantine_detector
    }

    /// Get mutable Byzantine detector
    pub fn byzantine_detector_mut(&mut self) -> &mut ByzantineDetector {
        &mut self.byzantine_detector
    }

    /// Get caches (read-only)
    pub fn caches(&self) -> &DagCaches {
        &self.caches
    }

    /// Get metrics (read-only)
    pub fn metrics(&self) -> &DagMetrics {
        &self.metrics
    }

    /// Get committee (read-only)
    pub fn committee(&self) -> &Committee {
        &self.committee
    }

    /// Get state synchronizer
    pub fn state_sync(&self) -> &StateSynchronizer {
        &self.state_sync
    }

    /// Get mutable state synchronizer
    pub fn state_sync_mut(&mut self) -> &mut StateSynchronizer {
        &mut self.state_sync
    }

    /// Get broadcaster
    pub fn broadcaster(&self) -> &VertexBroadcaster {
        &self.broadcaster
    }

    /// Get mutable broadcaster
    pub fn broadcaster_mut(&mut self) -> &mut VertexBroadcaster {
        &mut self.broadcaster
    }

    /// Get persistent store (optional)
    pub fn persistent_store(&self) -> Option<&PersistentDagStore> {
        self.persistent_store.as_ref()
    }

    /// Get mutable persistent store
    pub fn persistent_store_mut(&mut self) -> Option<&mut PersistentDagStore> {
        self.persistent_store.as_mut()
    }

    /// Get pruner
    pub fn pruner(&self) -> &DagPruner {
        &self.pruner
    }

    /// Get mutable pruner
    pub fn pruner_mut(&mut self) -> &mut DagPruner {
        &mut self.pruner
    }

    /// Get parallel validator
    pub fn parallel_validator(&self) -> &ParallelValidator {
        &self.parallel_validator
    }

    /// Get mutable parallel validator
    pub fn parallel_validator_mut(&mut self) -> &mut ParallelValidator {
        &mut self.parallel_validator
    }
}

#[cfg(test)]
mod tests {
    use kanari_types::transaction::Transaction;

    use super::*;

    #[test]
    fn test_dag_vertex_creation() {
        let vertex = DagVertex::new(
            1,
            "authority1".to_string(),
            vec![vec![0u8; 32]],
            vec![],
            vec![0u8; 32],
        );

        assert_eq!(vertex.round, 1);
        assert_eq!(vertex.author, "authority1");
        assert!(!vertex.id.is_empty());
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

        // Create genesis vertices (round 0)
        let vertex0 = DagVertex::new(0, "auth1".to_string(), vec![], vec![], vec![0u8; 32]);
        store.add_vertex(vertex0.clone()).unwrap();

        assert_eq!(store.current_round(), 0);
        assert!(store.get_vertex(&vertex0.id).is_some());
    }

    #[test]
    fn test_checkpoint_creation() {
        let checkpoint = Checkpoint::genesis();
        assert_eq!(checkpoint.sequence, 0);
        assert!(checkpoint.transactions.is_empty());
    }

    #[test]
    fn test_checkpoint_config_default() {
        let config = CheckpointConfig::default();
        assert_eq!(config.min_rounds, 10);
        assert_eq!(config.max_rounds, 100);
        assert_eq!(config.min_vertices, 100);
        assert_eq!(config.max_vertices, 10000);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_checkpoint_config_conservative() {
        let config = CheckpointConfig::conservative();
        assert!(config.min_rounds < CheckpointConfig::default().min_rounds);
        assert!(config.max_rounds < CheckpointConfig::default().max_rounds);
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
        for i in 0..11 {
            // Create unique transaction to ensure unique vertex ID
            let transaction = Transaction::Transfer {
                from: format!("sender{}", i),
                to: "receiver".to_string(),
                amount: i,
                gas_limit: 1000,
                gas_price: 1,
                sequence_number: i,
            };
            let tx = SignedTransaction::new(transaction);

            let vertex = DagVertex::new(
                0, // Round 0
                "auth1".to_string(),
                vec![],
                vec![tx],
                vec![i as u8; 32],
            );
            if let Err(e) = store.add_vertex(vertex) {
                eprintln!("Failed to add vertex {}: {}", i, e);
            }
        }

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
        for i in 0..10 {
            // Create unique transaction for each vertex
            let transaction = Transaction::Transfer {
                from: format!("sender{}", i),
                to: "receiver".to_string(),
                amount: i,
                gas_limit: 1000,
                gas_price: 1,
                sequence_number: i,
            };
            let tx = SignedTransaction::new(transaction);

            let vertex = DagVertex::new(
                0, // Round 0
                "auth1".to_string(),
                vec![],
                vec![tx],
                vec![i as u8; 32],
            );
            store.add_vertex(vertex).ok();
        }

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
        for i in 0..15 {
            // Create unique transaction for each vertex
            let transaction = Transaction::Transfer {
                from: format!("sender{}", i),
                to: "receiver".to_string(),
                amount: i,
                gas_limit: 1000,
                gas_price: 1,
                sequence_number: i,
            };
            let tx = SignedTransaction::new(transaction);

            let vertex = DagVertex::new(
                0, // Round 0
                "auth1".to_string(),
                vec![],
                vec![tx],
                vec![i as u8; 32],
            );
            store.add_vertex(vertex).ok();
        }

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
        store.current_round = 1; // Simulate at least 1 round
        for i in 0..5 {
            // Create unique transaction for each vertex
            let transaction = Transaction::Transfer {
                from: format!("sender{}", i),
                to: "receiver".to_string(),
                amount: i,
                gas_limit: 1000,
                gas_price: 1,
                sequence_number: i,
            };
            let tx = SignedTransaction::new(transaction);

            let vertex = DagVertex::new(
                0, // Round 0
                "auth1".to_string(),
                vec![],
                vec![tx],
                vec![i as u8; 32],
            );
            store.add_vertex(vertex).ok();
        }

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

        // Try invalid config
        let invalid_config = CheckpointConfig {
            min_rounds: 100,
            max_rounds: 50,
            min_vertices: 100,
            max_vertices: 1000,
        };
        assert!(store.set_checkpoint_config(invalid_config).is_err());
    }
}
