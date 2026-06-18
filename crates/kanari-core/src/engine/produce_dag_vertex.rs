// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use log::{info, warn};
use mysticeti_consensus::{
    committer::Committer as MysticetiCommitter,
    protocol::{
        ConsensusProtocol as MysticetiConsensusProtocol, Protocol as MysticetiRuntimeProtocol,
    },
};
use mysticeti_dag::{
    authority::Authority as MysticetiAuthority,
    block::{
        BlockReference as MysticetiBlockReference, RoundNumber as MysticetiRound,
        transaction::Transaction as MysticetiTransaction,
    },
    committee::Committee as MysticetiCommittee,
    context::TokioCtx as MysticetiTokioCtx,
    core::{Core as MysticetiCore, block_handler::RealBlockHandler as MysticetiBlockHandler},
    crypto::CryptoEngine as MysticetiCryptoEngine,
    metrics::Metrics as MysticetiMetrics,
    storage::Storage as MysticetiStorage,
};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

use crate::consensus::{
    Checkpoint, ConsensusRuntimeProtocol, DagMetrics, DagProductionPolicy, DagVertex,
    PersistentDagState, VertexId,
};

use super::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckpointProductionInfo {
    pub vertex_id: String,
    pub round: u64,
    pub tx_count: usize,
    pub executed: usize,
    pub failed: usize,
    pub events: Vec<Event>,
    pub checkpoint: Option<CheckpointInfo>,
    pub vertex: Option<DagVertex>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckpointInfo {
    pub sequence: u64,
    pub vertex_count: usize,
    pub tx_count: usize,
}

pub struct MysticetiBackend {
    _committee: Arc<MysticetiCommittee>,
    core: MysticetiCore<MysticetiTokioCtx, MysticetiCommitter>,
    transaction_sender: mpsc::Sender<Vec<MysticetiTransaction>>,
    protocol: MysticetiRuntimeProtocol,
}

impl MysticetiBackend {
    fn new(authority_count: usize) -> Result<Self> {
        let authority_count = authority_count.max(1);
        let committee = MysticetiCommittee::new_test(vec![1; authority_count]);
        let protocol_config = MysticetiConsensusProtocol::Mysticeti {
            leader_count: NonZeroUsize::new(authority_count.min(2).max(1))
                .expect("leader count is non-zero"),
        };
        let protocol = protocol_config
            .to_protocol(&committee)
            .map_err(|e| anyhow::anyhow!("Failed to build Mysticeti protocol: {}", e))?;
        let metrics = MysticetiMetrics::new_for_test(committee.len());
        let (storage, recovered) = MysticetiStorage::ephemeral(
            MysticetiAuthority::default(),
            metrics.clone(),
            committee.as_ref(),
        );
        let (block_handler, transaction_sender) =
            MysticetiBlockHandler::<MysticetiTokioCtx>::new(metrics.clone());
        let committer =
            MysticetiCommitter::new(committee.clone(), storage.block_reader().clone(), protocol);
        let protocol = protocol_config
            .to_protocol(&committee)
            .map_err(|e| anyhow::anyhow!("Failed to rebuild Mysticeti protocol: {}", e))?;
        let core = MysticetiCore::open(
            block_handler,
            MysticetiAuthority::default(),
            committee.clone(),
            metrics,
            storage,
            recovered,
            false,
            committer,
            MysticetiCryptoEngine::disabled(),
        );

        Ok(Self {
            _committee: committee,
            core,
            transaction_sender,
            protocol,
        })
    }

    fn runtime_protocol(&self) -> ConsensusRuntimeProtocol {
        ConsensusRuntimeProtocol::from_mysticeti(&self.protocol)
    }

    fn quorum_threshold(&self) -> usize {
        self.protocol.direct_commit_quorum as usize
    }

    fn try_advance(&mut self) {
        let committed = self.core.try_commit();
        if !committed.is_empty() {
            log::debug!("Mysticeti committed {} leader block(s)", committed.len());
        }
    }

    fn propose_block(
        &mut self,
        transactions: &[SignedTransaction],
        timestamp_ms: u64,
    ) -> Result<Option<MysticetiBlockSummary>> {
        if !transactions.is_empty() {
            self.transaction_sender
                .try_send(vec![signed_tx_batch_to_mysticeti_transaction(
                    transactions,
                    timestamp_ms,
                )])
                .map_err(|e| {
                    anyhow::anyhow!("Failed to submit transactions to Mysticeti Core: {}", e)
                })?;
        }

        self.core.drain_submitted_transactions();
        let Some(block) = self.core.try_new_block() else {
            return Ok(None);
        };
        let reference = *block.reference();
        Ok(Some(MysticetiBlockSummary {
            vertex_id: mysticeti_reference_to_vertex_id(&reference),
            round: block.round(),
            parents: block
                .includes()
                .iter()
                .map(mysticeti_reference_to_vertex_id)
                .collect(),
        }))
    }
}

struct MysticetiBlockSummary {
    vertex_id: VertexId,
    round: MysticetiRound,
    parents: Vec<VertexId>,
}

fn signed_tx_batch_to_mysticeti_transaction(
    transactions: &[SignedTransaction],
    timestamp_ms: u64,
) -> MysticetiTransaction {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"kanari:mysticeti-batch:v1");
    hasher.update(&timestamp_ms.to_le_bytes());
    hasher.update(&(transactions.len() as u64).to_le_bytes());
    for tx in transactions {
        hasher.update(tx.transaction_hash());
    }

    MysticetiTransaction::new(minibytes::Bytes::copy_from_slice(
        hasher.finalize().as_bytes(),
    ))
}

