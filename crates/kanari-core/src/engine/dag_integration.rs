// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use anyhow::Result;
use centauri::consensus::{
    DagConsensus, DagExecutionPlan, DagPendingSelection, DagProductionPlan, DagVertex, VertexId,
};
use log::{info, warn};
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub(crate) struct DagConsensusIntegration {
    engine: Arc<BlockchainEngine>,
    consensus: Arc<RwLock<DagConsensus>>,
}

#[derive(Debug, Clone)]
pub(crate) struct DagExecutionOutcome {
    pub state_root: Vec<u8>,
    pub executed: usize,
    pub failed: usize,
}

impl DagConsensusIntegration {
    pub(crate) fn new(engine: Arc<BlockchainEngine>, consensus: Arc<RwLock<DagConsensus>>) -> Self {
        Self { engine, consensus }
    }

    pub(crate) fn persist_consensus_state(&self) -> Result<()> {
        if self.engine.persistent_store.is_none() {
            return Ok(());
        }

        let state = {
            let consensus = self.consensus.read().unwrap_or_else(|e| e.into_inner());
            consensus.save_state()?
        };
        self.engine.persist_dag_state(state)
    }

    pub(crate) fn submit_vertex(
        &self,
        vertex: centauri::consensus::DagVertex,
    ) -> Result<Option<centauri::consensus::Checkpoint>> {
        let checkpoint = {
            let mut consensus = self.consensus.write().unwrap_or_else(|e| e.into_inner());
            consensus.add_vertex_and_try_commit(vertex)?
        };
        self.persist_consensus_state()?;
        Ok(checkpoint)
    }

    pub(crate) fn select_pending_for_production(
        &self,
        plan: &DagProductionPlan,
    ) -> DagPendingSelection {
        let chain = self
            .engine
            .blockchain
            .read()
            .unwrap_or_else(|e| e.into_inner());
        if plan.history_tx_hashes.is_empty() && !chain.has_executed_transactions() {
            let mut pending = self
                .engine
                .pending_txs
                .write()
                .unwrap_or_else(|e| e.into_inner());
            let included = if pending.len() <= 500_000 {
                let included = std::mem::take(&mut *pending);
                self.engine
                    .pending_tx_hashes
                    .write()
                    .unwrap_or_else(|e| e.into_inner())
                    .clear();
                self.engine.clear_pending_sender_counts();
                included
            } else {
                let included = pending.drain(..500_000).collect::<Vec<_>>();
                let remove_hashes = included
                    .iter()
                    .map(|tx| tx.transaction_hash().to_vec())
                    .collect::<HashSet<_>>();
                self.engine
                    .pending_tx_hashes
                    .write()
                    .unwrap_or_else(|e| e.into_inner())
                    .retain(|hash| !remove_hashes.contains(hash));
                self.engine.remove_pending_sender_counts(&included);
                included
            };
            return DagPendingSelection {
                included,
                remove_hashes: Vec::new(),
            };
        }

        let pending = self
            .engine
            .pending_txs
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let consensus = self.consensus.read().unwrap_or_else(|e| e.into_inner());

        consensus.select_pending_transactions(plan, &pending, |hash| {
            chain.has_executed_transactions() && chain.is_transaction_hash_executed(hash)
        })
    }

