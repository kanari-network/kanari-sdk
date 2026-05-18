// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::BlockchainEngine;
use anyhow::{Context, Result, bail};
use centauri::consensus::Checkpoint;
use kanari_move_runtime_v1::state::StateManager;
use kanari_types::transaction::SignedTransaction;
use log::{error, info, warn};
use std::sync::{Arc, RwLock};

impl BlockchainEngine {
    pub(crate) fn prepare_checkpoint_state(
        &self,
        checkpoint: &Checkpoint,
    ) -> Result<(Vec<u8>, StateManager, Vec<SignedTransaction>)> {
        let state_snapshot = self.state.read().unwrap_or_else(|e| e.into_inner()).clone();
        let state_arc = Arc::new(RwLock::new(state_snapshot));
        let mut to_execute: Vec<SignedTransaction> = Vec::new();

        {
            let chain = self.blockchain.read().unwrap_or_else(|e| e.into_inner());
            for signed_tx in &checkpoint.transactions {
                if !chain.is_transaction_executed(&hex::encode(signed_tx.hash())) {
                    to_execute.push(signed_tx.clone());
                }
            }
        }

        self.execute_tx_waves_parallel(
            to_execute.clone(),
            &state_arc,
            Some(checkpoint.timestamp),
            false, // persist_objects = false
            true,  // strict_mode = true
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
            let committed_hashes: std::collections::HashSet<_> =
                checkpoint.transactions.iter().map(|tx| tx.hash()).collect();
            pending.retain(|tx| !committed_hashes.contains(&tx.hash()));
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
        if !to_execute.is_empty() {
            let side_effect_state = Arc::new(RwLock::new(
                self.state.read().unwrap_or_else(|e| e.into_inner()).clone(),
            ));
            self.execute_tx_waves_parallel(
                to_execute,
                &side_effect_state,
                Some(checkpoint.timestamp),
                true, // persist_objects = true
                true, // strict_mode = true
            )?;
        }

        self.finalize_checkpoint(checkpoint, verified_state)
    }

    pub fn apply_checkpoint_optimized(
        &self,
        checkpoint: Checkpoint,
        precomputed_state: Arc<RwLock<StateManager>>,
    ) -> Result<()> {
        info!(
            "[ENGINE] Applying checkpoint {} (OPTIMIZED)",
            checkpoint.sequence
        );

        let computed_root = precomputed_state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .compute_state_root();
        if !self.checkpoint_root_matches(
            checkpoint.sequence,
            &computed_root,
            &checkpoint.state_root,
        )? {
            warn!("[ENGINE] State root mismatch! Fallback to standard application.");
            return self.apply_checkpoint(checkpoint);
        }

        self.finalize_checkpoint(
            checkpoint,
            precomputed_state
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
        )
    }

    pub fn apply_checkpoint(&self, checkpoint: Checkpoint) -> Result<()> {
        info!(
            "[ENGINE] Applying checkpoint {} with {} txs",
            checkpoint.sequence,
            checkpoint.transactions.len()
        );

        let (computed_root, verified_state, to_execute) =
            self.prepare_checkpoint_state(&checkpoint)?;
        if !self.checkpoint_root_matches(
            checkpoint.sequence,
            &computed_root,
            &checkpoint.state_root,
        )? {
            bail!(
                "[ENGINE] State root mismatch for checkpoint {}. expected={}, computed={}",
                checkpoint.sequence,
                hex::encode(&checkpoint.state_root),
                hex::encode(&computed_root)
            );
        }

        self.apply_prepared_checkpoint(checkpoint, verified_state, to_execute)
    }
}
