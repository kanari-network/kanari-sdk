// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::BlockchainEngine;
use crate::consensus::Checkpoint;
use anyhow::{Context, Result, bail};
use kanari_move_runtime_v2::state::{PrecomputedSmtChanges, StateManager};
use kanari_types::transaction::SignedTransaction;
use log::info;
use std::sync::{Arc, RwLock};

type CheckpointRawChanges = (Vec<(Vec<u8>, Vec<u8>)>, Vec<Vec<u8>>);

/// Checkpoint transaction hashes are canonical 32-byte digests. Converting once
/// here keeps the fixed-size set allocations out of the hot finalize path.
fn committed_hash_key(hash: &[u8], label: &'static str) -> [u8; 32] {
    kanari_types::hash::bytes32_expect(hash, label)
}

pub(crate) struct PreparedCheckpointState {
    pub state_root: Vec<u8>,
    pub smt_changes: Option<PrecomputedSmtChanges>,
    pub state: StateManager,
    pub transactions: Vec<SignedTransaction>,
    pub effects: Vec<kanari_types::transaction::TransactionEffects>,
}

impl BlockchainEngine {
    /// Repair native wallet overcount. On the owned fast path (conflict-free),
    /// repair the cached counter; otherwise repair the legacy counter.
    fn repair_native_wallet_overcount(
        state: &mut StateManager,
        assume_conflict_free: bool,
    ) -> Result<bool> {
        if assume_conflict_free {
            state.repair_cached_native_wallet_overcount()
        } else {
            state.repair_legacy_native_wallet_overcount()
        }
    }

    fn changeset_needs_native_wallet_repair(
        changeset: &kanari_move_runtime_v2::changeset::ChangeSet,
    ) -> bool {
        !changeset.owner_deltas.is_empty()
            || !changeset.native_gas_credits.is_empty()
            || !changeset.token_gas_credits.is_empty()
            || !changeset.treasuries.is_empty()
            || !changeset.token_balance_sets.is_empty()
            || !changeset.gas_object_refs.is_empty()
    }

    pub(crate) fn requires_runtime_side_effect_persistence(
        transactions: &[SignedTransaction],
    ) -> bool {
        transactions.iter().any(|signed_tx| {
            if signed_tx.transaction.is_native_balance_call() {
                return false;
            }

            matches!(
                signed_tx.transaction,
                kanari_types::transaction::Transaction::PublishModule { .. }
                    | kanari_types::transaction::Transaction::PublishPackage { .. }
                    | kanari_types::transaction::Transaction::UpgradeModule { .. }
                    | kanari_types::transaction::Transaction::UpgradePackage { .. }
                    | kanari_types::transaction::Transaction::ExecuteFunction { .. }
            )
        })
    }