fn mysticeti_reference_to_vertex_id(reference: &MysticetiBlockReference) -> VertexId {
    let mut id = [0u8; 32];
    id.copy_from_slice(reference.digest.as_ref());
    id
}

pub struct CoreDagConsensus {
    authority_id: String,
    authorities: Vec<String>,
    vertices: Vec<DagVertex>,
    checkpoints: Vec<Checkpoint>,
    current_round: u64,
    last_checkpoint_round: u64,
    metrics: DagMetrics,
    mysticeti: MysticetiBackend,
}

impl CoreDagConsensus {
    fn new(
        authority_id: String,
        authorities: Vec<String>,
        state: Option<PersistentDagState>,
    ) -> Result<Self> {
        let mut checkpoints = vec![Checkpoint::genesis()];
        let mut vertices = Vec::new();
        let mut current_round = 0;
        let mut last_checkpoint_round = 0;

        if let Some(state) = state {
            if !state.checkpoints.is_empty() {
                checkpoints = state.checkpoints;
            }
            vertices = state.vertices;
            current_round = state.current_round;
            last_checkpoint_round = state.last_checkpoint_round;
        }

        let mysticeti = MysticetiBackend::new(authorities.len())?;

        Ok(Self {
            authority_id,
            authorities,
            vertices,
            checkpoints,
            current_round,
            last_checkpoint_round,
            metrics: DagMetrics::default(),
            mysticeti,
        })
    }

