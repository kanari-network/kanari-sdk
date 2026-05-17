// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::dag_consensus::{DagVertex, Round};
use super::persistent_store::PersistentDagStore;

/// Configuration for DAG pruning and garbage collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruningConfig {
    /// Keep vertices from the last N rounds
    pub retention_rounds: u64,

    /// Keep the last N checkpoints
    pub retention_checkpoints: u64,

    /// Enable time-based pruning (seconds)
    pub retention_time_secs: Option<u64>,

    /// Minimum rounds before any pruning can occur (safety margin)
    pub min_rounds_before_pruning: u64,

    /// Enable automatic pruning
    pub auto_prune: bool,

    /// Prune every N rounds (when auto_prune is true)
    pub prune_interval_rounds: u64,
}

impl Default for PruningConfig {
    fn default() -> Self {
        Self {
            retention_rounds: 500,                // Less retention for 500K TPS
            retention_checkpoints: 200,           // Keep more checkpoints
            retention_time_secs: Some(86400 * 3), // 3 days (less storage)
            min_rounds_before_pruning: 50,        // Prune sooner
            auto_prune: true,
            prune_interval_rounds: 50, // More frequent pruning
        }
    }
}

impl PruningConfig {
    fn ensure_positive(value: u64, field: &str) -> Result<()> {
        if value == 0 {
            return Err(anyhow::anyhow!("{} must be > 0", field));
        }
        Ok(())
    }

    /// Moderate config for 8-16 core machines (10K-30K TPS)
    pub fn moderate() -> Self {
        Self {
            retention_rounds: 1000,            // Keep more data
            retention_checkpoints: 100,        // Fewer checkpoints
            retention_time_secs: Some(604800), // 7 days
            min_rounds_before_pruning: 100,    // Conservative pruning
            auto_prune: true,
            prune_interval_rounds: 100, // Prune every 100 rounds
        }
    }

    /// High-throughput config for 500K+ TPS - aggressive pruning
    pub fn high_throughput() -> Self {
        Self {
            retention_rounds: 200,            // Minimal retention
            retention_checkpoints: 500,       // Keep many checkpoints
            retention_time_secs: Some(86400), // 1 day only
            min_rounds_before_pruning: 20,    // Very early pruning
            auto_prune: true,
            prune_interval_rounds: 20, // Prune every 20 rounds
        }
    }

    pub fn validate(&self) -> Result<()> {
        Self::ensure_positive(self.retention_rounds, "retention_rounds")?;
        Self::ensure_positive(self.retention_checkpoints, "retention_checkpoints")?;
        if self.min_rounds_before_pruning < 10 {
            return Err(anyhow::anyhow!(
                "min_rounds_before_pruning must be >= 10 for safety"
            ));
        }
        Self::ensure_positive(self.prune_interval_rounds, "prune_interval_rounds")?;
        Ok(())
    }
}

/// Statistics from a pruning operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneStats {
    /// Number of vertices pruned
    pub vertices_pruned: usize,

    /// Number of checkpoints pruned
    pub checkpoints_pruned: usize,

    /// Number of vertices skipped (uncommitted)
    pub vertices_skipped: usize,

    /// Round cutoff used for pruning
    pub cutoff_round: Round,

    /// Checkpoint cutoff used for pruning
    pub cutoff_checkpoint: Option<u64>,

    /// Duration of pruning operation (milliseconds)
    pub duration_ms: u64,

    /// IDs of pruned vertices (for cache invalidation)
    pub pruned_vertex_ids: Vec<super::VertexId>,
}

/// DAG pruner for garbage collection and storage management
pub struct DagPruner {
    config: PruningConfig,
    last_prune_round: Round,
    /// Track the last pruned checkpoint sequence to avoid re-scanning
    last_pruned_checkpoint_seq: u64,
    /// Track the last cleaned round for O(1) pruning (not from 0)
    last_cleaned_round: Round,
    /// Track the last cleaned checkpoint sequence
    last_cleaned_checkpoint: u64,
    /// FIX #16: Track whether pruner has been initialized from store
    initialized: bool,
}

