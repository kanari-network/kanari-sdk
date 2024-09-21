use serde::{Deserialize, Serialize};
use crate::{chain_id::CHAIN_ID, transaction::Transaction};
use consensus_pow::HashAlgorithm;

// Define the Block struct
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
    pub nonce: u64,
    pub difficulty: u32,
}

// Implement the Block struct
impl<T: HashAlgorithm> Block<T> {
    pub fn new(index: u32, data: Vec<u8>, prev_hash: String, tokens: u64, transactions: Vec<Transaction>, miner_address: String, hasher: T, difficulty: u32) -> Block<T> {
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
            nonce: 0,
            difficulty,
        };
        block.mine();
        block
    }

    // Add a method to calculate the hash of the block
    pub fn calculate_hash(&self) -> String {
        let mut input = Vec::new();
        input.extend_from_slice(self.chain_id.as_bytes());
        input.extend_from_slice(&self.index.to_le_bytes());
        input.extend_from_slice(&self.timestamp.to_le_bytes());
        input.extend_from_slice(&self.data);
        input.extend_from_slice(self.prev_hash.as_bytes());
        input.extend_from_slice(&self.tokens.to_le_bytes());
        input.extend_from_slice(self.token_name.as_bytes());
        
        // Serialize transactions
        let transactions_serialized = serde_json::to_string(&self.transactions).unwrap();
        input.extend_from_slice(transactions_serialized.as_bytes());
        input.extend_from_slice(&self.nonce.to_le_bytes());

        self.hasher.log_input(&input);
        self.hasher.hash(&input)
    }

    // Add a method to mine the block
    pub fn mine(&mut self) {
        loop {
            let hash = self.calculate_hash();
            if hash.starts_with(&"0".repeat(self.difficulty as usize)) {
                self.hash = hash;
                break;
            }
            self.nonce += 1;
        }
    }

    // Add a method to verify the block
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
        if !self.hash.starts_with(&"0".repeat(self.difficulty as usize)) {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use consensus_pow::Blake3Algorithm;
    use crate::transaction::Transaction;

    #[test]
    fn test_calculate_hash() {
        let hasher = Blake3Algorithm;
        let block = Block::new(0, vec![1, 2, 3], "0000".to_string(), 100, vec![], "miner".to_string(), hasher, 1);
        let hash = block.calculate_hash();
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_mine() {
        let hasher = Blake3Algorithm;
        let mut block = Block::new(0, vec![1, 2, 3], "0000".to_string(), 100, vec![], "miner".to_string(), hasher, 1);
        block.mine();
        assert!(block.hash.starts_with("0"));
    }

    #[test]
    fn test_verify() {
        let hasher = Blake3Algorithm;
        let mut prev_block = Block::new(0, vec![1, 2, 3], "0000".to_string(), 100, vec![], "miner".to_string(), hasher.clone(), 1);
        prev_block.mine();
        let mut new_block = Block::new(1, vec![4, 5, 6], prev_block.hash.clone(), 200, vec![], "miner".to_string(), hasher, 1);
        new_block.mine();
        assert!(new_block.verify(&prev_block));
    }
}