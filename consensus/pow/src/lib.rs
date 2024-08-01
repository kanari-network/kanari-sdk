mod powtest;

use sha2::{Sha256, Digest};
pub struct Sha256Algorithm;

pub trait HashAlgorithm {
    fn hash(&self, input: &[u8]) -> String;
    fn log_input(&self, input: &[u8]);
}


impl HashAlgorithm for Sha256Algorithm {
    fn hash(&self, input: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input);
        format!("{:x}", hasher.finalize())
    }

    fn log_input(&self, input: &[u8]) {
        println!("Hashing input: {:?}", input);
    }
}

pub struct PoWBlock<T: HashAlgorithm> {
    pub index: u64,
    pub timestamp: u128,
    pub data: String,
    pub prev_block_hash: String,
    pub hash: String,
    pub nonce: u64,
    pub hasher: T,
}

impl<T: HashAlgorithm> PoWBlock<T> {
    pub fn new(index: u64, timestamp: u128, data: String, prev_block_hash: String, hasher: T) -> Self {
        PoWBlock {
            index,
            timestamp,
            data,
            prev_block_hash,
            hash: String::new(),
            nonce: 0,
            hasher,
        }
    }

    pub fn calculate_hash(&self) -> String {
        let mut input = Vec::new();
        input.extend_from_slice(self.index.to_string().as_bytes());
        input.extend_from_slice(self.timestamp.to_string().as_bytes());
        input.extend_from_slice(self.data.as_bytes());
        input.extend_from_slice(self.prev_block_hash.as_bytes());
        input.extend_from_slice(self.nonce.to_string().as_bytes());
        self.hasher.log_input(&input);
        self.hasher.hash(&input)
    }
}

// Proof of work function
pub fn proof_of_work<T: HashAlgorithm>(block: &mut PoWBlock<T>, difficulty: usize) {
    while &block.calculate_hash()[..difficulty] != &"0".repeat(difficulty) {
        block.nonce += 1;
    }
    block.hash = block.calculate_hash();
}

pub fn adjust_difficulty(current_miners: usize) -> usize {
    let base_difficulty = 4;
    let difficulty_increase_per_miner = 1;
    base_difficulty + (current_miners.saturating_sub(1) * difficulty_increase_per_miner)
}
