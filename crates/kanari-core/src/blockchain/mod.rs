// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Blockchain data structures and operations
use anyhow::Result;
use kanari_crypto::hash_data_blake3;
use kanari_crypto::keys::CurveType;
use kanari_move_runtime::changeset::Event;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::error;

mod merkle;
pub use merkle::{
    CompressedMerkleProof, batch_verify_merkle_proofs, compute_merkle_root,
    generate_merkle_multiproof, generate_merkle_proof, verify_merkle_proof,
};

mod dag_consensus;
pub use dag_consensus::{
    AuthorityId, Checkpoint, DagConsensus, DagStore, DagVertex, Round, VertexId, VertexMetadata,
};

mod vrf_leader;
pub use vrf_leader::{VrfLeaderElection, VrfOutput};

mod byzantine_detector;
pub use byzantine_detector::{
    ByzantineDetector, ByzantineEvidence, ByzantineFault, SlashingPenalty,
};

mod vertex_broadcast;
pub use vertex_broadcast::{
    CompressedBatch, DeltaSync, VertexBatch, VertexBloomFilter, VertexBroadcaster,
};

mod state_sync;
pub use state_sync::{FastSync, StateSynchronizer, SyncProgress, SyncRequest, SyncResponse};

mod light_client;
pub use light_client::{
    CheckpointBuilder, CheckpointSignature, LightCheckpoint, LightClient, LightClientQuery,
    StateProof, TransactionProof,
};

mod committee;
pub use committee::{
    Committee, CommitteeChange, CommitteeChangeTx, CommitteeManager, ValidatorInfo,
};

/// Signed transaction wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub signature: Vec<u8>,
}

impl SignedTransaction {
    pub fn new(transaction: Transaction) -> Self {
        Self {
            transaction,
            signature: vec![],
        }
    }

    pub fn sign(&mut self, private_key: &str, curve_type: CurveType) -> Result<()> {
        let tx_hash = self.transaction.hash();
        let signature = kanari_crypto::sign_message(private_key, &tx_hash, curve_type)
            .map_err(|e| anyhow::anyhow!("Failed to sign transaction: {}", e))?;
        self.signature = signature;
        Ok(())
    }

    pub fn verify_signature(&self) -> Result<bool> {
        if self.signature.is_empty() {
            anyhow::bail!("Transaction not signed");
        }

        let signature = &self.signature;

        let tx_hash = self.transaction.hash();
        let sender = self.transaction.sender_address();

        kanari_crypto::verify_signature(sender, &tx_hash, signature)
            .map_err(|e| anyhow::anyhow!("Signature verification failed: {}", e))
    }

    pub fn hash(&self) -> Vec<u8> {
        let serialized = match bcs::to_bytes(self) {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to serialize SignedTransaction for hashing: {}", e);
                Vec::new()
            }
        };
        hash_data_blake3(&serialized)
    }
}

/// Block header containing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub height: u64,
    pub timestamp: u64,
    pub prev_hash: Vec<u8>,
    pub state_root: Vec<u8>,
    pub merkle_root: Vec<u8>,
    pub tx_count: usize,
}

impl BlockHeader {
    pub fn new(
        height: u64,
        prev_hash: Vec<u8>,
        state_root: Vec<u8>,
        merkle_root: Vec<u8>,
        tx_count: usize,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            height,
            timestamp,
            prev_hash,
            state_root,
            merkle_root,
            tx_count,
        }
    }

    pub fn hash(&self) -> Vec<u8> {
        let serialized = match bcs::to_bytes(self) {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to serialize BlockHeader for hashing: {}", e);
                Vec::new()
            }
        };
        hash_data_blake3(&serialized)
    }
}

/// Transaction types in Kanari blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transaction {
    /// Publish a Move module
    PublishModule {
        sender: String,
        module_bytes: Vec<u8>,
        module_name: String,
        gas_limit: u64,
        gas_price: u64,
        sequence_number: u64,
    },
    /// Execute a Move function
    ExecuteFunction {
        sender: String,
        module: String,
        function: String,
        type_args: Vec<String>,
        args: Vec<Vec<u8>>,
        gas_limit: u64,
        gas_price: u64,
        sequence_number: u64,
    },
    /// Transfer coins
    Transfer {
        from: String,
        to: String,
        amount: u64,
        gas_limit: u64,
        gas_price: u64,
        sequence_number: u64,
    },
    /// Burn coins (remove from total supply)
    Burn {
        from: String,
        amount: u64,
        gas_limit: u64,
        gas_price: u64,
        sequence_number: u64,
    },
}

