// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! State Synchronization for DAG Consensus
//!
//! Enables nodes to:
//! - Join the network and catch up to current state
//! - Recover from crashes
//! - Sync missing vertices efficiently
//!
//! Inspired by Sui's checkpoint-based sync mechanism.

use crate::consensus::Committee;
use ahash::HashSet;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;

use super::{AuthorityId, Checkpoint, DagVertex, Round, VertexId};

const MAX_SYNC_CHECKPOINTS: usize = 100;
const MAX_SYNC_VERTICES: usize = 5000;

fn unix_timestamp_secs(log_context: &str) -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(e) => {
            tracing::warn!(
                "[SYNC] System clock error while reading {} timestamp: {}",
                log_context,
                e
            );
            0
        }
    }
}

/// State sync request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub requester: AuthorityId,
    pub last_checkpoint: u64,
    pub last_round: Round,
    pub missing_vertices: Vec<VertexId>,
}

/// State sync response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub checkpoints: Vec<Checkpoint>,
    pub vertices: Vec<DagVertex>,
    pub current_round: Round,
    pub state_root: Vec<u8>,
}

/// Sync progress tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProgress {
    pub start_checkpoint: u64,
    pub target_checkpoint: u64,
    pub current_checkpoint: u64,
    pub synced_vertices: usize,
    pub total_vertices: usize,
    pub started_at: u64,
}

impl SyncProgress {
    pub fn progress_percentage(&self) -> f64 {
        if self.total_vertices == 0 {
            return 100.0;
        }
        (self.synced_vertices as f64 / self.total_vertices as f64) * 100.0
    }

    pub fn is_complete(&self) -> bool {
        self.current_checkpoint >= self.target_checkpoint
            && self.synced_vertices >= self.total_vertices
    }
}

/// State synchronizer
pub struct StateSynchronizer {
    checkpoints: BTreeMap<u64, Checkpoint>,
    vertices_by_round: BTreeMap<Round, Vec<Arc<DagVertex>>>,
    vertex_lookup: BTreeMap<VertexId, (Round, usize)>,
    latest_checkpoint: u64,
    latest_round: Round,
    sync_progress: Option<SyncProgress>,

    /// Orphan vertices buffer (waiting for parents) - HashMap for O(1) lookup
    orphan_buffer: HashMap<VertexId, Arc<DagVertex>>,
    /// Parent tracking for orphans - HashMap for O(1) lookup
    waiting_for: HashMap<VertexId, Vec<VertexId>>, // parent_id -> list of orphan children waiting
    /// Track insertion order for FIFO eviction (VecDeque for efficient pop_front)
    orphan_insertion_order: VecDeque<VertexId>,
}

impl StateSynchronizer {
    fn unix_timestamp_secs() -> u64 {
        unix_timestamp_secs("state-sync")
    }

    fn checkpoint_state_root(&self, sequence: u64) -> Option<Vec<u8>> {
        self.checkpoints
            .get(&sequence)
            .map(|checkpoint| checkpoint.state_root.clone())
    }

    fn remove_orphan(&mut self, orphan_id: VertexId) {
        self.orphan_buffer.remove(&orphan_id);
        self.orphan_insertion_order.retain(|id| id != &orphan_id);
        self.waiting_for.retain(|_, children| {
            children.retain(|child_id| child_id != &orphan_id);
            !children.is_empty()
        });
    }

    fn collect_vertices_after_round_limited(
        &self,
        last_round: Round,
        max_vertices: usize,
    ) -> Vec<DagVertex> {
        let mut vertices = Vec::new();
        let start_round = last_round.saturating_add(1);
        if start_round > self.latest_round {
            return vertices;
        }
        for round in start_round..=self.latest_round {
            if let Some(round_vertices) = self.vertices_by_round.get(&round) {
                for vertex in round_vertices {
                    if vertices.len() >= max_vertices {
                        return vertices;
                    }
                    vertices.push((**vertex).clone());
                }
            }
        }
        vertices
    }

    fn find_vertex(&self, vertex_id: &VertexId) -> Option<DagVertex> {
        // Also search Orphan Buffer in case others request it
        if let Some(orphan) = self.orphan_buffer.get(vertex_id) {
            return Some((**orphan).clone());
        }

        let (round, index) = *self.vertex_lookup.get(vertex_id)?;
        self.vertices_by_round
            .get(&round)
            .and_then(|vertices| vertices.get(index))
            .map(|vertex| (**vertex).clone())
    }