    /// Applies the system clock prologue to the given state.
    pub(crate) fn apply_system_prologue_to_state(
        &self,
        state_arc: &Arc<RwLock<StateManager>>,
        timestamp_ms: u64,
        persist_objects: bool,
    ) -> Result<()> {
        let runtime = &self.runtime_pool[0];
        let mut state_write = state_arc.write().unwrap_or_else(|e| e.into_inner());
        let clock_id = runtime.ensure_system_clock(&mut state_write)?;
        let changeset = runtime.build_native_clock_consensus_commit_prologue(
            &state_write,
            clock_id,
            timestamp_ms,
        )?;
        // Apply without supply validation first, then repair legacy overcount
        // that may be exposed by the clock prologue changeset.
        state_write
            .apply_changeset_without_supply_validation(&changeset)
            .context("Failed to apply clock prologue changeset")?;

        if persist_objects {
            runtime.persist_created_objects(&changeset)?;
            runtime.persist_deleted_objects(&changeset)?;
        }

        if Self::changeset_needs_native_wallet_repair(&changeset) {
            Self::repair_native_wallet_overcount(&mut state_write, false)
                .context("Failed to repair native wallet overcount after clock prologue")?;
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

    /// Prepares checkpoint state by executing transactions and computing the new state root.
    pub(crate) fn prepare_checkpoint_state(
        &self,
        checkpoint: &Checkpoint,
    ) -> Result<PreparedCheckpointState> {
        self.prepare_checkpoint_state_inner(checkpoint, false, None)
    }

    /// Owned-fastpath variant that consumes the already-selected transactions
    /// instead of cloning every signed transaction out of the checkpoint.
    pub(crate) fn prepare_conflict_free_checkpoint_state_with_transactions(
        &self,
        checkpoint: &Checkpoint,
        transactions: Vec<SignedTransaction>,
    ) -> Result<PreparedCheckpointState> {
        self.prepare_checkpoint_state_inner(checkpoint, true, Some(transactions))
    }

    fn prepare_checkpoint_state_inner(
        &self,
        checkpoint: &Checkpoint,
        assume_conflict_free: bool,
        owned_transactions: Option<Vec<SignedTransaction>>,
    ) -> Result<PreparedCheckpointState> {
        let profile = Self::env_flag_enabled("KANARI_CHECKPOINT_PREPARE_PROFILE");
        let started_at = std::time::Instant::now();
        let mut state_snapshot = self.state_read().clone();
        Self::repair_native_wallet_overcount(&mut state_snapshot, assume_conflict_free).context(
            "Failed to repair native wallet overcount before checkpoint state execution",
        )?;
        let state_arc = Arc::new(RwLock::new(state_snapshot));
        let cloned_at = std::time::Instant::now();
        let candidate_txs =
            owned_transactions.unwrap_or_else(|| checkpoint.transactions.iter().cloned().collect());
        let has_transactions = !candidate_txs.is_empty();
        // One committed-index lookup per checkpoint instead of one per tx.
        // Results keep candidate order, so execution order is unchanged.
        let hashes =
            candidate_txs.iter().map(|tx| tx.transaction_hash().to_vec()).collect::<Vec<_>>();
        let committed = self.try_are_transactions_committed(&hashes)?;
        let to_execute = candidate_txs
            .into_iter()
            .zip(committed)
            .filter_map(|(tx, is_committed)| (!is_committed).then_some(tx))
            .collect::<Vec<_>>();

        if has_transactions {
            self.apply_system_prologue_to_state(&state_arc, checkpoint.timestamp, false)?;
        }
        let prologue_at = std::time::Instant::now();

        let (_, _, transaction_effects) = if assume_conflict_free {
            self.execute_conflict_free_transactions_parallel_with_effects(
                &to_execute,
                &state_arc,
                Some(checkpoint.timestamp),
                false,
                // This entry point is only used for owned-fastpath snapshots
                // whose conflict keys were already validated at selection.
                true,
            )?
        } else {
            self.execute_tx_waves_deterministic_parallel_with_effects(
                to_execute.clone(),
                &state_arc,
                Some(checkpoint.timestamp),
                false, // persist_objects = false
            )?
        };
        let executed_at = std::time::Instant::now();

        {
            let mut state_write = state_arc.write().unwrap_or_else(|e| e.into_inner());
            Self::repair_native_wallet_overcount(&mut state_write, assume_conflict_free).context(
                "Failed to repair native wallet overcount after checkpoint state execution",
            )?;
        }
        let repaired_at = std::time::Instant::now();

        // No other task holds the execution state at this point. Move it out
        // instead of cloning the full overlay/access-version maps on the
        // checkpoint hot path.
        let verified_state = match Arc::try_unwrap(state_arc) {
            Ok(lock) => lock.into_inner().unwrap_or_else(|poisoned| poisoned.into_inner()),
            Err(arc) => arc.read().unwrap_or_else(|e| e.into_inner()).clone(),
        };
        let (computed_root, smt_changes) =
            verified_state.try_compute_state_root_with_smt_changes()?;
        let rooted_at = std::time::Instant::now();
        if profile {
            eprintln!(
                "checkpoint prepare profile: txs={} clone_repair={:.6}s prologue={:.6}s execute={:.6}s repair={:.6}s root={:.6}s total={:.6}s",
                checkpoint.transactions.len(),
                cloned_at.duration_since(started_at).as_secs_f64(),
                prologue_at.duration_since(cloned_at).as_secs_f64(),
                executed_at.duration_since(prologue_at).as_secs_f64(),
                repaired_at.duration_since(executed_at).as_secs_f64(),
                rooted_at.duration_since(repaired_at).as_secs_f64(),
                rooted_at.duration_since(started_at).as_secs_f64(),
            );
        }
        Ok(PreparedCheckpointState {
            state_root: computed_root,
            smt_changes,
            state: verified_state,
            transactions: to_execute,
            effects: transaction_effects,
        })
    }

    /// Helper: Common steps for finalizing Checkpoint to database
    fn finalize_checkpoint(
        &self,
        checkpoint: Checkpoint,
        new_state: StateManager,
        precomputed_smt_changes: Option<PrecomputedSmtChanges>,
        validate_supply: bool,
    ) -> Result<()> {
        let profile = Self::env_flag_enabled("KANARI_CHECKPOINT_FINALIZE_PROFILE");
        let started_at = std::time::Instant::now();
        let inspected_at = std::time::Instant::now();
        let has_module_publish = checkpoint.transactions.iter().any(|signed_tx| {
            matches!(
                signed_tx.transaction,
                kanari_types::transaction::Transaction::PublishModule { .. }
                    | kanari_types::transaction::Transaction::PublishPackage { .. }
                    | kanari_types::transaction::Transaction::UpgradeModule { .. }
                    | kanari_types::transaction::Transaction::UpgradePackage { .. }
            )
        });
        let inline_checkpoint_metadata = self.persistent_store.as_ref().is_some_and(|store| {
            !Self::persist_blockchain_json_snapshot_enabled(store)
                && Self::history_retention_checkpoints().is_none()
        });

        // Supply validation, checkpoint metadata, and the durable marker are
        // independent reads of the immutable checkpoint/state. Encoding them in
        // parallel keeps the exact bytes and the marker ordering while hiding
        // the larger serialization behind validation. The same raw changes are
        // still passed to the single atomic commit below.
        let (validation_result, (metadata_result, marker_result)) = rayon::join(
            || {
                if validate_supply {
                    // Single canonical scan: bounds, index consistency, and no-lost-tokens.
                    new_state.validate_no_lost_tokens().context(
                        "Supply invariants failed before checkpoint commit (lost/overcount/index mismatch)",
                    )
                } else {
                    Ok(())
                }
            },
            || {
                rayon::join(
                    || -> Result<CheckpointRawChanges> {
                        let Some(store) = self.persistent_store.as_ref() else {
                            return Ok((Vec::new(), Vec::new()));
                        };
                        if inline_checkpoint_metadata {
                            Self::checkpoint_persistence_raw_changes(store, &checkpoint, true)
                                .context("Failed to encode checkpoint metadata for state commit")
                        } else {
                            Ok((Vec::new(), Vec::new()))
                        }
                    },
                    || -> Result<Vec<u8>> {
                        bcs::to_bytes(&checkpoint)
                            .context("Failed to serialize durable checkpoint commit marker")
                    },
                )
            },
        );
        // Preserve the original error priority: validation, then metadata,
        // then marker. Results are assembled in the original key order.
        validation_result?;
        let (mut raw_updates, raw_deletes) = metadata_result?;
        let marker = marker_result?;
        raw_updates.push((Self::pending_checkpoint_commit_key().to_vec(), marker));
        let validated_at = std::time::Instant::now();

        {
            // Keep the previous live state installed until the staged state and
            // durable recovery marker have committed successfully. A RocksDB
            // failure must not leave RPC/execution observing a new in-memory
            // state paired with old checkpoint metadata.
            let mut live_state = self.state_write();
            let mut committed_state = new_state;
            if self.persistent_store.is_some() {
                committed_state
                    .commit_with_raw_changes_verified_root_and_smt_changes(
                        raw_updates,
                        raw_deletes,
                        &checkpoint.state_root,
                        precomputed_smt_changes,
                    )
                    .context("Failed to commit state and checkpoint marker to RocksDB")?;
            } else {
                committed_state.commit().context("Failed to commit state to RocksDB")?;
            }
            *live_state = committed_state;
        }
        let committed_at = std::time::Instant::now();

        for runtime in &self.runtime_pool {
            runtime.clear_object_cache()?;
        }
        let cache_cleared_at = std::time::Instant::now();

        self.finalize_checkpoint_metadata(checkpoint, inline_checkpoint_metadata)?;
        let metadata_at = std::time::Instant::now();

        if has_module_publish {
            for runtime in &self.runtime_pool {
                runtime
                    .refresh_committed_modules()
                    .context("Failed to refresh Move VM modules after checkpoint commit")?;
            }
        }

        if let Some(store) = &self.persistent_store {
            store
                .delete(Self::pending_checkpoint_commit_key())
                .context("Failed to clear durable checkpoint commit marker")?;
        }
        let durable_marker_at = std::time::Instant::now();

        if profile {
            eprintln!(
                "checkpoint finalize profile: inspect={:.6}s validate={:.6}s commit={:.6}s clear_cache={:.6}s metadata_mempool={:.6}s durable_marker={:.6}s total={:.6}s",
                inspected_at.duration_since(started_at).as_secs_f64(),
                validated_at.duration_since(inspected_at).as_secs_f64(),
                committed_at.duration_since(validated_at).as_secs_f64(),
                cache_cleared_at.duration_since(committed_at).as_secs_f64(),
                metadata_at.duration_since(cache_cleared_at).as_secs_f64(),
                durable_marker_at.duration_since(metadata_at).as_secs_f64(),
                durable_marker_at.duration_since(started_at).as_secs_f64(),
            );
        }

        Ok(())
    }

    fn finalize_checkpoint_metadata(
        &self,
        checkpoint: Checkpoint,
        checkpoint_metadata_already_persisted: bool,
    ) -> Result<()> {
        let profile = Self::env_flag_enabled("KANARI_CHECKPOINT_METADATA_PROFILE");
        let started_at = std::time::Instant::now();
        let tx_count = checkpoint.transactions.len();
        let checkpoint_sequence = checkpoint.sequence;
        // Capture committed hashes before moving the checkpoint into chain
        // storage; cloning 100k signed transactions just to drain the mempool
        // is pure hot-path overhead.
        // Capture canonical hashes once, in checkpoint order. The same vector
        // feeds the mempool drain set and the blockchain index, so (sequence,
        // index) pairs and duplicate detection stay identical while the chain
        // write lock does not re-walk every signed transaction.
        let ordered_tx_hashes =
            crate::blockchain::Blockchain::checkpoint_transaction_hashes(&checkpoint)?;
        // Fixed-size keys avoid one heap allocation per committed transaction;
        // checkpoint hashes are canonical 32-byte digests.
        let committed_hashes: ahash::AHashSet<[u8; 32]> = ordered_tx_hashes
            .iter()
            .map(|hash| committed_hash_key(hash, "transaction hash"))
            .collect();
        // 1. Update blockchain metadata in-memory. Duplicate detection and the
        // vertex index key are prepared before taking the write lock; only
        // sequence/replay checks and insertion stay under the lock.
        let chain_vertex_set_hash =
            crate::blockchain::Blockchain::checkpoint_vertex_set_hash(&checkpoint);
        {
            let mut chain = self.blockchain_write();
            chain.add_checkpoint_with_prevalidated_hashes(
                checkpoint,
                true,
                ordered_tx_hashes,
                chain_vertex_set_hash,
            )?;
        }
        let chain_added_at = std::time::Instant::now();

        // 2. Persist blockchain state before draining the live mempool view.
        if self.persistent_store.is_some() && !checkpoint_metadata_already_persisted {
            let chain = self.blockchain_read();
            if let Err(e) = self.persist_blockchain_snapshot(&chain) {
                drop(chain);
                anyhow::bail!(
                    "Failed to persist blockchain metadata for checkpoint {}: {}. The committed state is protected by a durable recovery marker; restart the node to finish metadata recovery.",
                    checkpoint_sequence,
                    e
                );
            }
        }
        let persisted_at = std::time::Instant::now();

        // 3. Remove committed transactions from pending pool.
        {
            let mut mempool = self.mempool_write();
            // The mempool and checkpoint use different sort keys, so compare as
            // sets instead of relying on zip order. When every pending tx is
            // committed, all derived indexes can be cleared in O(1).
            // `pending_tx_hashes` mirrors `pending_txs`; test the set directly
            // instead of scanning the underlying records. The length guard
            // keeps this safe even if a future bug lets the two views diverge.
            let can_clear_all_pending = mempool.pending_txs.len()
                == mempool.pending_tx_hashes.len()
                && mempool.pending_txs.len() == committed_hashes.len()
                && committed_hashes
                    .iter()
                    .all(|hash| mempool.pending_tx_hashes.contains(hash.as_slice()));
            if can_clear_all_pending {
                // Dropping 100k pending records (each with several owned key
                // strings) is pure cleanup work. Move the whole state out and
                // release it off the checkpoint critical path; the live mempool
                // is already empty from the caller's perspective.
                let drained = std::mem::take(&mut *mempool);
                std::thread::spawn(move || drop(drained));
            } else {
                // Partition once instead of collecting metadata clones and
                // running a second retain. Survivor order and decrement order
                // stay identical to the previous implementation.
                let pending = std::mem::take(&mut mempool.pending_txs);
                let mut kept = Vec::with_capacity(pending.len());
                let mut removed_bytes = 0usize;
                for tx in pending {
                    if !committed_hashes.contains(&committed_hash_key(&tx.tx_hash, "pending hash"))
                    {
                        kept.push(tx);
                        continue;
                    }
                    removed_bytes = removed_bytes.saturating_add(tx.size_bytes);
                    Self::decrement_pending_count(
                        &mut mempool.pending_sender_counts,
                        &tx.normalized_sender,
                    );
                    Self::decrement_pending_count(
                        &mut mempool.pending_primary_access_counts,
                        &tx.primary_access_key,
                    );
                    Self::decrement_pending_count(
                        &mut mempool.pending_congestion_access_counts,
                        &tx.congestion_access_key,
                    );
                    Self::decrement_pending_count(
                        &mut mempool.pending_access_counts,
                        &tx.primary_access_key,
                    );
                    for access_key in &tx.access_keys {
                        Self::decrement_pending_count(
                            &mut mempool.pending_access_counts,
                            access_key,
                        );
                    }
                }
                mempool.pending_txs = kept;
                mempool.pending_bytes = mempool.pending_bytes.saturating_sub(removed_bytes);
                mempool.pending_tx_hashes.retain(|hash| {
                    !committed_hashes.contains(&committed_hash_key(hash, "pending hash"))
                });
            }
        }
        let mempool_drained_at = std::time::Instant::now();

        if profile {
            eprintln!(
                "checkpoint metadata profile: txs={} chain_add={:.6}s persist_snapshot={:.6}s mempool_drain={:.6}s total={:.6}s",
                tx_count,
                chain_added_at.duration_since(started_at).as_secs_f64(),
                persisted_at.duration_since(chain_added_at).as_secs_f64(),
                mempool_drained_at.duration_since(persisted_at).as_secs_f64(),
                mempool_drained_at.duration_since(started_at).as_secs_f64(),
            );
        }

        Ok(())
    }
    /// Applies a previously prepared checkpoint to the blockchain state.
    pub(crate) fn apply_prepared_checkpoint(
        &self,
        checkpoint: Checkpoint,
        verified_state: StateManager,
        smt_changes: Option<PrecomputedSmtChanges>,
        requires_runtime_side_effect_persistence: bool,
        validate_supply: bool,
    ) -> Result<()> {
        if requires_runtime_side_effect_persistence {
            tracing::debug!(
                checkpoint = checkpoint.sequence,
                "Skipping checkpoint side-effect replay: verified_state commit is the canonical source of truth, and replaying persistence against the same backing store can skew object versions during restart/recovery paths."
            );
        }

        self.finalize_checkpoint(checkpoint, verified_state, smt_changes, validate_supply)
    }

    /// Applies a checkpoint end-to-end, preparing state and finalizing it.
    pub fn apply_checkpoint(&self, checkpoint: Checkpoint) -> Result<()> {
        info!(
            "[ENGINE] Applying checkpoint {} with {} txs",
            checkpoint.sequence,
            checkpoint.transactions.len()
        );

        let prepared = self.prepare_checkpoint_state(&checkpoint)?;
        self.ensure_checkpoint_root_matches(&checkpoint, &prepared.state_root)?;

        let requires_runtime_side_effect_persistence =
            Self::requires_runtime_side_effect_persistence(&prepared.transactions);
        self.apply_prepared_checkpoint(
            checkpoint,
            prepared.state,
            prepared.smt_changes,
            requires_runtime_side_effect_persistence,
            true,
        )
    }
}
