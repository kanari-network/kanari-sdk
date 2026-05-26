// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Cross-shard DAG communication primitives built on top of `DagConsensus`.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::{AdaptiveQuorumConfig, DagConsensus, NetworkHealth};

pub type ShardId = u16;

/// Proof that a cross-shard payload was emitted by a specific shard checkpoint context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossShardProof {
    pub checkpoint_sequence: u64,
    pub checkpoint_hash: Vec<u8>,
    pub payload_hash: Vec<u8>,
}

impl CrossShardProof {
    pub fn verify_payload(&self, payload: &[u8]) -> bool {
        self.payload_hash == kanari_crypto::hash_data_blake3(payload)
    }
}

/// Serialized message transferred between shards.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossShardMessage {
    pub message_id: Vec<u8>,
    pub source_shard: ShardId,
    pub target_shard: ShardId,
    pub payload: Vec<u8>,
    pub proof: CrossShardProof,
}

/// Outcome of submitting a payload into a sharded DAG.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CrossShardDispatch {
    Local {
        shard_id: ShardId,
        payload_hash: Vec<u8>,
    },
    Remote(CrossShardMessage),
}

/// Simple outbound queue grouped by target shard.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossShardQueue {
    pending: BTreeMap<ShardId, VecDeque<CrossShardMessage>>,
}

impl CrossShardQueue {
    pub fn enqueue(&mut self, message: CrossShardMessage) {
        self.pending
            .entry(message.target_shard)
            .or_default()
            .push_back(message);
    }

    pub fn drain_for(&mut self, target_shard: ShardId) -> Vec<CrossShardMessage> {
        self.pending
            .remove(&target_shard)
            .map(|queue| queue.into_iter().collect())
            .unwrap_or_default()
    }

    pub fn pending_for(&self, target_shard: ShardId) -> usize {
        self.pending
            .get(&target_shard)
            .map(VecDeque::len)
            .unwrap_or(0)
    }

    pub fn total_pending(&self) -> usize {
        self.pending.values().map(VecDeque::len).sum()
    }
}

