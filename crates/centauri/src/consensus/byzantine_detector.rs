// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Byzantine Detection and Slashing
//!
//! Detects and penalizes Byzantine (malicious) behavior including:
//! - Double voting: Creating multiple vertices in the same round
//! - Invalid vertices: Vertices with incorrect parents or quorum
//! - Equivocation: Conflicting statements
//! - Withholding: Not participating when required

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

use crate::calculate_quorum;

use super::{AuthorityId, DagVertex, Round, VertexId};

/// Constants for Byzantine detection and reputation management
///
/// These values are tuned to balance security and network stability:
/// - High enough penalties to deter malicious behavior
/// - Low enough initial reputation to allow recovery from minor faults
const INITIAL_REPUTATION: u64 = 100;

/// Maximum number of fault records to retain per authority (DoS protection)
/// Prevents unbounded memory growth from historical fault tracking
const MAX_FAULTS_HISTORY: usize = 10_000;

/// Maximum number of penalty records to retain per authority (DoS protection)
/// Matches MAX_FAULTS_HISTORY for consistency
const MAX_PENALTIES_HISTORY: usize = 10_000;

/// Penalty amounts for different fault types
///
/// Penalties are designed to be proportional to the severity of the fault:
/// - Equivocation (30): Most severe - active attempt to confuse consensus
/// - Double voting (20): Severe - violates one-authority-one-vote rule
/// - Invalid vertex (10): Moderate - could be bug or attack
/// - Withholding (5): Minor - passive non-participation
const DOUBLE_VOTING_PENALTY: u64 = 20;
const INVALID_VERTEX_PENALTY: u64 = 10;
const EQUIVOCATION_PENALTY: u64 = 30;
const WITHHOLDING_PENALTY: u64 = 5;

/// Types of Byzantine faults that can be detected in the consensus protocol
///
/// Each fault type represents a different class of malicious or erroneous behavior
/// that could compromise the safety or liveness of the consensus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ByzantineFault {
    /// Double voting: An authority creates multiple vertices in the same round
    ///
    /// This violates the "one-authority-one-vote-per-round" invariant and could
    /// be used to manipulate leader election or confuse the ordering algorithm.
    ///
    /// **Severity**: High - Direct attack on consensus integrity
    /// **Penalty**: `DOUBLE_VOTING_PENALTY` (20 points)
    /// **Detection**: Track (authority, round) pairs and detect duplicates
    DoubleVoting {
        authority: AuthorityId,
        round: Round,
        vertices: Vec<VertexId>,
    },

    /// Invalid vertex: A vertex with incorrect structure or missing requirements
    ///
    /// This includes vertices with:
    /// - Invalid parent references (non-existent or from future rounds)
    /// - Insufficient quorum support (< 2f+1 unique authors)
    /// - Malformed transaction batches
    /// - Invalid cryptographic signatures
    ///
    /// **Severity**: Medium - Could be bug or deliberate attack
    /// **Penalty**: `INVALID_VERTEX_PENALTY` (10 points)
    /// **Detection**: Structural validation during vertex processing
    InvalidVertex {
        authority: AuthorityId,
        vertex_id: VertexId,
        reason: String,
    },

    /// Equivocation: An authority makes conflicting statements about the same data
    ///
    /// This is a generalization of double voting where an authority sends
    /// different messages to different peers (e.g., different parent sets).
    ///
    /// **Severity**: Critical - Most dangerous Byzantine behavior
    /// **Penalty**: `EQUIVOCATION_PENALTY` (30 points)
    /// **Detection**: Cross-reference messages received from different peers
    Equivocation {
        authority: AuthorityId,
        round: Round,
        evidence: Vec<u8>,
    },

    /// Withholding: An authority fails to participate when expected
    ///
    /// While not actively malicious, persistent withholding degrades network
    /// performance and could indicate a compromised node.
    ///
    /// **Severity**: Low - Passive non-participation
    /// **Penalty**: `WITHHOLDING_PENALTY` (5 points)
    /// **Detection**: Monitor participation rate over time windows
    Withholding {
        authority: AuthorityId,
        round: Round,
    },
}

/// Evidence of Byzantine fault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByzantineEvidence {
    pub fault: ByzantineFault,
    pub detected_at: u64,
    pub proof: Vec<u8>,
}

/// Slashing penalty
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingPenalty {
    pub authority: AuthorityId,
    pub amount: u64,
    pub reason: String,
    pub round: Round,
}

