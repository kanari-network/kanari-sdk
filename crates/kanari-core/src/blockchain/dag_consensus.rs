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

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

use super::SignedTransaction;
use super::byzantine_detector::ByzantineDetector;
use super::vrf_leader::{VrfLeaderElection, VrfOutput};
use kanari_crypto::hash_data_blake3;

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
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
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
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
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
}

impl DagStore {
    pub fn new(authorities: Vec<AuthorityId>) -> Self {
        let genesis_checkpoint = Checkpoint::genesis();

        Self {
            vertices: HashMap::new(),
            vertices_by_round: HashMap::new(),
            vertices_by_authority: HashMap::new(),
            checkpoints: vec![genesis_checkpoint],
            pending_vertices: VecDeque::new(),
            current_round: 0,
            authorities: authorities.into_iter().collect(),
        }
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
                if let Some(parent) = self.vertices.get(parent_id) {
                    if parent.round != vertex.round - 1 {
                        anyhow::bail!("Parent from wrong round");
                    }
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
    pub fn latest_checkpoint(&self) -> &Checkpoint {
        self.checkpoints.last().unwrap()
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

        self.checkpoints.push(checkpoint);
        Ok(())
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

    /// Fallback: Simple round-robin leader schedule
    /// Used when VRF is not available
    leader_schedule: HashMap<Round, AuthorityId>,
}

impl DagConsensus {
    pub fn new(authority_id: AuthorityId, authorities: Vec<AuthorityId>) -> Self {
        let mut store = DagStore::new(authorities.clone());

        // Initialize VRF-based leader election
        let mut vrf_election = VrfLeaderElection::new();

        // Register all authorities with random secrets (for demo)
        // In production, use proper key management
        for (i, auth) in authorities.iter().enumerate() {
            let secret = format!("secret_{}", i).into_bytes();
            vrf_election.register_authority(auth.clone(), secret);
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

        Self {
            store,
            authority_id,
            vrf_election,
            byzantine_detector,
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
        // Check for Byzantine faults before adding
        let total_authorities = self.store.num_authorities();
        self.byzantine_detector.check_double_voting(&vertex)?;
        self.byzantine_detector
            .check_vertex_validity(&vertex, total_authorities)?;

        self.store.add_vertex(vertex)
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
                let checkpoint = Checkpoint::new(
                    self.store.latest_checkpoint().sequence + 1,
                    vertices_to_commit,
                    all_transactions,
                    leader_vertex.metadata.state_root.clone(),
                    self.store.latest_checkpoint().hash(),
                );

                self.store.add_checkpoint(checkpoint.clone())?;
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
        // Verify VRF (in production, verify with public key)
        if !vrf.verify(&[]) {
            anyhow::bail!("Invalid VRF proof");
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
}

#[cfg(test)]
mod tests {
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
}
