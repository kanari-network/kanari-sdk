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

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{AuthorityId, Checkpoint, DagVertex, Round, VertexId};

/// State sync request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    /// Requesting node's authority ID
    pub requester: AuthorityId,

    /// Last checkpoint the requester has
    pub last_checkpoint: u64,

    /// Last round the requester has
    pub last_round: Round,

    /// Missing vertex IDs (if known)
    pub missing_vertices: Vec<VertexId>,
}

/// State sync response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    /// Checkpoints to sync (from last_checkpoint + 1 to current)
    pub checkpoints: Vec<Checkpoint>,

    /// Vertices needed to reach current state
    pub vertices: Vec<DagVertex>,

    /// Current round of the network
    pub current_round: Round,

    /// State root hash
    pub state_root: Vec<u8>,
}

/// Sync progress tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProgress {
    /// Starting checkpoint
    pub start_checkpoint: u64,

    /// Target checkpoint
    pub target_checkpoint: u64,

    /// Current checkpoint
    pub current_checkpoint: u64,

    /// Synced vertices count
    pub synced_vertices: usize,

    /// Total vertices to sync
    pub total_vertices: usize,

    /// Sync started timestamp
    pub started_at: u64,
}

impl SyncProgress {
    /// Calculate progress percentage
    pub fn progress_percentage(&self) -> f64 {
        if self.total_vertices == 0 {
            return 100.0;
        }
        (self.synced_vertices as f64 / self.total_vertices as f64) * 100.0
    }

    /// Check if sync is complete
    pub fn is_complete(&self) -> bool {
        self.current_checkpoint >= self.target_checkpoint
            && self.synced_vertices >= self.total_vertices
    }
}

/// State synchronizer
pub struct StateSynchronizer {
    /// Checkpoints stored locally
    checkpoints: HashMap<u64, Checkpoint>,

    /// Vertices by round
    vertices_by_round: HashMap<Round, Vec<DagVertex>>,

    /// Latest checkpoint sequence
    latest_checkpoint: u64,

    /// Latest round
    latest_round: Round,

    /// Sync progress
    sync_progress: Option<SyncProgress>,
}

impl StateSynchronizer {
    /// Create new state synchronizer
    pub fn new() -> Self {
        // Add genesis checkpoint
        let mut checkpoints = HashMap::new();
        checkpoints.insert(0, Checkpoint::genesis());

        Self {
            checkpoints,
            vertices_by_round: HashMap::new(),
            latest_checkpoint: 0,
            latest_round: 0,
            sync_progress: None,
        }
    }

    /// Add checkpoint
    pub fn add_checkpoint(&mut self, checkpoint: Checkpoint) {
        let seq = checkpoint.sequence;
        self.checkpoints.insert(seq, checkpoint);
        if seq > self.latest_checkpoint {
            self.latest_checkpoint = seq;
        }
    }

    /// Add vertex
    pub fn add_vertex(&mut self, vertex: DagVertex) {
        let round = vertex.round;
        self.vertices_by_round
            .entry(round)
            .or_default()
            .push(vertex);

        if round > self.latest_round {
            self.latest_round = round;
        }
    }

    /// Create sync request
    pub fn create_sync_request(&self, requester: AuthorityId) -> SyncRequest {
        SyncRequest {
            requester,
            last_checkpoint: self.latest_checkpoint,
            last_round: self.latest_round,
            missing_vertices: vec![],
        }
    }

    /// Handle sync request
    pub fn handle_sync_request(&self, request: &SyncRequest) -> Result<SyncResponse> {
        // Collect checkpoints from request.last_checkpoint + 1 to current
        let checkpoints: Vec<Checkpoint> = ((request.last_checkpoint + 1)..=self.latest_checkpoint)
            .filter_map(|seq| self.checkpoints.get(&seq).cloned())
            .collect();

        // Collect vertices from request.last_round + 1 to current
        let mut vertices = Vec::new();
        for round in (request.last_round + 1)..=self.latest_round {
            if let Some(round_vertices) = self.vertices_by_round.get(&round) {
                vertices.extend(round_vertices.clone());
            }
        }

        Ok(SyncResponse {
            checkpoints,
            vertices,
            current_round: self.latest_round,
            state_root: self
                .checkpoints
                .get(&self.latest_checkpoint)
                .map(|c| c.state_root.clone())
                .unwrap_or_default(),
        })
    }

