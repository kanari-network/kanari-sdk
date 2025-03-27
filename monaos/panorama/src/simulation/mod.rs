use consensus_pos::Blake3Algorithm;
use log::{debug, error, info, warn};
use rand::{thread_rng, Rng};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::block::{Block, Transaction};
use crate::blockchain::{save_blockchain, BALANCES, BLOCKCHAIN_DATA};

// Constants for Kari token
/// The amount of KA per Kari token based on the the fact that KA is
/// 10^-9 of a Kari token
const KA_PER_KARI: u64 = 1_000_000_000;

#[allow(unused_const)]
/// The total supply of Kari denominated in whole Kari tokens (100 Million)
const TOTAL_SUPPLY_KARI: u64 = 100_000_000;

/// The total supply of Kari denominated in KA (100 Million * 10^9)
const TOTAL_SUPPLY_KA: u64 = 100_000_000_000_000_000;

pub fn run_blockchain(running: Arc<Mutex<bool>>, address: String) {
    info!("Initializing blockchain with total supply of {} Kari ({} KA)", 
          TOTAL_SUPPLY_KARI, TOTAL_SUPPLY_KA);
    
    // Check if blockchain is already initialized (non-empty)
    if !BLOCKCHAIN_DATA.is_empty() {
        info!("Blockchain already initialized with {} blocks", BLOCKCHAIN_DATA.len());
        return;
    }
    
    // Create genesis block with the total supply of Kari
    let genesis_block = create_genesis_block(&address);
    
    // Add the block to the blockchain
    BLOCKCHAIN_DATA.add_block(genesis_block.clone());
    
    // Update balances
    {
        let mut balances = BALANCES.lock().unwrap();
        balances.insert(address.clone(), TOTAL_SUPPLY_KA);
    }
    
    // Save the blockchain
    match save_blockchain() {
        Ok(_) => info!("Genesis block with total supply created and saved successfully"),
        Err(e) => error!("Failed to save blockchain after genesis block creation: {}", e),
    }
    
    info!("Total supply of {} Kari ({} KA) minted to address: {}", 
          TOTAL_SUPPLY_KARI, TOTAL_SUPPLY_KA, address);
}

/// Create a genesis block containing the total supply of Kari tokens
fn create_genesis_block(address: &str) -> Block<Blake3Algorithm> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs();
    
    // Genesis block data
    let data = format!("Genesis Block - Total Supply: {} Kari", TOTAL_SUPPLY_KARI).into_bytes();
    
    // Create block with Blake3 algorithm
    Block::new(
        0,                      // index (genesis block is 0)
        data,                   // block data
        "0".repeat(64),         // previous hash (zeros for genesis)
        TOTAL_SUPPLY_KA,              // tokens (total supply in KA)
        Vec::new(),             // transactions (empty for genesis)
        address.to_string(),         // address (recipient of total supply)
        Blake3Algorithm::new(),       // hasher
    )
}

