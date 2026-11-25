// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::mempool_metadata::VerifiedMempoolTx;
use super::*;
use ahash::AHashSet;
use kanari_crypto::keys::{CurveType, KeyPair};
use kanari_crypto::signatures::zk_authenticator::record_zklogin_admission;
use kanari_types::env::env_flag;

/// Trait for normalizing address strings used across the engine.
pub(crate) trait NormalizeAddr {
    fn normalize_addr(addr: &str) -> String;
}

impl NormalizeAddr for BlockchainEngine {
    fn normalize_addr(addr: &str) -> String {
        kanari_types::address::normalize_address(addr)
    }
}

impl BlockchainEngine {
    /// Decrements the pending count for a given key, removing it if it reaches zero.
    pub(crate) fn decrement_pending_count(counts: &mut ahash::AHashMap<String, u64>, key: &str) {
        let should_remove = if let Some(count) = counts.get_mut(key) {
            *count = count.saturating_sub(1);
            *count == 0
        } else {
            false
        };
        if should_remove {
            counts.remove(key);
        }
    }

    fn pending_record_size_from_parts(
        serialized_signed_tx_bytes: usize,
        metadata: &PendingTransactionMetadata,
    ) -> Result<usize> {
        let effects_bytes =
            metadata.preview_effects.as_ref().map(bcs::serialized_size).transpose()?.unwrap_or(0);
        serialized_signed_tx_bytes
            .checked_add(effects_bytes)
            .and_then(|size| size.checked_add(128))
            .context("Pending transaction memory accounting overflow")
    }

    /// Submits a batch of signed transactions with metadata to the mempool.
    pub fn submit_transactions_batch_with_metadata(
        &self,
        signed_txs: Vec<SignedTransaction>,
        metadata: PendingTransactionMetadata,
    ) -> Result<Vec<Vec<u8>>> {
        self.submit_transactions_batch_internal(signed_txs, metadata)
    }

    /// Submits a batch of signed transactions to the mempool with default metadata.
    pub fn submit_transactions_batch(
        &self,
        signed_txs: Vec<SignedTransaction>,
    ) -> Result<Vec<Vec<u8>>> {
        self.submit_transactions_batch_internal(signed_txs, PendingTransactionMetadata::default())
    }

