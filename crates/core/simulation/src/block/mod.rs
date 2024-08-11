// block.rs
use serde::{Deserialize, Serialize};
use crate::{chain_id::CHAIN_ID, transaction::Transaction};
use consensus_pos::HashAlgorithm;


#[derive(Serialize, Deserialize, Clone)]
pub struct Block<T: HashAlgorithm> {
    pub chain_id: String,
    pub index: u32,
    pub timestamp: u64,
    pub data: Vec<u8>,
    pub hash: String,
    pub prev_hash: String,
    pub tokens: u64,
    pub token_name: String,
    pub transactions: Vec<Transaction>,
    pub miner_address: String,
    pub hasher: T,
}

impl<T: HashAlgorithm> Block<T> {
    pub fn new(index: u32, data: Vec<u8>, prev_hash: String, tokens: u64, transactions: Vec<Transaction>, miner_address: String, hasher: T) -> Block<T> {
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let mut block = Block {
            chain_id: CHAIN_ID.to_string(),
            index,
            timestamp,
            data,
            hash: String::new(),
            prev_hash,
            tokens,
            token_name: String::from("Kanari"),
            transactions,
            miner_address,
            hasher,
        };
        block.hash = block.calculate_hash();
        block
    }

    pub fn calculate_hash(&self) -> String {
        let mut input = Vec::new();
        input.extend_from_slice(self.chain_id.as_bytes());
        input.extend_from_slice(&self.index.to_le_bytes());
        input.extend_from_slice(&self.timestamp.to_le_bytes());
        input.extend_from_slice(&self.data);
        input.extend_from_slice(self.prev_hash.as_bytes());
        input.extend_from_slice(&self.tokens.to_le_bytes());
        input.extend_from_slice(self.token_name.as_bytes());
        
        let transactions_serialized = serde_json::to_string(&self.transactions).unwrap();
        input.extend_from_slice(transactions_serialized.as_bytes());

        self.hasher.log_input(&input);
        self.hasher.hash(&input)
    }

    pub fn verify(&self, prev_block: &Block<T>) -> bool {
        if self.index != prev_block.index + 1 {
            return false;
        }
        if self.prev_hash != prev_block.hash {
            return false;
        }

        let calculated_hash = self.calculate_hash();
        if self.hash != calculated_hash {
            return false;
        }
        true
    }
}