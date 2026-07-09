// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! DAG vertex transport type and DAG engine backed by mysticeti.
//!
//! [`DagVertex`] is a kanari-specific transport type that carries kanari
//! transactions over the network. All core DAG logic uses mysticeti types
//! (`mysticeti_dag::Block`, `mysticeti_consensus::protocol::Protocol`, etc.)
//! directly — no custom DAG wrappers.

use anyhow::{Context, Result};
use log::info;
use mysticeti_consensus::{
    committer::Committer as MysticetiCommitter,
    protocol::{
        ConsensusProtocol as MysticetiConsensusProtocol, Protocol as MysticetiRuntimeProtocol,
    },
};
use mysticeti_dag::{
    authority::Authority as MysticetiAuthority,
    block::{
        Block, BlockReference as MysticetiBlockReference, RoundNumber as MysticetiRound,
        transaction::Transaction as MysticetiTransaction,
    },
    committee::Committee as MysticetiCommittee,
    context::TokioCtx as MysticetiTokioCtx,
    core::{Core as MysticetiCore, block_handler::RealBlockHandler as MysticetiBlockHandler},
    crypto::{BlockDigest, CryptoEngine as MysticetiCryptoEngine},
    data::Data as MysticetiBlockData,
    metrics::Metrics as MysticetiMetrics,
    storage::Storage as MysticetiStorage,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

use crate::consensus::Checkpoint;
use kanari_move_runtime_v1::state::StateManager;
use kanari_types::event::Event;
use kanari_types::transaction::SignedTransaction;

use super::*;

// ---------------------------------------------------------------------------
// Kanari-specific DAG transport type
// ---------------------------------------------------------------------------

/// A DAG vertex for network transport.
///
/// This is a kanari-specific type that carries kanari [`SignedTransaction`]s
/// so they can be serialised (JSON) and exchanged over gossip.  The core DAG
/// engine inside `DagEngine` uses `mysticeti_dag::Block` directly.
///
/// Each parent entry is `(author_hex_id, digest)` — the authority ID of the
/// parent vertex author and its 32-byte digest. This allows the receiving node
/// to reconstruct full mysticeti `BlockReference`s when feeding the vertex
/// into mysticeti's DAG storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagVertex {
    pub id: [u8; 32],
    pub round: u64,
    pub author: String,
    pub chain_id: String,
    pub parents: Vec<(String, u64, [u8; 32])>,
    pub transactions: Arc<[SignedTransaction]>,
    pub timestamp: u64,
    pub signature: Vec<u8>,
    #[serde(skip)]
    pub cached_serialized_data: Option<Vec<u8>>,
}