    fn submit_transactions_batch_internal(
        &self,
        signed_txs: Vec<SignedTransaction>,
        metadata: PendingTransactionMetadata,
    ) -> Result<Vec<Vec<u8>>> {
        if signed_txs.is_empty() {
            return Ok(Vec::new());
        }
        let profile = env_flag("KANARI_MEMPOOL_ADMISSION_PROFILE");
        let started_at = std::time::Instant::now();

        if self.is_far_behind_observed_network() {
            let local = self.get_stats().height;
            let observed = self.observed_network_height();
            anyhow::bail!(
                "Node is catching up with network (local_height={}, observed_height={}); retry after sync",
                local,
                observed
            );
        }

        // Early size check to avoid unnecessary work
        let batch_size = signed_txs.len();
        let pending_len = self.mempool_read().pending_txs.len();

        if pending_len.saturating_add(batch_size) > MAX_MEMPOOL_SIZE {
            log::warn!("[MEMPOOL] Rejecting batch: Queue would exceed max size");
            anyhow::bail!("Mempool is currently full. Please try again later.");
        }

        let mut sender_cache = ahash::AHashMap::with_capacity(batch_size);
        for signed_tx in &signed_txs {
            let sender = signed_tx.transaction.sender_address();
            sender_cache.entry(sender.to_string()).or_insert_with(|| Self::normalize_addr(sender));
        }

        // Hash, verify, and extract metadata in one parallel pass.
        let mut verified_txs: Vec<VerifiedMempoolTx> = signed_txs
            .into_par_iter()
            .map(|signed_tx| -> Result<VerifiedMempoolTx> {
                // Serialize once and reuse the bytes for signature verification,
                // shape validation, and memory accounting. All three previously
                // re-encoded the same transaction independently.
                let serialized_transaction = bcs::to_bytes(&signed_tx.transaction)?;
                let serialized_signed_len = serialized_transaction
                    .len()
                    .checked_add(bcs::serialized_size(&signed_tx.signature)?)
                    .context("Signed transaction size overflow")?;
                let verified =
                    signed_tx.into_verified_from_serialized_transaction(&serialized_transaction)?;
                let tx_hash = verified.hash().to_vec();
                let tx = verified.transaction();
                // Shape-validate first so an invalid tx never lands in the
                // zkLogin replay guard. record_zklogin_admission is deferred
                // to the acceptance loop below so a tx that passes shape
                // validation but later fails lane/nonce checks is not locked
                // out of resubmission.
                self.validate_transaction_admission_shape_with_size(
                    tx,
                    serialized_transaction.len(),
                )?;
                let sender = tx.sender_address();
                let normalized_sender = sender_cache
                    .get(sender)
                    .cloned()
                    .unwrap_or_else(|| {
                        log::warn!(
                            "[MEMPOOL] Sender normalization cache miss for verified transaction; normalizing inline"
                        );
                        Self::normalize_addr(sender)
                    });
                let sender_address = sender.to_string();
                let nonce = tx.nonce();
                let primary_access_key = tx.primary_access_key();
                let congestion_access_key = Self::congestion_access_key(tx);
                let access_keys = tx.object_access_keys();
                let conflict_keys = tx.get_conflict_keys();
                let signed_tx = verified.into_signed_transaction();
                let size_bytes = Self::pending_record_size_from_parts(
                    serialized_signed_len,
                    &metadata,
                )?;
                Ok(VerifiedMempoolTx {
                    signed_tx,
                    tx_hash,
                    normalized_sender,
                    sender_address,
                    nonce,
                    primary_access_key,
                    congestion_access_key,
                    access_keys,
                    conflict_keys,
                    size_bytes,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let verified_at = std::time::Instant::now();
        let batch_bytes = verified_txs.iter().try_fold(0usize, |total, item| {
            total.checked_add(item.size_bytes).context("Mempool batch byte size overflow")
        })?;

        verified_txs.sort_by(|a, b| {
            a.primary_access_key
                .cmp(&b.primary_access_key)
                .then_with(|| a.normalized_sender.cmp(&b.normalized_sender))
                .then_with(|| a.nonce.cmp(&b.nonce))
                .then_with(|| a.tx_hash.cmp(&b.tx_hash))
        });
        let sorted_at = std::time::Instant::now();

        // Check executed transactions in parallel. Persistent index corruption
        // must fail admission closed instead of making an old transaction look
        // uncommitted. An empty replay index and no persistent store is a safe
        // fast path: no hash can possibly be committed yet.
        let skip_committed_check =
            self.persistent_store.is_none() && !self.blockchain_read().has_executed_transactions();
        let executed_hashes = if skip_committed_check {
            AHashSet::new()
        } else {
            use rayon::prelude::*;
            verified_txs
                .par_iter()
                .map(|item| -> Result<Option<Vec<u8>>> {
                    Ok(self
                        .try_is_transaction_committed(&item.tx_hash)?
                        .then(|| item.tx_hash.clone()))
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect::<AHashSet<_>>()
        };

        let checked_at = std::time::Instant::now();

        // Account sequence is intentionally not a consensus admission rule. Object refs,
        // gas refs, and duplicate transaction hashes are the canonical replay/double-spend guards.
        let mut mempool = self.mempool_write();
        if mempool.pending_txs.len().saturating_add(batch_size) > MAX_MEMPOOL_SIZE {
            anyhow::bail!("Mempool is currently full. Please try again later.");
        }
        if mempool.pending_bytes.saturating_add(batch_bytes) > MAX_MEMPOOL_BYTES {
            anyhow::bail!("Mempool byte budget is exhausted. Please try again later.");
        }
        let mut batch_hashes = AHashSet::with_capacity(batch_size);
        let mut accepted_hashes = Vec::with_capacity(batch_size);
        let mut accepted_counts_by_sender = ahash::AHashMap::new();
        let mut accepted_counts_by_access = ahash::AHashMap::new();
        let mut accepted_counts_by_primary_access = ahash::AHashMap::new();
        let mut accepted_counts_by_congestion_access = ahash::AHashMap::new();

        // Per-sender committed nonce watermark (persistent nodes). Any tx whose
        // nonce is not strictly greater than the sender's highest committed nonce
        // is a replay of the same or an earlier sequence, regardless of payload.
        //
        // Cap at JSON-safe range so legacy huge random nonces don't permanently
        // lock senders out.
        let sender_watermark = self
            .persistent_store
            .as_ref()
            .map(|store| super::BlockchainEngine::load_sender_nonce_watermark(store))
            .transpose()?
            .unwrap_or_default()
            .into_iter()
            .map(|(k, v)| (k, kanari_types::transaction::sanitize_nonce_watermark(v).unwrap_or(0)))
            .collect::<std::collections::BTreeMap<_, _>>();

        for item in &verified_txs {
            let tx_hash = &item.tx_hash;
            let sender = &item.normalized_sender;
            let nonce = item.nonce;
            let primary_access = &item.primary_access_key;
            let congestion_access = &item.congestion_access_key;

            if mempool.pending_tx_hashes.contains(tx_hash) || !batch_hashes.insert(tx_hash.clone())
            {
                let tx_hash_hex = hex::encode(tx_hash);
                anyhow::bail!("Transaction {} already in pending pool", tx_hash_hex);
            }
            if executed_hashes.contains(tx_hash) {
                let tx_hash_hex = hex::encode(tx_hash);
                anyhow::bail!("Transaction {} already executed", tx_hash_hex);
            }
            if let Some(max_committed) = sender_watermark.get(sender)
                && *max_committed >= nonce
            {
                anyhow::bail!(
                    "Transaction nonce {} for sender {} is stale: max committed nonce for sender is {}",
                    nonce,
                    sender,
                    max_committed
                );
            }

            let current_sender_depth = mempool
                .pending_sender_counts
                .get(sender)
                .copied()
                .unwrap_or(0)
                .saturating_add(accepted_counts_by_sender.get(sender).copied().unwrap_or(0));
            if current_sender_depth >= MAX_PENDING_PER_SENDER {
                anyhow::bail!(
                    "Sender {} has too many pending transactions: {} pending, max {} (reputation-adjusted)",
                    sender,
                    current_sender_depth,
                    MAX_PENDING_PER_SENDER
                );
            }

            let current_lane_depth = mempool
                .pending_primary_access_counts
                .get(primary_access)
                .copied()
                .unwrap_or(0)
                .saturating_add(
                    accepted_counts_by_primary_access.get(primary_access).copied().unwrap_or(0),
                );
            if current_lane_depth >= MAX_PENDING_PER_PRIMARY_ACCESS_LANE {
                anyhow::bail!(
                    "Transaction lane {} is saturated: {} pending transaction(s) already target this primary access key, max {}",
                    primary_access,
                    current_lane_depth,
                    MAX_PENDING_PER_PRIMARY_ACCESS_LANE
                );
            }
            let current_congestion_depth = mempool
                .pending_congestion_access_counts
                .get(congestion_access)
                .copied()
                .unwrap_or(0)
                .saturating_add(
                    accepted_counts_by_congestion_access
                        .get(congestion_access)
                        .copied()
                        .unwrap_or(0),
                );
            if current_congestion_depth >= MAX_PENDING_PER_PRIMARY_ACCESS_LANE {
                anyhow::bail!(
                    "Transaction congestion lane {} is saturated: {} pending transaction(s) already touch this canonical access key, max {}",
                    congestion_access,
                    current_congestion_depth,
                    MAX_PENDING_PER_PRIMARY_ACCESS_LANE
                );
            }

            // zkLogin: record admission only after all checks (nonce, lane,
            // congestion) have passed so a valid tx rejected for capacity
            // reasons is not locked out of resubmission.
            if let Some((CurveType::ZkLogin, _)) = KeyPair::parse_tagged_address(sender)
                && !record_zklogin_admission(tx_hash)
            {
                anyhow::bail!(
                    "Signature verification failed: zkLogin replay (transaction already admitted within the replay TTL)"
                );
            }
            accepted_hashes.push(tx_hash.clone());
            *accepted_counts_by_sender.entry(sender.clone()).or_insert(0) += 1;
            *accepted_counts_by_primary_access.entry(primary_access.clone()).or_insert(0) += 1;
            *accepted_counts_by_congestion_access.entry(congestion_access.clone()).or_insert(0) +=
                1;
            *accepted_counts_by_access.entry(primary_access.clone()).or_insert(0) += 1;
            for access_key in &item.access_keys {
                *accepted_counts_by_access.entry(access_key.clone()).or_insert(0) += 1;
            }
        }

        let accepted_at = std::time::Instant::now();

        // Commit admission and all derived indexes under the same lock used for checks.
        mempool.pending_txs.extend(
            verified_txs.into_iter().map(|item| item.into_pending_record(metadata.clone())),
        );
        mempool.pending_tx_hashes.extend(accepted_hashes.iter().cloned());
        mempool.pending_bytes = mempool.pending_bytes.saturating_add(batch_bytes);
        for (sender, count) in &accepted_counts_by_sender {
            *mempool.pending_sender_counts.entry(sender.clone()).or_insert(0) += *count;
        }
        for (primary_access, count) in &accepted_counts_by_primary_access {
            *mempool.pending_primary_access_counts.entry(primary_access.clone()).or_insert(0) +=
                *count;
        }
        for (congestion_access, count) in &accepted_counts_by_congestion_access {
            *mempool
                .pending_congestion_access_counts
                .entry(congestion_access.clone())
                .or_insert(0) += *count;
        }
        for (access_key, count) in &accepted_counts_by_access {
            *mempool.pending_access_counts.entry(access_key.clone()).or_insert(0) += *count;
        }
        if profile {
            let committed_at = std::time::Instant::now();
            eprintln!(
                "mempool admission profile: txs={} verify={:.6}s sort={:.6}s committed_check={:.6}s accept={:.6}s commit={:.6}s total={:.6}s",
                batch_size,
                verified_at.duration_since(started_at).as_secs_f64(),
                sorted_at.duration_since(verified_at).as_secs_f64(),
                checked_at.duration_since(sorted_at).as_secs_f64(),
                accepted_at.duration_since(checked_at).as_secs_f64(),
                committed_at.duration_since(accepted_at).as_secs_f64(),
                committed_at.duration_since(started_at).as_secs_f64(),
            );
        }
        Ok(accepted_hashes)
    }

    /// Executes a single transaction immediately, returning the transaction hash and changeset.
    pub fn execute_transaction_immediate(
        &self,
        signed_tx: SignedTransaction,
    ) -> Result<(Vec<u8>, ChangeSet)> {
        let verified = signed_tx.into_verified()?;
        let tx_hash = verified.hash().to_vec();
        let tx = verified.into_signed_transaction().transaction;
        self.validate_transaction_admission_shape(&tx)?;

        let changeset = {
            let state_snapshot = self.state_read().clone();
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

    /// Returns the number of pending transactions for a given sender address.
    pub(crate) fn pending_tx_count_for_sender(&self, sender: &str) -> u64 {
        let normalized_sender = Self::normalize_addr(sender);
        self.mempool_read().pending_sender_counts.get(&normalized_sender).copied().unwrap_or(0)
    }

    /// Returns the number of pending transactions targeting a given primary access key.
    #[cfg(test)]
    pub(crate) fn pending_tx_count_for_primary_access(&self, key: &str) -> u64 {
        self.mempool_read().pending_primary_access_counts.get(key).copied().unwrap_or(0)
    }

    /// Returns the number of pending transactions touching a given congestion access key.
    #[cfg(test)]
    pub(crate) fn pending_tx_count_for_congestion_access(&self, key: &str) -> u64 {
        self.mempool_read().pending_congestion_access_counts.get(key).copied().unwrap_or(0)
    }

    /// Returns a snapshot of all access keys currently referenced by pending transactions.
    pub fn pending_access_keys_snapshot(&self) -> std::collections::HashSet<String> {
        self.mempool_read().pending_access_counts.keys().cloned().collect()
    }

    /// Decrements pending sender counts for the given transactions.
    pub(crate) fn remove_pending_sender_counts(
        counts: &mut ahash::AHashMap<String, u64>,
        transactions: &[PendingTransactionRecord],
    ) {
        if transactions.is_empty() {
            return;
        }

        for tx in transactions {
            Self::decrement_pending_count(counts, &tx.normalized_sender);
        }
    }

    /// Decrements pending access counts for the given transactions.
    pub(crate) fn remove_pending_access_counts(
        counts: &mut ahash::AHashMap<String, u64>,
        transactions: &[PendingTransactionRecord],
    ) {
        if transactions.is_empty() {
            return;
        }

        for tx in transactions {
            Self::decrement_pending_count(counts, &tx.primary_access_key);
            for access_key in &tx.access_keys {
                Self::decrement_pending_count(counts, access_key);
            }
        }
    }

    /// Decrements pending primary access lane counts for the given transactions.
    pub(crate) fn remove_pending_primary_access_counts(
        counts: &mut ahash::AHashMap<String, u64>,
        transactions: &[PendingTransactionRecord],
    ) {
        if transactions.is_empty() {
            return;
        }

        for tx in transactions {
            Self::decrement_pending_count(counts, &tx.primary_access_key);
        }
    }

    /// Decrements pending congestion access lane counts for the given transactions.
    pub(crate) fn remove_pending_congestion_access_counts(
        counts: &mut ahash::AHashMap<String, u64>,
        transactions: &[PendingTransactionRecord],
    ) {
        if transactions.is_empty() {
            return;
        }

        for tx in transactions {
            Self::decrement_pending_count(counts, &tx.congestion_access_key);
        }
    }

    /// Removes pending transactions matching the given hashes and returns them.
    pub fn remove_pending_transactions_by_hashes(
        &self,
        hashes: &[Vec<u8>],
    ) -> Vec<PendingTransactionRecord> {
        if hashes.is_empty() {
            return Vec::new();
        }

        let target_hashes = hashes.iter().cloned().collect::<AHashSet<_>>();
        let mut mempool = self.mempool_write();
        let removed_transactions = mempool
            .pending_txs
            .iter()
            .filter(|tx| target_hashes.contains(&tx.tx_hash))
            .cloned()
            .collect::<Vec<_>>();

        if removed_transactions.is_empty() {
            return Vec::new();
        }

        mempool.pending_txs.retain(|tx| !target_hashes.contains(&tx.tx_hash));
        let removed_bytes = removed_transactions
            .iter()
            .fold(0usize, |total, record| total.saturating_add(record.size_bytes));
        mempool.pending_bytes = mempool.pending_bytes.saturating_sub(removed_bytes);
        mempool.pending_tx_hashes.retain(|hash| !target_hashes.contains(hash));
        Self::remove_pending_sender_counts(
            &mut mempool.pending_sender_counts,
            &removed_transactions,
        );
        Self::remove_pending_primary_access_counts(
            &mut mempool.pending_primary_access_counts,
            &removed_transactions,
        );
        Self::remove_pending_congestion_access_counts(
            &mut mempool.pending_congestion_access_counts,
            &removed_transactions,
        );
        Self::remove_pending_access_counts(
            &mut mempool.pending_access_counts,
            &removed_transactions,
        );
        self.record_invalid_pending_drop(removed_transactions.len());
        removed_transactions
    }
}
