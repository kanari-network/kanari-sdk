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
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::consensus::Committee;

use super::{AuthorityId, Checkpoint, DagVertex, Round, VertexId};

const MAX_SYNC_CHECKPOINTS: usize = 100;
const MAX_SYNC_VERTICES: usize = 5000;

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

    // --- FIX #14: Orphan Buffer with FIFO eviction tracking ---
    orphan_buffer: BTreeMap<VertexId, Arc<DagVertex>>,
    orphan_insertion_order: std::collections::VecDeque<VertexId>, // Track insertion order for proper FIFO eviction
    waiting_for: BTreeMap<VertexId, Vec<VertexId>>, // parent_id -> list of orphan children waiting
}

impl StateSynchronizer {
    fn unix_timestamp_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
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
        // ค้นหาใน Orphan Buffer ด้วย เผื่อคนอื่นขอมา
        if let Some(orphan) = self.orphan_buffer.get(vertex_id) {
            return Some((**orphan).clone());
        }

        let (round, index) = *self.vertex_lookup.get(vertex_id)?;
        self.vertices_by_round
            .get(&round)
            .and_then(|vertices| vertices.get(index))
            .map(|vertex| (**vertex).clone())
    }

    fn latest_state_root(&self) -> Vec<u8> {
        self.checkpoints
            .get(&self.latest_checkpoint)
            .map(|c| c.state_root.clone())
            .unwrap_or_default()
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
            orphan_buffer: BTreeMap::new(),
            orphan_insertion_order: std::collections::VecDeque::new(), // FIX #14
            waiting_for: BTreeMap::new(),
        }
    }

    pub fn add_checkpoint(&mut self, checkpoint: Checkpoint) {
        let seq = checkpoint.sequence;
        self.checkpoints.insert(seq, checkpoint);
        if seq > self.latest_checkpoint {
            self.latest_checkpoint = seq;
        }
    }

    /// Add vertex (คืนค่าจำนวน Vertex ที่เพิ่มสำเร็จ รวมถึงตัวกำพร้าที่ถูก Process)
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

        // --- FIX 5: ป้องกัน Cyclic DAG (A -> B -> A) และการอ้างอิง Round ผิดปกติ ---
        for parent_id in &vertex.parents {
            if let Some(parent) = self.find_vertex(parent_id)
                && parent.round >= vertex.round
            {
                tracing::warn!(
                    "Security Warning: Rejecting cyclic or invalid round progression. Vertex {}, Parent {}",
                    vertex.round,
                    parent.round
                );
                return 0; // เตะทิ้งทันทีหากบล็อกลูกมี Round น้อยกว่าหรือเท่ากับแม่
            }
        }

        // หาว่า Parent ตัวไหนยังไม่มา
        let missing_parents: Vec<_> = vertex
            .parents
            .iter()
            .filter(|p| !self.vertex_lookup.contains_key(*p))
            .copied()
            .collect();

        // ถ้ามี Parent หายไป ให้เก็บลง Orphan Buffer อย่าเพิ่งทิ้ง
        if vertex.round > 0 && !missing_parents.is_empty() {
            // --- FIX #14: CRITICAL - Proper FIFO eviction for orphan buffer ---
            // Previously used keys().next() which evicts by hash order (not insertion time)
            // causing memory leaks where old orphans with high hashes stay forever
            const MAX_ORPHAN_SIZE: usize = 50_000;

            if self.orphan_buffer.len() >= MAX_ORPHAN_SIZE {
                // FIX #14: Evict oldest orphan by insertion order (FIFO), not hash order
                if let Some(oldest_key) = self.orphan_insertion_order.pop_front() {
                    self.orphan_buffer.remove(&oldest_key);
                    // Also clean up waiting_for references
                    self.waiting_for.remove(&oldest_key);
                }
            }

            // Track insertion order for proper FIFO eviction
            self.orphan_insertion_order.push_back(vertex_id);
            self.orphan_buffer.insert(vertex_id, Arc::clone(&vertex));

            for p in missing_parents {
                let waiters = self.waiting_for.entry(p).or_default();
                // จำกัดจำนวนลูกที่รอคอย Parent เดียวกัน ไม่ให้กิน RAM อนันต์จากการยิงสแปม
                if waiters.len() < 1000 {
                    waiters.push(vertex_id);
                } else {
                    tracing::warn!("Max waiters limit reached for parent: {}", hex::encode(p));
                }
            }
            return 0; // ยังไม่ได้ถูกนำเข้าเชนหลัก
        }

        // ถ้า Parent ครบแล้ว นำเข้าเชนหลักและเคลียร์ Buffer
        self.insert_to_main_dag(vertex)
    }

    /// นำเข้าเชนหลัก และปลดปล่อยบล็อกลูกที่รอคอยอยู่แบบ Recursive
    fn insert_to_main_dag(&mut self, initial_vertex: Arc<DagVertex>) -> usize {
        let mut added_count = 0;
        let mut stack = vec![initial_vertex]; // ใช้ Stack ธรรมดาแทน Recursion

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
                        // ตรวจสอบว่าเด็กคนนี้ Parent คนอื่นๆ มาครบหมดหรือยัง
                        let still_missing = child
                            .parents
                            .iter()
                            .any(|p| !self.vertex_lookup.contains_key(p));

                        if !still_missing {
                            // FIX #14: Remove from insertion order tracking
                            self.orphan_insertion_order.retain(|id| id != &child_id);
                            self.orphan_buffer.remove(&child_id);
                            // นำเด็กลง Stack แทนการเรียก Recursive Call ทันที
                            stack.push(child);
                        }
                    }
                }
            }
        }

        added_count
    }

    pub fn create_sync_request(&self, requester: AuthorityId) -> SyncRequest {
        SyncRequest {
            requester,
            last_checkpoint: self.latest_checkpoint,
            last_round: self.latest_round,
            missing_vertices: vec![],
        }
    }

    pub fn handle_sync_request(&self, request: &SyncRequest) -> Result<SyncResponse> {
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
        let mut seen: BTreeSet<VertexId> = vertices.iter().map(|v| v.id).collect();
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
            .or_else(|| {
                self.checkpoints
                    .get(&request.last_checkpoint)
                    .map(|c| c.state_root.clone())
            })
            .unwrap_or_else(|| self.latest_state_root());

        Ok(SyncResponse {
            checkpoints,
            vertices,
            current_round: self.latest_round,
            state_root: response_state_root,
        })
    }

    // FIX 2: เพิ่มพารามิเตอร์ `committee: &Committee` เพื่อใช้ดึง Public Key มาตรวจสอบลายเซ็น
    pub fn apply_sync_response(
        &mut self,
        response: SyncResponse,
        committee: &Committee,
    ) -> Result<()> {
        let last_valid_checkpoint = self
            .get_latest_checkpoint()
            .ok_or_else(|| anyhow!("No local state"))?;

        // FIX 9: ตรวจสอบความถูกต้องของ State Root อย่างเคร่งครัด
        if response.checkpoints.is_empty()
            && response.state_root != last_valid_checkpoint.state_root
        {
            anyhow::bail!("State root mismatch with target sync point");
        }

        let expected_root = response
            .checkpoints
            .last()
            .map(|c| c.state_root.clone())
            .unwrap_or_else(|| self.latest_state_root());

        if expected_root != response.state_root {
            return Err(anyhow!(
                "State root mismatch in sync response: expected {:?}, got {:?}",
                expected_root,
                response.state_root
            ));
        }

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
            // --- FIX 2: ป้องกัน State Poisoning ด้วยการตรวจสอบ Signature ก่อนรับบล็อก ---
            if let Some(validator) = committee.get_validator(&vertex.author) {
                // แปลง Public Key Bytes เป็น VerifyingKey
                let pub_key_bytes: [u8; 32] =
                    validator.public_key.clone().try_into().unwrap_or([0u8; 32]);

                if let Ok(pub_key) = ed25519_dalek::VerifyingKey::from_bytes(&pub_key_bytes) {
                    // เรียกใช้ฟังก์ชันตรวจสอบลายเซ็น (Zero-copy optimization) จาก ParallelValidator
                    if crate::consensus::parallel_validator::ParallelValidator::verify_vertex_signature(&vertex, &pub_key).is_err() {
                        tracing::error!("[Security] Invalid signature in synced vertex: {}", hex::encode(vertex.id));
                        continue; // เตะบล็อกเถื่อนทิ้งทันที
                    }
                } else {
                    continue; // Public Key ผิดรูปแบบ
                }
            } else {
                tracing::warn!(
                    "[Security] Unknown author {} in synced vertex",
                    vertex.author
                );
                continue; // ข้ามบล็อกที่มาจากคนที่ไม่ได้อยู่ใน Committee
            }
            // ----------------------------------------------------------------------

            let added_count = self.add_vertex(vertex);
            if let Some(ref mut progress) = self.sync_progress {
                progress.synced_vertices += added_count;
            }
        }

        tracing::info!(
            "Sync complete: {} checkpoints, {} vertices ({} orphans), current round: {}",
            self.checkpoints.len(),
            self.vertices_by_round.values().map(Vec::len).sum::<usize>(),
            self.orphan_buffer.len(),
            self.latest_round
        );

        Ok(())
    }

    pub fn get_sync_progress(&self) -> Option<&SyncProgress> {
        self.sync_progress.as_ref()
    }

    pub fn is_syncing(&self) -> bool {
        self.sync_progress
            .as_ref()
            .map(|p| !p.is_complete())
            .unwrap_or(false)
    }

    pub fn get_latest_checkpoint(&self) -> Option<&Checkpoint> {
        self.checkpoints.get(&self.latest_checkpoint)
    }

    pub fn get_checkpoint(&self, sequence: u64) -> Option<&Checkpoint> {
        self.checkpoints.get(&sequence)
    }

    pub fn get_round_vertices(&self, round: Round) -> Option<&[Arc<DagVertex>]> {
        self.vertices_by_round.get(&round).map(|v| v.as_slice())
    }

    pub fn get_latest_round(&self) -> Round {
        self.latest_round
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

        // ล้าง Orphan ที่เก่าเกินไปและยังไม่มี Parent มาสักที (ป้องกัน Memory Leak)
        self.orphan_buffer.retain(|_, v| v.round >= before_round);
        let valid_orphans: BTreeSet<_> = self.orphan_buffer.keys().copied().collect();
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

    pub fn get_memory_stats(&self) -> (usize, usize, usize) {
        (
            self.checkpoints.len(),
            self.vertices_by_round.len(),
            self.orphan_buffer.len(),
        )
    }
}

