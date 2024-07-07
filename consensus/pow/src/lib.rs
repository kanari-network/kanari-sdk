use sha2::{Sha256, Digest};

pub struct PoWBlock {
    pub index: u64,
    pub timestamp: u128,
    pub data: String,
    pub prev_block_hash: String,
    pub hash: String,
    pub nonce: u64,
}

// Block implementation
impl PoWBlock {
    pub fn new(index: u64, timestamp: u128, data: String, prev_block_hash: String) -> Self {
        PoWBlock {
            index,
            timestamp,
            data,
            prev_block_hash,
            hash: String::new(),
            nonce: 0,
        }
    }

    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.index.to_string().as_bytes());
        hasher.update(self.timestamp.to_string().as_bytes());
        hasher.update(&self.data.as_bytes());
        hasher.update(&self.prev_block_hash.as_bytes());
        hasher.update(self.nonce.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

// Proof of work function
pub fn proof_of_work(block: &mut PoWBlock, difficulty: usize) {
    while &block.calculate_hash()[..difficulty] != &"0".repeat(difficulty) {
        block.nonce += 1;
    }
}

// Example adjustment function
pub fn adjust_difficulty(current_miners: usize) -> usize {
    // Example adjustment function: difficulty increases linearly with the number of miners
    let base_difficulty = 4; // Base difficulty with a minimal number of miners
    let difficulty_increase_per_miner = 1; // Increase in difficulty for each additional miner
    base_difficulty + (current_miners.saturating_sub(1) * difficulty_increase_per_miner)
}
