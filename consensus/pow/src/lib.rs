use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
// Block structure
struct Block {
    index: u64,
    timestamp: u128,
    data: String,
    prev_block_hash: String,
    nonce: u64,
}

// Block implementation
impl Block {
    fn new(index: u64, timestamp: u128, data: String, prev_block_hash: String) -> Self {
        Block {
            index,
            timestamp,
            data,
            prev_block_hash,
            nonce: 0,
        }
    }

    fn calculate_hash(&self) -> String {
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
fn proof_of_work(block: &mut Block, difficulty: usize) {
    while &block.calculate_hash()[..difficulty] != &"0".repeat(difficulty) {
        block.nonce += 1;
    }
}

// Example adjustment function
fn adjust_difficulty(current_miners: usize) -> usize {
    // Example adjustment function: difficulty increases linearly with the number of miners
    let base_difficulty = 4; // Base difficulty with a minimal number of miners
    let difficulty_increase_per_miner = 1; // Increase in difficulty for each additional miner
    base_difficulty + (current_miners.saturating_sub(1) * difficulty_increase_per_miner)
}

// Main function
fn main() {
    let current_time_millis = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis();
    let difficulty = adjust_difficulty(current_time_millis as usize % 10);

    let mut block = Block::new(1, current_time_millis, "Block data".to_string(), "0".to_string());
    proof_of_work(&mut block, difficulty);
    println!("Block mined with hash: {}, at difficulty: {}", block.calculate_hash(), difficulty);
}