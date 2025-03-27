use consensus_pos::Blake3Algorithm;
use log::{error, info, warn};

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::block::Block;
use crate::blockchain::{save_blockchain, BALANCES, BLOCKCHAIN_DATA};

// Constants for Kari token
/// The amount of KA per Kari token based on the the fact that KA is
///  1/100_000_000 of a Kari token
/// This means that 1 Kari = 100_000_000 KA
const KA_PER_KARI: u64 = 100_000_000;

/// The total supply of Kari denominated in whole Kari tokens (100 Million)
const TOTAL_SUPPLY_KARI: u64 = 100_000_000;

/// The total supply of KA (Kari tokens) in the blockchain
/// This is the total supply of Kari tokens multiplied by KA_PER_KARI
/// which is 100_000_000 * 100_000_000 = 10_000_000_000_000_000
/// which is 10_000_000_000_000_000 KA (10 Quadrillion KA)
const TOTAL_SUPPLY_KA: u64 = 10_000_000_000_000_000;

pub fn run_blockchain(_running: Arc<Mutex<bool>>, address: String) {
    info!("Initializing blockchain with total supply of {} Kari ({} KA)", 
          TOTAL_SUPPLY_KARI, TOTAL_SUPPLY_KA);
    
    // Check if blockchain is already initialized (non-empty)
    if !BLOCKCHAIN_DATA.is_empty() {
        info!("Blockchain already initialized with {} blocks", BLOCKCHAIN_DATA.len());
        
        // Log available balance for the address
        match crate::blockchain::get_balance(&address) {
            Ok(balance) => {
                let balance_in_kari = balance as f64 / KA_PER_KARI as f64;
                info!("Address {} has {:.9} Kari ({} KA)", address, balance_in_kari, balance);
            },
            Err(e) => warn!("Failed to get balance for address {}: {}", address, e),
        }
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
    let _timestamp = SystemTime::now()
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