    pub fn new() -> Self {
        let mut checkpoints = BTreeMap::new();
        checkpoints.insert(0, Checkpoint::genesis());

        Self {
            checkpoints,
            vertices_by_round: BTreeMap::new(),
            vertex_lookup: BTreeMap::new(),
            latest_checkpoint: 0,
            latest_round: 0,
            sync_progress: None,
            orphan_buffer: HashMap::new(),
            waiting_for: HashMap::new(),
            orphan_insertion_order: VecDeque::new(),
        }
    }

    pub fn add_checkpoint(&mut self, checkpoint: Checkpoint) {
        let seq = checkpoint.sequence;
        self.checkpoints.insert(seq, checkpoint);
        if seq > self.latest_checkpoint {
            self.latest_checkpoint = seq;
        }
    }

    /// Add vertex (returns count of successfully added vertices, including processed orphans)
    pub fn add_vertex(&mut self, vertex: DagVertex) -> usize {
        self.add_vertex_arc(Arc::new(vertex))
    }

    /// Add vertex using Arc (Orphan Block Resolution Engine)
    pub fn add_vertex_arc(&mut self, vertex: Arc<DagVertex>) -> usize {
        if vertex.verify().is_err() {
            return 0;
        }

        let vertex_id = vertex.id;

        if self.vertex_lookup.contains_key(&vertex_id)
            || self.orphan_buffer.contains_key(&vertex_id)
        {
            return 0;
        }

        // FIX 5: Prevent Cyclic DAG (A -> B -> A) and abnormal round references
        for parent_id in &vertex.parents {
            if let Some(parent) = self.find_vertex(parent_id)
                && parent.round >= vertex.round
            {
                tracing::warn!(
                    "Security Warning: Rejecting cyclic or invalid round progression. Vertex {}, Parent {}",
                    vertex.round,
                    parent.round
                );
                return 0; // Immediately reject if child round is less than or equal to parent
            }
        }

        // Find which parents are missing
        let missing_parents: Vec<_> = vertex
            .parents
            .iter()
            .filter(|p| !self.vertex_lookup.contains_key(*p))
            .copied()
            .collect();

        // If parents are missing, store in Orphan Buffer instead of discarding
        if vertex.round > 0 && !missing_parents.is_empty() {
            // --- FIX #14: CRITICAL - Proper FIFO eviction for orphan buffer ---
            // Previously used keys().next() which evicts by hash order (not insertion time)
            // causing memory leaks where old orphans with high hashes stay forever
            const MAX_ORPHAN_SIZE: usize = 50_000;

            if self.orphan_buffer.len() >= MAX_ORPHAN_SIZE {
                // FIX #14: Evict oldest orphan by insertion order (FIFO), not hash order
                if let Some(oldest_key) = self.orphan_insertion_order.pop_front() {
                    self.remove_orphan(oldest_key);
                }
            }

            // Track insertion order for proper FIFO eviction
            self.orphan_insertion_order.push_back(vertex_id);
            self.orphan_buffer.insert(vertex_id, Arc::clone(&vertex));

            for p in missing_parents {
                let waiters = self.waiting_for.entry(p).or_default();
                // Limit children waiting for same parent to prevent infinite RAM usage from spam
                if waiters.len() < 1000 {
                    waiters.push(vertex_id);
                } else {
                    tracing::warn!("Max waiters limit reached for parent: {}", hex::encode(p));
                }
            }
            return 0; // Not yet added to main chain
        }

        // If all parents present, add to main chain and clear buffer
        self.insert_to_main_dag(vertex)
    }

    /// Insert into main chain and recursively release waiting child blocks
    fn insert_to_main_dag(&mut self, initial_vertex: Arc<DagVertex>) -> usize {
        let mut added_count = 0;
        let mut stack = vec![initial_vertex]; // Use regular stack instead of recursion

        while let Some(vertex) = stack.pop() {
            let round = vertex.round;
            let vertex_id = vertex.id;

            let entry = self.vertices_by_round.entry(round).or_default();
            let idx = entry.len();
            entry.push(Arc::clone(&vertex));
            self.vertex_lookup.insert(vertex_id, (round, idx));

            if round > self.latest_round {
                self.latest_round = round;
            }
            added_count += 1;

            if let Some(children) = self.waiting_for.remove(&vertex_id) {
                for child_id in children {
                    if let Some(child) = self.orphan_buffer.get(&child_id).cloned() {
                        // Check if this child's other parents have arrived yet
                        let still_missing = child
                            .parents
                            .iter()
                            .any(|p| !self.vertex_lookup.contains_key(p));

                        if !still_missing {
                            self.remove_orphan(child_id);
                            // Push child onto stack instead of making recursive call immediately
                            stack.push(child);
                        }
                    }
                }
            }
        }

        added_count
    }