    pub fn production_policy(&self) -> DagProductionPolicy {
        let parent_round = self.current_round;
        let target_round = self.current_round.saturating_add(1);
        let parent_vertices: Vec<&DagVertex> = self
            .vertices
            .iter()
            .filter(|vertex| vertex.round == parent_round)
            .collect();
        let parent_ids = parent_vertices.iter().map(|vertex| vertex.id).collect();
        let parent_authors: Vec<String> = parent_vertices
            .iter()
            .map(|vertex| vertex.author.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let missing_parent_authors = self
            .authorities
            .iter()
            .filter(|authority| !parent_authors.contains(authority))
            .cloned()
            .collect();
        let local_has_vertex_in_current_round = parent_vertices
            .iter()
            .any(|vertex| vertex.author == self.authority_id);

        DagProductionPolicy {
            current_round: self.current_round,
            parent_round,
            target_round,
            parent_ids,
            parent_authors: parent_authors.clone(),
            missing_parent_authors,
            parent_author_count: parent_authors.len(),
            quorum_size: self.mysticeti.quorum_threshold(),
            local_has_vertex_in_current_round,
            using_catch_up_round: false,
        }
    }

    pub fn metrics(&self) -> &DagMetrics {
        &self.metrics
    }

    pub fn protocol(&self) -> ConsensusRuntimeProtocol {
        self.mysticeti.runtime_protocol()
    }

    fn save_state(&self) -> PersistentDagState {
        PersistentDagState {
            vertices: self.vertices.clone(),
            checkpoints: self.checkpoints.clone(),
            current_round: self.current_round,
            last_checkpoint_round: self.last_checkpoint_round,
        }
    }

    fn latest_vertices_by_authority(&self, authority: &str, limit: usize) -> Vec<DagVertex> {
        self.vertices
            .iter()
            .rev()
            .filter(|vertex| vertex.author == authority)
            .take(limit)
            .cloned()
            .collect()
    }

    fn known_vertex(&self, id: &VertexId) -> bool {
        self.vertices.iter().any(|vertex| &vertex.id == id)
    }

    fn add_vertex(&mut self, vertex: DagVertex) -> Result<bool> {
        if vertex.transactions.len() != vertex.metadata.tx_count {
            anyhow::bail!("Transaction count mismatch");
        }
        if self.known_vertex(&vertex.id) {
            return Ok(false);
        }
        self.current_round = self.current_round.max(vertex.round);
        self.metrics.vertices_created = self.metrics.vertices_created.saturating_add(1);
        self.vertices.push(vertex);
        Ok(true)
    }

    fn record_checkpoint(&mut self, checkpoint: Checkpoint) {
        self.last_checkpoint_round = self.current_round;
        self.metrics.checkpoints_created = self.metrics.checkpoints_created.saturating_add(1);
        self.checkpoints.push(checkpoint);
        self.mysticeti.try_advance();
    }
}

#[derive(Clone)]
pub struct DagEngine {
    engine: Arc<BlockchainEngine>,
    consensus: Arc<RwLock<CoreDagConsensus>>,
    authority_id: String,
    local_signing_key: ed25519_dalek::SigningKey,
}

impl DagEngine {
    pub fn new_secure(
        engine: Arc<BlockchainEngine>,
        authority_id: String,
        authorities: Vec<String>,
        local_signing_key: ed25519_dalek::SigningKey,
        authority_public_keys: BTreeMap<String, Vec<u8>>,
    ) -> Result<Self> {
        let local_public_key = local_signing_key.verifying_key().to_bytes().to_vec();
        let expected_public_key = authority_public_keys
            .get(&authority_id)
            .ok_or_else(|| anyhow::anyhow!("Missing consensus public key for {}", authority_id))?;
        if *expected_public_key != local_public_key {
            anyhow::bail!("Consensus signing key does not match local authority public key");
        }
        Self::from_parts(engine, authority_id, authorities, local_signing_key)
    }

    fn from_parts(
        engine: Arc<BlockchainEngine>,
        authority_id: String,
        authorities: Vec<String>,
        local_signing_key: ed25519_dalek::SigningKey,
    ) -> Result<Self> {
        let state = Self::aligned_dag_state(&engine);
        let consensus = CoreDagConsensus::new(authority_id.clone(), authorities, state)?;
        let dag_engine = Self {
            engine,
            consensus: Arc::new(RwLock::new(consensus)),
            authority_id,
            local_signing_key,
        };
        dag_engine.persist_consensus_state()?;
        Ok(dag_engine)
    }

    fn aligned_dag_state(engine: &BlockchainEngine) -> Option<PersistentDagState> {
        let blockchain_checkpoints = {
            let chain = engine.blockchain.read().unwrap_or_else(|e| e.into_inner());
            chain.dag_checkpoints.iter().cloned().collect::<Vec<_>>()
        };
        if blockchain_checkpoints.is_empty() {
            return engine.persisted_dag_state.clone();
        }

        let dag_seq = engine
            .persisted_dag_state
            .as_ref()
            .and_then(|state| state.checkpoints.last())
            .map(|checkpoint| checkpoint.sequence)
            .unwrap_or(0);
        let chain_seq = blockchain_checkpoints
            .last()
            .map(|checkpoint| checkpoint.sequence)
            .unwrap_or(0);
        if dag_seq != chain_seq {
            warn!(
                "Persisted DAG checkpoint sequence ({}) does not match blockchain ({}); using blockchain checkpoints.",
                dag_seq, chain_seq
            );
            return Some(PersistentDagState {
                vertices: engine
                    .persisted_dag_state
                    .as_ref()
                    .map(|state| state.vertices.clone())
                    .unwrap_or_default(),
                checkpoints: blockchain_checkpoints,
                current_round: 0,
                last_checkpoint_round: 0,
            });
        }

        engine.persisted_dag_state.clone()
    }

