// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;

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
        let pending_count = match self.pending_txs.read() {
            Ok(guard) => guard.len(),
            Err(poisoned) => {
                log::error!(
                    "Pending txs lock poisoned in submit_transactions_batch, recovering..."
                );
                poisoned.into_inner().len()
            }
        };

        if pending_count.saturating_add(batch_size) > MAX_MEMPOOL_SIZE {
            log::warn!("[MEMPOOL] Rejecting batch: Queue would exceed max size");
            anyhow::bail!("Mempool is currently full. Please try again later.");
        }

        // Read locks once and clone only what's needed
        let (pending_hashes, pending_by_sender) = {
            let pending_hashes = self
                .pending_tx_hashes
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone();

            // Build sender count map efficiently
            let mut pending_by_sender: std::collections::HashMap<String, u64> =
                std::collections::HashMap::with_capacity(pending_hashes.len() / 10);

            let pending_snapshot = match self.pending_txs.read() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    log::error!(
                        "Pending txs lock poisoned in submit_transactions_batch sequence scan, recovering..."
                    );
                    poisoned.into_inner()
                }
            };

            for pending_tx in pending_snapshot.iter() {
                let sender = Self::normalize_addr(pending_tx.transaction.sender_address());
                *pending_by_sender.entry(sender).or_insert(0) += 1;
            }

            (pending_hashes, pending_by_sender)
        };

        // Parallel signature verification and metadata extraction
        let batch_metadata = signed_txs
            .par_iter()
            .map(
                |signed_tx| -> Result<(Vec<u8>, String, u64, AccountAddress)> {
                    let tx_hash = signed_tx.transaction.hash();
                    if !signed_tx.verify_signature_for_hash(&tx_hash)? {
                        anyhow::bail!("Invalid or missing transaction signature");
                    }

                    Ok((
                        tx_hash,
                        Self::normalize_addr(signed_tx.transaction.sender_address()),
                        signed_tx.transaction.sequence_number(),
                        KanariAddress::parse_to_account_address(
                            signed_tx.transaction.sender_address(),
                        )?,
                    ))
                },
            )
            .collect::<Result<Vec<_>>>()?;

        // Batch read account sequences to minimize state lock contention
        let base_sequences = {
            let state = self.state.read().unwrap_or_else(|e| e.into_inner());
            let mut sequences = std::collections::HashMap::with_capacity(batch_metadata.len());
            for (_, sender, _, sender_addr) in &batch_metadata {
                sequences.entry(sender.clone()).or_insert_with(|| {
                    state
                        .get_account(sender_addr)
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
                std::collections::HashSet::new()
            } else {
                use rayon::prelude::*;
                batch_metadata
                    .par_iter()
                    .filter_map(|(tx_hash, _, _, _)| {
                        if chain.is_transaction_hash_executed(tx_hash) {
                            Some(tx_hash.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<std::collections::HashSet<_>>()
            }
        };

        // Validate sequence numbers and collect accepted hashes
        let mut batch_hashes = std::collections::HashSet::with_capacity(batch_size);
        let mut next_sequence_by_sender = base_sequences;
        for (sender, pending_count) in pending_by_sender {
            *next_sequence_by_sender.entry(sender).or_insert(0) += pending_count;
        }

        let mut accepted_hashes = Vec::with_capacity(batch_size);
        for (tx_hash, sender, tx_seq, _) in &batch_metadata {
            let tx_hash_hex = hex::encode(tx_hash);

            if pending_hashes.contains(tx_hash) || !batch_hashes.insert(tx_hash.clone()) {
                anyhow::bail!("Transaction {} already in pending pool", tx_hash_hex);
            }
            if executed_hashes.contains(tx_hash) {
                anyhow::bail!("Transaction {} already executed", tx_hash_hex);
            }

            let expected_seq = next_sequence_by_sender.entry(sender.clone()).or_insert(0);

            if *tx_seq < *expected_seq {
                anyhow::bail!(
                    "Sequence number too low: expected {}, got {}",
                    expected_seq,
                    tx_seq
                );
            }
            if *tx_seq > *expected_seq {
                anyhow::bail!(
                    "Sequence number too high: expected {}, got {}, sender: {}",
                    expected_seq,
                    tx_seq,
                    sender
                );
            }

            *expected_seq += 1;
            accepted_hashes.push(tx_hash.clone());
        }

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
        self.pending_tx_hashes
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .extend(accepted_hashes.iter().cloned());

        Ok(accepted_hashes)
    }

    pub fn execute_transaction_immediate(
        &self,
        signed_tx: SignedTransaction,
    ) -> Result<(Vec<u8>, ChangeSet)> {
        if !signed_tx.verify_signature()? {
            anyhow::bail!("Invalid transaction signature");
        }

        let tx_hash = signed_tx.transaction.hash();
        let tx = signed_tx.transaction;

        let changeset = {
            let mut state_snapshot = match self.state.read() {
                Ok(guard) => guard.clone(),
                Err(poisoned) => {
                    log::error!(
                        "State lock poisoned in execute_transaction_immediate, recovering..."
                    );
                    poisoned.into_inner().clone()
                }
            };
            let sender_addr = tx.sender_address();
            let addr = KanariAddress::parse_to_account_address(sender_addr)?;

            self.for_each_pending_tx_from_sender(sender_addr, |_| {
                if let Some(mut acct) = state_snapshot.get_account(&addr) {
                    acct.increment_sequence();
                    if let Err(e) = state_snapshot.save_account(&acct) {
                        error!("Failed to save account during sequence update: {}", e);
                    }
                }
            });
            let state_arc = Arc::new(RwLock::new(state_snapshot));
            let runtime = &self.runtime_pool[0];
            self.execute_transaction_with_runtime(&tx, runtime, &state_arc, None)?
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
                    Ok(cs) => Ok((tx.transaction.hash(), cs)),
                    Err(e) => Err(anyhow::anyhow!("Parallel execution failed: {}", e)),
                };

                (tx, final_result)
            })
            .collect()
    }

    pub(crate) fn for_each_pending_tx_from_sender<F>(&self, sender: &str, mut f: F)
    where
        F: FnMut(&SignedTransaction),
    {
        if let Ok(pending) = self.pending_txs.read() {
            let normalized_sender = Self::normalize_addr(sender);
            for ptx in pending.iter() {
                if Self::normalize_addr(ptx.transaction.sender_address()) == normalized_sender {
                    f(ptx);
                }
            }
        }
    }

    pub(crate) fn normalize_addr(addr: &str) -> String {
        use std::str::FromStr;
        KanariAddress::from_str(addr)
            .map(|a| a.to_hex())
            .unwrap_or_else(|_| addr.trim_start_matches("0x").to_lowercase())
    }
}