impl Default for StateSynchronizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Fast state sync using checkpoints (skip intermediate vertices)
pub struct FastSync {
    checkpoint_interval: Round,
    checkpoints: Vec<Checkpoint>,
}

impl FastSync {
    pub fn new(checkpoint_interval: Round) -> Self {
        Self {
            checkpoint_interval,
            checkpoints: vec![Checkpoint::genesis()],
        }
    }

    pub fn add_checkpoint(&mut self, checkpoint: Checkpoint) {
        self.checkpoints.push(checkpoint);
    }

    pub fn get_fast_sync_checkpoint(&self, min_age: u64) -> Option<&Checkpoint> {
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

    pub fn checkpoints_to_skip(&self, from_checkpoint: u64, to_checkpoint: u64) -> u64 {
        to_checkpoint.saturating_sub(from_checkpoint)
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
        assert!(sync.get_checkpoint(1).is_some());
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
        use serde::Serialize;

        let mut sync = StateSynchronizer::new();

        // 1. สร้าง Keypair และ Committee จำลองสำหรับการทดสอบ
        let keypair = Ed25519Keypair::generate();
        let pub_key_bytes = keypair.public().to_bytes().to_vec();

        let validator = ValidatorInfo {
            authority_id: "auth1".to_string(),
            public_key: pub_key_bytes,
            stake: 100,
            network_address: "127.0.0.1:9000".to_string(),
            active: true,
        };
        let committee = Committee::new(0, vec![validator]);

        // 2. สร้าง Vertex ชั่วคราว
        let mut vertex =
            DagVertex::new_for_test(1, "auth1".to_string(), vec![], vec![], vec![0u8; 32], 0);

        // 3. สร้างลายเซ็นจำลอง (Sign) เพื่อให้ผ่านระบบ ParallelValidator::verify_vertex_signature
        #[derive(Serialize)]
        struct DagVertexSigningRef<'a> {
            id: &'a VertexId,
            round: Round,
            author: &'a AuthorityId,
            chain_id: &'a String,
            parents: &'a Vec<VertexId>,
            transactions: &'a Vec<kanari_types::transaction::SignedTransaction>,
            timestamp: u64,
            signature: &'static [u8],
            metadata: &'a crate::consensus::dag_consensus::VertexMetadata,
        }

        let signing_ref = DagVertexSigningRef {
            id: &vertex.id,
            round: vertex.round,
            author: &vertex.author,
            chain_id: &vertex.chain_id,
            parents: &vertex.parents,
            transactions: &vertex.transactions,
            timestamp: vertex.timestamp,
            signature: &[],
            metadata: &vertex.metadata,
        };
        let payload = bcs::to_bytes(&signing_ref).unwrap();
        vertex.signature = keypair.sign(&payload);

        // 4. สร้าง SyncResponse
        let response = SyncResponse {
            checkpoints: vec![],
            vertices: vec![vertex],
            current_round: 1,
            state_root: Checkpoint::genesis().state_root,
        };

        // 5. ทดสอบใช้งานโดยส่ง &committee เข้าไปด้วย
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

        // ใส่บล็อกลูกก่อน (สถานการณ์เน็ตเวิร์กสลับลำดับ)
        assert_eq!(sync.add_vertex(child), 0); // ยังไม่เข้าเชนหลัก 
        assert_eq!(sync.orphan_buffer.len(), 1); // แต่ไปอยู่ในสถานรับเลี้ยงเด็กกำพร้า
        assert_eq!(sync.latest_round, 0);

        // พอใส่บล็อกแม่ตามหลังมา
        assert_eq!(sync.add_vertex(parent), 2); // มันจะปลดล็อกลูกออกมาด้วย เลยคืนค่ากลับมาเป็น 2 บล็อก
        assert_eq!(sync.orphan_buffer.len(), 0); // สถานรับเลี้ยงเด็กว่างเปล่า
        assert_eq!(sync.latest_round, 2); // ซิงค์ทะลุไปถึงรอบของลูกได้ทันที
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