    /// Apply sync response
    pub fn apply_sync_response(&mut self, response: SyncResponse) -> Result<()> {
        // Initialize sync progress
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
            started_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        });

        // Add checkpoints
        for checkpoint in response.checkpoints {
            let seq = checkpoint.sequence;
            self.add_checkpoint(checkpoint);
            if let Some(ref mut progress) = self.sync_progress {
                progress.current_checkpoint = seq;
            }
        }

        // Add vertices
        for vertex in response.vertices {
            self.add_vertex(vertex);
            if let Some(ref mut progress) = self.sync_progress {
                progress.synced_vertices += 1;
            }
        }

        // Verify state root matches
        if let Some(checkpoint) = self.checkpoints.get(&self.latest_checkpoint)
            && checkpoint.state_root != response.state_root
        {
            return Err(anyhow!(
                "State root mismatch after sync: expected {:?}, got {:?}",
                response.state_root,
                checkpoint.state_root
            ));
        }

        tracing::info!(
            "Sync complete: {} checkpoints, {} vertices, current round: {}",
            self.checkpoints.len(),
            self.vertices_by_round
                .values()
                .map(|v| v.len())
                .sum::<usize>(),
            self.latest_round
        );

        Ok(())
    }

    /// Get sync progress
    pub fn get_sync_progress(&self) -> Option<&SyncProgress> {
        self.sync_progress.as_ref()
    }

    /// Check if currently syncing
    pub fn is_syncing(&self) -> bool {
        self.sync_progress
            .as_ref()
            .map(|p| !p.is_complete())
            .unwrap_or(false)
    }

    /// Get latest checkpoint
    pub fn get_latest_checkpoint(&self) -> Option<&Checkpoint> {
        self.checkpoints.get(&self.latest_checkpoint)
    }

    /// Get checkpoint by sequence
    pub fn get_checkpoint(&self, sequence: u64) -> Option<&Checkpoint> {
        self.checkpoints.get(&sequence)
    }

    /// Get vertices for round
    pub fn get_round_vertices(&self, round: Round) -> Option<&[DagVertex]> {
        self.vertices_by_round.get(&round).map(|v| v.as_slice())
    }

    /// Get latest round
    pub fn get_latest_round(&self) -> Round {
        self.latest_round
    }

    /// Prune old checkpoints and vertices to prevent memory leak
    pub fn prune_old_data(&mut self, before_checkpoint: u64, before_round: Round) {
        // Keep only recent checkpoints
        self.checkpoints.retain(|seq, _| *seq >= before_checkpoint);

        // Keep only recent rounds
        self.vertices_by_round
            .retain(|round, _| *round >= before_round);

        tracing::debug!(
            "Pruned StateSynchronizer: {} checkpoints, {} rounds remaining",
            self.checkpoints.len(),
            self.vertices_by_round.len()
        );
    }

    /// Get memory usage statistics
    pub fn get_memory_stats(&self) -> (usize, usize) {
        (self.checkpoints.len(), self.vertices_by_round.len())
    }
}

impl Default for StateSynchronizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Fast state sync using checkpoints (skip intermediate vertices)
pub struct FastSync {
    /// Checkpoint interval (commit checkpoint every N rounds)
    checkpoint_interval: Round,

    /// Checkpoint store
    checkpoints: Vec<Checkpoint>,
}

impl FastSync {
    /// Create new fast sync
    pub fn new(checkpoint_interval: Round) -> Self {
        Self {
            checkpoint_interval,
            checkpoints: vec![Checkpoint::genesis()],
        }
    }

    /// Add checkpoint
    pub fn add_checkpoint(&mut self, checkpoint: Checkpoint) {
        self.checkpoints.push(checkpoint);
    }

    /// Get checkpoint for fast sync
    /// Returns the latest checkpoint that's at least `min_age` checkpoints old
    pub fn get_fast_sync_checkpoint(&self, min_age: u64) -> Option<&Checkpoint> {
        // Use checkpoint_interval to determine if we should use fast sync
        let interval_check = self.checkpoint_interval > 0;

        let current_seq = self.checkpoints.last().map(|c| c.sequence).unwrap_or(0);

        if !interval_check {
            return self.checkpoints.last();
        }

        self.checkpoints
            .iter()
            .rev()
            .find(|c| current_seq - c.sequence >= min_age)
    }

    /// Calculate checkpoints to skip when fast syncing
    pub fn checkpoints_to_skip(&self, from_checkpoint: u64, to_checkpoint: u64) -> u64 {
        to_checkpoint.saturating_sub(from_checkpoint)
    }
}

#[cfg(test)]
mod tests {
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

        // Add checkpoint
        let mut checkpoint = Checkpoint::genesis();
        checkpoint.sequence = 1;
        sync.add_checkpoint(checkpoint);

        assert_eq!(sync.latest_checkpoint, 1);
        assert!(sync.get_checkpoint(1).is_some());
    }

    #[test]
    fn test_sync_request_response() {
        let mut sync = StateSynchronizer::new();

        // Add some data
        let vertex = DagVertex::new(1, "auth1".to_string(), vec![], vec![], vec![0u8; 32]);
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
    fn test_apply_sync_response() {
        let mut sync = StateSynchronizer::new();

        let vertex = DagVertex::new(1, "auth1".to_string(), vec![], vec![], vec![0u8; 32]);

        let response = SyncResponse {
            checkpoints: vec![],
            vertices: vec![vertex],
            current_round: 1,
            state_root: vec![0u8; 32],
        };

        assert!(sync.apply_sync_response(response).is_ok());
        assert_eq!(sync.latest_round, 1);
    }

    #[test]
    fn test_fast_sync() {
        let mut fast_sync = FastSync::new(10);

        let mut checkpoint = Checkpoint::genesis();
        checkpoint.sequence = 1;
        // Checkpoint doesn't have epoch field, use sequence instead
        fast_sync.add_checkpoint(checkpoint);

        assert_eq!(fast_sync.checkpoints.len(), 2); // Genesis + 1
    }
}