impl DagPruner {
    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn elapsed_ms_since(start: SystemTime) -> u64 {
        SystemTime::now()
            .duration_since(start)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    /// Create a new DAG pruner with the given configuration
    pub fn new(config: PruningConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            last_prune_round: 0,
            last_pruned_checkpoint_seq: 0,
            last_cleaned_round: 0,
            last_cleaned_checkpoint: 0,
            initialized: false, // FIX #16: Not yet initialized from store
        })
    }

    /// FIX #16: Initialize pruner from persistent store to avoid restart freeze
    /// When node restarts, this method queries the database to find the actual
    /// last cleaned round instead of starting from 0 (which causes million-query loops)
    pub fn init_from_store(&mut self, store: &PersistentDagStore) -> Result<()> {
        if self.initialized {
            // Already initialized, skip
            return Ok(());
        }

        // Query the minimum round that still exists in the database
        if let Some(min_round) = store.get_min_existing_round()? {
            // Set last_cleaned_round to just before the minimum existing round
            // This ensures we don't try to prune rounds that don't exist
            self.last_cleaned_round = min_round.saturating_sub(1);

            tracing::info!(
                "Initialized pruner from store: last_cleaned_round = {} (min existing round: {})",
                self.last_cleaned_round,
                min_round
            );
        } else {
            // Database is empty, start from 0
            self.last_cleaned_round = 0;
            tracing::debug!("Database empty, starting pruning from round 0");
        }

        // Also initialize checkpoint tracking
        if let Some(min_checkpoint) = store.get_min_existing_checkpoint_sequence()? {
            self.last_cleaned_checkpoint = min_checkpoint.saturating_sub(1);
            tracing::info!(
                "Initialized pruner checkpoint tracking: last_cleaned_checkpoint = {}",
                self.last_cleaned_checkpoint
            );
        }

        self.initialized = true;
        Ok(())
    }

    /// Check if pruning should run for the given round
    pub fn should_prune(&self, current_round: Round) -> bool {
        if !self.config.auto_prune {
            return false;
        }

        if current_round < self.config.min_rounds_before_pruning {
            return false;
        }

        let rounds_since_last_prune = current_round.saturating_sub(self.last_prune_round);
        rounds_since_last_prune >= self.config.prune_interval_rounds
    }

    /// Prune old vertices and checkpoints from storage
    pub fn prune(
        &mut self,
        store: &PersistentDagStore,
        current_round: Round,
        latest_checkpoint_seq: Option<u64>,
    ) -> Result<PruneStats> {
        let start = SystemTime::now();

        // Safety check: don't prune if we're too early
        if current_round < self.config.min_rounds_before_pruning {
            return Err(anyhow::anyhow!(
                "Cannot prune: current_round {} < min_rounds_before_pruning {}",
                current_round,
                self.config.min_rounds_before_pruning
            ));
        }

        // Calculate cutoff round
        let cutoff_round = current_round.saturating_sub(self.config.retention_rounds);

        // Prune vertices and collect pruned IDs
        let (vertices_pruned, vertices_skipped, pruned_ids) =
            self.prune_vertices(store, cutoff_round)?;

        // Prune checkpoints
        let (checkpoints_pruned, cutoff_checkpoint) = if let Some(latest_cp) = latest_checkpoint_seq
        {
            self.prune_checkpoints(store, latest_cp)?
        } else {
            (0, None)
        };

        // Update tracking state after successful prune
        self.update_prune_state(cutoff_round, cutoff_checkpoint);

        // Update last prune round
        self.last_prune_round = current_round;

        let duration_ms = Self::elapsed_ms_since(start);

        Ok(PruneStats {
            vertices_pruned,
            checkpoints_pruned,
            vertices_skipped,
            cutoff_round,
            cutoff_checkpoint,
            duration_ms,
            pruned_vertex_ids: pruned_ids,
        })
    }

    /// Prune vertices older than the cutoff round
    /// Returns (pruned_count, skipped_count, pruned_vertex_ids)
    fn prune_vertices(
        &mut self,
        store: &PersistentDagStore,
        cutoff_round: Round,
    ) -> Result<(usize, usize, Vec<super::VertexId>)> {
        // FIX #16: Initialize from store on first prune to avoid restart freeze
        if !self.initialized {
            self.init_from_store(store)?;
        }

        let mut pruned = 0;
        let mut skipped = 0;
        let mut pruned_ids = Vec::new();

        // FIX #1: Start from last_cleaned_round instead of 0 for O(1) pruning
        let start_round = self.last_cleaned_round;

        tracing::debug!(
            "Pruning vertices from round {} to {}",
            start_round,
            cutoff_round
        );

        let mut next_round_to_scan = cutoff_round;
        let mut blocked_round = None;

        for round in start_round..cutoff_round {
            let vertex_ids = store.get_vertices_by_round(round)?;

            if vertex_ids.is_empty() {
                continue;
            }

            // Check each vertex individually
            let mut vertices_to_prune = Vec::new();
            let mut round_skipped = 0;

            for vertex_id in &vertex_ids {
                let vertex = match store.get_vertex(vertex_id)? {
                    Some(v) => v,
                    None => continue,
                };

                let is_checkpointed = self.is_vertex_safe_to_prune(&vertex);
                if !is_checkpointed {
                    if let Some(retention_secs) = self.config.retention_time_secs
                        && self.is_vertex_old_enough(&vertex, retention_secs)
                    {
                        // Can prune this vertex
                        vertices_to_prune.push(*vertex_id);
                        continue;
                    }
                    // Cannot prune - not checkpointed and not old enough
                    skipped += 1;
                    round_skipped += 1;
                    continue;
                }

                if let Some(retention_secs) = self.config.retention_time_secs
                    && !self.is_vertex_old_enough(&vertex, retention_secs)
                {
                    skipped += 1;
                    round_skipped += 1;
                    continue;
                }

                // Vertex can be pruned
                vertices_to_prune.push(*vertex_id);
            }

            // Prune the selected vertices
            if !vertices_to_prune.is_empty() {
                // If we're pruning ALL vertices in this round, use batch delete
                if vertices_to_prune.len() == vertex_ids.len() {
                    let pruned_in_round = store.prune_entire_round(round)?;
                    pruned += pruned_in_round;
                    pruned_ids.extend(vertices_to_prune);
                } else {
                    // Partial prune - delete individual vertices
                    for vertex_id in &vertices_to_prune {
                        store.prune_vertex_fast(vertex_id)?;
                        pruned += 1;
                        pruned_ids.push(*vertex_id);
                    }
                }
            }

            if round_skipped > 0 && blocked_round.is_none() {
                blocked_round = Some(round);
            }
        }

        // Update tracking state
        if let Some(blocked_round) = blocked_round {
            next_round_to_scan = blocked_round;
        }
        self.last_cleaned_round = next_round_to_scan;
        tracing::debug!(
            "Pruned {} vertices, skipped {}, updated last_cleaned_round to {}",
            pruned,
            skipped,
            next_round_to_scan
        );

        Ok((pruned, skipped, pruned_ids))
    }

    /// Prune checkpoints older than retention policy
    fn prune_checkpoints(
        &mut self,
        store: &PersistentDagStore,
        latest_checkpoint_seq: u64,
    ) -> Result<(usize, Option<u64>)> {
        if latest_checkpoint_seq < self.config.retention_checkpoints {
            return Ok((0, None));
        }

        // Calculate cutoff: keep retention_checkpoints most recent ones
        let cutoff_checkpoint = latest_checkpoint_seq
            .saturating_add(1)
            .saturating_sub(self.config.retention_checkpoints);
        let mut pruned = 0;

        // FIX #1: Start from last_cleaned_checkpoint instead of 0
        let start_seq = self.last_cleaned_checkpoint;

        // Prune checkpoints before cutoff (exclusive)
        for seq in start_seq..cutoff_checkpoint {
            if store.get_checkpoint(seq)?.is_some() {
                store.delete_checkpoint(seq)?;
                pruned += 1;
            }
        }

        // Update tracking state
        self.last_cleaned_checkpoint = cutoff_checkpoint;

        Ok((pruned, Some(cutoff_checkpoint - 1)))
    }

    /// Check if a vertex is safe to prune (has been checkpointed)
    fn is_vertex_safe_to_prune(&self, vertex: &DagVertex) -> bool {
        // A vertex is safe to prune if it's been included in a checkpoint
        vertex.metadata.is_checkpoint || vertex.metadata.checkpoint_seq.is_some()
    }

    /// Check if a vertex is old enough to prune based on time
    fn is_vertex_old_enough(&self, vertex: &DagVertex, retention_secs: u64) -> bool {
        let now = Self::now_secs();

        let age_secs = now.saturating_sub(vertex.timestamp);
        age_secs >= retention_secs
    }

    /// Force prune all vertices before a specific round (admin operation)
    pub fn force_prune_before_round(
        &mut self,
        store: &PersistentDagStore,
        before_round: Round,
    ) -> Result<usize> {
        // Use the new optimized prune_old_vertices with start_round parameter
        let pruned = store.prune_old_vertices(self.last_cleaned_round, before_round)?;
        self.last_cleaned_round = before_round;
        self.last_prune_round = before_round;
        Ok(pruned)
    }

    /// Get the current pruning configuration
    pub fn config(&self) -> &PruningConfig {
        &self.config
    }

    /// Update pruning configuration
    pub fn update_config(&mut self, config: PruningConfig) -> Result<()> {
        config.validate()?;
        self.config = config;
        Ok(())
    }

    /// Get the round of the last pruning operation
    pub fn last_prune_round(&self) -> Round {
        self.last_prune_round
    }

    /// Update tracking state after successful prune
    pub fn update_prune_state(&mut self, cutoff_round: Round, cutoff_checkpoint: Option<u64>) {
        self.last_prune_round = cutoff_round;
        if let Some(seq) = cutoff_checkpoint {
            self.last_pruned_checkpoint_seq = seq;
        }
        // Also update cleaned tracking for O(1) pruning
        self.last_cleaned_round = cutoff_round;
        if let Some(seq) = cutoff_checkpoint {
            self.last_cleaned_checkpoint = seq;
        }
    }
}