impl DagVertex {
    pub fn new<T>(
        round: u64,
        author: String,
        chain_id: String,
        parents: Vec<(String, u64, [u8; 32])>,
        transactions: T,
        timestamp: u64,
    ) -> Self
    where
        T: Into<Arc<[SignedTransaction]>>,
    {
        Self {
            id: [0u8; 32],
            round,
            author,
            chain_id,
            parents,
            transactions: transactions.into(),
            timestamp,
            signature: Vec::new(),
            cached_serialized_data: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Checkpoint production info (public API)
// ---------------------------------------------------------------------------

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

struct StagedCheckpoint {
    checkpoint: Checkpoint,
    verified_state: StateManager,
    to_execute: Vec<SignedTransaction>,
    validate_supply: bool,
}

// ---------------------------------------------------------------------------
// DAG production policy (public API)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagProductionPolicy {
    pub current_round: u64,
    pub parent_round: u64,
    pub target_round: u64,
    pub parent_ids: Vec<[u8; 32]>,
    pub parent_authors: Vec<String>,
    pub missing_parent_authors: Vec<String>,
    pub parent_author_count: usize,
    pub quorum_size: usize,
    pub local_has_vertex_in_current_round: bool,
    pub using_catch_up_round: bool,
}

impl DagProductionPolicy {
    pub fn should_wait_for_current_round_quorum(&self) -> bool {
        self.current_round > 0
            && self.parent_round == self.current_round
            && self.local_has_vertex_in_current_round
            && self.parent_author_count < self.quorum_size
    }
}

// ---------------------------------------------------------------------------
// Mysticeti backend — thin wrapper around mysticeti's `Core`
// ---------------------------------------------------------------------------

struct MysticetiBackend {
    core: MysticetiCore<MysticetiTokioCtx, MysticetiCommitter>,
    transaction_sender: mpsc::Sender<Vec<MysticetiTransaction>>,
    protocol: MysticetiRuntimeProtocol,
    /// Canonical ordered list of kanari authority IDs (hex).
    authorities: Vec<String>,
}

impl MysticetiBackend {
    fn new(authority_count: usize, authorities: Vec<String>) -> Result<Self> {
        let authority_count = authority_count.max(1);
        let committee = MysticetiCommittee::new_test(vec![1; authority_count]);
        let protocol_config = MysticetiConsensusProtocol::Mysticeti {
            leader_count: NonZeroUsize::new(authority_count.clamp(1, 2))
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
        // Re-derive protocol config for the committer to avoid consuming `protocol`
        // (Protocol does not implement Clone).
        let committer_protocol = protocol_config.to_protocol(&committee).map_err(|e| {
            anyhow::anyhow!("Failed to build Mysticeti protocol for committer: {}", e)
        })?;
        let committer = MysticetiCommitter::new(
            committee.clone(),
            storage.block_reader().clone(),
            committer_protocol,
        );
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
            core,
            transaction_sender,
            protocol,
            authorities,
        })
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
        let parents: Vec<(String, u64, [u8; 32])> = block
            .includes()
            .iter()
            .map(|block_ref| {
                let digest = mysticeti_reference_to_vertex_id(block_ref);
                let author_id = self
                    .authorities
                    .get(block_ref.authority.index())
                    .cloned()
                    .unwrap_or_else(|| format!("auth{}", block_ref.authority.index() + 1));
                (author_id, block_ref.round, digest)
            })
            .collect();
        Ok(Some(MysticetiBlockSummary {
            vertex_id: mysticeti_reference_to_vertex_id(&reference),
            round: block.round(),
            parents,
        }))
    }

    /// Add a block received from the network into mysticeti's storage.
    ///
    /// Constructs a mysticeti `Block` from the kanari `DagVertex` and feeds it
    /// into `Core::add_blocks()`. This is critical for the threshold clock to
    /// advance — without blocks from other authorities the clock stays stuck
    /// and the node cannot produce rounds beyond its own first vertex.
    fn add_network_block(&mut self, vertex: &DagVertex) -> Result<()> {
        // Map vertex author to mysticeti authority index
        let author_idx = self
            .authorities
            .iter()
            .position(|a| a == &vertex.author)
            .ok_or_else(|| anyhow::anyhow!("Unknown vertex author: {}", vertex.author))?;
        let author = MysticetiAuthority::new(author_idx as u64);

        // Build includes (parent BlockReferences) from the vertex's parent list.
        // Each parent entry is (author_hex_id, round, digest).
        let mut includes = Vec::with_capacity(vertex.parents.len());
        for (parent_author, parent_round, _parent_digest) in &vertex.parents {
            let parent_author_idx = self
                .authorities
                .iter()
                .position(|a| a == parent_author)
                .ok_or_else(|| anyhow::anyhow!("Unknown parent authority: {}", parent_author))?;
            let parent_authority = MysticetiAuthority::new(parent_author_idx as u64);
            includes.push(MysticetiBlockReference {
                authority: parent_authority,
                round: *parent_round,
                digest: BlockDigest::synthetic(*parent_round, parent_authority),
            });
        }

        // Create a disabled crypto engine so the block digest matches the
        // deterministic synthetic digest used by the originator.
        let crypto = MysticetiCryptoEngine::disabled();
        let timestamp_ns = vertex.timestamp * 1_000_000;

        // Build the mysticeti block with empty transaction batch.
        // Kanari transaction payloads are carried in the DagVertex cache
        // (state.vertices), not in the mysticeti block itself.
        let block = Block::new(
            author,
            vertex.round,
            includes,
            vec![],
            timestamp_ns,
            &crypto,
        );
        let block = MysticetiBlockData::new(block);

        // Inject into mysticeti's core — this updates the threshold clock
        // and makes the block visible to the block_reader for parent validation.
        self.core.add_blocks(vec![block]);
        Ok(())
    }
}

struct MysticetiBlockSummary {
    vertex_id: [u8; 32],
    round: MysticetiRound,
    parents: Vec<(String, u64, [u8; 32])>,
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

pub fn mysticeti_reference_to_vertex_id(reference: &MysticetiBlockReference) -> [u8; 32] {
    let mut id = [0u8; 32];
    id.copy_from_slice(reference.digest.as_ref());
    id
}

// ---------------------------------------------------------------------------
// DagEngine — public API for DAG operations
// ---------------------------------------------------------------------------

/// Mutable state behind `DagEngine`'s read-write lock.
struct DagEngineState {
    mysticeti: MysticetiBackend,
    /// Kanari transaction payloads for DAG blocks.
    /// Mysticeti stores block metadata (via WAL), but does not store
    /// kanari-specific `SignedTransaction`s. This cache bridges that gap
    /// so network gossip can include full transaction data.
    vertices: Vec<DagVertex>,
}

#[derive(Clone)]
pub struct DagEngine {
    engine: Arc<BlockchainEngine>,
    /// Consensus state (mysticeti backend + vertex cache) behind a
    /// read-write lock for thread-safe access.
    state: Arc<RwLock<DagEngineState>>,
    /// Local authority ID (hex string).
    authority_id: String,
    /// All authorities in the committee (hex strings).
    authorities: Vec<String>,
    /// Local signing key for producing DAG vertices.
    local_signing_key: ed25519_dalek::SigningKey,
    /// Public keys of all authorities, keyed by authority ID.
    authority_public_keys: BTreeMap<String, Vec<u8>>,
    /// Checkpoints staged for finalization after execution.
    staged_checkpoints: Arc<RwLock<BTreeMap<[u8; 32], StagedCheckpoint>>>,
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
        Self::from_parts(
            engine,
            authority_id,
            authorities,
            local_signing_key,
            authority_public_keys,
        )
    }

