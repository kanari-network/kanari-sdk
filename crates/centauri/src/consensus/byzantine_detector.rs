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
use std::collections::HashMap;

use super::{AuthorityId, DagVertex, Round, VertexId};

/// Types of Byzantine faults
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ByzantineFault {
    /// Authority created multiple vertices in the same round
    DoubleVoting {
        authority: AuthorityId,
        round: Round,
        vertices: Vec<VertexId>,
    },

    /// Vertex has invalid parents (wrong round, insufficient quorum)
    InvalidVertex {
        authority: AuthorityId,
        vertex_id: VertexId,
        reason: String,
    },

    /// Authority signed conflicting statements
    Equivocation {
        authority: AuthorityId,
        round: Round,
        evidence: Vec<u8>,
    },

    /// Authority failed to participate when required
    Withholding {
        authority: AuthorityId,
        round: Round,
    },
}

/// Evidence of Byzantine fault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByzantineEvidence {
    /// The fault detected
    pub fault: ByzantineFault,

    /// Timestamp when detected
    pub detected_at: u64,

    /// Proof/evidence of the fault
    pub proof: Vec<u8>,
}

/// Slashing penalty
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingPenalty {
    /// Authority being slashed
    pub authority: AuthorityId,

    /// Amount to slash (percentage or absolute)
    pub amount: u64,

    /// Reason for slashing
    pub reason: String,

    /// Round when slashing occurred
    pub round: Round,
}

/// Byzantine detector
pub struct ByzantineDetector {
    /// Detected faults
    faults: Vec<ByzantineEvidence>,

    /// Slashing penalties applied
    penalties: Vec<SlashingPenalty>,

    /// Authority reputation scores (0-100)
    reputation: HashMap<AuthorityId, u64>,

    /// Vertices created by each authority per round
    vertices_by_authority_round: HashMap<(AuthorityId, Round), Vec<VertexId>>,
}

impl ByzantineDetector {
    /// Create a new Byzantine detector
    pub fn new() -> Self {
        Self {
            faults: Vec::new(),
            penalties: Vec::new(),
            reputation: HashMap::new(),
            vertices_by_authority_round: HashMap::new(),
        }
    }

    /// Initialize authority with reputation score
    pub fn init_authority(&mut self, authority: AuthorityId) {
        self.reputation.insert(authority, 100); // Start with perfect score
    }

    /// Check for double voting
    pub fn check_double_voting(&mut self, vertex: &DagVertex) -> Result<()> {
        let key = (vertex.author.clone(), vertex.round);

        let existing_vertices = self
            .vertices_by_authority_round
            .entry(key.clone())
            .or_default();

        existing_vertices.push(vertex.id.clone());

        // Detect double voting: multiple vertices in same round
        if existing_vertices.len() > 1 {
            let fault = ByzantineFault::DoubleVoting {
                authority: vertex.author.clone(),
                round: vertex.round,
                vertices: existing_vertices.clone(),
            };

            self.report_fault(fault)?;
        }

        Ok(())
    }

    /// Check vertex validity
    pub fn check_vertex_validity(
        &mut self,
        vertex: &DagVertex,
        total_authorities: usize,
    ) -> Result<()> {
        // Check quorum
        if vertex.round > 0 {
            let f = (total_authorities - 1) / 3;
            let quorum = 2 * f + 1;

            if vertex.parents.len() < quorum {
                let fault = ByzantineFault::InvalidVertex {
                    authority: vertex.author.clone(),
                    vertex_id: vertex.id.clone(),
                    reason: format!(
                        "Insufficient parents: {} < {} (quorum)",
                        vertex.parents.len(),
                        quorum
                    ),
                };

                self.report_fault(fault)?;
            }
        }

        Ok(())
    }

    /// Report a Byzantine fault
    pub fn report_fault(&mut self, fault: ByzantineFault) -> Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let evidence = ByzantineEvidence {
            fault: fault.clone(),
            detected_at: timestamp,
            proof: vec![], // Simplified; in production, include cryptographic proof
        };

        self.faults.push(evidence);

        // Apply penalty
        match &fault {
            ByzantineFault::DoubleVoting {
                authority, round, ..
            } => {
                self.slash_authority(authority, 20, "Double voting detected", *round)?;
            }
            ByzantineFault::InvalidVertex { authority, .. } => {
                self.slash_authority(authority, 10, "Invalid vertex", 0)?;
            }
            ByzantineFault::Equivocation {
                authority, round, ..
            } => {
                self.slash_authority(authority, 30, "Equivocation", *round)?;
            }
            ByzantineFault::Withholding { authority, round } => {
                self.slash_authority(authority, 5, "Withholding", *round)?;
            }
        }