    pub fn handle_sync_request(&self, request: &SyncRequest) -> Result<SyncResponse> {
        // FIX #9: CRITICAL - Limit missing_vertices to prevent DoS attack
        // Previously allowed unlimited missing_vertices causing O(N) lookups and memory exhaustion
        const MAX_MISSING_VERTICES: usize = 10_000;

        if request.missing_vertices.len() > MAX_MISSING_VERTICES {
            tracing::warn!(
                "[Security] SyncRequest from {} has {} missing_vertices (max: {}) - rejecting",
                request.requester,
                request.missing_vertices.len(),
                MAX_MISSING_VERTICES
            );
            return Err(anyhow!(
                "SyncRequest rejected: too many missing_vertices ({}, max: {})",
                request.missing_vertices.len(),
                MAX_MISSING_VERTICES
            ));
        }

        let checkpoint_start = request.last_checkpoint.saturating_add(1);
        let checkpoints: Vec<Checkpoint> = if checkpoint_start > self.latest_checkpoint {
            Vec::new()
        } else {
            (checkpoint_start..=self.latest_checkpoint)
                .take(MAX_SYNC_CHECKPOINTS)
                .filter_map(|seq| self.checkpoints.get(&seq).cloned())
                .collect()
        };

        let mut vertices =
            self.collect_vertices_after_round_limited(request.last_round, MAX_SYNC_VERTICES);
        let mut seen: HashSet<VertexId> = vertices.iter().map(|v| v.id).collect();

        for vertex_id in &request.missing_vertices {
            if vertices.len() >= MAX_SYNC_VERTICES {
                break;
            }
            if seen.contains(vertex_id) {
                continue;
            }
            if let Some(vertex) = self.find_vertex(vertex_id) {
                seen.insert(vertex.id);
                vertices.push(vertex);
            }
        }
        let response_state_root = checkpoints
            .last()
            .map(|c| c.state_root.clone())
            .or_else(|| self.checkpoint_state_root(request.last_checkpoint))
            .ok_or_else(|| {
                anyhow!(
                    "SyncRequest references unknown checkpoint {}",
                    request.last_checkpoint
                )
            })?;

        Ok(SyncResponse {
            checkpoints,
            vertices,
            current_round: self.latest_round,
            state_root: response_state_root,
        })
    }

    // FIX 2: Add parameter `committee: &Committee` to fetch Public Key for signature verification
    pub fn apply_sync_response(
        &mut self,
        response: SyncResponse,
        committee: &Committee,
    ) -> Result<()> {
        // FIX #7: CRITICAL - Do NOT trust state_root from remote response for validation!
        // Previously used response.checkpoints.last().state_root to validate response.state_root
        // This is circular validation - attacker can send fake state_root with matching checkpoint
        //
        // Correct approach: Validate each vertex individually via signature verification,
        // then compute local state root after applying vertices. Never trust remote state_root.

        let start_checkpoint = self.latest_checkpoint;
        let total_vertices = response.vertices.len();

        self.sync_progress = Some(SyncProgress {
            start_checkpoint,
            target_checkpoint: response
                .checkpoints
                .last()
                .map(|c| c.sequence)
                .unwrap_or(start_checkpoint),
            current_checkpoint: start_checkpoint,
            synced_vertices: 0,
            total_vertices,
            started_at: Self::unix_timestamp_secs(),
        });

        for checkpoint in response.checkpoints {
            // FIX #4: CRITICAL - Do NOT trust checkpoints from untrusted peers!
            // Checkpoints represent committed state and must be calculated locally through consensus.
            // Accepting remote checkpoints without QuorumCertificate verification allows attackers
            // to inject fake State Roots and corrupt the node's view of history.
            //
            // Proper approach: Only sync vertices, then let local try_commit() generate checkpoints.
            // Alternative (Fast Sync): Require QuorumCertificate with 2f+1 validator signatures.
            //
            // For now, we skip remote checkpoints to maintain trustless operation.
            tracing::debug!(
                "Skipping remote checkpoint {} during sync (trustless mode)",
                checkpoint.sequence
            );
        }

        for vertex in response.vertices {
            // --- FIX 2: Prevent State Poisoning by verifying Signature before accepting block ---
            if let Some(validator) = committee.get_validator(&vertex.author) {
                // Convert Public Key Bytes to VerifyingKey
                let Ok(pub_key_bytes) = <[u8; 32]>::try_from(validator.public_key.clone()) else {
                    tracing::error!(
                        "[Security] Invalid public key length for synced vertex author {}",
                        vertex.author
                    );
                    continue;
                };

                if let Ok(pub_key) = ed25519_dalek::VerifyingKey::from_bytes(&pub_key_bytes) {
                    // Call signature verification function (Zero-copy optimization) from ParallelValidator
                    if crate::consensus::parallel_validator::ParallelValidator::verify_vertex_signature(&vertex, &pub_key).is_err() {
                        tracing::error!("[Security] Invalid signature in synced vertex: {}", hex::encode(vertex.id));
                        continue; // Immediately reject malicious block
                    }
                } else {
                    continue; // Public Key is malformed
                }
            } else {
                tracing::warn!(
                    "[Security] Unknown author {} in synced vertex",
                    vertex.author
                );
                continue; // Skip block from unknown Committee member
            }
            // ----------------------------------------------------------------------

            let added_count = self.add_vertex(vertex);
            if let Some(ref mut progress) = self.sync_progress {
                progress.synced_vertices += added_count;
            }
        }

        // FIX #7: After syncing vertices, verify state consistency locally
        // Do NOT use response.state_root - it cannot be trusted
        tracing::info!(
            "Sync complete: {} vertices applied ({} orphans), current round: {}. \
             Local state root should be recomputed from synced vertices.",
            self.vertices_by_round.values().map(Vec::len).sum::<usize>(),
            self.orphan_buffer.len(),
            self.latest_round
        );

        Ok(())
    }