    pub(crate) fn remove_pending_hashes(&self, remove_hashes: &[Vec<u8>]) {
        if remove_hashes.is_empty() {
            return;
        }

        let remove_set: HashSet<_> = remove_hashes.iter().cloned().collect();
        let mut pending = self
            .engine
            .pending_txs
            .write()
            .unwrap_or_else(|e| e.into_inner());
        if pending.len() == remove_hashes.len() {
            pending.clear();
            self.engine
                .pending_tx_hashes
                .write()
                .unwrap_or_else(|e| e.into_inner())
                .clear();
            self.engine.clear_pending_sender_counts();
            return;
        }

        let removed_transactions = pending
            .iter()
            .filter(|tx| remove_set.contains(tx.transaction_hash()))
            .cloned()
            .collect::<Vec<_>>();
        pending.retain(|tx| !remove_set.contains(tx.transaction_hash()));
        self.engine
            .pending_tx_hashes
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|hash| !remove_set.contains(hash));
        self.engine
            .remove_pending_sender_counts(&removed_transactions);
    }

    pub(crate) fn remove_pending_transactions(&self, transactions: &[SignedTransaction]) -> usize {
        let remove_hashes: Vec<Vec<u8>> = transactions
            .iter()
            .map(|tx| tx.transaction_hash().to_vec())
            .collect();
        self.remove_pending_hashes(&remove_hashes);
        self.engine
            .pending_txs
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    pub(crate) fn execute_vertex_plan(
        &self,
        history_vertices: &[VertexId],
        transactions: &[SignedTransaction],
        timestamp: u64,
        include_history: bool,
    ) -> Result<DagExecutionOutcome> {
        if history_vertices.is_empty() && Self::can_skip_zero_cost_native_preview(transactions) {
            let state_root = self
                .engine
                .blockchain
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .latest_checkpoint()
                .state_root
                .clone();

            return Ok(DagExecutionOutcome {
                state_root,
                executed: transactions.len(),
                failed: 0,
            });
        }

        let chain = self
            .engine
            .blockchain
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let consensus = self.consensus.read().unwrap_or_else(|e| e.into_inner());
        let DagExecutionPlan {
            transactions: all_to_execute,
            ..
        } = consensus.execution_plan_for_current_txs(
            history_vertices,
            transactions,
            include_history,
            |hash| chain.has_executed_transactions() && chain.is_transaction_hash_executed(hash),
        );
        drop(consensus);

        drop(chain);

        if Self::can_skip_zero_cost_native_preview(&all_to_execute) {
            let state_root = self
                .engine
                .blockchain
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .latest_checkpoint()
                .state_root
                .clone();

            return Ok(DagExecutionOutcome {
                state_root,
                executed: all_to_execute.len(),
                failed: 0,
            });
        }

        let state_clone = self
            .engine
            .state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let state_arc = Arc::new(RwLock::new(state_clone));

        let (executed_count, failed_count) = self.engine.execute_tx_waves_deterministic_parallel(
            all_to_execute,
            &state_arc,
            Some(timestamp),
            false,
        )?;

        let state_root = self
            .engine
            .blockchain
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .latest_checkpoint()
            .state_root
            .clone();

        Ok(DagExecutionOutcome {
            state_root,
            executed: executed_count,
            failed: failed_count,
        })
    }

    fn can_skip_zero_cost_native_preview(transactions: &[SignedTransaction]) -> bool {
        !transactions.is_empty()
            && transactions.iter().all(|tx| {
                tx.transaction.gas_price() == 0
                    && tx
                        .transaction
                        .native_call()
                        .map(|native_call| native_call.required_native_amount() == 0)
                        .unwrap_or(false)
            })
    }

    pub(crate) fn validate_network_vertex(
        &self,
        vertex: &DagVertex,
    ) -> Result<Option<DagExecutionOutcome>> {
        if vertex.transactions.is_empty() {
            return Ok(None);
        }

        let consensus = self.consensus.read().unwrap_or_else(|e| e.into_inner());
        let DagExecutionPlan {
            transactions: all_to_execute,
            ..
        } = {
            let chain = self
                .engine
                .blockchain
                .read()
                .unwrap_or_else(|e| e.into_inner());
            consensus.execution_plan_for_network_vertex(vertex, |hash| {
                chain.has_executed_transactions() && chain.is_transaction_hash_executed(hash)
            })?
        };
        drop(consensus);

        if all_to_execute.is_empty() {
            return Ok(Some(DagExecutionOutcome {
                state_root: self
                    .engine
                    .state
                    .read()
                    .unwrap_or_else(|e| e.into_inner())
                    .compute_state_root(),
                executed: 0,
                failed: 0,
            }));
        }

        let state_clone = self
            .engine
            .state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let state_arc = Arc::new(RwLock::new(state_clone));

        let (executed_count, failed_count) = self.engine.execute_tx_waves_deterministic_parallel(
            all_to_execute,
            &state_arc,
            Some(vertex.timestamp),
            false,
        )?;

        let state_root = match state_arc.write() {
            Ok(guard) => guard.compute_state_root(),
            Err(poisoned) => poisoned.into_inner().compute_state_root(),
        };

        if state_root != vertex.metadata.state_root {
            let latest_checkpoint_root = self
                .engine
                .blockchain
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .latest_checkpoint()
                .state_root
                .clone();

            if vertex.metadata.state_root == latest_checkpoint_root {
                info!(
                    "[DAG SYNC] Vertex round {} uses provisional checkpoint root {}; computed validation root is {}",
                    vertex.round,
                    hex::encode(&vertex.metadata.state_root),
                    hex::encode(&state_root)
                );

                return Ok(Some(DagExecutionOutcome {
                    state_root,
                    executed: executed_count,
                    failed: failed_count,
                }));
            }

            warn!(
                "[DAG SYNC] Non-canonical vertex state root mismatch for round {}. advertised={}, local_preview={}, transactions={}. Accepting vertex and deferring canonical root to checkpoint replay.",
                vertex.round,
                hex::encode(&vertex.metadata.state_root),
                hex::encode(&state_root),
                vertex.transactions.len()
            );
        }

        Ok(Some(DagExecutionOutcome {
            state_root,
            executed: executed_count,
            failed: failed_count,
        }))
    }

    pub(crate) fn finalize_checkpoint(
        &self,
        checkpoint: centauri::consensus::Checkpoint,
        log_prefix: &str,
    ) -> Result<centauri::consensus::Checkpoint> {
        let already_finalized = self.current_chain_height() >= checkpoint.sequence;
        let checkpoint = self.apply_checkpoint_once(checkpoint, log_prefix, true)?;

        if already_finalized {
            self.reconcile_consensus_to_finalized_chain()?;
            return Ok(checkpoint);
        }

        {
            let mut consensus = self.consensus.write().unwrap_or_else(|e| e.into_inner());
            consensus.add_checkpoint(checkpoint.clone())?;
        }
        self.persist_consensus_state()?;
        Ok(checkpoint)
    }

    fn reconcile_consensus_to_finalized_chain(&self) -> Result<()> {
        let checkpoints = {
            let chain = self
                .engine
                .blockchain
                .read()
                .unwrap_or_else(|e| e.into_inner());
            chain.dag_checkpoints.iter().cloned().collect::<Vec<_>>()
        };

        if checkpoints.len() <= 1 {
            return Ok(());
        }

        {
            let mut consensus = self.consensus.write().unwrap_or_else(|e| e.into_inner());
            let mut state = consensus.save_state()?;
            state.checkpoints = checkpoints;
            state.last_checkpoint_round = state.current_round;
            consensus.load_state(state)?;
        }

        self.persist_consensus_state()
    }

    pub(crate) fn apply_checkpoint_once(
        &self,
        mut checkpoint: centauri::consensus::Checkpoint,
        log_prefix: &str,
        allow_root_override: bool,
    ) -> Result<centauri::consensus::Checkpoint> {
        let current_height = self.current_chain_height();
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

        if !checkpoint.transactions.is_empty()
            && checkpoint.state_root == previous_checkpoint_root
            && Self::can_skip_zero_cost_native_preview(&checkpoint.transactions)
        {
            let verified_state = self
                .engine
                .state
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            self.engine.apply_prepared_checkpoint(
                checkpoint.clone(),
                verified_state,
                Vec::new(),
            )?;
            return Ok(checkpoint);
        }

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

        log::info!(
            "{} Checkpoint {} root finalized by canonical replay: provisional={}, computed={}, txs={}",
            log_prefix,
            checkpoint.sequence,
            hex::encode(&checkpoint.state_root),
            hex::encode(&computed_root),
            checkpoint.transactions.len()
        );
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

    fn current_chain_height(&self) -> u64 {
        self.engine
            .blockchain
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .height()
    }
}
