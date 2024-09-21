pub mod test;

use serde::{Deserialize, Serialize};
use blake3::Hasher;
use hex::encode;
use bincode;

#[derive(Serialize, Deserialize, Clone)]
pub struct Blake3Algorithm;

pub trait HashAlgorithm {
    fn hash(&self, input: &[u8]) -> String;
    fn log_input(&self, input: &[u8]);
}

impl HashAlgorithm for Blake3Algorithm {
    fn hash(&self, input: &[u8]) -> String {
        let mut hasher = Hasher::new();
        hasher.update(input);
        let result = hasher.finalize();
        encode(result.as_bytes())
    }

    fn log_input(&self, input: &[u8]) {
        println!("Hashing input: {:?}", input);
    }
}

#[derive(Serialize, Deserialize)]
pub struct PoWBlock<T: HashAlgorithm> {
    pub index: u64,
    pub timestamp: u128,
    pub data: String,
    pub prev_block_hash: String,
    pub hash: String,
    pub validator: String,
    pub hasher: T,
    pub nonce: u64,
    pub difficulty: u32,
}

impl<T: HashAlgorithm> PoWBlock<T> {
    pub fn new(index: u64, data: String, prev_block_hash: String, validator: String, hasher: T, difficulty: u32) -> Self {
        PoWBlock {
            index,
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(),
            data,
            prev_block_hash,
            hash: String::new(),
            validator,
            hasher,
            nonce: 0,
            difficulty,
        }
    }

    pub fn calculate_hash(&self) -> String {
        let mut input = Vec::new();
        input.extend_from_slice(self.index.to_string().as_bytes());
        input.extend_from_slice(self.timestamp.to_string().as_bytes());
        input.extend_from_slice(self.data.as_bytes());
        input.extend_from_slice(self.prev_block_hash.as_bytes());
        input.extend_from_slice(self.validator.as_bytes());
        input.extend_from_slice(self.nonce.to_string().as_bytes());
        
        // Serialize the input using bincode for efficiency
        let serialized_input = bincode::serialize(&input).unwrap();
        
        self.hasher.log_input(&serialized_input);
        self.hasher.hash(&serialized_input)
    }

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
}

pub fn proof_of_work<T: HashAlgorithm>(block: &mut PoWBlock<T>) {
    block.mine();
}

// Function to process multiple blocks in parallel
pub fn process_blocks_in_parallel<T: HashAlgorithm + Send + Sync>(blocks: &mut [PoWBlock<T>]) {
    blocks.iter_mut().for_each(|block| {
        proof_of_work(block);
    });
}