    fn from_parts(
        engine: Arc<BlockchainEngine>,
        authority_id: String,
        authorities: Vec<String>,
        local_signing_key: ed25519_dalek::SigningKey,
        authority_public_keys: BTreeMap<String, Vec<u8>>,
    ) -> Result<Self> {
        let mysticeti = MysticetiBackend::new(authorities.len(), authorities.clone())?;
        let dag_engine = Self {
            engine,
            state: Arc::new(RwLock::new(DagEngineState {
                mysticeti,
                vertices: Vec::new(),
            })),
            authority_id,
            authorities,
            local_signing_key,
            authority_public_keys,
            staged_checkpoints: Arc::new(RwLock::new(BTreeMap::new())),
        };
        Ok(dag_engine)
    }

    /// Query the DAG production policy from mysticeti's authoritative storage.
    pub fn production_policy(&self) -> DagProductionPolicy {
        let state = lock_read(&self.state);
        let block_reader = state.mysticeti.core.block_reader();
        let current_round = block_reader.highest_round();
        let parent_round = current_round;
        let target_round = current_round.saturating_add(1);

        let mysticeti_blocks = block_reader.get_blocks_by_round(parent_round);

        let parent_ids: Vec<[u8; 32]> = mysticeti_blocks
            .iter()
            .map(|block| {
                let mut id = [0u8; 32];
                id.copy_from_slice(block.reference().digest.as_ref());
                id
            })
            .collect();

        let parent_authors: Vec<String> = mysticeti_blocks
            .iter()
            .map(|block| self.authority_id(block.author()))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        let missing_parent_authors = self
            .authorities
            .iter()
            .filter(|authority| !parent_authors.contains(authority))
            .cloned()
            .collect();

        let local_has_vertex_in_current_round = mysticeti_blocks
            .iter()
            .any(|block| self.authority_id(block.author()) == self.authority_id);

        DagProductionPolicy {
            current_round,
            parent_round,
            target_round,
            parent_ids,
            parent_authors: parent_authors.clone(),
            missing_parent_authors,
            parent_author_count: parent_authors.len(),
            quorum_size: state.mysticeti.protocol.direct_commit_quorum as usize,
            local_has_vertex_in_current_round,
            using_catch_up_round: false,
        }
    }

