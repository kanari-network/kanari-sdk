
// consensus/pos/src/lib.rs
use serde::{Deserialize, Serialize};
use blake3::Hasher;
use hex::encode;

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

pub struct PoSBlock<T: HashAlgorithm> {
    pub index: u64,
    pub timestamp: u128,
    pub data: String,
    pub prev_block_hash: String,
    pub hash: String,
    pub validator: String,
    pub hasher: T,
}



impl<T: HashAlgorithm> PoSBlock<T> {
    pub fn new(index: u64, data: String, prev_block_hash: String, validator: String, hasher: T) -> Self {
        PoSBlock {
            index,
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(),
            data,
            prev_block_hash,
            hash: String::new(),
            validator,
            hasher,
        }
    }

    pub fn calculate_hash(&self) -> String {
        let mut input = Vec::new();
        input.extend_from_slice(self.index.to_string().as_bytes());
        input.extend_from_slice(self.timestamp.to_string().as_bytes());
        input.extend_from_slice(self.data.as_bytes());
        input.extend_from_slice(self.prev_block_hash.as_bytes());
        input.extend_from_slice(self.validator.as_bytes());
        self.hasher.log_input(&input);
        self.hasher.hash(&input)
    }
}

pub fn proof_of_stake<T: HashAlgorithm>(block: &mut PoSBlock<T>) {
    block.hash = block.calculate_hash();
}


