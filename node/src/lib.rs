use std::collections::HashSet;
use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref BLOCK_HASHES: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

#[allow(dead_code)]
struct Block {
    index: u32,
    timestamp: u64,
    hash: String,
    // Other fields...
}

#[allow(dead_code)]
impl Block {
    fn new(index: u32, timestamp: u64, hash: &str) -> Self {
        Block {
            index,
            timestamp,
            hash: hash.to_string(),
            // Initialize other fields...
        }
    }

    fn block_exists(hash: &str) -> bool {
        let hashes = BLOCK_HASHES.lock().unwrap();
        hashes.contains(hash)
    }

    fn add_block(block: Block) -> bool {
        if Self::block_exists(&block.hash) {
            println!("Block with hash {} already exists.", block.hash);
            false
        } else {
            let mut hashes = BLOCK_HASHES.lock().unwrap();
            hashes.insert(block.hash.clone());
            // Add block to blockchain here...
            println!("Block added successfully.");
            true
        }
    }
}