impl Transaction {
    pub fn hash(&self) -> Vec<u8> {
        let serialized = match bcs::to_bytes(self) {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to serialize Transaction for hashing: {}", e);
                Vec::new()
            }
        };
        hash_data_blake3(&serialized)
    }

    pub fn sender(&self) -> &str {
        match self {
            Transaction::PublishModule { sender, .. } => sender,
            Transaction::ExecuteFunction { sender, .. } => sender,
            Transaction::Transfer { from, .. } => from,
            Transaction::Burn { from, .. } => from,
        }
    }

    pub fn sender_address(&self) -> &str {
        self.sender()
    }

    pub fn sequence_number(&self) -> u64 {
        match self {
            Transaction::PublishModule {
                sequence_number, ..
            } => *sequence_number,
            Transaction::ExecuteFunction {
                sequence_number, ..
            } => *sequence_number,
            Transaction::Transfer {
                sequence_number, ..
            } => *sequence_number,
            Transaction::Burn {
                sequence_number, ..
            } => *sequence_number,
        }
    }

    pub fn gas_limit(&self) -> u64 {
        match self {
            Transaction::PublishModule { gas_limit, .. } => *gas_limit,
            Transaction::ExecuteFunction { gas_limit, .. } => *gas_limit,
            Transaction::Transfer { gas_limit, .. } => *gas_limit,
            Transaction::Burn { gas_limit, .. } => *gas_limit,
        }
    }

    pub fn gas_price(&self) -> u64 {
        match self {
            Transaction::PublishModule { gas_price, .. } => *gas_price,
            Transaction::ExecuteFunction { gas_price, .. } => *gas_price,
            Transaction::Transfer { gas_price, .. } => *gas_price,
            Transaction::Burn { gas_price, .. } => *gas_price,
        }
    }

    /// Create a transfer transaction with default gas settings
    pub fn new_transfer(from: String, to: String, amount: u64) -> Self {
        Self::Transfer {
            from,
            to,
            amount,
            gas_limit: 100_000, // Default gas limit
            gas_price: 1000,    // Default gas price (1000 Mist)
            sequence_number: 0,
        }
    }

    /// Create a burn transaction with default gas settings
    pub fn new_burn(from: String, amount: u64) -> Self {
        Self::Burn {
            from,
            amount,
            gas_limit: 100_000,
            gas_price: 1000,
            sequence_number: 0,
        }
    }
}

/// Block containing transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<SignedTransaction>,
    pub events: Vec<Event>,
}

impl Block {
    pub fn new(
        height: u64,
        prev_hash: Vec<u8>,
        state_root: Vec<u8>,
        transactions: Vec<SignedTransaction>,
        events: Vec<Event>,
    ) -> Self {
        let tx_count = transactions.len();

        // Compute merkle root from transaction hashes
        let tx_hashes: Vec<Vec<u8>> = transactions.iter().map(|tx| tx.hash()).collect();
        let merkle_root = compute_merkle_root(&tx_hashes);

        let header = BlockHeader::new(height, prev_hash, state_root, merkle_root, tx_count);

        Self {
            header,
            transactions,
            events,
        }
    }

    pub fn genesis() -> Self {
        Self::new(0, vec![0u8; 32], vec![0u8; 32], vec![], vec![])
    }

    pub fn hash(&self) -> Vec<u8> {
        self.header.hash()
    }