/// Serialized state for Byzantine detector persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ByzantineDetectorState {
    faults: Vec<ByzantineEvidence>,
    penalties: Vec<SlashingPenalty>,
    reputation: HashMap<AuthorityId, u64>, // HashMap for O(1) lookup
    vertices_by_authority_round: BTreeMap<(AuthorityId, Round), Vec<VertexId>>,
}

/// Byzantine detector
pub struct ByzantineDetector {
    faults: Vec<ByzantineEvidence>,
    penalties: Vec<SlashingPenalty>,
    reputation: HashMap<AuthorityId, u64>, // HashMap for O(1) lookup
    vertices_by_authority_round: BTreeMap<(AuthorityId, Round), Vec<VertexId>>,
}

impl ByzantineDetector {
    fn fault_authority(fault: &ByzantineFault) -> &str {
        match fault {
            ByzantineFault::DoubleVoting { authority, .. } => authority,
            ByzantineFault::InvalidVertex { authority, .. } => authority,
            ByzantineFault::Equivocation { authority, .. } => authority,
            ByzantineFault::Withholding { authority, .. } => authority,
        }
    }

    fn retain_recent_records<T>(records: &mut Vec<T>, max_entries: usize) {
        if records.len() > max_entries {
            let remove_count = records.len() - max_entries;
            records.drain(0..remove_count);
        }
    }

    fn retain_round_tracking(&mut self, before_round: Round) {
        self.vertices_by_authority_round
            .retain(|(_, round), _| *round >= before_round);
    }

    fn fault_penalty(fault: &ByzantineFault) -> (&str, u64, Round, &str) {
        match fault {
            ByzantineFault::DoubleVoting {
                authority, round, ..
            } => (
                authority,
                DOUBLE_VOTING_PENALTY,
                *round,
                "Double voting detected",
            ),
            ByzantineFault::InvalidVertex { authority, .. } => {
                (authority, INVALID_VERTEX_PENALTY, 0, "Invalid vertex")
            }
            ByzantineFault::Equivocation {
                authority, round, ..
            } => (authority, EQUIVOCATION_PENALTY, *round, "Equivocation"),
            ByzantineFault::Withholding { authority, round } => {
                (authority, WITHHOLDING_PENALTY, *round, "Withholding")
            }
        }
    }

    pub fn new() -> Self {
        Self {
            faults: Vec::new(),
            penalties: Vec::new(),
            reputation: HashMap::new(),
            vertices_by_authority_round: BTreeMap::new(),
        }
    }

    pub fn init_authority(&mut self, authority: AuthorityId) {
        self.reputation.insert(authority, INITIAL_REPUTATION);
    }

    // FIX 1: Prevent O(N^2) DoS Attack
    pub fn check_double_voting(&mut self, vertex: &DagVertex) -> Result<()> {
        let key = (vertex.author.clone(), vertex.round);
        let existing = self.vertices_by_authority_round.entry(key).or_default();

        // Normal case: First vote in this round
        if existing.is_empty() {
            existing.push(vertex.id);
            return Ok(());
        }

        // Anomaly detected: Second vote found
        if existing.len() == 1 && existing[0] != vertex.id {
            existing.push(vertex.id);
            let fault = ByzantineFault::DoubleVoting {
                authority: vertex.author.clone(),
                round: vertex.round,
                vertices: existing.clone(),
            };
            // Report and apply penalty
            self.report_fault(fault)?;

            // FIX #8: CRITICAL - Reject the vertex after slashing
            // Previously returned Ok(()) which allowed double-voted vertex into DAG
            return Err(anyhow::anyhow!(
                "Double voting detected and slashed. Vertex rejected from DAG."
            ));
        }

        // If existing.len() >= 2, already caught and penalized
        // Return immediately (O(1) time) to avoid redundant processing
        Ok(())
    }

    pub fn check_vertex_validity(
        &mut self,
        vertex: &DagVertex,
        total_authorities: usize,
    ) -> Result<()> {
        if total_authorities == 0 {
            anyhow::bail!("Critical Error: Total authorities cannot be zero");
        }

        if vertex.round > 0 {
            let quorum = calculate_quorum(total_authorities);

            if vertex.parents.len() < quorum {
                let fault = ByzantineFault::InvalidVertex {
                    authority: vertex.author.clone(),
                    vertex_id: vertex.id,
                    reason: format!(
                        "Insufficient parents: {} < {} (quorum)",
                        vertex.parents.len(),
                        quorum
                    ),
                };
                self.report_fault(fault)?;
                anyhow::bail!("Invalid vertex: insufficient parents for quorum");
            }
        }
        Ok(())
    }

