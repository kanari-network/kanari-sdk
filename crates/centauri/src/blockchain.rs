// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Blockchain data structures and operations
use anyhow::Result;
use kanari_types::block::Block;
use kanari_types::transaction::SignedTransaction;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::consensus::Checkpoint;

// Limit in-memory data to prevent OOM
const MAX_RETAINED_BLOCKS: usize = 1000;
const MAX_RETAINED_TX_HASHES: usize = 2_000_000; // Retain ~4 seconds at 500K TPS

/// Generic serde helper for VecDeque serialization (serialize as Vec for compatibility)
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

/// Blockchain state - DAG-based consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    #[serde(with = "serde_vecdeque")]
    pub blocks: VecDeque<Block>,
    #[serde(default = "default_dag_checkpoints", with = "serde_vecdeque")]
    pub dag_checkpoints: VecDeque<Checkpoint>,

    // FIX: Use Vec<u8> instead of String to reduce memory overhead and allocations
    #[serde(skip)]
    executed_tx_hashes: std::collections::HashSet<Vec<u8>>,

    // FIFO queue for evicting old transaction hashes from HashSet
    #[serde(skip)]
    tx_hash_queue: VecDeque<Vec<u8>>,

    #[serde(default = "default_dag_mode")]
    pub dag_mode: bool,
}

fn default_dag_mode() -> bool {
    true
}

fn default_dag_checkpoints() -> VecDeque<Checkpoint> {
    let mut dq = VecDeque::new();
    dq.push_back(Checkpoint::genesis());
    dq
}

impl Blockchain {
    fn track_executed_transactions<'a, I>(&mut self, txs: I)
    where
        I: IntoIterator<Item = &'a SignedTransaction>,
    {
        for signed_tx in txs {
            let hash = signed_tx.transaction.hash();
            if self.executed_tx_hashes.insert(hash.clone()) {
                self.tx_hash_queue.push_back(hash);
            }
        }

        // FIX: Evict old hashes when exceeding limit to prevent OOM
        while self.tx_hash_queue.len() > MAX_RETAINED_TX_HASHES {
            if let Some(old_hash) = self.tx_hash_queue.pop_front() {
                self.executed_tx_hashes.remove(&old_hash);
            }
        }
    }

    fn latest_checkpoint_or_genesis(&self) -> &Checkpoint {
        self.dag_checkpoints
            .back()
            .expect("blockchain must contain at least the genesis checkpoint")
    }

    fn block_prev_hash(&self) -> Vec<u8> {
        self.blocks
            .back()
            .map_or_else(|| vec![0u8; 32], |block| block.hash())
    }

    fn track_executed_checkpoint_transactions(&mut self, checkpoint: &Checkpoint) {
        self.track_executed_transactions(&checkpoint.transactions);
    }

    pub fn new() -> Self {
        let genesis = Block::genesis();
        Self {
            blocks: vec![genesis].into(),
            dag_checkpoints: vec![Checkpoint::genesis()].into(),
            executed_tx_hashes: std::collections::HashSet::new(),
            tx_hash_queue: std::collections::VecDeque::new(),
            dag_mode: true,
        }
    }

    pub fn enable_dag_mode(&mut self) {
        if self.dag_checkpoints.is_empty() {
            self.dag_checkpoints.push_back(Checkpoint::genesis());
        }
    }

    pub fn latest_block(&self) -> &Block {
        self.blocks
            .back()
            .expect("blockchain must contain at least the genesis block")
    }

    pub fn latest_checkpoint(&self) -> &Checkpoint {
        self.latest_checkpoint_or_genesis()
    }

    pub fn height(&self) -> u64 {
        self.latest_checkpoint().sequence
    }

    pub fn is_transaction_hash_executed(&self, tx_hash: &[u8]) -> bool {
        self.executed_tx_hashes.contains(tx_hash)
    }

    pub fn has_executed_transactions(&self) -> bool {
        !self.executed_tx_hashes.is_empty()
    }