    pub fn prune_old_data(&mut self, before_checkpoint: u64, before_round: Round) {
        self.checkpoints.retain(|seq, _| *seq >= before_checkpoint);
        self.vertices_by_round
            .retain(|round, _| *round >= before_round);
        self.vertex_lookup.clear();
        for (round, vertices) in &self.vertices_by_round {
            for (idx, vertex) in vertices.iter().enumerate() {
                self.vertex_lookup.insert(vertex.id, (*round, idx));
            }
        }

        // Prune orphans that are too old and have never had their parent arrive (prevent memory leak)
        let stale_orphans: Vec<_> = self
            .orphan_buffer
            .iter()
            .filter(|(_, v)| v.round < before_round)
            .map(|(id, _)| *id)
            .collect();
        for orphan_id in stale_orphans {
            self.remove_orphan(orphan_id);
        }
        let valid_orphans: HashSet<_> = self.orphan_buffer.keys().copied().collect();
        self.waiting_for.retain(|_, children| {
            children.retain(|c| valid_orphans.contains(c));
            !children.is_empty()
        });

        tracing::debug!(
            "Pruned StateSynchronizer: {} checkpoints, {} rounds, {} orphans remaining",
            self.checkpoints.len(),
            self.vertices_by_round.len(),
            self.orphan_buffer.len()
        );
    }
}

impl Default for StateSynchronizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Fast state sync using checkpoints (skip intermediate vertices)
#[cfg(test)]
pub struct FastSync {
    checkpoints: Vec<Checkpoint>,
}

#[cfg(test)]
impl FastSync {
    pub fn new(_checkpoint_interval: Round) -> Self {
        Self {
            checkpoints: vec![Checkpoint::genesis()],
        }
    }

    pub fn add_checkpoint(&mut self, checkpoint: Checkpoint) {
        self.checkpoints.push(checkpoint);
    }
}

#[cfg(test)]
mod tests {
    use crate::consensus::{Ed25519Keypair, ValidatorInfo};

    use super::*;

    #[test]
    fn test_sync_progress() {
        let progress = SyncProgress {
            start_checkpoint: 0,
            target_checkpoint: 10,
            current_checkpoint: 5,
            synced_vertices: 50,
            total_vertices: 100,
            started_at: 0,
        };

        assert_eq!(progress.progress_percentage(), 50.0);
        assert!(!progress.is_complete());
    }

    #[test]
    fn test_state_synchronizer() {
        let mut sync = StateSynchronizer::new();

        let mut checkpoint = Checkpoint::genesis();
        checkpoint.sequence = 1;
        sync.add_checkpoint(checkpoint);

        assert_eq!(sync.latest_checkpoint, 1);
        assert!(sync.checkpoints.contains_key(&1));
    }

    #[test]
    fn test_sync_request_response() {
        let mut sync = StateSynchronizer::new();

        let vertex =
            DagVertex::new_for_test(1, "auth1".to_string(), vec![], vec![], vec![0u8; 32], 0);
        sync.add_vertex(vertex);

        let request = SyncRequest {
            requester: "auth2".to_string(),
            last_checkpoint: 0,
            last_round: 0,
            missing_vertices: vec![],
        };

        let response = sync.handle_sync_request(&request).unwrap();
        assert_eq!(response.current_round, 1);
        assert_eq!(response.vertices.len(), 1);
    }