    pub fn report_fault(&mut self, fault: ByzantineFault) -> Result<()> {
        // FIX #9: Use deterministic evidence timestamp based on fault round instead of system time
        // SystemTime::now() causes non-deterministic evidence hashes across different nodes
        // Using the fault's round number ensures all nodes generate identical evidence
        let detected_at = match &fault {
            ByzantineFault::DoubleVoting { round, .. } => *round,
            ByzantineFault::InvalidVertex { .. } => 0, // Genesis or unknown round
            ByzantineFault::Equivocation { round, .. } => *round,
            ByzantineFault::Withholding { round, .. } => *round,
        };

        let evidence = ByzantineEvidence {
            fault: fault.clone(),
            detected_at,
            proof: vec![],
        };

        self.faults.push(evidence);

        let (authority, penalty, round, reason) = Self::fault_penalty(&fault);
        self.slash_authority(authority, penalty, reason, round)?;

        Ok(())
    }

    fn slash_authority(
        &mut self,
        authority: &str,
        penalty: u64,
        reason: &str,
        round: Round,
    ) -> Result<()> {
        let reputation = self.reputation.entry(authority.to_string()).or_insert(100);

        // Deduct reputation score (minimum 0)
        *reputation = reputation.saturating_sub(penalty);

        let slashing = SlashingPenalty {
            authority: authority.to_string(),
            amount: penalty,
            reason: reason.to_string(),
            round,
        };

        self.penalties.push(slashing);

        tracing::warn!(
            "Slashed authority {} by {} points (reason: {}). New reputation: {}",
            authority,
            penalty,
            reason,
            reputation
        );

        Ok(())
    }

    // --- FIX 2: System Export/Import State to Disk (Persistent State) ---
    /// Export fault history for persistence (for new node startup)
    pub fn export_state(&self) -> Result<Vec<u8>> {
        let state = ByzantineDetectorState {
            faults: self.faults.clone(),
            penalties: self.penalties.clone(),
            reputation: self.reputation.clone(),
            vertices_by_authority_round: self.vertices_by_authority_round.clone(),
        };
        bcs::to_bytes(&state)
            .map_err(|e| anyhow::anyhow!("Failed to serialize Byzantine state: {}", e))
    }

    /// Load fault history back after new node startup
    pub fn import_state(&mut self, data: &[u8]) -> Result<()> {
        let state: ByzantineDetectorState = bcs::from_bytes(data)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize Byzantine state: {}", e))?;

        self.faults = state.faults;
        self.penalties = state.penalties;
        self.reputation = state.reputation;
        self.vertices_by_authority_round = state.vertices_by_authority_round;
        Ok(())
    }

    pub fn get_reputation(&self, authority: &str) -> u64 {
        self.reputation.get(authority).copied().unwrap_or(0)
    }

    pub fn is_trusted(&self, authority: &str, threshold: u64) -> bool {
        self.get_reputation(authority) >= threshold
    }

    pub fn get_faults(&self) -> &[ByzantineEvidence] {
        &self.faults
    }

    pub fn get_penalties(&self) -> &[SlashingPenalty] {
        &self.penalties
    }

    pub fn get_authority_faults(&self, authority: &str) -> Vec<&ByzantineEvidence> {
        self.faults
            .iter()
            .filter(|evidence| Self::fault_authority(&evidence.fault) == authority)
            .collect()
    }

    pub fn prune_before_round(&mut self, before_round: Round) {
        self.retain_round_tracking(before_round);
        tracing::debug!(
            "Pruned Byzantine detector data before round {}, remaining entries: {}",
            before_round,
            self.vertices_by_authority_round.len()
        );
    }

    pub fn memory_usage(&self) -> usize {
        self.faults.len() * 256
            + self.penalties.len() * 128
            + self.reputation.len() * 32
            + self.vertices_by_authority_round.len() * 64
    }

    pub fn reset_reputation(&mut self, authority: &str, score: u64) {
        self.reputation
            .insert(authority.to_string(), score.min(INITIAL_REPUTATION));
    }

    pub fn ban_authority(&mut self, authority: &str) {
        self.reputation.insert(authority.to_string(), 0);
        tracing::warn!("Authority {} has been banned (reputation = 0)", authority);
    }

    pub fn prune_old_rounds(&mut self, before_round: Round) {
        self.retain_round_tracking(before_round);

        Self::retain_recent_records(&mut self.faults, MAX_FAULTS_HISTORY);
        Self::retain_recent_records(&mut self.penalties, MAX_PENALTIES_HISTORY);

        tracing::debug!(
            "Pruned Byzantine detector data before round {} ({} faults, {} penalties remaining)",
            before_round,
            self.faults.len(),
            self.penalties.len()
        );
    }