/// Minimal two-phase commit lifecycle for cross-shard operations.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AtomicCommitPhase {
    Prepare,
    Commit,
    Abort,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtomicCommitPlan {
    pub operation_id: Vec<u8>,
    pub coordinator_shard: ShardId,
    pub participant_shards: Vec<ShardId>,
    pub phase: AtomicCommitPhase,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtomicCommitVote {
    pub operation_id: Vec<u8>,
    pub shard_id: ShardId,
    pub prepared: bool,
}

/// A shard-local DAG with verified cross-shard ingress/egress queues.
pub struct ShardedDag {
    shard_id: ShardId,
    num_shards: ShardId,
    local_dag: DagConsensus,
    outbound: CrossShardQueue,
    inbound: VecDeque<CrossShardMessage>,
}

impl ShardedDag {
    pub fn new(
        shard_id: ShardId,
        num_shards: ShardId,
        authority_id: String,
        authorities: Vec<String>,
    ) -> Result<Self> {
        if num_shards == 0 {
            anyhow::bail!("num_shards must be greater than zero");
        }
        if shard_id >= num_shards {
            anyhow::bail!(
                "invalid shard id {} for shard count {}",
                shard_id,
                num_shards
            );
        }

        Ok(Self {
            shard_id,
            num_shards,
            local_dag: DagConsensus::try_with_chain_id(
                authority_id,
                authorities,
                format!("kanari-shard-{shard_id}"),
            )?,
            outbound: CrossShardQueue::default(),
            inbound: VecDeque::new(),
        })
    }

    pub fn shard_id(&self) -> ShardId {
        self.shard_id
    }

    pub fn num_shards(&self) -> ShardId {
        self.num_shards
    }

    pub fn local_dag(&self) -> &DagConsensus {
        &self.local_dag
    }

    pub fn local_dag_mut(&mut self) -> &mut DagConsensus {
        &mut self.local_dag
    }

    pub fn enable_adaptive_quorum(&mut self, config: AdaptiveQuorumConfig) {
        self.local_dag.enable_adaptive_quorum(config);
    }

    pub fn update_network_health(&mut self, health: NetworkHealth) {
        self.local_dag.update_network_health(health);
    }

    pub fn route_payload(&self, routing_key: &[u8]) -> ShardId {
        if self.num_shards == 1 {
            return 0;
        }

        let digest = kanari_crypto::hash_data_blake3(routing_key);
        let route_seed = u16::from_le_bytes([digest[0], digest[1]]);
        route_seed % self.num_shards
    }

    fn create_proof(&self, payload: &[u8]) -> Result<CrossShardProof> {
        let checkpoint = self.local_dag.latest_checkpoint();
        Ok(CrossShardProof {
            checkpoint_sequence: checkpoint.sequence,
            checkpoint_hash: checkpoint.hash()?,
            payload_hash: kanari_crypto::hash_data_blake3(payload),
        })
    }

    fn build_message(
        &self,
        target_shard: ShardId,
        payload: Vec<u8>,
        proof: CrossShardProof,
    ) -> CrossShardMessage {
        let mut id_material = Vec::new();
        id_material.extend_from_slice(&self.shard_id.to_le_bytes());
        id_material.extend_from_slice(&target_shard.to_le_bytes());
        id_material.extend_from_slice(&proof.checkpoint_sequence.to_le_bytes());
        id_material.extend_from_slice(&proof.payload_hash);
        let message_id = kanari_crypto::hash_data_blake3(&id_material);

        CrossShardMessage {
            message_id,
            source_shard: self.shard_id,
            target_shard,
            payload,
            proof,
        }
    }

    pub fn submit_payload(
        &mut self,
        routing_key: &[u8],
        payload: Vec<u8>,
    ) -> Result<CrossShardDispatch> {
        let target_shard = self.route_payload(routing_key);
        let payload_hash = kanari_crypto::hash_data_blake3(&payload);

        if target_shard == self.shard_id {
            return Ok(CrossShardDispatch::Local {
                shard_id: self.shard_id,
                payload_hash,
            });
        }

        let proof = self.create_proof(&payload)?;
        let message = self.build_message(target_shard, payload, proof);
        self.outbound.enqueue(message.clone());
        Ok(CrossShardDispatch::Remote(message))
    }

    pub fn drain_outbound_for(&mut self, target_shard: ShardId) -> Vec<CrossShardMessage> {
        self.outbound.drain_for(target_shard)
    }

    pub fn outbound_queue(&self) -> &CrossShardQueue {
        &self.outbound
    }

    pub fn receive_message(&mut self, message: CrossShardMessage) -> Result<()> {
        if message.target_shard != self.shard_id {
            anyhow::bail!(
                "cross-shard message target mismatch: expected {}, got {}",
                self.shard_id,
                message.target_shard
            );
        }
        if message.source_shard == self.shard_id {
            anyhow::bail!("cross-shard message source cannot match target shard");
        }
        if !message.proof.verify_payload(&message.payload) {
            anyhow::bail!("cross-shard message payload proof verification failed");
        }

        self.inbound.push_back(message);
        Ok(())
    }

    pub fn pop_inbound(&mut self) -> Option<CrossShardMessage> {
        self.inbound.pop_front()
    }

    pub fn begin_atomic_commit(
        &self,
        participant_shards: &[ShardId],
        payload_hash: &[u8],
    ) -> Result<AtomicCommitPlan> {
        let participants: Vec<ShardId> = participant_shards
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        if participants.is_empty() {
            anyhow::bail!("atomic commit requires at least one participant shard");
        }
        if participants.iter().any(|shard| *shard >= self.num_shards) {
            anyhow::bail!("atomic commit contains shard outside configured range");
        }

        let coordinator_shard = *participants
            .first()
            .ok_or_else(|| anyhow!("missing atomic commit coordinator"))?;

        let mut operation_material = Vec::new();
        for shard in &participants {
            operation_material.extend_from_slice(&shard.to_le_bytes());
        }
        operation_material.extend_from_slice(payload_hash);

        Ok(AtomicCommitPlan {
            operation_id: kanari_crypto::hash_data_blake3(&operation_material),
            coordinator_shard,
            participant_shards: participants,
            phase: AtomicCommitPhase::Prepare,
        })
    }

    pub fn advance_atomic_commit(
        &self,
        plan: &AtomicCommitPlan,
        votes: &[AtomicCommitVote],
    ) -> AtomicCommitPhase {
        let mut acknowledged = BTreeSet::new();
        for vote in votes {
            if vote.operation_id != plan.operation_id {
                continue;
            }
            if !vote.prepared {
                return AtomicCommitPhase::Abort;
            }
            acknowledged.insert(vote.shard_id);
        }

        if plan
            .participant_shards
            .iter()
            .all(|shard| acknowledged.contains(shard))
        {
            AtomicCommitPhase::Commit
        } else {
            AtomicCommitPhase::Prepare
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authorities() -> Vec<String> {
        vec![
            "auth1".to_string(),
            "auth2".to_string(),
            "auth3".to_string(),
            "auth4".to_string(),
        ]
    }

    fn single_authority() -> Vec<String> {
        vec!["auth1".to_string()]
    }

    fn routing_key_for_target(dag: &ShardedDag, target: ShardId) -> Vec<u8> {
        for candidate in 0u32..10_000 {
            let key = format!("route-key-{candidate}");
            if dag.route_payload(key.as_bytes()) == target {
                return key.into_bytes();
            }
        }
        panic!("failed to find routing key for target shard {target}");
    }

    fn advance_to_checkpoint_one(dag: &mut ShardedDag) {
        let round1 = dag
            .local_dag_mut()
            .create_vertex(vec![], vec![1u8; 32], 1)
            .unwrap();
        dag.local_dag_mut().add_vertex(round1).unwrap();

        let round2 = dag
            .local_dag_mut()
            .create_vertex(vec![], vec![2u8; 32], 2)
            .unwrap();
        dag.local_dag_mut().add_vertex(round2).unwrap();

        let round3 = dag
            .local_dag_mut()
            .create_vertex(vec![], vec![3u8; 32], 3)
            .unwrap();
        dag.local_dag_mut().add_vertex(round3).unwrap();

        let checkpoint = dag
            .local_dag_mut()
            .try_commit()
            .unwrap()
            .expect("checkpoint should be created");
        dag.local_dag_mut().add_checkpoint(checkpoint).unwrap();
    }

    #[test]
    fn test_route_payload_is_deterministic() {
        let dag = ShardedDag::new(0, 4, "auth1".to_string(), authorities()).unwrap();
        let a = dag.route_payload(b"account-0x1");
        let b = dag.route_payload(b"account-0x1");
        assert_eq!(a, b);
        assert!(a < dag.num_shards());
    }

    #[test]
    fn test_remote_submission_enqueues_outbound_message() {
        let mut dag = ShardedDag::new(0, 4, "auth1".to_string(), authorities()).unwrap();
        let routing_key = routing_key_for_target(&dag, 1);

        let dispatch = dag
            .submit_payload(&routing_key, b"cross-shard-transfer".to_vec())
            .unwrap();

        match dispatch {
            CrossShardDispatch::Local { .. } => {
                panic!("expected a remote dispatch for this routing key")
            }
            CrossShardDispatch::Remote(message) => {
                assert_eq!(dag.outbound_queue().pending_for(message.target_shard), 1);
                assert!(message.proof.verify_payload(&message.payload));
            }
        }
    }

    #[test]
    fn test_local_submission_does_not_enqueue_outbound_message() {
        let mut dag = ShardedDag::new(0, 1, "auth1".to_string(), authorities()).unwrap();

        let dispatch = dag
            .submit_payload(b"local-only", b"same-shard".to_vec())
            .unwrap();

        match dispatch {
            CrossShardDispatch::Local {
                shard_id,
                payload_hash,
            } => {
                assert_eq!(shard_id, dag.shard_id());
                assert_eq!(payload_hash, kanari_crypto::hash_data_blake3(b"same-shard"));
                assert_eq!(dag.outbound_queue().total_pending(), 0);
            }
            CrossShardDispatch::Remote(_) => panic!("expected local dispatch"),
        }
    }

    #[test]
    fn test_receive_message_verifies_target_and_proof() {
        let mut source = ShardedDag::new(0, 2, "auth1".to_string(), authorities()).unwrap();
        let mut target = ShardedDag::new(1, 2, "auth2".to_string(), authorities()).unwrap();
        let routing_key = routing_key_for_target(&source, 1);

        let message = match source
            .submit_payload(&routing_key, b"hello-shard-1".to_vec())
            .unwrap()
        {
            CrossShardDispatch::Remote(message) => message,
            CrossShardDispatch::Local { .. } => panic!("expected remote message"),
        };

        target.receive_message(message.clone()).unwrap();
        assert_eq!(target.pop_inbound(), Some(message));
    }

    #[test]
    fn test_cross_shard_relay_round_trip_clears_outbound_and_delivers_inbound() {
        let mut source = ShardedDag::new(0, 2, "auth1".to_string(), authorities()).unwrap();
        let mut target = ShardedDag::new(1, 2, "auth2".to_string(), authorities()).unwrap();
        let routing_key = routing_key_for_target(&source, 1);

        source
            .submit_payload(&routing_key, b"relay-payload".to_vec())
            .unwrap();
        assert_eq!(source.outbound_queue().pending_for(1), 1);

        let drained = source.drain_outbound_for(1);
        assert_eq!(drained.len(), 1);
        assert_eq!(source.outbound_queue().total_pending(), 0);

        target.receive_message(drained[0].clone()).unwrap();
        assert_eq!(target.pop_inbound(), Some(drained[0].clone()));
        assert!(target.pop_inbound().is_none());
    }

    #[test]
    fn test_remote_submission_uses_latest_checkpoint_context_in_proof() {
        let mut source = ShardedDag::new(0, 2, "auth1".to_string(), single_authority()).unwrap();
        advance_to_checkpoint_one(&mut source);
        let latest_checkpoint = source.local_dag().latest_checkpoint();
        let routing_key = routing_key_for_target(&source, 1);

        let message = match source
            .submit_payload(&routing_key, b"checkpoint-aware-proof".to_vec())
            .unwrap()
        {
            CrossShardDispatch::Remote(message) => message,
            CrossShardDispatch::Local { .. } => panic!("expected remote message"),
        };

        assert_eq!(
            message.proof.checkpoint_sequence,
            latest_checkpoint.sequence
        );
        assert_eq!(
            message.proof.checkpoint_hash,
            latest_checkpoint.hash().unwrap()
        );
    }

    #[test]
    fn test_drain_outbound_for_clears_only_requested_target_queue() {
        let mut dag = ShardedDag::new(0, 3, "auth1".to_string(), authorities()).unwrap();
        let route_to_one = routing_key_for_target(&dag, 1);
        let route_to_two = routing_key_for_target(&dag, 2);

        dag.submit_payload(&route_to_one, b"msg-one".to_vec())
            .unwrap();
        dag.submit_payload(&route_to_two, b"msg-two".to_vec())
            .unwrap();

        let shard_one_messages = dag.drain_outbound_for(1);
        assert_eq!(shard_one_messages.len(), 1);
        assert_eq!(dag.outbound_queue().pending_for(1), 0);
        assert_eq!(dag.outbound_queue().pending_for(2), 1);

        let shard_two_messages = dag.drain_outbound_for(2);
        assert_eq!(shard_two_messages.len(), 1);
        assert_eq!(dag.outbound_queue().total_pending(), 0);
    }

    #[test]
    fn test_sharded_dag_can_enable_adaptive_quorum_and_route_remote_messages() {
        let mut source = ShardedDag::new(0, 2, "auth1".to_string(), authorities()).unwrap();
        let routing_key = routing_key_for_target(&source, 1);

        source.enable_adaptive_quorum(AdaptiveQuorumConfig::default());
        source.update_network_health(NetworkHealth {
            connectivity_ratio: 0.35,
            delivery_success_ratio: 0.45,
            timeout_ratio: 0.40,
            median_latency_ms: 3_200,
        });

        assert!(
            source.local_dag().committee().required_quorum()
                >= source.local_dag().committee().quorum_size
        );

        let dispatch = source
            .submit_payload(&routing_key, b"adaptive-cross-shard".to_vec())
            .unwrap();
        assert!(matches!(dispatch, CrossShardDispatch::Remote(_)));
    }

    #[test]
    fn test_receive_message_rejects_invalid_target() {
        let mut dag = ShardedDag::new(0, 2, "auth1".to_string(), authorities()).unwrap();
        let invalid = CrossShardMessage {
            message_id: vec![1, 2, 3],
            source_shard: 1,
            target_shard: 1,
            payload: b"payload".to_vec(),
            proof: CrossShardProof {
                checkpoint_sequence: 0,
                checkpoint_hash: vec![0; 32],
                payload_hash: kanari_crypto::hash_data_blake3(b"payload"),
            },
        };

        assert!(dag.receive_message(invalid).is_err());
    }

    #[test]
    fn test_receive_message_rejects_tampered_payload() {
        let mut source = ShardedDag::new(0, 2, "auth1".to_string(), authorities()).unwrap();
        let mut target = ShardedDag::new(1, 2, "auth2".to_string(), authorities()).unwrap();
        let routing_key = routing_key_for_target(&source, 1);

        let mut message = match source
            .submit_payload(&routing_key, b"original-payload".to_vec())
            .unwrap()
        {
            CrossShardDispatch::Remote(message) => message,
            CrossShardDispatch::Local { .. } => panic!("expected remote message"),
        };

        message.payload = b"tampered-payload".to_vec();

        assert!(target.receive_message(message).is_err());
    }

    #[test]
    fn test_atomic_commit_transitions_to_commit_once_all_votes_arrive() {
        let dag = ShardedDag::new(0, 4, "auth1".to_string(), authorities()).unwrap();
        let payload_hash = kanari_crypto::hash_data_blake3(b"atomic-op");
        let plan = dag.begin_atomic_commit(&[0, 2, 3], &payload_hash).unwrap();

        let prepare_only = dag.advance_atomic_commit(
            &plan,
            &[AtomicCommitVote {
                operation_id: plan.operation_id.clone(),
                shard_id: 0,
                prepared: true,
            }],
        );
        assert_eq!(prepare_only, AtomicCommitPhase::Prepare);

        let commit = dag.advance_atomic_commit(
            &plan,
            &[
                AtomicCommitVote {
                    operation_id: plan.operation_id.clone(),
                    shard_id: 0,
                    prepared: true,
                },
                AtomicCommitVote {
                    operation_id: plan.operation_id.clone(),
                    shard_id: 2,
                    prepared: true,
                },
                AtomicCommitVote {
                    operation_id: plan.operation_id.clone(),
                    shard_id: 3,
                    prepared: true,
                },
            ],
        );
        assert_eq!(commit, AtomicCommitPhase::Commit);
    }

    #[test]
    fn test_atomic_commit_aborts_on_negative_vote() {
        let dag = ShardedDag::new(0, 3, "auth1".to_string(), authorities()).unwrap();
        let payload_hash = kanari_crypto::hash_data_blake3(b"atomic-op");
        let plan = dag.begin_atomic_commit(&[0, 1], &payload_hash).unwrap();
        let phase = dag.advance_atomic_commit(
            &plan,
            &[AtomicCommitVote {
                operation_id: plan.operation_id.clone(),
                shard_id: 1,
                prepared: false,
            }],
        );
        assert_eq!(phase, AtomicCommitPhase::Abort);
    }

    #[test]
    fn test_begin_atomic_commit_deduplicates_and_sorts_participants() {
        let dag = ShardedDag::new(0, 4, "auth1".to_string(), authorities()).unwrap();
        let payload_hash = kanari_crypto::hash_data_blake3(b"dedupe-op");

        let plan = dag
            .begin_atomic_commit(&[3, 1, 3, 2, 1], &payload_hash)
            .unwrap();

        assert_eq!(plan.coordinator_shard, 1);
        assert_eq!(plan.participant_shards, vec![1, 2, 3]);
    }
}