/// Pruning policy strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PruningPolicy {
    /// Keep vertices from last N rounds
    RoundBased(u64),

    /// Keep vertices from last N checkpoints
    CheckpointBased(u64),

    /// Keep vertices newer than N seconds
    TimeBased(u64),

    /// Combine multiple policies (most conservative)
    Hybrid,
}

impl PruningPolicy {
    pub fn to_config(&self) -> PruningConfig {
        match self {
            PruningPolicy::RoundBased(rounds) => PruningConfig {
                retention_rounds: *rounds,
                retention_checkpoints: 100,
                retention_time_secs: None,
                ..Default::default()
            },
            PruningPolicy::CheckpointBased(checkpoints) => PruningConfig {
                retention_rounds: 1000,
                retention_checkpoints: *checkpoints,
                retention_time_secs: None,
                ..Default::default()
            },
            PruningPolicy::TimeBased(secs) => PruningConfig {
                retention_rounds: 500,
                retention_checkpoints: 50,
                retention_time_secs: Some(*secs),
                ..Default::default()
            },
            PruningPolicy::Hybrid => PruningConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{AuthorityId, Checkpoint};
    use super::*;
    use tempfile::TempDir;

    fn create_test_vertex(round: Round, author: AuthorityId, checkpointed: bool) -> DagVertex {
        let mut vertex =
            DagVertex::new_for_test(round, author, vec![], vec![], vec![round as u8; 32], 0);
        vertex.metadata.is_checkpoint = checkpointed;
        if checkpointed {
            vertex.metadata.checkpoint_seq = Some(round);
        }
        vertex
    }

    fn create_test_checkpoint(sequence: u64) -> Checkpoint {
        let mut vertex_id = [0u8; 32];
        vertex_id.fill(sequence as u8);

        Checkpoint {
            sequence,
            vertices: vec![vertex_id],
            transactions: vec![],
            state_root: vec![sequence as u8; 32],
            timestamp: 12345,
            prev_checkpoint_hash: vec![0u8; 32],
        }
    }

    #[test]
    fn test_pruning_config_validation() {
        let valid_config = PruningConfig::default();
        assert!(valid_config.validate().is_ok());

        let invalid_config = PruningConfig {
            retention_rounds: 0,
            ..Default::default()
        };
        assert!(invalid_config.validate().is_err());

        let invalid_config = PruningConfig {
            min_rounds_before_pruning: 5,
            ..Default::default()
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_should_prune() {
        let config = PruningConfig {
            auto_prune: true,
            min_rounds_before_pruning: 100,
            prune_interval_rounds: 50,
            ..Default::default()
        };

        let pruner = DagPruner::new(config).unwrap();

        // Too early to prune
        assert!(!pruner.should_prune(50));
        assert!(!pruner.should_prune(99));

        // First prune at min_rounds_before_pruning
        assert!(pruner.should_prune(100));
        assert!(pruner.should_prune(150));
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn test_basic_vertex_pruning() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = PersistentDagStore::new(temp_dir.path())?;

        // Create vertices in different rounds
        for round in 0..10 {
            let vertex = create_test_vertex(round, "validator_0".to_string(), true);
            store.put_vertex(&vertex)?;
        }

        // Configure pruner to keep last 5 rounds
        let config = PruningConfig {
            retention_rounds: 5,
            min_rounds_before_pruning: 10,
            retention_time_secs: None, // Disable time-based pruning for this test
            ..Default::default()
        };

        let mut pruner = DagPruner::new(config)?;

        // Debug: verify initial state
        assert_eq!(
            pruner.last_cleaned_round, 0,
            "Pruner should start with last_cleaned_round = 0"
        );

        let stats = pruner.prune(&store, 10, None)?;

        // Should prune rounds 0-4 (current=10, retention=5, cutoff=5)
        assert_eq!(stats.vertices_pruned, 5);
        assert_eq!(stats.cutoff_round, 5);

        // Verify vertices are actually gone
        for round in 0..5 {
            let vertices = store.get_vertices_by_round(round)?;
            assert_eq!(vertices.len(), 0, "Round {} should be pruned", round);
        }

        // Verify recent vertices are kept
        for round in 5..10 {
            let vertices = store.get_vertices_by_round(round)?;
            assert_eq!(vertices.len(), 1, "Round {} should be kept", round);
        }

        Ok(())
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn test_safety_check_uncommitted_vertices() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = PersistentDagStore::new(temp_dir.path())?;

        // Create mix of checkpointed and uncommitted vertices
        for round in 0..10 {
            let checkpointed = round % 2 == 0; // Even rounds checkpointed
            let vertex = create_test_vertex(round, "validator_0".to_string(), checkpointed);
            store.put_vertex(&vertex)?;
        }

        let config = PruningConfig {
            retention_rounds: 3,
            min_rounds_before_pruning: 10,
            retention_time_secs: None, // Disable time-based pruning
            ..Default::default()
        };

        let mut pruner = DagPruner::new(config)?;
        let stats = pruner.prune(&store, 10, None)?;

        // Should only prune checkpointed vertices (0, 2, 4, 6)
        // Cutoff is round 7, so prunes: 0, 2, 4, 6
        assert_eq!(stats.vertices_pruned, 4);
        assert_eq!(stats.vertices_skipped, 3); // Rounds 1, 3, 5 are uncommitted

        Ok(())
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn test_checkpoint_pruning() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = PersistentDagStore::new(temp_dir.path())?;

        // Create 20 checkpoints
        for seq in 0..20 {
            let checkpoint = create_test_checkpoint(seq);
            store.put_checkpoint(&checkpoint)?;
        }

        let config = PruningConfig {
            retention_checkpoints: 10,
            min_rounds_before_pruning: 10,
            retention_time_secs: None, // Disable time-based pruning
            ..Default::default()
        };

        let mut pruner = DagPruner::new(config)?;
        let stats = pruner.prune(&store, 100, Some(19))?;

        // Should prune checkpoints 0-9 (keep last 10: 10-19)
        assert_eq!(stats.checkpoints_pruned, 10);
        assert_eq!(stats.cutoff_checkpoint, Some(9));

        // Verify checkpoints are pruned
        for seq in 0..10 {
            assert!(
                store.get_checkpoint(seq)?.is_none(),
                "Checkpoint {} should be pruned",
                seq
            );
        }

        // Verify recent checkpoints are kept
        for seq in 10..20 {
            assert!(
                store.get_checkpoint(seq)?.is_some(),
                "Checkpoint {} should be kept",
                seq
            );
        }

        Ok(())
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn test_force_prune() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = PersistentDagStore::new(temp_dir.path())?;

        // Create vertices
        for round in 0..20 {
            let vertex = create_test_vertex(round, "validator_0".to_string(), true);
            store.put_vertex(&vertex)?;
        }

        let config = PruningConfig::default();
        let mut pruner = DagPruner::new(config)?;

        // Force prune everything before round 15
        let pruned = pruner.force_prune_before_round(&store, 15)?;
        assert_eq!(pruned, 15);
        assert_eq!(pruner.last_prune_round(), 15);

        Ok(())
    }

    #[test]
    fn test_pruning_policy_conversion() {
        let policy = PruningPolicy::RoundBased(500);
        let config = policy.to_config();
        assert_eq!(config.retention_rounds, 500);

        let policy = PruningPolicy::CheckpointBased(50);
        let config = policy.to_config();
        assert_eq!(config.retention_checkpoints, 50);

        let policy = PruningPolicy::TimeBased(86400);
        let config = policy.to_config();
        assert_eq!(config.retention_time_secs, Some(86400));
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn test_time_based_pruning() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = PersistentDagStore::new(temp_dir.path())?;

        // Create old and new vertices (both in round 0 so they'll be considered for pruning)
        let mut old_vertex = create_test_vertex(0, "validator_0".to_string(), true);
        old_vertex.timestamp = 1000; // Very old timestamp (1970)
        store.put_vertex(&old_vertex)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut new_vertex = create_test_vertex(0, "validator_1".to_string(), true);
        new_vertex.timestamp = now - 100; // Recent timestamp (100 seconds ago)
        store.put_vertex(&new_vertex)?;

        let config = PruningConfig {
            retention_rounds: 10,
            retention_time_secs: Some(1000), // 1000 seconds retention
            min_rounds_before_pruning: 10,
            ..Default::default()
        };

        let mut pruner = DagPruner::new(config)?;
        let stats = pruner.prune(&store, 11, None)?;

        // Old vertex should be pruned (very old), new vertex should be skipped (recent)
        assert_eq!(stats.vertices_pruned, 1, "Should prune 1 old vertex");
        assert_eq!(stats.vertices_skipped, 1, "Should skip 1 recent vertex");

        Ok(())
    }

    #[test]
    fn test_pruning_disabled() {
        let config = PruningConfig {
            auto_prune: false,
            ..Default::default()
        };

        let pruner = DagPruner::new(config).unwrap();

        // Should never prune when auto_prune is false
        assert!(!pruner.should_prune(1000));
        assert!(!pruner.should_prune(10000));
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn test_old_orphan_vertex_can_be_pruned_by_time() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = PersistentDagStore::new(temp_dir.path())?;
        let mut orphan = create_test_vertex(0, "validator_0".to_string(), false);
        orphan.timestamp = 1;
        store.put_vertex(&orphan)?;

        let config = PruningConfig {
            retention_rounds: 1,
            retention_time_secs: Some(1),
            min_rounds_before_pruning: 10,
            ..Default::default()
        };
        let mut pruner = DagPruner::new(config)?;
        let stats = pruner.prune(&store, 11, None)?;
        assert_eq!(stats.vertices_pruned, 1);
        Ok(())
    }
}