    pub fn verify(&self, prev_block: &Block) -> Result<()> {
        // Verify height
        if self.header.height != prev_block.header.height + 1 {
            anyhow::bail!(
                "Invalid block height: expected {}, got {}",
                prev_block.header.height + 1,
                self.header.height
            );
        }

        // Verify prev_hash
        let expected_prev_hash = prev_block.hash();
        if self.header.prev_hash != expected_prev_hash {
            anyhow::bail!(
                "Invalid previous hash: expected {}, got {}",
                hex::encode(&expected_prev_hash),
                hex::encode(&self.header.prev_hash)
            );
        }

        // Verify block hash integrity (recompute and compare)
        let computed_hash = self.hash();
        let header_hash = self.header.hash();
        if computed_hash != header_hash {
            anyhow::bail!(
                "Block hash mismatch: computed {}, header {}",
                hex::encode(&computed_hash),
                hex::encode(&header_hash)
            );
        }

        // Verify timestamp (must be >= previous block's timestamp)
        if self.header.height > 1 && self.header.timestamp < prev_block.header.timestamp {
            anyhow::bail!(
                "Invalid timestamp: {} < {}",
                self.header.timestamp,
                prev_block.header.timestamp
            );
        }

        // Verify transaction count matches header
        if self.transactions.len() != self.header.tx_count {
            anyhow::bail!(
                "Transaction count mismatch: header says {}, actual {}",
                self.header.tx_count,
                self.transactions.len()
            );
        }

        // Verify merkle root
        let tx_hashes: Vec<Vec<u8>> = self.transactions.iter().map(|tx| tx.hash()).collect();
        let computed_merkle_root = compute_merkle_root(&tx_hashes);
        if self.header.merkle_root != computed_merkle_root {
            anyhow::bail!(
                "Merkle root mismatch: header {}, computed {}",
                hex::encode(&self.header.merkle_root),
                hex::encode(&computed_merkle_root)
            );
        }

        // Verify each transaction has valid structure
        for (i, signed_tx) in self.transactions.iter().enumerate() {
            // Verify transaction hash is valid
            let tx_hash = signed_tx.transaction.hash();
            if tx_hash.is_empty() {
                anyhow::bail!("Transaction {} has empty hash", i);
            }

            // Verify sender address format
            let sender = signed_tx.transaction.sender_address();
            if sender.is_empty() {
                anyhow::bail!("Transaction {} has empty sender address", i);
            }

            // Require a valid signature for every transaction
            signed_tx.verify_signature().map_err(|e| {
                anyhow::anyhow!("Invalid or missing signature for transaction {}: {}", i, e)
            })?;
        }

        Ok(())
    }
}

/// Blockchain state - supports both linear chain and DAG modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    /// Linear chain mode: blocks in sequence
    pub blocks: Vec<Block>,

    /// DAG mode: checkpoints (committed state)
    #[serde(skip)]
    pub dag_checkpoints: Vec<Checkpoint>,

    /// Track executed transactions (for deduplication)
    #[serde(skip)]
    executed_tx_hashes: std::collections::HashSet<String>,

    /// Operating mode: true = DAG mode, false = linear chain mode
    #[serde(default)]
    pub dag_mode: bool,
}

impl Blockchain {
    pub fn new() -> Self {
        let genesis = Block::genesis();
        Self {
            blocks: vec![genesis],
            dag_checkpoints: vec![Checkpoint::genesis()],
            executed_tx_hashes: std::collections::HashSet::new(),
            dag_mode: false, // Default to linear chain mode
        }
    }

    /// Create blockchain in DAG mode
    pub fn new_with_dag() -> Self {
        Self {
            blocks: vec![Block::genesis()],
            dag_checkpoints: vec![Checkpoint::genesis()],
            executed_tx_hashes: std::collections::HashSet::new(),
            dag_mode: true,
        }
    }

    /// Switch to DAG mode
    pub fn enable_dag_mode(&mut self) {
        if !self.dag_mode {
            self.dag_mode = true;
            // Initialize DAG checkpoints if empty
            if self.dag_checkpoints.is_empty() {
                self.dag_checkpoints.push(Checkpoint::genesis());
            }
        }
    }

    /// Switch to linear chain mode
    pub fn disable_dag_mode(&mut self) {
        self.dag_mode = false;
    }

    pub fn latest_block(&self) -> &Block {
        self.blocks
            .last()
            .expect("blockchain must contain at least the genesis block")
    }

    /// Get latest checkpoint (for DAG mode)
    pub fn latest_checkpoint(&self) -> &Checkpoint {
        if self.dag_checkpoints.is_empty() {
            // This should never happen as we initialize with genesis
            panic!("DAG checkpoints is empty - should contain at least genesis");
        }
        self.dag_checkpoints.last().unwrap()
    }

    pub fn height(&self) -> u64 {
        if self.dag_mode {
            // In DAG mode, height = checkpoint sequence number
            self.latest_checkpoint().sequence
        } else {
            // In linear chain mode, height = block height
            self.latest_block().header.height
        }
    }

    /// Check if transaction hash has been executed before (deduplication)
    pub fn is_transaction_executed(&self, tx_hash: &str) -> bool {
        self.executed_tx_hashes.contains(tx_hash)
    }

    /// Mark transaction as executed (for deduplication tracking)
    pub fn mark_transaction_executed(&mut self, tx_hash: String) {
        self.executed_tx_hashes.insert(tx_hash);
    }

    /// Rebuild executed transaction hash set from blockchain history
    /// Call this after loading blockchain from disk
    pub fn rebuild_tx_hash_index(&mut self) {
        self.executed_tx_hashes.clear();
        for block in &self.blocks {
            for signed_tx in &block.transactions {
                let tx_hash = hex::encode(signed_tx.hash());
                self.executed_tx_hashes.insert(tx_hash);
            }
        }
    }

