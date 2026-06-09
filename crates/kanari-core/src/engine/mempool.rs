// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use ahash::AHashSet;

impl BlockchainEngine {
    pub fn submit_transactions_batch(
        &self,
        signed_txs: Vec<SignedTransaction>,
    ) -> Result<Vec<Vec<u8>>> {
        if signed_txs.is_empty() {
            return Ok(Vec::new());
        }

        // Early size check to avoid unnecessary work
        let batch_size = signed_txs.len();
        let (pending_hashes, pending_by_sender) = {
            let pending_hashes = self
                .pending_tx_hashes
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            let pending_by_sender = self
                .pending_sender_counts
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone();

            (pending_hashes, pending_by_sender)
        };

        if pending_hashes.len().saturating_add(batch_size) > MAX_MEMPOOL_SIZE {
            log::warn!("[MEMPOOL] Rejecting batch: Queue would exceed max size");
            anyhow::bail!("Mempool is currently full. Please try again later.");
        }

        let mut sender_cache = ahash::AHashMap::with_capacity(batch_size);
        for signed_tx in &signed_txs {
            let sender = signed_tx.transaction.sender_address();
            sender_cache
                .entry(sender.to_string())
                .or_insert_with(|| Self::normalize_addr(sender));
        }

        // Hash, verify, and extract metadata in one parallel pass.
        let batch_metadata = signed_txs
            .par_iter()
            .map(|signed_tx| -> Result<(Vec<u8>, String, u64)> {
                let tx_hash = signed_tx.verified_transaction_hash()?;
                let sender = signed_tx.transaction.sender_address();
                Ok((
                    tx_hash,
                    sender_cache
                        .get(sender)
                        .expect("sender cache must contain every batch sender")
                        .clone(),
                    signed_tx.transaction.sequence_number(),
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        // Batch read account sequences to minimize state lock contention
        let base_sequences = {
            let state = self.state_read();
            let mut sequences = std::collections::HashMap::with_capacity(batch_metadata.len());
            for (_, sender, _) in &batch_metadata {
                sequences.entry(sender.clone()).or_insert_with(|| {
                    KanariAddress::parse_to_account_address(sender)
                        .ok()
                        .and_then(|sender_addr| state.get_account(&sender_addr))
                        .map(|acc| acc.sequence_number)
                        .unwrap_or(0)
                });
            }
            sequences
        };

        // Check executed transactions in parallel
        let executed_hashes = {
            let chain = match self.blockchain.read() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    log::error!(
                        "Blockchain lock poisoned in submit_transactions_batch, recovering..."
                    );
                    poisoned.into_inner()
                }
            };

            if !chain.has_executed_transactions() {
                AHashSet::new()
            } else {
                use rayon::prelude::*;
                batch_metadata
                    .par_iter()
                    .filter_map(|(tx_hash, _, _)| {
                        if chain.is_transaction_hash_executed(tx_hash) {
                            Some(tx_hash.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .collect::<AHashSet<_>>()
            }
        };

        // Validate duplicates globally, then validate sequence numbers per sender in parallel.
        let mut batch_hashes = AHashSet::with_capacity(batch_size);
        let mut accepted_hashes = Vec::with_capacity(batch_size);
        let mut accepted_counts_by_sender = ahash::AHashMap::new();
        let mut sequence_groups = ahash::AHashMap::new();
        for (tx_hash, sender, tx_seq) in &batch_metadata {
            if pending_hashes.contains(tx_hash) || !batch_hashes.insert(tx_hash.clone()) {
                let tx_hash_hex = hex::encode(tx_hash);
                anyhow::bail!("Transaction {} already in pending pool", tx_hash_hex);
            }
            if executed_hashes.contains(tx_hash) {
                let tx_hash_hex = hex::encode(tx_hash);
                anyhow::bail!("Transaction {} already executed", tx_hash_hex);
            }

            accepted_hashes.push(tx_hash.clone());
            *accepted_counts_by_sender.entry(sender.clone()).or_insert(0) += 1;
            sequence_groups
                .entry(sender.clone())
                .or_insert_with(Vec::new)
                .push(*tx_seq);
        }

        let sequence_groups = sequence_groups
            .into_iter()
            .map(|(sender, tx_sequences)| {
                let expected_start = base_sequences.get(&sender).copied().unwrap_or(0)
                    + pending_by_sender.get(&sender).copied().unwrap_or(0);
                (sender, expected_start, tx_sequences)
            })
            .collect::<Vec<_>>();

        sequence_groups.par_iter().try_for_each(
            |(sender, expected_start, tx_sequences)| -> Result<()> {
                for (expected_seq, tx_seq) in (*expected_start..).zip(tx_sequences.iter().copied())
                {
                    if tx_seq < expected_seq {
                        anyhow::bail!(
                            "Sequence number too low: expected {}, got {}",
                            expected_seq,
                            tx_seq
                        );
                    }
                    if tx_seq > expected_seq {
                        anyhow::bail!(
                            "Sequence number too high: expected {}, got {}, sender: {}",
                            expected_seq,
                            tx_seq,
                            sender
                        );
                    }
                }
                Ok(())
            },
        )?;

        // Write to mempool with minimal lock duration
        {
            let mut pending = match self.pending_txs.write() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    log::error!(
                        "Pending txs lock poisoned in submit_transactions_batch write, recovering..."
                    );
                    poisoned.into_inner()
                }
            };

            if pending.len().saturating_add(batch_size) > MAX_MEMPOOL_SIZE {
                anyhow::bail!("Mempool is currently full. Please try again later.");
            }

            pending.extend(signed_txs);
        }

        // Update hash set separately to reduce lock contention
        if !accepted_hashes.is_empty() {
            self.pending_tx_hashes
                .write()
                .unwrap_or_else(|e| e.into_inner())
                .extend(accepted_hashes.iter().cloned());
            self.add_pending_sender_counts(&accepted_counts_by_sender);
        }

        Ok(accepted_hashes)
    }

    pub fn execute_transaction_immediate(
        &self,
        signed_tx: SignedTransaction,
    ) -> Result<(Vec<u8>, ChangeSet)> {
        if !signed_tx.verify_signature()? {
            anyhow::bail!("Invalid transaction signature");
        }

        let tx_hash = signed_tx.transaction_hash().to_vec();
        let tx = signed_tx.transaction;

        let changeset = {
            let mut state_snapshot = self.state_read().clone();
            let sender_addr = tx.sender_address();
            let addr = KanariAddress::parse_to_account_address(sender_addr)?;

            for _ in 0..self.pending_tx_count_for_sender(sender_addr) {
                if let Some(mut acct) = state_snapshot.get_account(&addr) {
                    acct.increment_sequence();
                    if let Err(e) = state_snapshot.save_account(&acct) {
                        error!("Failed to save account during sequence update: {}", e);
                    }
                }
            }
            let state_arc = Arc::new(RwLock::new(state_snapshot));
            let runtime = self.runtime_pool[0]
                .spawn_isolated_worker()
                .context("Failed to create isolated runtime for immediate execution")?;
            let changeset =
                self.execute_transaction_with_runtime(&tx, &runtime, &state_arc, None)?;
            runtime.clear_object_cache()?;
            changeset
        };

        Ok((tx_hash, changeset))
    }

    pub fn execute_transactions_parallel(
        &self,
        txs: Vec<SignedTransaction>,
    ) -> Vec<ParallelTxResult> {
        log::info!(
            "[PARALLEL ENGINE] Firing up Rayon to execute {} txs concurrently!",
            txs.len()
        );

        let state_arc = &self.state;

        txs.into_par_iter()
            .map(|tx| {
                let thread_idx = rayon::current_thread_index().unwrap_or(0);
                let runtime = &self.runtime_pool[thread_idx % self.runtime_pool.len()];

                let result = self.execute_transaction_with_runtime_internal(
                    &tx.transaction,
                    runtime,
                    state_arc,
                    true,
                    None,
                    false,
                );

                let final_result = match result {
                    Ok(cs) => Ok((tx.transaction_hash().to_vec(), cs)),
                    Err(e) => Err(anyhow::anyhow!("Parallel execution failed: {}", e)),
                };

                (tx, final_result)
            })
            .collect()
    }

    pub(crate) fn pending_tx_count_for_sender(&self, sender: &str) -> u64 {
        let normalized_sender = Self::normalize_addr(sender);
        self.pending_sender_counts
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&normalized_sender)
            .copied()
            .unwrap_or(0)
    }

    pub(crate) fn add_pending_sender_counts(&self, accepted_counts: &ahash::AHashMap<String, u64>) {
        let mut counts = self
            .pending_sender_counts
            .write()
            .unwrap_or_else(|e| e.into_inner());
        for (sender, count) in accepted_counts {
            *counts.entry(sender.clone()).or_insert(0) += *count;
        }
    }

    pub(crate) fn remove_pending_sender_counts(&self, transactions: &[SignedTransaction]) {
        if transactions.is_empty() {
            return;
        }

        let mut counts = self
            .pending_sender_counts
            .write()
            .unwrap_or_else(|e| e.into_inner());
        for tx in transactions {
            let sender = Self::normalize_addr(tx.transaction.sender_address());
            let should_remove = if let Some(count) = counts.get_mut(&sender) {
                *count = count.saturating_sub(1);
                *count == 0
            } else {
                false
            };
            if should_remove {
                counts.remove(&sender);
            }
        }
    }

    pub(crate) fn clear_pending_sender_counts(&self) {
        self.pending_sender_counts
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    pub(crate) fn normalize_addr(addr: &str) -> String {
        use std::str::FromStr;
        KanariAddress::from_str(addr)
            .map(|a| a.to_hex())
            .unwrap_or_else(|_| addr.trim_start_matches("0x").to_lowercase())
    }
}