    /// Map a mysticeti `Authority` to the kanari authority ID string.
    fn authority_id(&self, authority: MysticetiAuthority) -> String {
        let idx = authority.index();
        self.authorities
            .get(idx)
            .cloned()
            .unwrap_or_else(|| format!("auth{}", idx + 1))
    }

    /// Export metrics as a Prometheus string via mysticeti.
    pub fn metrics_prometheus(&self) -> String {
        let state = lock_read(&self.state);
        let block_reader = state.mysticeti.core.block_reader();
        let round = block_reader.highest_round();
        let chain = lock_read(&self.engine.blockchain);
        let checkpoint_count = chain.dag_checkpoints.len().saturating_sub(1);
        format!(
            "# HELP dag_vertices_created_total Total DAG vertices created\n\
             # TYPE dag_vertices_created_total counter\n\
             dag_vertices_created_total {}\n\
             # HELP dag_checkpoints_created_total Total DAG checkpoints created\n\
             # TYPE dag_checkpoints_created_total counter\n\
             dag_checkpoints_created_total {}\n",
            round, checkpoint_count,
        )
    }

    pub fn produce_vertex(&self) -> Result<CheckpointProductionInfo> {
        let policy = self.production_policy();
        let mut transactions = self.engine.pending_transactions_snapshot();
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

        let timestamp = {
            let chain = lock_read(&self.engine.blockchain);
            chain
                .latest_checkpoint()
                .timestamp
                .saturating_add(1)
                .max(chain.height().saturating_add(1))
        };

        let (_state_root, executed, failed, verified_state, to_execute, validate_supply) = {
            let mut state_snapshot = self.engine.state_read().clone();
            state_snapshot
                .repair_legacy_native_wallet_overcount()
                .context(
                    "Failed to repair legacy native wallet overcount before DAG state execution",
                )?;
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
            {
                let mut state_write = state_arc.write().unwrap_or_else(|e| e.into_inner());
                state_write
                    .repair_legacy_native_wallet_overcount()
                    .context(
                        "Failed to repair legacy native wallet overcount after DAG state execution",
                    )?;
            }
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

        let (vertex_id, round, parent_entries) = {
            let mut state = lock_write(&self.state);
            let mysticeti_block = state.mysticeti.propose_block(&transactions, timestamp)?;
            match mysticeti_block {
                Some(block) => {
                    // MysticetiBlockSummary.parents is already Vec<(String, u64, [u8; 32])>
                    (block.vertex_id, block.round, block.parents)
                }
                None => {
                    // When mysticeti doesn't produce a block (e.g. threshold clock
                    // hasn't advanced), use the policy parents with authority lookup.
                    let parents: Vec<(String, u64, [u8; 32])> = policy
                        .parent_ids
                        .iter()
                        .map(|digest| {
                            let author = state
                                .vertices
                                .iter()
                                .find(|v| v.id == *digest)
                                .map(|v| v.author.clone())
                                .unwrap_or_else(|| {
                                    log::warn!(
                                        "Parent vertex {} not found in cache",
                                        hex::encode(digest)
                                    );
                                    "unknown".to_string()
                                });
                            let v_round = state
                                .vertices
                                .iter()
                                .find(|v| v.id == *digest)
                                .map(|v| v.round)
                                .unwrap_or(policy.parent_round);
                            (author, v_round, *digest)
                        })
                        .collect();
                    (
                        policy.parent_ids.first().copied().unwrap_or([0u8; 32]),
                        policy.target_round,
                        parents,
                    )
                }
            }
        };

        let mut vertex = DagVertex::new(
            round,
            self.authority_id.clone(),
            "kanari-v2-mysticeti".to_string(),
            parent_entries,
            transactions,
            timestamp,
        );
        vertex.id = vertex_id;
        use ed25519_dalek::Signer;
        vertex.signature = self.local_signing_key.sign(&vertex.id).to_bytes().to_vec();

        self.stage_locally_produced_vertex(&vertex, verified_state, to_execute, validate_supply)?;
        let checkpoint = self.finalize_staged_checkpoint(vertex.id)?;
        let checkpoint_info = Some(CheckpointInfo {
            sequence: checkpoint.sequence,
            vertex_count: checkpoint.vertices.len(),
            tx_count: checkpoint.transactions.len(),
        });

        let vertex_id_hex = hex::encode(vertex.id);
        info!(
            "[DAG v2] Produced Mysticeti-backed vertex {} round {} txs {}",
            vertex_id_hex, vertex.round, tx_count
        );

        Ok(CheckpointProductionInfo {
            vertex_id: vertex_id_hex,
            round: vertex.round,
            tx_count,
            executed,
            failed,
            events: Vec::new(),
            checkpoint: checkpoint_info,
            vertex: Some(vertex),
        })
    }

    fn stage_locally_produced_vertex(
        &self,
        vertex: &DagVertex,
        verified_state: StateManager,
        to_execute: Vec<SignedTransaction>,
        validate_supply: bool,
    ) -> Result<Checkpoint> {
        {
            let mut state = lock_write(&self.state);
            state.vertices.push(vertex.clone());
        }

        let prev_hash = {
            let chain = lock_read(&self.engine.blockchain);
            chain.latest_checkpoint().hash()?
        };
        let checkpoint = Checkpoint::new(
            self.engine.get_stats().height.saturating_add(1),
            vec![vertex.id],
            vertex.transactions.clone(),
            // Use empty state root initially; it will be set after finalization.
            vec![],
            vertex.timestamp,
            prev_hash,
        );

        let mut staged = lock_write(&self.staged_checkpoints);
        staged.insert(
            vertex.id,
            StagedCheckpoint {
                checkpoint: checkpoint.clone(),
                verified_state,
                to_execute,
                validate_supply,
            },
        );
        Ok(checkpoint)
    }

    fn finalize_staged_checkpoint(&self, vertex_id: [u8; 32]) -> Result<Checkpoint> {
        let staged = lock_write(&self.staged_checkpoints)
            .remove(&vertex_id)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Missing staged checkpoint for vertex {}",
                    hex::encode(vertex_id)
                )
            })?;