    fn persist_consensus_state(&self) -> Result<()> {
        let state = self
            .consensus
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .save_state();
        self.engine.persist_dag_state(state)
    }

    pub fn produce_vertex(&self) -> Result<CheckpointProductionInfo> {
        let policy = {
            let consensus = self.consensus.read().unwrap_or_else(|e| e.into_inner());
            consensus.production_policy()
        };
        let mut transactions = self
            .engine
            .pending_txs
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        transactions.sort_by(|a, b| {
            a.transaction
                .sender_address()
                .cmp(b.transaction.sender_address())
                .then_with(|| {
                    a.transaction
                        .sequence_number()
                        .cmp(&b.transaction.sequence_number())
                })
                .then_with(|| a.transaction_hash().cmp(b.transaction_hash()))
        });
        let tx_count = transactions.len();
        if tx_count == 0 {
            anyhow::bail!("No new transactions to checkpoint");
        }
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().saturating_mul(1000))
            .unwrap_or(0);

        let (state_root, executed, failed, verified_state, to_execute, validate_supply) = {
            let state_snapshot = self.engine.state_read().clone();
            let state_arc = Arc::new(RwLock::new(state_snapshot));
            self.engine
                .execute_system_prologue_to_state_for_dag_v2(&state_arc, timestamp)?;
            let mut validate_supply = true;
            let (executed, failed) = match self
                .engine
                .apply_zero_effect_native_batch(&transactions, &state_arc)?
            {
                Some(result) => {
                    validate_supply = false;
                    result
                }
                None => self.engine.execute_tx_waves_deterministic_parallel(
                    transactions.clone(),
                    &state_arc,
                    Some(timestamp),
                    false,
                )?,
            };
            let verified_state = state_arc.read().unwrap_or_else(|e| e.into_inner()).clone();
            let state_root = verified_state.compute_state_root();
            (
                state_root,
                executed,
                failed,
                verified_state,
                if validate_supply {
                    transactions.clone()
                } else {
                    Vec::new()
                },
                validate_supply,
            )
        };

        let mysticeti_block = {
            let mut consensus = self.consensus.write().unwrap_or_else(|e| e.into_inner());
            consensus
                .mysticeti
                .propose_block(&transactions, timestamp)?
        };
        let (vertex_id, round, parents) = mysticeti_block
            .map(|block| (block.vertex_id, block.round, block.parents))
            .unwrap_or((
                policy.parent_ids.first().copied().unwrap_or([0u8; 32]),
                policy.target_round,
                policy.parent_ids,
            ));

        let mut vertex = DagVertex::new(
            round,
            self.authority_id.clone(),
            "kanari-v2-mysticeti".to_string(),
            parents,
            transactions,
            state_root,
            timestamp,
        );
        vertex.id = vertex_id;
        vertex.cached_hash = Some(vertex_id.to_vec());
        use ed25519_dalek::Signer;
        vertex.signature = self.local_signing_key.sign(&vertex.id).to_bytes().to_vec();

        let checkpoint =
            self.finalize_vertex(&vertex, verified_state, to_execute, validate_supply)?;
        let checkpoint_info = Some(CheckpointInfo {
            sequence: checkpoint.sequence,
            vertex_count: checkpoint.vertices.len(),
            tx_count: checkpoint.transactions.len(),
        });

        let vertex_id = hex::encode(vertex.id);
        info!(
            "[DAG v2] Produced Mysticeti-backed vertex {} round {} txs {}",
            vertex_id, vertex.round, tx_count
        );

        Ok(CheckpointProductionInfo {
            vertex_id,
            round: vertex.round,
            tx_count,
            executed,
            failed,
            events: Vec::new(),
            checkpoint: checkpoint_info,
            vertex: Some(vertex),
        })
    }