    #[test]
    fn test_sync_request_missing_vertices_included() {
        let mut sync = StateSynchronizer::new();

        let vertex_round_1 =
            DagVertex::new_for_test(1, "auth1".to_string(), vec![], vec![], vec![1u8; 32], 0);
        let vertex_round_2 =
            DagVertex::new_for_test(2, "auth2".to_string(), vec![], vec![], vec![2u8; 32], 0);
        let missing_id = vertex_round_1.id;

        sync.add_vertex(vertex_round_1);
        sync.add_vertex(vertex_round_2);

        let request = SyncRequest {
            requester: "auth3".to_string(),
            last_checkpoint: 0,
            last_round: 2,
            missing_vertices: vec![missing_id],
        };

        let response = sync.handle_sync_request(&request).unwrap();
        assert_eq!(response.vertices.len(), 1);
        assert_eq!(response.vertices[0].id, missing_id);
    }

    #[test]
    fn test_apply_sync_response() {
        let mut sync = StateSynchronizer::new();

        // 1. Create Keypair and Committee for testing
        let keypair = Ed25519Keypair::generate();
        let pub_key_bytes = keypair.public().to_bytes().to_vec();

        let validator = ValidatorInfo {
            authority_id: "auth1".to_string(),
            public_key: pub_key_bytes,
            network_address: "127.0.0.1:9000".to_string(),
            active: true,
        };
        let committee = Committee::new(0, vec![validator]);

        // 2. Create temporary Vertex
        let mut vertex =
            DagVertex::new_for_test(1, "auth1".to_string(), vec![], vec![], vec![0u8; 32], 0);

        // 3. Create mock signature to pass ParallelValidator::verify_vertex_signature
        vertex.signature = keypair.sign(&vertex.id);

        // 4. Create SyncResponse
        let response = SyncResponse {
            checkpoints: vec![],
            vertices: vec![vertex],
            current_round: 1,
            state_root: Checkpoint::genesis().state_root,
        };

        // 5. Test by passing &committee
        assert!(sync.apply_sync_response(response, &committee).is_ok());
        assert_eq!(sync.latest_round, 1);
    }
    #[test]
    fn test_orphan_block_resolution() {
        let mut sync = StateSynchronizer::new();

        let parent =
            DagVertex::new_for_test(1, "auth1".to_string(), vec![], vec![], vec![0u8; 32], 0);
        let child = DagVertex::new_for_test(
            2,
            "auth2".to_string(),
            vec![parent.id],
            vec![],
            vec![0u8; 32],
            0,
        );

        // Add child block first (network reordering scenario)
        assert_eq!(sync.add_vertex(child), 0); // Not yet in main chain 
        assert_eq!(sync.orphan_buffer.len(), 1); // But stored in orphan buffer
        assert_eq!(sync.latest_round, 0);

        // When parent arrives later
        assert_eq!(sync.add_vertex(parent), 2); // Unlocks child too, returns 2 blocks
        assert_eq!(sync.orphan_buffer.len(), 0); // Orphan buffer is empty
        assert_eq!(sync.latest_round, 2); // Synced through to child's round immediately
    }

    #[test]
    fn test_fast_sync() {
        let mut fast_sync = FastSync::new(10);

        let mut checkpoint = Checkpoint::genesis();
        checkpoint.sequence = 1;
        fast_sync.add_checkpoint(checkpoint);

        assert_eq!(fast_sync.checkpoints.len(), 2);
    }

    #[test]
    fn test_sync_response_is_paginated() {
        let mut source = StateSynchronizer::new();
        for seq in 1..=150 {
            let mut checkpoint = Checkpoint::genesis();
            checkpoint.sequence = seq;
            source.add_checkpoint(checkpoint);
        }
        for i in 0..6000u64 {
            source.add_vertex(DagVertex::new_for_test(
                1 + (i / 1000),
                format!("auth{}", i % 4),
                Vec::new(),
                Vec::new(),
                vec![i as u8; 32],
                i,
            ));
        }
        let response = source
            .handle_sync_request(&SyncRequest {
                requester: "auth_sync".to_string(),
                last_checkpoint: 0,
                last_round: 0,
                missing_vertices: Vec::new(),
            })
            .unwrap();
        assert!(response.checkpoints.len() <= MAX_SYNC_CHECKPOINTS);
        assert!(response.vertices.len() <= MAX_SYNC_VERTICES);
    }
}