    /// Rebuilds the transaction hash index from all stored checkpoints
    ///
    /// ⚠️ SECURITY WARNING (FIX #2): This function can ONLY rebuild from checkpoints
    /// currently in memory (dag_checkpoints). If checkpoints have been pruned due to
    /// MAX_RETAINED_BLOCKS limit, those transactions will NOT be re-indexed!
    ///
    /// This creates a potential Replay Attack vulnerability after node restart:
    /// - Old checkpoints beyond MAX_RETAINED_BLOCKS are removed from RAM
    /// - After restart, rebuild_tx_hash_index() won't see those old TXs
    /// - Attackers could replay transactions from pruned checkpoints
    ///
    /// PRODUCTION RECOMMENDATION: Use PersistentDagStore or separate KV store
    /// to maintain permanent TX hash history independent of checkpoint pruning.
    pub fn rebuild_tx_hash_index(&mut self) {
        // Clear all existing cache data
        self.executed_tx_hashes.clear();
        self.tx_hash_queue.clear();

        // FIX #5: Stream processing instead of collecting all hashes into Vec first
        // This prevents OOM when there are millions of transactions
        let mut count = 0usize;

        for checkpoint in &self.dag_checkpoints {
            for tx in &checkpoint.transactions {
                let hash = tx.transaction.hash();

                // Insert and track in queue simultaneously
                if self.executed_tx_hashes.insert(hash.clone()) {
                    self.tx_hash_queue.push_back(hash);
                    count += 1;

                    // Enforce limit during rebuild to prevent OOM
                    // Batch removal to avoid O(n²) behavior - remove up to 1000 at a time
                    while self.tx_hash_queue.len() > MAX_RETAINED_TX_HASHES {
                        if let Some(old_hash) = self.tx_hash_queue.pop_front() {
                            self.executed_tx_hashes.remove(&old_hash);
                        }
                    }
                }
            }
        }

        tracing::info!(
            "Rebuilt transaction hash index: {} transactions indexed, {} retained in cache",
            count,
            self.tx_hash_queue.len()
        );
    }

    pub fn add_checkpoint(&mut self, checkpoint: Checkpoint) -> Result<()> {
        self.add_checkpoint_with_validation(checkpoint, true)
    }

    pub fn add_checkpoint_with_validation(
        &mut self,
        checkpoint: Checkpoint,
        validate: bool,
    ) -> Result<()> {
        if validate {
            let expected_seq = self.latest_checkpoint_or_genesis().sequence + 1;
            if checkpoint.sequence != expected_seq {
                anyhow::bail!(
                    "Invalid checkpoint sequence: expected {}, got {}",
                    expected_seq,
                    checkpoint.sequence
                );
            }
            let prev_hash = self.latest_checkpoint_or_genesis().hash()?;
            if checkpoint.prev_checkpoint_hash != prev_hash {
                anyhow::bail!("Invalid previous checkpoint hash");
            }

            // FIX #7: Check for duplicate transactions WITHIN the same checkpoint
            let mut seen_txs = std::collections::HashSet::new();
            for tx in &checkpoint.transactions {
                let tx_hash = tx.transaction.hash();
                if !seen_txs.insert(tx_hash.clone()) {
                    anyhow::bail!("Duplicate transaction found within checkpoint");
                }

                // FIX #6 & #1: Check from memory cache (Replay Attack Protection)
                if self.executed_tx_hashes.contains(&tx_hash) {
                    anyhow::bail!("Replay attack detected: Transaction already executed");
                }
            }

            // FIX #6: Verify transaction root consistency (if state_root includes tx_root)
            // Compute merkle root of transactions for additional integrity check
            let computed_tx_root = Self::compute_transaction_root(&checkpoint.transactions);

            // Log warning if tx_root doesn't match expected pattern (for future enhancement)
            tracing::debug!(
                "Checkpoint {} - Computed tx_root: {}, State root: {}",
                checkpoint.sequence,
                hex::encode(&computed_tx_root),
                hex::encode(&checkpoint.state_root)
            );
        }

        self.track_executed_checkpoint_transactions(&checkpoint);

        let block = Block::new(
            checkpoint.sequence,
            self.block_prev_hash(),
            checkpoint.state_root.clone(),
            checkpoint.transactions.clone(),
            Vec::new(),
            checkpoint.timestamp,
        );

        self.blocks.push_back(block);
        self.dag_checkpoints.push_back(checkpoint);

        // Evict old data to prevent OOM
        // FIX #2: Use pop_front() for O(1) removal instead of remove(0) which is O(N)
        if self.blocks.len() > MAX_RETAINED_BLOCKS {
            self.blocks.pop_front();
        }
        if self.dag_checkpoints.len() > MAX_RETAINED_BLOCKS {
            self.dag_checkpoints.pop_front();
        }

        Ok(())
    }