    fn finalize_vertex(
        &self,
        vertex: &DagVertex,
        verified_state: StateManager,
        to_execute: Vec<SignedTransaction>,
        validate_supply: bool,
    ) -> Result<Checkpoint> {
        {
            let mut consensus = self.consensus.write().unwrap_or_else(|e| e.into_inner());
            consensus.add_vertex(vertex.clone())?;
        }

        let prev_hash = {
            let chain = self
                .engine
                .blockchain
                .read()
                .unwrap_or_else(|e| e.into_inner());
            chain.latest_checkpoint().hash()?
        };
        let checkpoint = Checkpoint::new(
            self.engine.get_stats().height.saturating_add(1),
            vec![vertex.id],
            vertex.transactions.clone(),
            vertex.metadata.state_root.clone(),
            vertex.timestamp,
            prev_hash,
        );
        self.engine.apply_prepared_checkpoint(
            checkpoint.clone(),
            verified_state,
            to_execute,
            validate_supply,
        )?;

        {
            let mut consensus = self.consensus.write().unwrap_or_else(|e| e.into_inner());
            consensus.record_checkpoint(checkpoint.clone());
        }
        self.persist_consensus_state()?;
        Ok(checkpoint)
    }

    pub fn consensus(&self) -> Arc<RwLock<CoreDagConsensus>> {
        self.consensus.clone()
    }

    pub fn latest_own_vertices(&self, limit: usize) -> Vec<DagVertex> {
        let consensus = self.consensus.read().unwrap_or_else(|e| e.into_inner());
        consensus.latest_vertices_by_authority(&self.authority_id, limit)
    }

    pub fn add_network_vertex(&self, vertex: DagVertex) -> Result<()> {
        let mut consensus = self.consensus.write().unwrap_or_else(|e| e.into_inner());
        if consensus.known_vertex(&vertex.id) {
            return Ok(());
        }
        info!(
            "[DAG v2 SYNC] Accepted network vertex {} round {} txs {}",
            hex::encode(vertex.id),
            vertex.round,
            vertex.transactions.len()
        );
        consensus.add_vertex(vertex)?;
        drop(consensus);
        self.persist_consensus_state()
    }
}

impl BlockchainEngine {
    fn execute_system_prologue_to_state_for_dag_v2(
        &self,
        state_arc: &Arc<RwLock<StateManager>>,
        timestamp_ms: u64,
    ) -> Result<()> {
        let runtime = &self.runtime_pool[0];
        let mut state_write = state_arc.write().unwrap_or_else(|e| e.into_inner());
        let clock_id = runtime.ensure_system_clock(&mut state_write)?;
        let changeset = runtime.execute_clock_consensus_commit_prologue(clock_id, timestamp_ms)?;
        state_write.apply_changeset(&changeset)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_engine_defaults_to_mysticeti_protocol() {
        let engine = Arc::new(BlockchainEngine::new_in_memory().unwrap());
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[11u8; 32]);
        let mut public_keys = BTreeMap::new();
        public_keys.insert(
            "auth1".to_string(),
            signing_key.verifying_key().to_bytes().to_vec(),
        );
        let dag_engine = DagEngine::new_secure(
            engine,
            "auth1".to_string(),
            vec![
                "auth1".to_string(),
                "auth2".to_string(),
                "auth3".to_string(),
                "auth4".to_string(),
            ],
            signing_key,
            public_keys,
        )
        .unwrap();
        let protocol = dag_engine
            .consensus
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .protocol();

        assert_eq!(protocol.protocol, "mysticeti");
        assert_eq!(protocol.wave_length, 3);
        assert_eq!(protocol.direct_commit_quorum, 3);
        assert!(protocol.pipeline);
        assert!(protocol.leader_wait);
    }

    #[test]
    fn test_dag_engine_secure_constructor_rejects_mismatched_local_key() {
        let engine = Arc::new(BlockchainEngine::new_in_memory().unwrap());
        let expected = ed25519_dalek::SigningKey::from_bytes(&[11u8; 32]);
        let wrong = ed25519_dalek::SigningKey::from_bytes(&[33u8; 32]);
        let mut public_keys = BTreeMap::new();
        public_keys.insert(
            "auth1".to_string(),
            expected.verifying_key().to_bytes().to_vec(),
        );

        let result = DagEngine::new_secure(
            engine,
            "auth1".to_string(),
            vec!["auth1".to_string()],
            wrong,
            public_keys,
        );

        assert!(result.is_err());
    }
}
