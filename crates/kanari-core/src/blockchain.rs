// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! In-memory blockchain state with bounded checkpoint retention.
//!
//! The [`Blockchain`] struct maintains the ordered chain of DAG checkpoints,
//! a lifetime transaction hash index for replay protection, and a bounded
//! working set of recent checkpoints (capped at 1000). Older checkpoints
//! are evicted from the deque but their transaction count is preserved
//! for monotonicity.

use crate::consensus::Checkpoint;
use anyhow::Result;
use kanari_types::error::KanariUnwrapExt;
use kanari_types::transaction::{SignedTransaction, TransactionEffects};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

const MAX_RETAINED_BLOCKS: usize = 1000;
const MAX_RETAINED_TX_HASHES: usize = 2_000_000;

mod serde_vecdeque {
    use super::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S, T>(data: &VecDeque<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        data.iter().collect::<Vec<_>>().serialize(serializer)
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<VecDeque<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        let vec: Vec<T> = Vec::deserialize(deserializer)?;
        Ok(vec.into_iter().collect())
    }
}

/// In-memory blockchain state with bounded checkpoint retention.
///
/// Maintains the ordered chain of DAG checkpoints, a lifetime transaction
/// hash index for replay protection, and a bounded working set of recent
/// checkpoints (capped at [`MAX_RETAINED_BLOCKS`]). Older checkpoints are
/// evicted from the deque but their transaction count is preserved in
/// `total_transaction_count` for monotonicity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    #[serde(default = "default_dag_checkpoints", with = "serde_vecdeque")]
    pub dag_checkpoints: VecDeque<Checkpoint>,
    #[serde(skip)]
    executed_tx_hashes: HashSet<Vec<u8>>,
    #[serde(skip)]
    tx_hash_queue: VecDeque<Vec<u8>>,
    #[serde(skip)]
    tx_location_index: HashMap<Vec<u8>, (u64, usize)>,
    #[serde(default)]
    total_transaction_count: usize,
}

fn default_dag_checkpoints() -> VecDeque<Checkpoint> {
    let mut checkpoints = VecDeque::new();
    checkpoints.push_back(Checkpoint::genesis());
    checkpoints
}

impl Blockchain {
    /// Creates a new blockchain pre-loaded with the genesis checkpoint.
    pub fn new() -> Self {
        Self {
            dag_checkpoints: vec![Checkpoint::genesis()].into(),
            executed_tx_hashes: HashSet::new(),
            tx_hash_queue: VecDeque::new(),
            tx_location_index: HashMap::new(),
            total_transaction_count: 0,
        }
    }

    /// Returns a reference to the most recently committed checkpoint.
    pub fn latest_checkpoint(&self) -> &Checkpoint {
        self.dag_checkpoints
            .back()
            .invariant("blockchain must contain at least the genesis checkpoint")
    }

    /// Returns the sequence number of the latest checkpoint (chain height).
    pub fn height(&self) -> u64 {
        self.latest_checkpoint().sequence
    }

    /// Returns `true` if the given transaction hash has already been executed
    /// and is present in the replay-protection index.
    pub fn is_transaction_hash_executed(&self, tx_hash: &[u8]) -> bool {
        self.executed_tx_hashes.contains(tx_hash)
    }

    /// Restore the deterministic genesis checkpoint in snapshots written by
    /// older retention logic that evicted sequence zero.
    pub(crate) fn ensure_genesis_retained(&mut self) -> bool {
        if self
            .dag_checkpoints
            .front()
            .is_some_and(|checkpoint| checkpoint.sequence == 0)
        {
            return false;
        }

        self.dag_checkpoints.push_front(Checkpoint::genesis());
        while self.dag_checkpoints.len() > MAX_RETAINED_BLOCKS {
            self.dag_checkpoints.remove(1);
        }
        true
    }

    fn retained_transaction_count(&self) -> usize {
        self.dag_checkpoints
            .iter()
            .map(|checkpoint| checkpoint.transactions.len())
            .sum()
    }

    /// Rebuilds the in-memory transaction hash index from the retained
    /// checkpoints. Must be called after deserializing a persisted blockchain
    /// whose index fields were skipped during serde.
    pub fn rebuild_tx_hash_index(&mut self) {
        // `total_transaction_count` is persisted independently from the
        // bounded checkpoint window. Rebuilding the query indexes after a
        // restart must not make the lifetime transaction count fall when old
        // checkpoints have already been evicted.
        let persisted_total = self.total_transaction_count;
        self.executed_tx_hashes.clear();
        self.tx_hash_queue.clear();
        self.tx_location_index.clear();
        self.total_transaction_count = 0;

        for checkpoint in &self.dag_checkpoints {
            for (index, tx) in checkpoint.transactions.iter().enumerate() {
                let hash = tx.transaction_hash().to_vec();
                if self.executed_tx_hashes.insert(hash.clone()) {
                    self.tx_hash_queue.push_back(hash.clone());
                    self.total_transaction_count = self.total_transaction_count.saturating_add(1);
                }
                self.tx_location_index
                    .insert(hash, (checkpoint.sequence, index));

                while self.tx_hash_queue.len() > MAX_RETAINED_TX_HASHES {
                    if let Some(old_hash) = self.tx_hash_queue.pop_front() {
                        self.executed_tx_hashes.remove(&old_hash);
                        self.tx_location_index.remove(&old_hash);
                    }
                }
            }
        }
        self.total_transaction_count = self.total_transaction_count.max(persisted_total);
    }