    pub fn add_block(&mut self, block: Block) -> Result<()> {
        self.add_block_with_validation(block, true)
    }

    pub fn add_block_with_validation(&mut self, block: Block, validate: bool) -> Result<()> {
        // Only allow adding blocks in linear chain mode
        if self.dag_mode {
            anyhow::bail!("Cannot add blocks directly in DAG mode. Use add_checkpoint instead.");
        }

        if validate {
            let prev_block = self.latest_block();
            block.verify(prev_block)?;

            // Check for duplicate transactions in this block
            for signed_tx in &block.transactions {
                let tx_hash = hex::encode(signed_tx.hash());
                if self.is_transaction_executed(&tx_hash) {
                    anyhow::bail!("Duplicate transaction detected: {}", tx_hash);
                }
            }
        }

        // Add transaction hashes to executed set
        for signed_tx in &block.transactions {
            let tx_hash = hex::encode(signed_tx.hash());
            self.mark_transaction_executed(tx_hash);
        }

        self.blocks.push(block);
        Ok(())
    }

    /// Add checkpoint (for DAG mode)
    pub fn add_checkpoint(&mut self, checkpoint: Checkpoint) -> Result<()> {
        if !self.dag_mode {
            anyhow::bail!("Cannot add checkpoints in linear chain mode. Use add_block instead.");
        }

        // Verify checkpoint sequence
        let expected_seq = self.latest_checkpoint().sequence + 1;
        if checkpoint.sequence != expected_seq {
            anyhow::bail!(
                "Invalid checkpoint sequence: expected {}, got {}",
                expected_seq,
                checkpoint.sequence
            );
        }

        // Verify previous checkpoint hash
        let prev_hash = self.latest_checkpoint().hash();
        if checkpoint.prev_checkpoint_hash != prev_hash {
            anyhow::bail!("Invalid previous checkpoint hash");
        }

        // Check for duplicate transactions
        for signed_tx in &checkpoint.transactions {
            let tx_hash = hex::encode(signed_tx.hash());
            if self.is_transaction_executed(&tx_hash) {
                anyhow::bail!("Duplicate transaction detected in checkpoint: {}", tx_hash);
            }
        }

        // Mark transactions as executed
        for signed_tx in &checkpoint.transactions {
            let tx_hash = hex::encode(signed_tx.hash());
            self.mark_transaction_executed(tx_hash);
        }

        self.dag_checkpoints.push(checkpoint);
        Ok(())
    }

    pub fn get_block(&self, height: u64) -> Option<&Block> {
        self.blocks.iter().find(|b| b.header.height == height)
    }

    /// Get checkpoint by sequence number (for DAG mode)
    pub fn get_checkpoint(&self, sequence: u64) -> Option<&Checkpoint> {
        self.dag_checkpoints
            .iter()
            .find(|cp| cp.sequence == sequence)
    }

    pub fn get_transaction_count(&self) -> usize {
        if self.dag_mode {
            // Count transactions in checkpoints
            self.dag_checkpoints
                .iter()
                .map(|cp| cp.transactions.len())
                .sum()
        } else {
            // Count transactions in blocks
            self.blocks.iter().map(|b| b.transactions.len()).sum()
        }
    }
}

impl Default for Blockchain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
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
    }

    #[test]
    fn test_add_block() {
        let mut chain = Blockchain::new();
        let prev_hash = chain.latest_block().hash();

        let block = Block::new(1, prev_hash, vec![0u8; 32], vec![], vec![]);
        chain.add_block(block).unwrap();

        assert_eq!(chain.height(), 1);
        assert_eq!(chain.blocks.len(), 2);
    }

    #[test]
    fn test_block_verification() {
        let chain = Blockchain::new();
        let prev_block = chain.latest_block();

        let valid_block = Block::new(1, prev_block.hash(), vec![0u8; 32], vec![], vec![]);
        assert!(valid_block.verify(prev_block).is_ok());
        let invalid_block = Block::new(2, prev_block.hash(), vec![0u8; 32], vec![], vec![]);
        assert!(invalid_block.verify(prev_block).is_err());
    }

    #[test]
    fn test_transaction_hash() {
        let tx = Transaction::new_transfer("0x1".to_string(), "0x2".to_string(), 1000);

        let hash1 = tx.hash();
        let hash2 = tx.hash();
        assert_eq!(hash1, hash2);
    }
}
