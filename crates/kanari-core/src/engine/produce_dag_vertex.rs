// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! DAG-based block production for Kanari blockchain engine
//! Integrates DAG consensus with parallel transaction execution

use anyhow::Result;
use centauri::calculate_quorum;
use centauri::consensus::{DagConsensus, VertexId};
use log::{error, info};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};

use super::*;

/// DAG Block Production Info
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DagBlockInfo {
    pub vertex_id: String,
    pub round: u64,
    pub tx_count: usize,
    pub executed: usize,
    pub failed: usize,
    pub events: Vec<Event>,
    pub checkpoint: Option<CheckpointInfo>,
    /// The actual DAG vertex for network broadcast
    pub vertex: Option<centauri::consensus::DagVertex>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckpointInfo {
    pub sequence: u64,
    pub vertex_count: usize,
    pub tx_count: usize,
}

type StateCache = Arc<RwLock<LruCache<Vec<u8>, Arc<RwLock<StateManager>>>>>;

/// DAG-enabled blockchain engine
#[derive(Clone)]
pub struct DagEngine {
    /// Reference to the base blockchain engine
    engine: Arc<BlockchainEngine>,

    /// DAG consensus instance
    consensus: Arc<RwLock<DagConsensus>>,

    /// This node's authority ID
    authority_id: String,

    /// Cache for execution results to avoid re-execution in apply_checkpoint
    /// Maps VertexId -> Post-execution State
    state_cache: StateCache,
}

impl DagEngine {
    fn persist_consensus_state(&self) -> Result<()> {
        let state = {
            let consensus = self.consensus.read().unwrap_or_else(|e| e.into_inner());
            consensus.save_state()?
        };
        self.engine.persist_dag_state(state)
    }

    fn apply_checkpoint_once(
        &self,
        mut checkpoint: centauri::consensus::Checkpoint,
        log_prefix: &str,
        allow_root_override: bool,
    ) -> Result<centauri::consensus::Checkpoint> {
        let current_height = self.engine.get_stats().height;
        if current_height >= checkpoint.sequence {
            if let Some(canonical) = self.canonical_checkpoint_if_current(checkpoint.sequence) {
                info!(
                    "{} Checkpoint {} already finalized at height {}, reusing canonical checkpoint",
                    log_prefix, checkpoint.sequence, current_height
                );
                return Ok(canonical);
            }

            info!(
                "{} Checkpoint {} already covered by blockchain height {}, skipping re-apply",
                log_prefix, checkpoint.sequence, current_height
            );
            return Ok(checkpoint);
        }

        let previous_checkpoint_root = {
            let chain = self
                .engine
                .blockchain
                .read()
                .unwrap_or_else(|e| e.into_inner());
            chain.latest_checkpoint().state_root.clone()
        };

        let (computed_root, verified_state, to_execute) =
            self.engine.prepare_checkpoint_state(&checkpoint)?;

        if checkpoint.transactions.is_empty() {
            checkpoint.state_root = previous_checkpoint_root;
            self.engine.apply_prepared_checkpoint(
                checkpoint.clone(),
                verified_state,
                to_execute,
            )?;
            return Ok(checkpoint);
        }

        if self.engine.checkpoint_root_matches(
            checkpoint.sequence,
            &computed_root,
            &checkpoint.state_root,
        )? {
            self.engine.apply_prepared_checkpoint(
                checkpoint.clone(),
                verified_state,
                to_execute,
            )?;
            return Ok(checkpoint);
        }

        if !allow_root_override {
            anyhow::bail!(
                "{} Checkpoint {} state root mismatch: expected={}, computed={}",
                log_prefix,
                checkpoint.sequence,
                hex::encode(&checkpoint.state_root),
                hex::encode(&computed_root)
            );
        }

        checkpoint.state_root = computed_root;
        self.engine
            .apply_prepared_checkpoint(checkpoint.clone(), verified_state, to_execute)?;
        Ok(checkpoint)
    }

    fn canonical_checkpoint_if_current(
        &self,
        sequence: u64,
    ) -> Option<centauri::consensus::Checkpoint> {
        let chain = self
            .engine
            .blockchain
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let latest = chain.latest_checkpoint();
        (latest.sequence == sequence).then(|| latest.clone())
    }

    fn has_committed_transaction(&self, tx_hash: &[u8]) -> bool {
        if self
            .engine
            .blockchain
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .is_transaction_executed(&hex::encode(tx_hash))
        {
            return true;
        }

        let consensus = self.consensus.read().unwrap_or_else(|e| e.into_inner());
        consensus.has_executed_transaction(tx_hash)
    }

    /// Create a new DAG engine with default configuration (optimized for 500K TPS)
    pub fn new(
        engine: Arc<BlockchainEngine>,
        authority_id: String,
        authorities: Vec<String>,
    ) -> Result<Self> {
        // Enable DAG mode on blockchain
        {
            let mut blockchain = engine.blockchain.write().unwrap_or_else(|e| e.into_inner());
            blockchain.enable_dag_mode();
        }

        let mut consensus = DagConsensus::new(authority_id.clone(), authorities);

        // Load persisted DAG state if it exists
        if let Some(dag_state) = &engine.persisted_dag_state {
            let (blockchain_checkpoint_seq, blockchain_checkpoint_hash) = {
                let chain = engine.blockchain.read().unwrap_or_else(|e| e.into_inner());
                (
                    chain.latest_checkpoint().sequence,
                    chain.latest_checkpoint().hash()?,
                )
            };
            let dag_checkpoint_seq = dag_state
                .checkpoints
                .last()
                .map(|checkpoint| checkpoint.sequence)
                .unwrap_or(0);
            let dag_checkpoint_hash = dag_state
                .checkpoints
                .last()
                .map(|checkpoint| checkpoint.hash())
                .transpose()?
                .unwrap_or_else(|| {
                    centauri::consensus::Checkpoint::genesis()
                        .hash()
                        .expect("genesis checkpoint hash should be infallible")
                });

            if dag_checkpoint_seq != blockchain_checkpoint_seq {
                error!(
                    "Persisted DAG state checkpoint sequence ({}) does not match blockchain ({}) - ignoring stale DAG state.",
                    dag_checkpoint_seq, blockchain_checkpoint_seq
                );
            } else if dag_checkpoint_hash != blockchain_checkpoint_hash {
                error!(
                    "Persisted DAG state checkpoint hash ({}) does not match blockchain ({}), ignoring stale DAG state.",
                    hex::encode(dag_checkpoint_hash),
                    hex::encode(blockchain_checkpoint_hash)
                );
            } else if let Err(e) = consensus.load_state(dag_state.clone()) {
                error!(
                    "Failed to load persisted DAG state: {}. Creating fresh state.",
                    e
                );
            } else {
                info!("Successfully loaded persisted DAG state.");
            }
        }

        Ok(Self {
            engine,
            consensus: Arc::new(RwLock::new(consensus)),
            authority_id,
            state_cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(10).expect("Failed to create NonZeroUsize for state cache."),
            ))),
        })
    }

    // =====================================================================
    // 💡 HELPER: Collect unique, unexecuted Transactions (History + Current)
    // =====================================================================
    fn collect_unexecuted_txs<'a>(
        &self,
        history_vertices: &[VertexId],
        current_txs: impl Iterator<Item = &'a kanari_types::transaction::SignedTransaction>,
        chain: &centauri::blockchain::Blockchain,
        include_history: bool,
    ) -> Result<Vec<kanari_types::transaction::SignedTransaction>> {
        let mut seen_tx_hashes = std::collections::HashSet::new();
        let mut all_to_execute = Vec::new();
        let consensus = self.consensus.read().unwrap();

        // 1. Fetch from old History (Parent generation Vertices)
        if include_history {
            for v_id in history_vertices {
                if let Some(v) = consensus.store().get_vertex(v_id) {
                    for signed_tx in &v.transactions {
                        let tx_hash = signed_tx.hash();
                        if seen_tx_hashes.insert(tx_hash.clone())
                            && !chain.is_transaction_executed(&hex::encode(&tx_hash))
                        {
                            all_to_execute.push(signed_tx.clone());
                        }
                    }
                }
            }
        }

        // 2. Fetch from Current (Latest Vertex)
        for signed_tx in current_txs {
            let tx_hash = signed_tx.hash();
            if seen_tx_hashes.insert(tx_hash.clone())
                && !chain.is_transaction_executed(&hex::encode(&tx_hash))
            {
                all_to_execute.push(signed_tx.clone());
            }
        }

        Ok(all_to_execute)
    }

    /// Produce a DAG vertex with pending transactions
    pub fn produce_vertex(&self) -> Result<DagBlockInfo> {
        let (
            history_vertices,
            history_tx_hashes,
            current_round,
            parent_round,
            target_round,
            parent_author_count,
            quorum_size,
            using_catch_up_round,
        ) = {
            let consensus = self.consensus.read().unwrap_or_else(|e| e.into_inner());
            let current_round = consensus.store().current_round();
            let current_round_vertices = consensus.store().get_vertices_in_round(current_round);
            let has_local_vertex_in_current_round = current_round_vertices
                .iter()
                .any(|vertex| vertex.author == self.authority_id);
            let current_round_parent_author_count = current_round_vertices
                .iter()
                .map(|v| v.author.clone())
                .collect::<std::collections::HashSet<_>>()
                .len();
            let quorum_size = calculate_quorum(consensus.committee().validators.len());

            let (
                parent_round,
                target_round,
                parent_vertices,
                parent_author_count,
                using_catch_up_round,
            ) = if current_round > 0
                && !has_local_vertex_in_current_round
                && current_round_parent_author_count < quorum_size
            {
                let catch_up_parent_round = current_round.saturating_sub(1);
                let catch_up_parent_vertices = consensus
                    .store()
                    .get_vertices_in_round(catch_up_parent_round);
                let catch_up_parent_author_count = catch_up_parent_vertices
                    .iter()
                    .map(|v| v.author.clone())
                    .collect::<std::collections::HashSet<_>>()
                    .len();

                (
                    catch_up_parent_round,
                    current_round,
                    catch_up_parent_vertices,
                    catch_up_parent_author_count,
                    true,
                )
            } else {
                (
                    current_round,
                    current_round + 1,
                    current_round_vertices,
                    current_round_parent_author_count,
                    false,
                )
            };

            let parents: Vec<VertexId> = parent_vertices.iter().map(|v| v.id).collect();

            let history_vertices = consensus.collect_history_for_parents(&parents)?;
            let mut history_tx_hashes = std::collections::BTreeSet::new();
            for v_id in &history_vertices {
                if let Some(v) = consensus.store().get_vertex(v_id) {
                    for tx in &v.transactions {
                        history_tx_hashes.insert(tx.hash());
                    }
                }
            }
            (
                history_vertices,
                history_tx_hashes,
                current_round,
                parent_round,
                target_round,
                parent_author_count,
                quorum_size,
                using_catch_up_round,
            )
        };

        let (transactions, tx_to_remove_from_pending) = {
            let _state = match self.engine.state.read() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    error!("State lock poisoned during transaction collection, recovering...");
                    poisoned.into_inner()
                }
            };
            let chain = match self.engine.blockchain.read() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    error!("Blockchain lock poisoned during transaction collection, recovering...");
                    poisoned.into_inner()
                }
            };
            let pending = match self.engine.pending_txs.read() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    error!(
                        "Pending txs lock poisoned during transaction collection, recovering..."
                    );
                    poisoned.into_inner()
                }
            };

            let mut to_include = Vec::new();
            let mut to_remove = Vec::new();

            for tx in pending.iter().take(500_000) {
                let hash = tx.hash();

                if history_tx_hashes.contains(&hash) {
                    continue;
                } else if chain.is_transaction_executed(&hex::encode(&hash))
                    || self.has_committed_transaction(&hash)
                {
                    to_remove.push(hash);
                } else {
                    to_include.push(tx.clone());
                }
            }
            (to_include, to_remove)
        };

        if !tx_to_remove_from_pending.is_empty() {
            let mut pending = match self.engine.pending_txs.write() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    error!("Pending txs lock poisoned during removal, recovering...");
                    poisoned.into_inner()
                }
            };
            let remove_set: std::collections::HashSet<_> =
                tx_to_remove_from_pending.into_iter().collect();
            pending.retain(|tx| !remove_set.contains(&tx.hash()));
        }

        if transactions.is_empty() && history_vertices.is_empty() {
            anyhow::bail!("No new transactions and no history to commit");
        }

        if transactions.is_empty() && current_round > 0 && parent_author_count < quorum_size {
            anyhow::bail!(
                "DAG_WAITING: not producing empty vertex for round {} with partial parents ({}/{})",
                target_round,
                parent_author_count,
                quorum_size
            );
        }

        let tx_count = transactions.len();
        // FIX: Use as_secs() instead of as_millis() to match validation expectations (seconds, not milliseconds)
        let proposed_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs()) // ✅ CORRECT - seconds
            .unwrap_or(0);
        let timestamp = {
            let consensus = self.consensus.read().unwrap_or_else(|e| e.into_inner());
            consensus.suggest_vertex_timestamp(proposed_timestamp)
        };

        // Process transactions using the engine helper
        let (executed_state, state_root, executed, failed) = {
            let state_guard = match self.engine.state.read() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    error!("State lock poisoned during produce_vertex, recovering...");
                    poisoned.into_inner()
                }
            };
            let state_clone = state_guard.clone();
            let chain = match self.engine.blockchain.read() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    error!("Blockchain lock poisoned during produce_vertex, recovering...");
                    poisoned.into_inner()
                }
            };
            let state_arc = Arc::new(RwLock::new(state_clone));

            // Fetch all TXs using Helper
            let all_to_execute = self.collect_unexecuted_txs(
                &history_vertices,
                transactions.iter(),
                &chain,
                !transactions.is_empty(),
            )?;

            info!(
                "[DAG] Executing {} transactions in parallel waves",
                all_to_execute.len()
            );

            // Execute transactions in parallel waves
            let (executed_count, failed_count) = self
                .engine
                .execute_tx_waves_parallel(
                    all_to_execute,
                    &state_arc,
                    Some(timestamp),
                    false, // persist_objects = false (wait for Commit confirmation before saving)
                    false, // strict_mode = false (on failure, just increment failed_count and continue)
                )
                .unwrap_or((0, 0));

            let root = match state_arc.write() {
                Ok(guard) => guard.compute_state_root(),
                Err(poisoned) => {
                    error!("State arc lock poisoned during compute_state_root, recovering...");
                    poisoned.into_inner().compute_state_root()
                }
            };
            (state_arc, root, executed_count, failed_count)
        };

        let events: Vec<Event> = Vec::new();
        let vertex = {
            let mut consensus = self.consensus.write().unwrap_or_else(|e| e.into_inner());
            let parent_ids = consensus.store().get_vertex_ids_in_round(parent_round);
            let v = consensus.create_vertex_for_round(
                target_round,
                parent_ids,
                transactions.clone(),
                state_root.clone(),
                timestamp,
            )?;
            if using_catch_up_round {
                info!(
                    "[DAG] Created catch-up vertex for round {} using parents from round {}",
                    v.round, parent_round
                );
            }
            info!(
                "[DAG] Created vertex for round {} with {} transactions",
                v.round,
                transactions.len()
            );
            v
        };

        {
            let mut cache = self.state_cache.write().unwrap_or_else(|e| e.into_inner());
            cache.put(vertex.id.to_vec(), executed_state);
        }

        let vertex_id = hex::encode(vertex.id);
        let round = vertex.round;
        let vertex_for_broadcast = vertex.clone();

        {
            let mut consensus = self.consensus.write().unwrap_or_else(|e| e.into_inner());
            consensus.add_vertex(vertex)?;
        }
        self.persist_consensus_state()?;

        let checkpoint_info = {
            let checkpoint = {
                let mut consensus = self.consensus.write().unwrap_or_else(|e| e.into_inner());
                consensus.try_commit()?
            };

            if let Some(checkpoint) = checkpoint {
                let checkpoint = self.apply_checkpoint_once(checkpoint, "[DAG]", true)?;

                {
                    let mut consensus = self.consensus.write().unwrap_or_else(|e| e.into_inner());
                    consensus.add_checkpoint(checkpoint.clone())?;
                }
                self.persist_consensus_state()?;

                Some(CheckpointInfo {
                    sequence: checkpoint.sequence,
                    vertex_count: checkpoint.vertices.len(),
                    tx_count: checkpoint.transactions.len(),
                })
            } else {
                None
            }
        };

        Ok(DagBlockInfo {
            vertex_id,
            round,
            tx_count,
            executed,
            failed,
            events,
            checkpoint: checkpoint_info,
            vertex: Some(vertex_for_broadcast),
        })
    }

    pub fn consensus(&self) -> Arc<RwLock<DagConsensus>> {
        self.consensus.clone()
    }

    pub fn needs_progress(&self) -> bool {
        let consensus = self.consensus.read().unwrap_or_else(|e| e.into_inner());
        let store = consensus.store();
        let current_round = store.current_round();
        let last_checkpoint_round = store.last_checkpoint_round();
        let latest_local_round = store
            .get_vertices_by_authority(&self.authority_id)
            .into_iter()
            .map(|vertex| vertex.round)
            .max()
            .unwrap_or(0);

        current_round > last_checkpoint_round
            || (current_round > 0 && latest_local_round < current_round)
    }

    pub fn engine(&self) -> Arc<BlockchainEngine> {
        self.engine.clone()
    }

    pub fn authority_id(&self) -> &str {
        &self.authority_id
    }

    pub fn latest_own_vertex(&self) -> Option<centauri::consensus::DagVertex> {
        self.latest_own_vertices(1).into_iter().next()
    }

    pub fn latest_own_vertices(&self, limit: usize) -> Vec<centauri::consensus::DagVertex> {
        let consensus = self.consensus.read().unwrap_or_else(|e| e.into_inner());
        let mut vertices: Vec<_> = consensus
            .store()
            .get_vertices_by_authority(&self.authority_id)
            .into_iter()
            .map(|vertex| (*vertex).clone())
            .collect();

        vertices.sort_by_key(|vertex| vertex.round);
        let keep_from = vertices.len().saturating_sub(limit);
        vertices.split_off(keep_from)
    }

    pub fn sync_checkpoint(&self, checkpoint: centauri::consensus::Checkpoint) -> Result<()> {
        let mut consensus = self.consensus.write().unwrap_or_else(|e| e.into_inner());
        consensus.add_checkpoint(checkpoint)
    }

    pub fn add_network_vertex(&self, vertex: centauri::consensus::DagVertex) -> Result<()> {
        let vertex_id_hex = hex::encode(vertex.id);
        info!(
            "[DAG SYNC] Received vertex {} for round {} from network with {} transactions",
            vertex_id_hex,
            vertex.round,
            vertex.transactions.len()
        );

        {
            let consensus = match self.consensus.read() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    error!("Consensus lock poisoned during vertex check, recovering...");
                    poisoned.into_inner()
                }
            };
            let current_round = consensus.store().current_round();
            const MAX_FUTURE_EMPTY_VERTEX_ROUNDS: u64 = 20;
            if vertex.transactions.is_empty()
                && vertex.round > current_round.saturating_add(MAX_FUTURE_EMPTY_VERTEX_ROUNDS)
            {
                info!(
                    "[DAG SYNC] Ignoring far-future empty vertex {} at round {} (current round: {})",
                    vertex_id_hex, vertex.round, current_round
                );
                return Ok(());
            }
            if consensus.has_vertex(&vertex.id) {
                info!(
                    "[DAG SYNC] Vertex {} (round {}) already exists, skipping",
                    vertex_id_hex, vertex.round
                );
                return Ok(());
            }
        }

        let transactions = vertex.transactions.clone();

        if !transactions.is_empty() {
            {
                let mut pending = match self.engine.pending_txs.write() {
                    Ok(guard) => guard,
                    Err(poisoned) => {
                        error!("Pending txs lock poisoned during sync removal, recovering...");
                        poisoned.into_inner()
                    }
                };
                let tx_hashes: std::collections::HashSet<Vec<u8>> =
                    transactions.iter().map(|tx| tx.hash()).collect();
                pending.retain(|tx| !tx_hashes.contains(&tx.hash()));

                if !pending.is_empty() {
                    info!(
                        "[DAG SYNC] Removed {} transactions from pending pool (keeping {})",
                        transactions.len(),
                        pending.len()
                    );
                }
            }

            // Process network vertex transactions using the engine helper
            let (computed_state_root, executed, failed) = {
                let state_clone = match self.engine.state.read() {
                    Ok(guard) => guard.clone(),
                    Err(poisoned) => {
                        error!("State lock poisoned during sync clone, recovering...");
                        poisoned.into_inner().clone()
                    }
                };
                let chain = match self.engine.blockchain.read() {
                    Ok(guard) => guard,
                    Err(poisoned) => {
                        error!("Blockchain lock poisoned during sync, recovering...");
                        poisoned.into_inner()
                    }
                };
                let state_arc = Arc::new(RwLock::new(state_clone));

                let history_vertices = {
                    let consensus = match self.consensus.read() {
                        Ok(guard) => guard,
                        Err(poisoned) => {
                            error!(
                                "Consensus lock poisoned during history collection, recovering..."
                            );
                            poisoned.into_inner()
                        }
                    };
                    consensus.collect_history_for_parents(&vertex.parents)?
                };

                let all_to_execute = self.collect_unexecuted_txs(
                    &history_vertices,
                    transactions.iter(),
                    &chain,
                    !transactions.is_empty(),
                )?;

                if all_to_execute.is_empty() {
                    info!(
                        "[DAG SYNC] No new transactions to execute for vertex round {} (ID: {})",
                        vertex.round, vertex_id_hex
                    );
                } else {
                    info!(
                        "[DAG SYNC] Validating {} transactions in parallel waves for vertex round {}",
                        all_to_execute.len(),
                        vertex.round
                    );
                }

                // Execute transactions in parallel waves
                let (executed_count, failed_count) = self
                    .engine
                    .execute_tx_waves_parallel(
                        all_to_execute,
                        &state_arc,
                        Some(vertex.timestamp),
                        false, // persist_objects = false
                        false, // strict_mode = false
                    )
                    .unwrap_or((0, 0));

                let root = match state_arc.write() {
                    Ok(guard) => guard.compute_state_root(),
                    Err(poisoned) => {
                        error!(
                            "State arc lock poisoned during sync compute_state_root, recovering..."
                        );
                        poisoned.into_inner().compute_state_root()
                    }
                };
                (root, executed_count, failed_count)
            };

            info!(
                "[DAG SYNC] Validation result for vertex round {}: executed={}, failed={}, computed_root={}",
                vertex.round,
                executed,
                failed,
                hex::encode(&computed_state_root)
            );

            let expected_state_root = &vertex.metadata.state_root;
            if computed_state_root != *expected_state_root {
                warn!(
                    "[DAG SYNC] State root mismatch for vertex round {}. Expected: {}, computed: {}, transactions: {}. Accepting vertex; checkpoint validation remains authoritative.",
                    vertex.round,
                    hex::encode(expected_state_root),
                    hex::encode(&computed_state_root),
                    transactions.len()
                );
            } else {
                info!(
                    "[DAG SYNC] State root validated successfully for vertex round {}: {}",
                    vertex.round,
                    hex::encode(&computed_state_root)
                );
            }
        }

        let checkpoint = {
            let mut consensus = self.consensus.write().unwrap_or_else(|e| e.into_inner());
            consensus.add_vertex(vertex)?;
            consensus.try_commit()?
        };
        self.persist_consensus_state()?;

        if let Some(checkpoint) = checkpoint {
            info!(
                "[DAG SYNC] Committed checkpoint {} with {} transactions",
                checkpoint.sequence,
                checkpoint.transactions.len()
            );

            let checkpoint = match self.apply_checkpoint_once(checkpoint, "[DAG SYNC]", true) {
                Ok(checkpoint) => checkpoint,
                Err(e) => {
                    error!(
                        "[DAG SYNC] Failed to apply committed checkpoint to engine: {}",
                        e
                    );
                    return Err(e);
                }
            };

            let mut consensus = self.consensus.write().unwrap_or_else(|e| e.into_inner());
            consensus.add_checkpoint(checkpoint)?;
            drop(consensus);
            self.persist_consensus_state()?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_engine_creation() {
        let engine = Arc::new(BlockchainEngine::new().unwrap());
        let authorities = vec![
            "auth1".to_string(),
            "auth2".to_string(),
            "auth3".to_string(),
            "auth4".to_string(),
        ];

        let dag_engine = DagEngine::new(engine, "auth1".to_string(), authorities);
        assert!(dag_engine.is_ok());
    }

    #[test]
    fn test_apply_checkpoint_once_reuses_canonical_checkpoint() {
        let engine = Arc::new(BlockchainEngine::new().unwrap());
        let authorities = vec![
            "auth1".to_string(),
            "auth2".to_string(),
            "auth3".to_string(),
            "auth4".to_string(),
        ];
        let dag_engine = DagEngine::new(engine.clone(), "auth1".to_string(), authorities).unwrap();

        let prev_hash = {
            let chain = engine.blockchain.read().unwrap_or_else(|e| e.into_inner());
            chain.latest_checkpoint().hash().unwrap()
        };
        let canonical_root = engine
            .state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .compute_state_root();
        let checkpoint = centauri::consensus::Checkpoint::new(
            1,
            vec![],
            vec![],
            canonical_root.clone(),
            1,
            prev_hash,
        );

        let verified_state = engine
            .state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        engine
            .apply_prepared_checkpoint(checkpoint.clone(), verified_state, vec![])
            .unwrap();

        let mut stale_checkpoint = checkpoint.clone();
        stale_checkpoint.state_root = vec![9u8; 32];

        let resolved = dag_engine
            .apply_checkpoint_once(stale_checkpoint, "[TEST]", false)
            .unwrap();

        assert_eq!(resolved.sequence, checkpoint.sequence);
        assert_eq!(resolved.state_root, checkpoint.state_root);
    }

    #[test]
    fn test_can_create_catch_up_vertex_for_partial_round() {
        let authorities = vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()];

        let engine_a = Arc::new(BlockchainEngine::new().unwrap());
        let dag_a = DagEngine::new(engine_a, "0x1".to_string(), authorities.clone()).unwrap();
        let remote_round_one = dag_a.produce_vertex().unwrap();
        let remote_vertex = remote_round_one.vertex.unwrap();
        assert_eq!(remote_vertex.round, 1);

        let engine_b = Arc::new(BlockchainEngine::new().unwrap());
        let dag_b = DagEngine::new(engine_b, "0x2".to_string(), authorities).unwrap();
        dag_b.add_network_vertex(remote_vertex).unwrap();

        let catch_up_vertex = dag_b.produce_vertex().unwrap();
        assert_eq!(catch_up_vertex.round, 1);
    }

    #[test]
    fn test_empty_vertex_does_not_reexecute_history_transactions() {
        let authorities = vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()];

        let mut engine_a = BlockchainEngine::new().unwrap();
        engine_a.set_authorities("0x1".to_string(), authorities.clone());
        let engine_a = Arc::new(engine_a);
        let dag_a = DagEngine::new(engine_a, "0x1".to_string(), authorities.clone()).unwrap();

        let tx = SignedTransaction::new(Transaction::Transfer {
            from: "0x1".to_string(),
            to: "0x2".to_string(),
            amount: 1,
            gas_limit: 1000,
            gas_price: 1,
            sequence_number: 0,
        });
        dag_a
            .engine
            .pending_txs
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push(tx);

        let tx_vertex = dag_a.produce_vertex().unwrap();
        assert_eq!(tx_vertex.tx_count, 1);
        assert_eq!(tx_vertex.executed, 1);

        let remote_round_one = tx_vertex.vertex.unwrap();

        let mut engine_b = BlockchainEngine::new().unwrap();
        engine_b.set_authorities("0x2".to_string(), authorities);
        let engine_b = Arc::new(engine_b);
        let dag_b = DagEngine::new(
            engine_b,
            "0x2".to_string(),
            vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()],
        )
        .unwrap();

        dag_b.add_network_vertex(remote_round_one).unwrap();

        let empty_vertex = dag_b.produce_vertex().unwrap();
        assert_eq!(empty_vertex.tx_count, 0);
        assert_eq!(empty_vertex.executed, 0);
    }

    #[test]
    fn test_apply_checkpoint_once_overrides_provisional_root() {
        let mut engine = BlockchainEngine::new().unwrap();
        engine.set_authorities(
            "0x1".to_string(),
            vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()],
        );
        let engine = Arc::new(engine);
        let dag_engine = DagEngine::new(
            engine.clone(),
            "0x1".to_string(),
            vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()],
        )
        .unwrap();

        let tx = SignedTransaction::new(Transaction::Transfer {
            from: "0x1".to_string(),
            to: "0x2".to_string(),
            amount: 1,
            gas_limit: 1000,
            gas_price: 1,
            sequence_number: 0,
        });

        let prev_hash = {
            let chain = engine.blockchain.read().unwrap_or_else(|e| e.into_inner());
            chain.latest_checkpoint().hash().unwrap()
        };

        let provisional_checkpoint =
            centauri::consensus::Checkpoint::new(1, vec![], vec![tx], vec![7u8; 32], 1, prev_hash);

        let resolved = dag_engine
            .apply_checkpoint_once(provisional_checkpoint, "[TEST]", true)
            .unwrap();

        assert_ne!(resolved.state_root, vec![7u8; 32]);
        assert_eq!(resolved.sequence, 1);
    }

    #[test]
    fn test_apply_checkpoint_once_keeps_previous_root_for_empty_checkpoint() {
        let mut engine = BlockchainEngine::new().unwrap();
        engine.set_authorities(
            "0x2".to_string(),
            vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()],
        );
        let engine = Arc::new(engine);
        let dag_engine = DagEngine::new(
            engine.clone(),
            "0x2".to_string(),
            vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()],
        )
        .unwrap();

        let (prev_hash, previous_root) = {
            let chain = engine.blockchain.read().unwrap_or_else(|e| e.into_inner());
            (
                chain.latest_checkpoint().hash().unwrap(),
                chain.latest_checkpoint().state_root.clone(),
            )
        };

        let provisional_checkpoint =
            centauri::consensus::Checkpoint::new(1, vec![], vec![], vec![7u8; 32], 1, prev_hash);

        let resolved = dag_engine
            .apply_checkpoint_once(provisional_checkpoint, "[TEST]", true)
            .unwrap();

        assert_eq!(resolved.state_root, previous_root);
        assert_eq!(resolved.sequence, 1);
    }
}