    /// Appends a new checkpoint to the chain.
    ///
    /// When `validate` is true, enforces consecutive sequencing, correct
    /// previous-hash linkage, and absence of duplicate or replayed transactions.
    /// Old checkpoints beyond [`MAX_RETAINED_BLOCKS`] are evicted (genesis is
    /// always retained).
    pub fn add_checkpoint_with_validation(
        &mut self,
        checkpoint: Checkpoint,
        validate: bool,
    ) -> Result<()> {
        if validate {
            let expected_seq = self.latest_checkpoint().sequence + 1;
            if checkpoint.sequence != expected_seq {
                anyhow::bail!(
                    "Invalid checkpoint sequence: expected {}, got {}",
                    expected_seq,
                    checkpoint.sequence
                );
            }
            if checkpoint.prev_checkpoint_hash != self.latest_checkpoint().hash()? {
                anyhow::bail!("Invalid previous checkpoint hash");
            }

            let mut seen_txs = HashSet::new();
            for tx in checkpoint.transactions.iter() {
                let tx_hash = tx.transaction_hash().to_vec();
                if !seen_txs.insert(tx_hash.clone()) {
                    anyhow::bail!("Duplicate transaction found within checkpoint");
                }
                if self.executed_tx_hashes.contains(&tx_hash) {
                    anyhow::bail!("Replay attack detected: Transaction already executed");
                }
            }
        }

        self.track_checkpoint_transactions(&checkpoint);
        self.dag_checkpoints.push_back(checkpoint);

        if self.dag_checkpoints.len() > MAX_RETAINED_BLOCKS
            // Sequence zero anchors the network genesis manifest and must
            // survive bounded history retention. Evict the oldest non-genesis
            // checkpoint instead.
            && let Some(evicted) = self.dag_checkpoints.remove(1)
        {
            for tx in evicted.transactions.iter() {
                let hash = tx.transaction_hash().to_vec();
                self.tx_location_index.remove(&hash);
            }
        }
        Ok(())
    }

    fn track_checkpoint_transactions(&mut self, checkpoint: &Checkpoint) {
        for (index, tx) in checkpoint.transactions.iter().enumerate() {
            let hash = tx.transaction_hash().to_vec();
            if self.executed_tx_hashes.insert(hash.clone()) {
                self.tx_hash_queue.push_back(hash.clone());
                self.total_transaction_count = self.total_transaction_count.saturating_add(1);
            }
            self.tx_location_index
                .insert(hash, (checkpoint.sequence, index));
        }

        while self.tx_hash_queue.len() > MAX_RETAINED_TX_HASHES {
            if let Some(old_hash) = self.tx_hash_queue.pop_front() {
                self.executed_tx_hashes.remove(&old_hash);
                self.tx_location_index.remove(&old_hash);
            }
        }
    }

    /// Looks up a checkpoint by its sequence number.
    ///
    /// Returns `None` if the checkpoint has been evicted from the retained
    /// window or does not exist.
    pub fn get_checkpoint(&self, sequence: u64) -> Option<&Checkpoint> {
        self.dag_checkpoints
            .iter()
            .find(|checkpoint| checkpoint.sequence == sequence)
    }

    /// Returns the lifetime transaction count (monotonically increasing).
    pub fn get_transaction_count(&self) -> usize {
        self.total_transaction_count
            .max(self.tx_location_index.len())
            .max(self.retained_transaction_count())
    }

    /// Resolves a transaction hash to its signed transaction, checkpoint
    /// sequence, state root, and optional effects.
    ///
    /// First checks the fast `tx_location_index`, then falls back to a full
    /// reverse scan of retained checkpoints.
    pub fn get_transaction_location_with_effect(
        &self,
        tx_hash: &[u8],
    ) -> Option<(&SignedTransaction, u64, &[u8], Option<&TransactionEffects>)> {
        if let Some((sequence, index)) = self.tx_location_index.get(tx_hash)
            && let Some(checkpoint) = self.get_checkpoint(*sequence)
            && let Some(tx) = checkpoint.transactions.get(*index)
        {
            return Some((
                tx,
                checkpoint.sequence,
                checkpoint.state_root.as_slice(),
                checkpoint.transaction_effects.get(*index),
            ));
        }

        self.dag_checkpoints.iter().rev().find_map(|checkpoint| {
            checkpoint
                .transactions
                .iter()
                .enumerate()
                .find(|(_, tx)| tx.transaction_hash() == tx_hash)
                .map(|(index, tx)| {
                    (
                        tx,
                        checkpoint.sequence,
                        checkpoint.state_root.as_slice(),
                        checkpoint.transaction_effects.get(index),
                    )
                })
        })
    }
}

impl Default for Blockchain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "../tests/unit/blockchain_tests.rs"]
mod tests;