        Ok(())
    }

    /// Slash an authority's reputation
    fn slash_authority(
        &mut self,
        authority: &str,
        penalty: u64,
        reason: &str,
        round: Round,
    ) -> Result<()> {
        let reputation = self.reputation.entry(authority.to_string()).or_insert(100);

        // Reduce reputation (minimum 0)
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

    /// Get authority reputation score
    pub fn get_reputation(&self, authority: &str) -> u64 {
        self.reputation.get(authority).copied().unwrap_or(0)
    }

    /// Check if authority is trusted (reputation > threshold)
    pub fn is_trusted(&self, authority: &str, threshold: u64) -> bool {
        self.get_reputation(authority) >= threshold
    }

    /// Get all faults
    pub fn get_faults(&self) -> &[ByzantineEvidence] {
        &self.faults
    }

    /// Get all penalties
    pub fn get_penalties(&self) -> &[SlashingPenalty] {
        &self.penalties
    }

    /// Get faults for a specific authority
    pub fn get_authority_faults(&self, authority: &str) -> Vec<&ByzantineEvidence> {
        self.faults
            .iter()
            .filter(|evidence| match &evidence.fault {
                ByzantineFault::DoubleVoting {
                    authority: auth, ..
                } => auth == authority,
                ByzantineFault::InvalidVertex {
                    authority: auth, ..
                } => auth == authority,
                ByzantineFault::Equivocation {
                    authority: auth, ..
                } => auth == authority,
                ByzantineFault::Withholding {
                    authority: auth, ..
                } => auth == authority,
            })
            .collect()
    }

    /// Reset authority reputation (e.g., after governance decision)
    pub fn reset_reputation(&mut self, authority: &str, score: u64) {
        self.reputation
            .insert(authority.to_string(), score.min(100));
    }

    /// Ban authority (set reputation to 0)
    pub fn ban_authority(&mut self, authority: &str) {
        self.reputation.insert(authority.to_string(), 0);
        tracing::warn!("Authority {} has been banned (reputation = 0)", authority);
    }
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
        DagVertex {
            id: vec![vertex_id],
            round,
            author: author.to_string(),
            parents: vec![vec![0], vec![1], vec![2]], // 3 parents for quorum
            transactions: vec![],
            timestamp: 0,
            signature: vec![],
            metadata: super::super::VertexMetadata {
                tx_count: 0,
                total_gas_used: 0,
                state_root: vec![],
                is_checkpoint: false,
                checkpoint_seq: None,
            },
        }
    }

    #[test]
    fn test_double_voting_detection() {
        let mut detector = ByzantineDetector::new();
        detector.init_authority("auth1".to_string());

        let vertex1 = create_test_vertex(1, "auth1", 1);
        let vertex2 = create_test_vertex(1, "auth1", 2); // Same round!

        // First vertex is OK
        assert!(detector.check_double_voting(&vertex1).is_ok());

        // Second vertex in same round = double voting
        assert!(detector.check_double_voting(&vertex2).is_ok());

        // Should have detected 1 fault
        assert_eq!(detector.get_faults().len(), 1);

        // Reputation should be reduced
        assert_eq!(detector.get_reputation("auth1"), 80); // 100 - 20
    }

    #[test]
    fn test_invalid_vertex_detection() {
        let mut detector = ByzantineDetector::new();
        detector.init_authority("auth1".to_string());

        let mut vertex = create_test_vertex(1, "auth1", 1);
        vertex.parents = vec![vec![0]]; // Only 1 parent (insufficient)

        // Should detect invalid vertex
        assert!(detector.check_vertex_validity(&vertex, 4).is_ok());

        // Should have 1 fault
        assert_eq!(detector.get_faults().len(), 1);

        // Reputation reduced
        assert_eq!(detector.get_reputation("auth1"), 90); // 100 - 10
    }

    #[test]
    fn test_reputation_system() {
        let mut detector = ByzantineDetector::new();
        detector.init_authority("auth1".to_string());

        assert_eq!(detector.get_reputation("auth1"), 100);
        assert!(detector.is_trusted("auth1", 50));

        // Slash
        detector
            .slash_authority("auth1", 30, "Test slash", 1)
            .unwrap();
        assert_eq!(detector.get_reputation("auth1"), 70);

        // Still trusted above 50
        assert!(detector.is_trusted("auth1", 50));

        // Not trusted above 80
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
}