    pub fn get_memory_stats(&self) -> ByzantineMemoryStats {
        ByzantineMemoryStats {
            tracked_rounds: self.vertices_by_authority_round.len(),
            total_faults: self.faults.len(),
            total_penalties: self.penalties.len(),
            tracked_authorities: self.reputation.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ByzantineMemoryStats {
    pub tracked_rounds: usize,
    pub total_faults: usize,
    pub total_penalties: usize,
    pub tracked_authorities: usize,
}

impl Default for ByzantineDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_vertex(round: Round, author: &str, vertex_id: u8) -> DagVertex {
        let mut id = [0u8; 32];
        id[0] = vertex_id;

        let mut parent1 = [0u8; 32];
        parent1[0] = 0;
        let mut parent2 = [0u8; 32];
        parent2[0] = 1;
        let mut parent3 = [0u8; 32];
        parent3[0] = 2;

        DagVertex {
            chain_id: "test_chain".to_string(),
            id,
            round,
            author: author.to_string(),
            parents: vec![parent1, parent2, parent3],
            transactions: vec![],
            timestamp: 0,
            signature: vec![],
            metadata: crate::consensus::dag_consensus::VertexMetadata {
                tx_count: 0,
                total_gas_used: 0,
                state_root: vec![],
                is_checkpoint: false,
                checkpoint_seq: None,
            },
            cached_serialized_data: None,
            cached_hash: None,
        }
    }

    #[test]
    fn test_double_voting_detection_and_dos_protection() {
        let mut detector = ByzantineDetector::new();
        detector.init_authority("auth1".to_string());

        let vertex1 = create_test_vertex(1, "auth1", 1);
        let vertex2 = create_test_vertex(1, "auth1", 2);
        let vertex3 = create_test_vertex(1, "auth1", 3); // Spam attempt 3

        // First attempt passes
        assert!(detector.check_double_voting(&vertex1).is_ok());
        assert_eq!(detector.get_faults().len(), 0);

        // FIX #8: Second attempt is caught and REJECTED (returns Err)
        let result = detector.check_double_voting(&vertex2);
        assert!(result.is_err(), "Double voting should be rejected");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Double voting detected")
        );
        assert_eq!(detector.get_faults().len(), 1);
        assert_eq!(detector.get_reputation("auth1"), 80);

        // Spam attempt 3 will be ignored (DoS protection) - still returns Ok because already caught
        assert!(detector.check_double_voting(&vertex3).is_ok());
        assert_eq!(detector.get_faults().len(), 1); // Does not increase
        assert_eq!(detector.get_reputation("auth1"), 80); // Score not deducted again
    }

    #[test]
    fn test_invalid_vertex_detection() {
        let mut detector = ByzantineDetector::new();
        detector.init_authority("auth1".to_string());

        let mut vertex = create_test_vertex(1, "auth1", 1);
        let insufficient_parent = [0u8; 32];
        vertex.parents = vec![insufficient_parent];

        assert!(detector.check_vertex_validity(&vertex, 4).is_ok());
        assert_eq!(detector.get_faults().len(), 1);
        assert_eq!(detector.get_reputation("auth1"), 90);
    }

    #[test]
    fn test_reputation_system() {
        let mut detector = ByzantineDetector::new();
        detector.init_authority("auth1".to_string());

        assert_eq!(detector.get_reputation("auth1"), 100);
        assert!(detector.is_trusted("auth1", 50));

        detector
            .slash_authority("auth1", 30, "Test slash", 1)
            .unwrap();
        assert_eq!(detector.get_reputation("auth1"), 70);
        assert!(detector.is_trusted("auth1", 50));
        assert!(!detector.is_trusted("auth1", 80));
    }

    #[test]
    fn test_ban_authority() {
        let mut detector = ByzantineDetector::new();
        detector.init_authority("auth1".to_string());

        detector.ban_authority("auth1");
        assert_eq!(detector.get_reputation("auth1"), 0);
        assert!(!detector.is_trusted("auth1", 1));
    }

    #[test]
    fn test_state_persistence() {
        let mut detector1 = ByzantineDetector::new();
        detector1.init_authority("auth1".to_string());
        detector1
            .slash_authority("auth1", 40, "Bad node", 1)
            .unwrap();

        // Export State to Byte Array
        let exported = detector1.export_state().unwrap();

        // Import State to new Detector
        let mut detector2 = ByzantineDetector::new();
        detector2.import_state(&exported).unwrap();

        // Verify that the hacker still has 60 score, not reverted to 100
        assert_eq!(detector2.get_reputation("auth1"), 60);
        assert_eq!(detector2.get_penalties().len(), 1);
    }
}