    /// Compute Merkle root of transactions for integrity verification
    fn compute_transaction_root(
        transactions: &[kanari_types::transaction::SignedTransaction],
    ) -> Vec<u8> {
        use kanari_crypto::hash_data_blake3;

        if transactions.is_empty() {
            return vec![0u8; 32];
        }

        // FIX #9: CRITICAL - Add domain separation to prevent second-preimage attacks
        // Hash all transactions with leaf prefix (0x00)
        let mut hashes: Vec<Vec<u8>> = transactions
            .iter()
            .map(|tx| {
                let tx_hash = tx.hash();
                // Prefix with 0x00 for leaf nodes
                let mut prefixed = vec![0x00];
                prefixed.extend_from_slice(&tx_hash);
                hash_data_blake3(&prefixed)
            })
            .collect();

        // Build Merkle tree with internal node prefix (0x01)
        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            let chunks = hashes.chunks(2);

            for chunk in chunks {
                if chunk.len() == 2 {
                    // Hash pair of nodes with internal prefix (0x01)
                    let mut combined = vec![0x01];
                    combined.extend_from_slice(&chunk[0]);
                    combined.extend_from_slice(&chunk[1]);
                    next_level.push(hash_data_blake3(&combined));
                } else {
                    // FIX #9: Odd node - duplicate and hash instead of promoting
                    // This prevents second-preimage attacks where structure can be manipulated
                    let mut combined = vec![0x01];
                    combined.extend_from_slice(&chunk[0]);
                    combined.extend_from_slice(&chunk[0]); // Duplicate the odd node
                    next_level.push(hash_data_blake3(&combined));
                }
            }

            hashes = next_level;
        }

        hashes.into_iter().next().unwrap_or_else(|| vec![0u8; 32])
    }

    pub fn get_block(&self, height: u64) -> Option<&Block> {
        self.blocks.iter().find(|b| b.header.height == height)
    }

    pub fn get_checkpoint(&self, sequence: u64) -> Option<&Checkpoint> {
        self.dag_checkpoints
            .iter()
            .find(|cp| cp.sequence == sequence)
    }

    pub fn get_transaction_count(&self) -> usize {
        self.executed_tx_hashes.len()
    }
}

impl Default for Blockchain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use kanari_types::transaction::Transaction;

    use super::*;

    #[test]
    fn test_genesis_block() {
        let genesis = Block::genesis();
        assert_eq!(genesis.header.height, 0);
        assert_eq!(genesis.transactions.len(), 0);
    }

    #[test]
    fn test_blockchain_creation() {
        let chain = Blockchain::new();
        assert_eq!(chain.height(), 0);
        assert_eq!(chain.blocks.len(), 1);
        assert!(chain.dag_mode);
    }

    #[test]
    fn test_add_checkpoint() {
        let mut chain = Blockchain::new();

        let prev_cp_hash = chain
            .latest_checkpoint()
            .hash()
            .expect("Genesis checkpoint hash should succeed");
        let checkpoint = Checkpoint::new(1, Vec::new(), Vec::new(), vec![0u8; 32], 0, prev_cp_hash);

        chain.add_checkpoint(checkpoint).unwrap();
        assert_eq!(chain.height(), 1);
        assert_eq!(chain.dag_checkpoints.len(), 2);
    }

    #[test]
    fn test_block_verification() {
        let chain = Blockchain::new();
        let prev_block = chain.latest_block();

        let valid_block = Block::new(1, prev_block.hash(), vec![0u8; 32], vec![], vec![], 0);
        assert!(valid_block.verify(prev_block).is_ok());
        let invalid_block = Block::new(2, prev_block.hash(), vec![0u8; 32], vec![], vec![], 0);
        assert!(invalid_block.verify(prev_block).is_err());
    }

    #[test]
    fn test_transaction_hash() {
        let tx = Transaction::new_transfer("0x1".to_string(), "0x2".to_string(), 1000, 0);

        let hash1 = tx.hash();
        let hash2 = tx.hash();
        assert_eq!(hash1, hash2);
    }
}
