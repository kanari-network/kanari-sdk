use super::BlockchainEngine;
use anyhow::{Context, Result};
use centauri::consensus::Checkpoint;
use kanari_move_runtime::TransactionScheduler;
use kanari_move_runtime::changeset::ChangeSet;
use kanari_move_runtime::state::StateManager;
use kanari_types::transaction::SignedTransaction;
use log::{error, info, warn};
use rayon::prelude::*;
use std::sync::{Arc, RwLock};

impl BlockchainEngine {
    /// Apply a committed checkpoint using a pre-computed state (Optimization)
    /// This skips re-execution of transactions if we already have the resulting state.
    pub fn apply_checkpoint_optimized(
        &self,
        checkpoint: Checkpoint,
        precomputed_state: Arc<RwLock<StateManager>>,
    ) -> Result<()> {
        info!(
            "[ENGINE] Applying checkpoint {} (OPTIMIZED) with {} transactions",
            checkpoint.sequence,
            checkpoint.transactions.len()
        );

        // 1. Verify state root
        let computed_root = precomputed_state.read().unwrap().compute_state_root();
        if computed_root != checkpoint.state_root {
            warn!(
                "[ENGINE] Optimized application failed: State root mismatch! Fallback to standard application. Expected: {}, Computed: {}",
                hex::encode(&checkpoint.state_root),
                hex::encode(&computed_root)
            );
            return self.apply_checkpoint(checkpoint);
        }

        // 2. Update canonical state
        {
            let mut state = self.state.write().unwrap();
            *state = precomputed_state.read().unwrap().clone();
            // Restore persistence as requested
            state
                .commit()
                .context("Failed to commit optimized state to RocksDB")?;
        }

        // 3. Update blockchain
        {
            let mut chain = self.blockchain.write().unwrap();
            chain.add_checkpoint_with_validation(checkpoint.clone(), false)?;
        }

        // 4. Remove committed transactions from pending pool
        {
            let mut pending = self.pending_txs.write().unwrap();
            let committed_hashes: std::collections::HashSet<_> =
                checkpoint.transactions.iter().map(|tx| tx.hash()).collect();
            pending.retain(|tx| !committed_hashes.contains(&tx.hash()));
        }

        // 5. Persist blockchain and state
        if let Some(store) = &self.persistent_store {
            let chain = self.blockchain.read().unwrap();
            // We persist the full blockchain struct.
            // Note: In a production environment with millions of blocks,
            // we should store blocks individually in the DB (e.g., block_height -> block_data)
            // and only keep the head metadata in the "blockchain" key.
            if let Err(e) = store.save(b"blockchain", &*chain) {
                error!("Failed to persist blockchain: {}", e);
            }
        }

        Ok(())
    }

    /// Apply a committed checkpoint to the state.
    /// This executes all transactions in the checkpoint and updates the canonical state.
    pub fn apply_checkpoint(&self, mut checkpoint: Checkpoint) -> Result<()> {
        info!(
            "[ENGINE] Applying checkpoint {} with {} transactions, state_root: {}",
            checkpoint.sequence,
            checkpoint.transactions.len(),
            hex::encode(&checkpoint.state_root)
        );

        // 1. Create a clone of the current state to work on
        let state_snapshot = self.state.read().unwrap().clone();
        let state_arc = Arc::new(RwLock::new(state_snapshot));

        // 2. Filter transactions that are already executed
        let mut to_execute: Vec<SignedTransaction> = Vec::new();
        let mut skipped_count = 0;

        {
            let chain = self.blockchain.read().unwrap();
            for signed_tx in &checkpoint.transactions {
                let tx_hash_hex = hex::encode(signed_tx.hash());
                if chain.is_transaction_executed(&tx_hash_hex) {
                    skipped_count += 1;
                    continue;
                }
                to_execute.push(signed_tx.clone());
            }
        }

        // 3. Partition and execute in parallel waves
        let waves = TransactionScheduler::schedule(to_execute);
        let mut executed_count = 0;

        for wave in waves {
            let results: Vec<Result<ChangeSet>> = wave
                .par_iter()
                .enumerate()
                .map(|(i, signed_tx)| {
                    let pool_idx = i % self.runtime_pool.len();
                    let runtime = &self.runtime_pool[pool_idx];

                    self.execute_transaction_with_runtime_skip_seq_persist(
                        &signed_tx.transaction,
                        runtime,
                        &state_arc,
                        Some(checkpoint.timestamp),
                    )
                })
                .collect();

            for res in results {
                match res {
                    Ok(cs) => {
                        let mut state_write = state_arc.write().unwrap();
                        if let Err(e) = state_write.apply_changeset(&cs) {
                            error!(
                                "[ENGINE] Failed to apply changeset in checkpoint {}: {}",
                                checkpoint.sequence, e
                            );
                            anyhow::bail!("Failed to apply changeset: {}", e);
                        }
                        executed_count += 1;
                    }
                    Err(e) => {
                        error!(
                            "[ENGINE] Fatal error executing transaction in checkpoint {}: {}",
                            checkpoint.sequence, e
                        );
                        anyhow::bail!("Fatal error executing checkpoint transaction: {}", e);
                    }
                }
            }
        }

        if skipped_count > 0 {
            info!(
                "[ENGINE] Checkpoint {} summary: {} executed, {} skipped (already in blockchain)",
                checkpoint.sequence, executed_count, skipped_count
            );
        }

        // 3. Verify the final state root
        let verified_state = {
            let state_read = state_arc.read().unwrap();
            let computed_root = state_read.compute_state_root();
            if computed_root != checkpoint.state_root {
                let expected_hex = hex::encode(&checkpoint.state_root);
                let computed_hex = hex::encode(&computed_root);

                // In DAG mode, the checkpoint's state root might be from a leader vertex
                // that didn't see the exact same history as the checkpoint's total order.
                // We update to the computed root to ensure consistency.
                warn!(
                    "[ENGINE] State root mismatch in checkpoint {}! Updating to computed root.\n  Expected (from leader): {}\n  Computed (from execution): {}",
                    checkpoint.sequence, expected_hex, computed_hex
                );
                checkpoint.state_root = computed_root;
            }
            state_read.clone()
        };

        // 4. Update canonical state by replacing it with the verified state
        {
            let mut state = self.state.write().unwrap();
            *state = verified_state;
            state
                .commit()
                .context("Failed to commit state to RocksDB")?;
        }

        // 5. Update blockchain
        {
            let mut chain = self.blockchain.write().unwrap();
            // Add checkpoint without strict validation (already validated locally)
            chain.add_checkpoint_with_validation(checkpoint.clone(), false)?;
        }

        // 6. Remove committed transactions from pending pool
        {
            let mut pending = self.pending_txs.write().unwrap();
            let committed_hashes: std::collections::HashSet<_> =
                checkpoint.transactions.iter().map(|tx| tx.hash()).collect();
            pending.retain(|tx| !committed_hashes.contains(&tx.hash()));
        }

        // 7. Persist blockchain and state
        if let Some(store) = &self.persistent_store {
            let chain = self.blockchain.read().unwrap();
            // We persist the full blockchain struct.
            // Note: In a production environment with millions of blocks,
            // we should store blocks individually in the DB (e.g., block_height -> block_data)
            // and only keep the head metadata in the "blockchain" key.
            if let Err(e) = store.save(b"blockchain", &*chain) {
                error!("Failed to persist blockchain: {}", e);
            }
        }

        Ok(())
    }
}
