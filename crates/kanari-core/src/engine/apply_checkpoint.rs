// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::BlockchainEngine;
use anyhow::{Context, Result, bail};
use centauri::consensus::Checkpoint;
use kanari_move_runtime_v1::state::StateManager;
use kanari_types::transaction::SignedTransaction;
use log::{error, info};
use std::sync::{Arc, RwLock};

impl BlockchainEngine {
    fn requires_runtime_side_effect_persistence(transactions: &[SignedTransaction]) -> bool {
        transactions.iter().any(|signed_tx| {
            matches!(
                signed_tx.transaction,
                kanari_types::transaction::Transaction::PublishModule { .. }
                    | kanari_types::transaction::Transaction::ExecuteFunction { .. }
            )
        })
    }

    fn apply_system_prologue_to_state(
        &self,
        state_arc: &Arc<RwLock<StateManager>>,
        timestamp_ms: u64,
        persist_objects: bool,
    ) -> Result<()> {
        let runtime = &self.runtime_pool[0];
        let mut state_write = state_arc.write().unwrap_or_else(|e| e.into_inner());
        let clock_id = runtime.ensure_system_clock(&mut state_write)?;
        let changeset = runtime.execute_clock_consensus_commit_prologue(clock_id, timestamp_ms)?;
        state_write.apply_changeset(&changeset)?;

        if persist_objects {
            runtime.persist_created_objects(&changeset);
            runtime.persist_deleted_objects(&changeset);
        }

        Ok(())
    }

    fn ensure_checkpoint_root_matches(
        &self,
        checkpoint: &Checkpoint,
        computed_root: &[u8],
    ) -> Result<()> {
        if self.checkpoint_root_matches(
            checkpoint.sequence,
            computed_root,
            &checkpoint.state_root,
        )? {
            return Ok(());
        }

        bail!(
            "[ENGINE] State root mismatch for checkpoint {}. expected={}, computed={}",
            checkpoint.sequence,
            hex::encode(&checkpoint.state_root),
            hex::encode(computed_root)
        );
    }

    pub(crate) fn prepare_checkpoint_state(
        &self,
        checkpoint: &Checkpoint,
    ) -> Result<(Vec<u8>, StateManager, Vec<SignedTransaction>)> {
        let state_snapshot = self.state.read().unwrap_or_else(|e| e.into_inner()).clone();
        let state_arc = Arc::new(RwLock::new(state_snapshot));
        let to_execute: Vec<SignedTransaction> = {
            let chain = self.blockchain.read().unwrap_or_else(|e| e.into_inner());
            checkpoint
                .transactions
                .iter()
                .filter(|signed_tx| {
                    !chain.is_transaction_hash_executed(&signed_tx.transaction.hash())
                })
                .cloned()
                .collect()
        };

        if !checkpoint.transactions.is_empty() {
            self.apply_system_prologue_to_state(&state_arc, checkpoint.timestamp, false)?;
        }

        self.execute_tx_waves_deterministic_parallel(
            to_execute.clone(),
            &state_arc,
            Some(checkpoint.timestamp),
            false, // persist_objects = false
        )?;

        let verified_state = state_arc.read().unwrap_or_else(|e| e.into_inner()).clone();
        let computed_root = verified_state.compute_state_root();
        Ok((computed_root, verified_state, to_execute))
    }

    /// Helper: Common steps for finalizing Checkpoint to database
    fn finalize_checkpoint(&self, checkpoint: Checkpoint, new_state: StateManager) -> Result<()> {
        new_state
            .validate_supply_invariants()
            .context("Supply invariants failed before checkpoint commit")?;

        // 1. Update canonical state
        {
            let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
            *state = new_state;
            state
                .commit()
                .context("Failed to commit state to RocksDB")?;
        }

        // 2. Update blockchain
        {
            let mut chain = self.blockchain.write().unwrap_or_else(|e| e.into_inner());
            chain.add_checkpoint_with_validation(checkpoint.clone(), false)?;
        }

        // 3. Remove committed transactions from pending pool
        {
            let mut pending = self.pending_txs.write().unwrap_or_else(|e| e.into_inner());
            let committed_hashes: std::collections::HashSet<_> = checkpoint
                .transactions
                .iter()
                .map(|tx| tx.transaction.hash())
                .collect();
            pending.retain(|tx| !committed_hashes.contains(&tx.transaction.hash()));
            self.pending_tx_hashes
                .write()
                .unwrap_or_else(|e| e.into_inner())
                .retain(|hash| !committed_hashes.contains(hash));
        }

        // 4. Persist blockchain state
        if let Some(store) = &self.persistent_store {
            let chain = self.blockchain.read().unwrap_or_else(|e| e.into_inner());
            if let Err(e) = store.save(b"blockchain", &*chain) {
                error!("Failed to persist blockchain: {}", e);
            }
        }
        Ok(())
    }

    pub(crate) fn apply_prepared_checkpoint(
        &self,
        checkpoint: Checkpoint,
        verified_state: StateManager,
        to_execute: Vec<SignedTransaction>,
    ) -> Result<()> {
        if !to_execute.is_empty() && Self::requires_runtime_side_effect_persistence(&to_execute) {
            let side_effect_state = Arc::new(RwLock::new(
                self.state.read().unwrap_or_else(|e| e.into_inner()).clone(),
            ));
            self.apply_system_prologue_to_state(&side_effect_state, checkpoint.timestamp, true)?;
            self.execute_tx_waves_deterministic_parallel(
                to_execute,
                &side_effect_state,
                Some(checkpoint.timestamp),
                true, // persist_objects = true
            )?;
        }

        self.finalize_checkpoint(checkpoint, verified_state)
    }

    pub fn apply_checkpoint(&self, checkpoint: Checkpoint) -> Result<()> {
        info!(
            "[ENGINE] Applying checkpoint {} with {} txs",
            checkpoint.sequence,
            checkpoint.transactions.len()
        );

        let (computed_root, verified_state, to_execute) =
            self.prepare_checkpoint_state(&checkpoint)?;
        self.ensure_checkpoint_root_matches(&checkpoint, &computed_root)?;

        self.apply_prepared_checkpoint(checkpoint, verified_state, to_execute)
    }
}
