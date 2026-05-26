// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! DAG-based block production for Kanari blockchain engine
//! Integrates DAG consensus with parallel transaction execution

use anyhow::Result;
use centauri::consensus::{
    DagConsensus, DagNetworkVertexAction, DagPendingSelection, DagProductionPlan,
};
use log::{error, info};
use std::sync::{Arc, RwLock};

use super::dag_integration::DagConsensusIntegration;
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

/// DAG-enabled blockchain engine
#[derive(Clone)]
pub struct DagEngine {
    /// Reference to the base blockchain engine
    engine: Arc<BlockchainEngine>,

    /// DAG consensus instance
    consensus: Arc<RwLock<DagConsensus>>,

    /// This node's authority ID
    authority_id: String,
}

impl DagEngine {
    fn integration(&self) -> DagConsensusIntegration {
        DagConsensusIntegration::new(self.engine.clone(), self.consensus.clone())
    }

    #[cfg(test)]
    fn apply_checkpoint_once(
        &self,
        checkpoint: centauri::consensus::Checkpoint,
        log_prefix: &str,
        allow_root_override: bool,
    ) -> Result<centauri::consensus::Checkpoint> {
        self.integration()
            .apply_checkpoint_once(checkpoint, log_prefix, allow_root_override)
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
        })
    }

    // =====================================================================
    // 💡 HELPER: Collect unique, unexecuted Transactions (History + Current)
    // =====================================================================

    /// Produce a DAG vertex with pending transactions
    pub fn produce_vertex(&self) -> Result<DagBlockInfo> {
        let DagProductionPlan {
            policy,
            history_vertices,
            history_tx_hashes,
        } = {
            let consensus = self.consensus.read().unwrap_or_else(|e| e.into_inner());
            consensus.production_plan()?
        };

        let integration = self.integration();

        let DagPendingSelection {
            included: transactions,
            remove_hashes: tx_to_remove_from_pending,
        } = integration.select_pending_for_production(&DagProductionPlan {
            policy: policy.clone(),
            history_vertices: history_vertices.clone(),
            history_tx_hashes: history_tx_hashes.clone(),
        });

        integration.remove_pending_hashes(&tx_to_remove_from_pending);

        let production_plan = DagProductionPlan {
            policy: policy.clone(),
            history_vertices: history_vertices.clone(),
            history_tx_hashes: history_tx_hashes.clone(),
        };
        {
            let consensus = self.consensus.read().unwrap_or_else(|e| e.into_inner());
            consensus.ensure_production_allowed(&production_plan, transactions.len())?;
        }

        let tx_count = transactions.len();
        // Convert wall-clock time to the millisecond unit expected by the Move clock prologue.
        let proposed_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs()) // ✅ CORRECT - seconds
            .unwrap_or(0);
        let proposed_timestamp = proposed_timestamp.saturating_mul(1000);
        let timestamp = {
            let consensus = self.consensus.read().unwrap_or_else(|e| e.into_inner());
            consensus.suggest_vertex_timestamp_for_plan(&production_plan, proposed_timestamp)
        };

        let outcome = integration.execute_vertex_plan(
            &history_vertices,
            &transactions,
            timestamp,
            !transactions.is_empty(),
        )?;
        info!(
            "[DAG] Executing vertex plan for {} transactions",
            transactions.len()
        );
        let state_root = outcome.state_root;
        let executed = outcome.executed;
        let failed = outcome.failed;

        let events: Vec<Event> = Vec::new();
        let vertex = {
            let mut consensus = self.consensus.write().unwrap_or_else(|e| e.into_inner());
            let v = consensus.create_vertex_from_plan(
                &production_plan,
                transactions.clone(),
                state_root.clone(),
                timestamp,
            )?;
            info!(
                "[DAG] Created vertex for round {} with {} transactions",
                v.round,
                transactions.len()
            );
            v
        };

        let vertex_id = hex::encode(vertex.id);
        let round = vertex.round;
        let vertex_for_broadcast = vertex.clone();
        let checkpoint_info = {
            let checkpoint = integration.submit_vertex(vertex)?;

            if let Some(checkpoint) = checkpoint {
                let checkpoint = integration.finalize_checkpoint(checkpoint, "[DAG]")?;

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
        consensus.needs_progress()
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
        consensus.latest_vertices_by_authority(&self.authority_id, limit)
    }

    pub fn sync_checkpoint(&self, checkpoint: centauri::consensus::Checkpoint) -> Result<()> {
        let mut consensus = self.consensus.write().unwrap_or_else(|e| e.into_inner());
        consensus.add_checkpoint(checkpoint)
    }

    pub fn add_network_vertex(&self, vertex: centauri::consensus::DagVertex) -> Result<()> {
        let vertex_id_hex = hex::encode(vertex.id);
        let integration = self.integration();
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
            const MAX_FUTURE_EMPTY_VERTEX_ROUNDS: u64 = 20;
            match consensus.classify_network_vertex(&vertex, MAX_FUTURE_EMPTY_VERTEX_ROUNDS) {
                DagNetworkVertexAction::Accept => {}
                DagNetworkVertexAction::IgnoreExisting => {
                    info!(
                        "[DAG SYNC] Vertex {} (round {}) already exists, skipping",
                        vertex_id_hex, vertex.round
                    );
                    return Ok(());
                }
                DagNetworkVertexAction::IgnoreFarFutureEmpty { current_round } => {
                    info!(
                        "[DAG SYNC] Ignoring far-future empty vertex {} at round {} (current round: {})",
                        vertex_id_hex, vertex.round, current_round
                    );
                    return Ok(());
                }
            }
        }

        let transactions = vertex.transactions.clone();

        if !transactions.is_empty() {
            let outcome = integration.validate_network_vertex(&vertex)?;
            let Some(outcome) = outcome else {
                unreachable!("non-empty transactions must produce validation outcome");
            };
            let executed = outcome.executed;
            let failed = outcome.failed;

            info!(
                "[DAG SYNC] Validation result for vertex round {}: executed={}, failed={}, state_root={}",
                vertex.round,
                executed,
                failed,
                hex::encode(&outcome.state_root)
            );
        }

        let checkpoint = integration.submit_vertex(vertex)?;

        if !transactions.is_empty() {
            let pending_remaining = integration.remove_pending_transactions(&transactions);
            if pending_remaining > 0 {
                info!(
                    "[DAG SYNC] Removed {} transactions from pending pool (keeping {})",
                    transactions.len(),
                    pending_remaining
                );
            }
        }

        if let Some(checkpoint) = checkpoint {
            info!(
                "[DAG SYNC] Committed checkpoint {} with {} transactions",
                checkpoint.sequence,
                checkpoint.transactions.len()
            );

            if let Err(e) = integration.finalize_checkpoint(checkpoint, "[DAG SYNC]") {
                error!(
                    "[DAG SYNC] Failed to apply committed checkpoint to engine: {}",
                    e
                );
                return Err(e);
            }
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
    fn test_partial_round_allows_safe_catch_up_vertex() {
        let authorities = vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()];

        let engine_a = Arc::new(BlockchainEngine::new_in_memory().unwrap());
        let dag_a = DagEngine::new(engine_a, "0x1".to_string(), authorities.clone()).unwrap();
        let remote_round_one = dag_a.produce_vertex().unwrap();
        let remote_vertex = remote_round_one.vertex.unwrap();

        let engine_b = Arc::new(BlockchainEngine::new_in_memory().unwrap());
        let dag_b = DagEngine::new(engine_b, "0x2".to_string(), authorities).unwrap();
        dag_b.add_network_vertex(remote_vertex).unwrap();
        let catch_up_vertex = dag_b.produce_vertex().unwrap();
        assert_eq!(catch_up_vertex.round, 1);
    }

    #[test]
    fn test_genesis_bootstrap_allows_empty_round_one_vertex() {
        let authorities = vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()];

        let engine = Arc::new(BlockchainEngine::new_in_memory().unwrap());
        let dag = DagEngine::new(engine, "0x1".to_string(), authorities).unwrap();

        let vertex = dag.produce_vertex().unwrap();
        assert_eq!(vertex.round, 1);
        assert_eq!(vertex.tx_count, 0);
        assert_eq!(vertex.executed, 0);
    }

    #[test]
    fn test_empty_vertex_does_not_reexecute_history_transactions_after_quorum() {
        let authorities = vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()];

        let mut engine_a = BlockchainEngine::new_in_memory().unwrap();
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

        let mut engine_c = BlockchainEngine::new_in_memory().unwrap();
        engine_c.set_authorities("0x3".to_string(), authorities.clone());
        let engine_c = Arc::new(engine_c);
        let dag_c = DagEngine::new(engine_c, "0x3".to_string(), authorities.clone()).unwrap();
        let quorum_round_one = dag_c.produce_vertex().unwrap().vertex.unwrap();

        let mut engine_b = BlockchainEngine::new_in_memory().unwrap();
        engine_b.set_authorities("0x2".to_string(), authorities.clone());
        let engine_b = Arc::new(engine_b);
        let dag_b = DagEngine::new(engine_b, "0x2".to_string(), authorities).unwrap();

        dag_b.add_network_vertex(remote_round_one).unwrap();
        dag_b.add_network_vertex(quorum_round_one).unwrap();

        let empty_vertex = dag_b.produce_vertex().unwrap();
        assert_eq!(empty_vertex.tx_count, 0);
        assert_eq!(empty_vertex.executed, 0);
    }

    #[test]
    fn test_network_vertex_state_root_mismatch_is_non_fatal() {
        let authorities = vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()];

        let mut source_engine = BlockchainEngine::new_in_memory().unwrap();
        source_engine.set_authorities("0x1".to_string(), authorities.clone());
        let source_engine = Arc::new(source_engine);
        let source_dag =
            DagEngine::new(source_engine, "0x1".to_string(), authorities.clone()).unwrap();

        let tx = SignedTransaction::new(Transaction::Transfer {
            from: "0x1".to_string(),
            to: "0x2".to_string(),
            amount: 1,
            gas_limit: 1000,
            gas_price: 1,
            sequence_number: 0,
        });
        source_dag
            .engine
            .pending_txs
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push(tx.clone());

        let remote_vertex = source_dag.produce_vertex().unwrap().vertex.unwrap();

        let mut target_engine = BlockchainEngine::new_in_memory().unwrap();
        target_engine.set_authorities("0x2".to_string(), authorities.clone());
        target_engine.execute_system_prologue(123).unwrap();
        let target_engine = Arc::new(target_engine);
        target_engine
            .pending_txs
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push(tx.clone());
        let target_dag =
            DagEngine::new(target_engine.clone(), "0x2".to_string(), authorities).unwrap();

        target_dag.add_network_vertex(remote_vertex).unwrap();
        assert_eq!(
            target_engine
                .pending_txs
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .len(),
            0
        );
    }

    #[test]
    fn test_network_vertex_accepts_preview_root_from_deterministic_replay() {
        let authorities = vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()];

        let mut source_engine = BlockchainEngine::new_in_memory().unwrap();
        source_engine.set_authorities("0x1".to_string(), authorities.clone());
        let source_engine = Arc::new(source_engine);
        let source_dag =
            DagEngine::new(source_engine, "0x1".to_string(), authorities.clone()).unwrap();

        let mut tx = SignedTransaction::new(Transaction::Transfer {
            from: "0x1".to_string(),
            to: "0x2".to_string(),
            amount: 1,
            gas_limit: 1000,
            gas_price: 1,
            sequence_number: 0,
        });
        tx.signature = vec![1];
        source_dag
            .engine
            .pending_txs
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push(tx);

        let remote_vertex = source_dag.produce_vertex().unwrap().vertex.unwrap();
        let source_checkpoint_root = source_dag
            .engine
            .blockchain
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .latest_checkpoint()
            .state_root
            .clone();
        assert_ne!(remote_vertex.metadata.state_root, source_checkpoint_root);

        let mut target_engine = BlockchainEngine::new_in_memory().unwrap();
        target_engine.set_authorities("0x2".to_string(), authorities.clone());
        let target_engine = Arc::new(target_engine);
        let target_dag = DagEngine::new(target_engine, "0x2".to_string(), authorities).unwrap();

        target_dag.add_network_vertex(remote_vertex).unwrap();
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