        self.engine.apply_prepared_checkpoint(
            staged.checkpoint.clone(),
            staged.verified_state,
            staged.to_execute,
            staged.validate_supply,
        )?;

        // Notify mysticeti to try committing after the checkpoint is recorded.
        {
            let mut state = lock_write(&self.state);
            let committed = state.mysticeti.core.try_commit();
            if !committed.is_empty() {
                log::debug!("Mysticeti committed {} leader block(s)", committed.len());
            }
        }

        Ok(staged.checkpoint)
    }

    pub fn latest_own_vertices(&self, limit: usize) -> Vec<DagVertex> {
        let state = lock_read(&self.state);
        let block_reader = state.mysticeti.core.block_reader();
        let own_blocks = block_reader.get_own_blocks(0, limit);
        if own_blocks.is_empty() {
            return Vec::new();
        }
        // Match mysticeti blocks to cached vertices by digest.
        let own_digests: Vec<[u8; 32]> = own_blocks
            .iter()
            .map(|b| {
                let mut d = [0u8; 32];
                d.copy_from_slice(b.reference().digest.as_ref());
                d
            })
            .collect();
        state
            .vertices
            .iter()
            .filter(|v| own_digests.contains(&v.id))
            .take(limit)
            .cloned()
            .collect()
    }

    /// Validate a network `DagVertex` against both kanari rules and mysticeti storage.
    ///
    /// Parent existence is checked against mysticeti's authoritative `block_reader()`
    /// rather than the local kanari vertex cache, ensuring DAG integrity is
    /// verified against the actual consensus state.
    fn validate_network_vertex(&self, state: &DagEngineState, vertex: &DagVertex) -> Result<()> {
        const DAG_CHAIN_ID: &str = "kanari-v2-mysticeti";

        if vertex.chain_id != DAG_CHAIN_ID {
            anyhow::bail!("Invalid DAG vertex chain id: {}", vertex.chain_id);
        }
        if vertex.round == 0 {
            anyhow::bail!("Invalid DAG vertex round 0");
        }
        if !self.authorities.contains(&vertex.author) {
            anyhow::bail!("Unknown DAG vertex author: {}", vertex.author);
        }
        // Duplicate detection is handled by mysticeti storage; skip
        // kanari-level cache checks. The block_reader() is authoritative.

        let public_key_bytes = self
            .authority_public_keys
            .get(&vertex.author)
            .ok_or_else(|| anyhow::anyhow!("Missing consensus public key for {}", vertex.author))?;
        let public_key_bytes: [u8; 32] = public_key_bytes.as_slice().try_into().map_err(|_| {
            anyhow::anyhow!("Invalid consensus public key length for {}", vertex.author)
        })?;
        let verifying_key =
            ed25519_dalek::VerifyingKey::from_bytes(&public_key_bytes).map_err(|e| {
                anyhow::anyhow!("Invalid consensus public key for {}: {}", vertex.author, e)
            })?;
        let signature_bytes: [u8; 64] = vertex.signature.as_slice().try_into().map_err(|_| {
            anyhow::anyhow!("Invalid DAG vertex signature length for {}", vertex.author)
        })?;
        let signature = ed25519_dalek::Signature::from_bytes(&signature_bytes);
        use ed25519_dalek::Verifier;
        verifying_key.verify(&vertex.id, &signature).map_err(|e| {
            anyhow::anyhow!("Invalid DAG vertex signature for {}: {}", vertex.author, e)
        })?;

        // Validate transaction uniqueness within the vertex.
        let mut seen_tx_hashes = HashSet::new();
        for (index, tx) in vertex.transactions.iter().enumerate() {
            let tx_hash = tx.verified_transaction_hash().map_err(|e| {
                anyhow::anyhow!(
                    "Invalid transaction {} in DAG vertex {}: {}",
                    index + 1,
                    hex::encode(vertex.id),
                    e
                )
            })?;
            if !seen_tx_hashes.insert(tx_hash.to_vec()) {
                anyhow::bail!(
                    "Duplicate transaction inside DAG vertex {}",
                    hex::encode(vertex.id)
                );
            }
        }

        // Validate parent uniqueness within the vertex.
        let mut seen_parents = HashSet::new();
        for (_, _, parent_digest) in &vertex.parents {
            if !seen_parents.insert(*parent_digest) {
                anyhow::bail!("Duplicate parent in DAG vertex {}", hex::encode(vertex.id));
            }
        }

        if vertex.round == 1 {
            if !vertex.parents.is_empty() {
                anyhow::bail!("Round-1 DAG vertex must not have parents");
            }
            return Ok(());
        }

        if vertex.parents.is_empty() {
            anyhow::bail!("DAG vertex round {} is missing parents", vertex.round);
        }

        // Validate parent references — first check the kanari vertex cache,
        // and also check mysticeti's storage for already-injected blocks.
        let expected_parent_round = vertex.round - 1;
        let block_reader = state.mysticeti.core.block_reader();
        let mysticeti_parent_blocks = block_reader.get_blocks_by_round(expected_parent_round);

        let mut parent_authors = HashSet::new();
        for (parent_author, _parent_round, parent_id) in &vertex.parents {
            // 1. Check mysticeti's block_reader first (blocks that have been
            //    injected via add_network_block).
            let found_in_mysticeti = mysticeti_parent_blocks.iter().any(|block| {
                let mut digest = [0u8; 32];
                digest.copy_from_slice(block.reference().digest.as_ref());
                digest == *parent_id
            });

            if !found_in_mysticeti {
                // 2. Fall back to the kanari vertex cache (vertices received
                //    via gossip that haven't been injected into mysticeti yet).
                let found_in_cache = state.vertices.iter().any(|v| v.id == *parent_id);

                if !found_in_cache {
                    anyhow::bail!(
                        "Missing parent {} (author {}) for DAG vertex {}",
                        hex::encode(parent_id),
                        parent_author,
                        hex::encode(vertex.id)
                    );
                }
            }

            if !parent_authors.insert(parent_author.clone()) {
                anyhow::bail!(
                    "Duplicate parent author in DAG vertex {}",
                    hex::encode(vertex.id)
                );
            }
        }

        Ok(())
    }

    pub fn add_network_vertex(&self, vertex: DagVertex) -> Result<()> {
        let mut state = lock_write(&self.state);
        // Dedup against local cache (kanari transaction payloads).
        // Mysticeti handles block-level dedup in its own storage.
        if state.vertices.iter().any(|v| v.id == vertex.id) {
            return Ok(());
        }
        self.validate_network_vertex(&state, &vertex)?;

        // Feed the vertex into mysticeti's core so that its block_reader
        // can find it (for parent validation of downstream vertices) and the
        // threshold clock advances with blocks from other authorities.
        state
            .mysticeti
            .add_network_block(&vertex)
            .context("Failed to inject network vertex into Mysticeti")?;

        info!(
            "[DAG v2 SYNC] Accepted + injected network vertex {} round {} txs {}",
            hex::encode(vertex.id),
            vertex.round,
            vertex.transactions.len()
        );
        state.vertices.push(vertex);
        Ok(())
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
        // Apply without supply validation first, then repair legacy overcount
        // that may be exposed by the clock prologue changeset.
        state_write
            .apply_changeset_without_supply_validation(&changeset)
            .context("Failed to apply clock prologue changeset")?;
        state_write
            .repair_legacy_native_wallet_overcount()
            .context("Failed to repair native wallet overcount after clock prologue")?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Lock helpers with poison recovery
// ---------------------------------------------------------------------------

fn lock_read<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|e| {
        log::error!("RwLock poisoned, recovering");
        e.into_inner()
    })
}

