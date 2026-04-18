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
use std::collections::BTreeMap;

use super::{AuthorityId, DagVertex, Round, VertexId};

/// Types of Byzantine faults
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ByzantineFault {
    DoubleVoting {
        authority: AuthorityId,
        round: Round,
        vertices: Vec<VertexId>,
    },
    InvalidVertex {
        authority: AuthorityId,
        vertex_id: VertexId,
        reason: String,
    },
    Equivocation {
        authority: AuthorityId,
        round: Round,
        evidence: Vec<u8>,
    },
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
    reputation: BTreeMap<AuthorityId, u64>,
    vertices_by_authority_round: BTreeMap<(AuthorityId, Round), Vec<VertexId>>,
}

/// Byzantine detector
pub struct ByzantineDetector {
    faults: Vec<ByzantineEvidence>,
    penalties: Vec<SlashingPenalty>,
    reputation: BTreeMap<AuthorityId, u64>,
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
            } => (authority, 20, *round, "Double voting detected"),
            ByzantineFault::InvalidVertex { authority, .. } => (authority, 10, 0, "Invalid vertex"),
            ByzantineFault::Equivocation {
                authority, round, ..
            } => (authority, 30, *round, "Equivocation"),
            ByzantineFault::Withholding { authority, round } => {
                (authority, 5, *round, "Withholding")
            }
        }
    }

    pub fn new() -> Self {
        Self {
            faults: Vec::new(),
            penalties: Vec::new(),
            reputation: BTreeMap::new(),
            vertices_by_authority_round: BTreeMap::new(),
        }
    }

    pub fn init_authority(&mut self, authority: AuthorityId) {
        self.reputation.insert(authority, 100);
    }

    // --- FIX 1: ป้องกัน O(N^2) DoS Attack ---
    pub fn check_double_voting(&mut self, vertex: &DagVertex) -> Result<()> {
        let key = (vertex.author.clone(), vertex.round);
        let existing = self.vertices_by_authority_round.entry(key).or_default();

        // กรณีปกติ: ยังไม่เคยโหวตในรอบนี้
        if existing.is_empty() {
            existing.push(vertex.id);
            return Ok(());
        }

        // กรณีผิดปกติครั้งแรก: พบการโหวตครั้งที่ 2
        if existing.len() == 1 && existing[0] != vertex.id {
            existing.push(vertex.id);
            let fault = ByzantineFault::DoubleVoting {
                authority: vertex.author.clone(),
                round: vertex.round,
                vertices: existing.clone(),
            };
            // รายงานและหักคะแนน
            self.report_fault(fault)?;

            // FIX #8: CRITICAL - Reject the vertex after slashing
            // Previously returned Ok(()) which allowed double-voted vertex into DAG
            return Err(anyhow::anyhow!(
                "Double voting detected and slashed. Vertex rejected from DAG."
            ));
        }

        // ถ้า existing.len() >= 2 แปลว่าเคยจับได้และลงโทษไปแล้ว
        // ให้ return กลับทันที (O(1) Time) ไม่ต้องเสียเวลาประมวลผลซ้ำ
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
            let f = (total_authorities - 1) / 3;
            let quorum = 2 * f + 1;

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

        // ตัดคะแนนความประพฤติ (ต่ำสุดคือ 0)
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

    // --- FIX 2: ระบบ Export/Import สถานะลง Disk (Persistent State) ---
    /// ส่งออกประวัติคนร้ายเพื่อเก็บถาวร (สำหรับเปิดโหนดใหม่)
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

    /// โหลดประวัติคนร้ายกลับมาหลังจากเปิดโหนดใหม่
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
            .insert(authority.to_string(), score.min(100));
    }

    pub fn ban_authority(&mut self, authority: &str) {
        self.reputation.insert(authority.to_string(), 0);
        tracing::warn!("Authority {} has been banned (reputation = 0)", authority);
    }

    pub fn prune_old_rounds(&mut self, before_round: Round) {
        self.retain_round_tracking(before_round);

        const MAX_FAULTS: usize = 10000;
        const MAX_PENALTIES: usize = 10000;

        Self::retain_recent_records(&mut self.faults, MAX_FAULTS);
        Self::retain_recent_records(&mut self.penalties, MAX_PENALTIES);

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
        let vertex3 = create_test_vertex(1, "auth1", 3); // สแปมครั้งที่ 3

        // ครั้งแรกผ่าน
        assert!(detector.check_double_voting(&vertex1).is_ok());
        assert_eq!(detector.get_faults().len(), 0);

        // FIX #8: ครั้งที่สองจับได้และ REJECT (return Err)
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

        // สแปมครั้งที่สาม จะไม่สนใจแล้ว (กัน DoS attack) - still returns Ok because already caught
        assert!(detector.check_double_voting(&vertex3).is_ok());
        assert_eq!(detector.get_faults().len(), 1); // ไม่เพิ่มขึ้น
        assert_eq!(detector.get_reputation("auth1"), 80); // คะแนนไม่โดนหักซ้ำซ้อน
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

        // ส่งออก State ลง Byte Array
        let exported = detector1.export_state().unwrap();

        // นำเข้า State ไปยัง Detector ตัวใหม่
        let mut detector2 = ByzantineDetector::new();
        detector2.import_state(&exported).unwrap();

        // ตรวจสอบว่าแฮกเกอร์ยังมีคะแนน 60 อยู่ ไม่ใช่กลับมาเป็น 100
        assert_eq!(detector2.get_reputation("auth1"), 60);
        assert_eq!(detector2.get_penalties().len(), 1);
    }
}