fn lock_write<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|e| {
        log::error!("RwLock poisoned, recovering");
        e.into_inner()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanari_crypto::keys::{CurveType, generate_keypair};
    use kanari_types::transaction::{SignedTransaction, Transaction};

    fn authority_key(seed: u8) -> ed25519_dalek::SigningKey {
        ed25519_dalek::SigningKey::from_bytes(&[seed; 32])
    }

    fn signed_transfer(sequence_number: u64) -> SignedTransaction {
        let sender = generate_keypair(CurveType::Ed25519).unwrap();
        let recipient = generate_keypair(CurveType::Ed25519).unwrap();
        let tx = Transaction::new_transfer(
            sender.tagged_address(),
            "0xaaaa".to_string(),
            recipient.address,
            1,
            sequence_number,
        );
        let mut signed_tx = SignedTransaction::new(tx);
        signed_tx
            .sign(&sender.private_key, sender.curve_type)
            .unwrap();
        signed_tx
    }

    fn signed_network_vertex(
        author: &str,
        signing_key: &ed25519_dalek::SigningKey,
        round: u64,
        parents: Vec<(String, u64, [u8; 32])>,
    ) -> DagVertex {
        let tx = signed_transfer(0);
        let mut vertex = DagVertex::new(
            round,
            author.to_string(),
            "kanari-v2-mysticeti".to_string(),
            parents,
            vec![tx],
            123,
        );
        use ed25519_dalek::Signer;
        vertex.signature = signing_key.sign(&vertex.id).to_bytes().to_vec();
        vertex
    }

    #[test]
    fn test_dag_engine_defaults_to_mysticeti_protocol() {
        let engine = Arc::new(BlockchainEngine::new_in_memory().unwrap());
        let signing_key = authority_key(11);
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

        let state = lock_read(&dag_engine.state);
        assert_eq!(state.mysticeti.protocol.wave_length, 3);
        assert_eq!(state.mysticeti.protocol.direct_commit_quorum, 3);
        assert!(state.mysticeti.protocol.pipeline);
        assert!(state.mysticeti.protocol.leader_wait);
    }

    #[test]
    fn test_dag_engine_secure_constructor_rejects_mismatched_local_key() {
        let engine = Arc::new(BlockchainEngine::new_in_memory().unwrap());
        let expected = authority_key(11);
        let wrong = authority_key(33);
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

    fn build_test_dag_engine(
        authorities: Vec<String>,
        local_authority: &str,
    ) -> (Arc<BlockchainEngine>, DagEngine, ed25519_dalek::SigningKey) {
        let engine = Arc::new(BlockchainEngine::new_in_memory().unwrap());
        let local_key = authority_key(11);
        let remote_key = authority_key(22);
        let mut public_keys = BTreeMap::new();
        for auth in &authorities {
            let key = if auth == local_authority {
                &local_key
            } else {
                &remote_key
            };
            public_keys.insert(auth.clone(), key.verifying_key().to_bytes().to_vec());
        }
        let dag_engine = DagEngine::new_secure(
            engine.clone(),
            local_authority.to_string(),
            authorities,
            local_key,
            public_keys,
        )
        .unwrap();
        (engine, dag_engine, remote_key)
    }

    #[test]
    fn test_add_network_vertex_accepts_valid_remote_vertex() {
        let (_engine, dag_engine, remote_key) =
            build_test_dag_engine(vec!["auth1".to_string(), "auth2".to_string()], "auth1");

        let vertex = signed_network_vertex("auth2", &remote_key, 1, vec![]);
        dag_engine.add_network_vertex(vertex).unwrap();

        let state = lock_read(&dag_engine.state);
        assert_eq!(state.vertices.len(), 1);
        assert_eq!(state.vertices[0].author, "auth2");
    }

    #[test]
    fn test_add_network_vertex_rejects_invalid_signature() {
        let (_engine, dag_engine, _remote_key) =
            build_test_dag_engine(vec!["auth1".to_string(), "auth2".to_string()], "auth1");
        let wrong_key = authority_key(33);

        let vertex = signed_network_vertex("auth2", &wrong_key, 1, vec![]);
        let error = dag_engine.add_network_vertex(vertex).unwrap_err();
        assert!(error.to_string().contains("Invalid DAG vertex signature"));
    }

    #[test]
    fn test_add_network_vertex_rejects_missing_parent() {
        let (_engine, dag_engine, remote_key) =
            build_test_dag_engine(vec!["auth1".to_string(), "auth2".to_string()], "auth1");

        let mut vertex = signed_network_vertex("auth2", &remote_key, 2, vec![]);
        vertex.parents = vec![
            ("auth1".to_string(), 1u64, [1u8; 32]),
            ("auth2".to_string(), 1u64, [2u8; 32]),
        ];
        let error = dag_engine.add_network_vertex(vertex).unwrap_err();
        assert!(
            error.to_string().contains("Missing parent")
                || error.to_string().contains("missing parents")
        );
    }
}
