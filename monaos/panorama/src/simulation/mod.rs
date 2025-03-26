use log::{error, info, warn, debug};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use consensus_pos::Blake3Algorithm;
use rand::{thread_rng, Rng};

use crate::block::{Block, Transaction};
use crate::blockchain::{save_blockchain, BALANCES, BLOCKCHAIN, TOTAL_TOKENS};

// Constants for Kari token
const KARI_DECIMALS: u8 = 9;  // 9 decimal places
const KARI_BASE: u64 = 1_000_000_000;  // 10^9
const INITIAL_KARI_SUPPLY: u64 = 100_000_000 * KARI_BASE;  // 100 million kari with 9 decimal places
const BLOCK_TIME: u64 = 5;  // Target time between blocks in seconds
const TRANSACTION_FEE_PERCENT: u64 = 1;  // 1% transaction fee

pub fn run_blockchain(running: Arc<Mutex<bool>>, address: String) {
    info!("Initializing blockchain with {} kari (base unit: 10^{} decimals)", 
          INITIAL_KARI_SUPPLY / KARI_BASE, KARI_DECIMALS);
    
    info!("Node address: {}", address);
    
    unsafe {
        if BLOCKCHAIN.is_empty() {
            // Initialize with genesis block containing all tokens
            create_genesis_block();
            
            // Save initial blockchain state
            match save_blockchain() {
                Ok(_) => info!("Genesis blockchain state saved successfully"),
                Err(e) => error!("Failed to save genesis blockchain state: {}", e),
            }
        } else {
            info!("Blockchain already initialized with {} blocks", BLOCKCHAIN.len());
            info!("Current total supply: {} kari", TOTAL_TOKENS / KARI_BASE);
        }
    }
    
    // Start blockchain processing
    info!("Blockchain system running - producing blocks every {} seconds", BLOCK_TIME);
    
    let mut last_block_time = Instant::now();
    
    // Example of using the running flag to control the lifecycle
    while let Ok(guard) = running.lock() {
        if !*guard {
            break;
        }
        drop(guard); // Release lock before time-consuming operations
        
        // Check if it's time to create a new block
        if last_block_time.elapsed() >= Duration::from_secs(BLOCK_TIME) {
            // Mine a new block
            match mine_new_block(&address) {
                Ok(block_hash) => {
                    info!("Created new block with hash: {}", block_hash);
                    
                    // Save blockchain after each new block
                    match save_blockchain() {
                        Ok(_) => debug!("Blockchain state saved successfully"),
                        Err(e) => error!("Failed to save blockchain state: {}", e),
                    }
                },
                Err(e) => error!("Failed to create new block: {}", e),
            }
            
            last_block_time = Instant::now();
        }
        
        // Don't burn CPU
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    
    info!("Blockchain system shutting down");
}

// Helper function to create the genesis block
fn create_genesis_block() {
    use consensus_pos::Blake3Algorithm;
    
    let genesis_address = "genesis_kari_foundation".to_string();
    let genesis_data = format!("Genesis Block - Initial Supply: {} kari", 
                             INITIAL_KARI_SUPPLY / KARI_BASE).into_bytes();
    
    info!("Creating genesis block with address: {}", genesis_address);
    
    // Create empty transactions vector for genesis block
    let transactions = Vec::new();
    
    // Create the genesis block with Blake3Algorithm instantiated directly
    let genesis_block = Block::new(
        0,                  // index
        genesis_data,       // data
        "0".to_string(),    // prev_hash (zeros for genesis)
        INITIAL_KARI_SUPPLY, // tokens - all initial supply
        transactions,       // empty transactions for genesis
        genesis_address.clone(), // genesis address
        Blake3Algorithm{},  // Direct instantiation instead of using ::new()
    );
    
    // Add genesis block to blockchain
    unsafe {
        BLOCKCHAIN.push_back(genesis_block);
        TOTAL_TOKENS = INITIAL_KARI_SUPPLY;
    }
    
    // Update balance for genesis address
    {
        let mut balances = BALANCES.lock().unwrap();
        balances.insert(genesis_address, INITIAL_KARI_SUPPLY);
    }
    
    info!("Genesis block created with hash: {}", 
          unsafe { BLOCKCHAIN.front().unwrap().hash.clone() });
    info!("Total supply created: {} kari", INITIAL_KARI_SUPPLY / KARI_BASE);
}

// Function to mine a new block
fn mine_new_block(miner_address: &str) -> Result<String, String> {
    let previous_hash: String;
    let new_index: u32;
    
    unsafe {
        if BLOCKCHAIN.is_empty() {
            return Err("Blockchain is empty, cannot mine new block".into());
        }
        
        let prev_block = BLOCKCHAIN.back().unwrap();
        previous_hash = prev_block.hash.clone();
        new_index = prev_block.index + 1;
    }
    
    // Generate some transactions for demonstration
    let transactions = generate_sample_transactions(miner_address);
    
    // Calculate total transaction fees (1% of each transaction amount)
    let total_fees: u64 = transactions.iter()
        .map(|tx| tx.amount * TRANSACTION_FEE_PERCENT / 100)
        .sum();
    
    // Create block data with timestamp and transaction info
    let data = format!("Block {} mined by {} at {} with {} transactions", 
                      new_index, 
                      miner_address, 
                      SystemTime::now()
                          .duration_since(UNIX_EPOCH)
                          .unwrap()
                          .as_secs(),
                      transactions.len())
                .into_bytes();
    
    // Create the new block (with zero new tokens - fixed supply)
    let new_block = Block::new(
        new_index,
        data,
        previous_hash,
        0,  // No new tokens created - fixed supply at 100M
        transactions.clone(),
        miner_address.to_string(),
        Blake3Algorithm{},
    );
    
    let block_hash = new_block.hash.clone();
    
    // Update blockchain
    unsafe {
        BLOCKCHAIN.push_back(new_block);
        // Total supply remains unchanged since no new tokens are created
    }
    
    // Update balances for transaction fees
    {
        let mut balances = BALANCES.lock().unwrap();
        
        // Process all transactions, including fee collection
        for tx in &transactions {
            // Deduct full amount from sender (including fee)
            *balances.entry(tx.sender.clone()).or_insert(0) -= tx.amount;
            
            // Calculate and deduct fee from the transferred amount
            let fee = tx.amount * TRANSACTION_FEE_PERCENT / 100;
            let net_amount = tx.amount - fee;
            
            // Add net amount to receiver
            *balances.entry(tx.receiver.clone()).or_insert(0) += net_amount;
            
            // Add fee to miner
            *balances.entry(miner_address.to_string()).or_insert(0) += fee;
        }
        
        // Log miner's earnings from fees
        if total_fees > 0 {
            info!("Miner earned {} kari in transaction fees", total_fees / KARI_BASE);
            info!("Current miner balance: {} kari", 
                  balances.get(miner_address).unwrap_or(&0) / KARI_BASE);
        }
    }
    
    info!("Block {} mined with {} transactions, fees collected: {} kari, total supply: {} kari", 
          new_index, transactions.len(), total_fees / KARI_BASE, unsafe { TOTAL_TOKENS / KARI_BASE });
    
    Ok(block_hash)
}

// Function to generate sample transactions with transaction fees
fn generate_sample_transactions(miner_address: &str) -> Vec<Transaction> {
    let mut transactions = Vec::new();
    let mut rng = thread_rng();
    
    // Number of transactions to generate (random between 0-5 for demonstration)
    let num_transactions = rng.gen_range(0..=5);
    
    // Get balances to check if transactions are valid
    let balances = BALANCES.lock().unwrap();
    
    // Generate random transaction if balances exist
    if balances.len() > 1 {
        // Convert hashmap keys to vector for random selection
        let addresses: Vec<String> = balances.keys().cloned().collect();
        
        for _ in 0..num_transactions {
            // Select random sender and receiver
            let sender_idx = rng.gen_range(0..addresses.len());
            let mut receiver_idx = rng.gen_range(0..addresses.len());
            
            // Ensure sender and receiver are different
            while receiver_idx == sender_idx {
                receiver_idx = rng.gen_range(0..addresses.len());
            }
            
            let sender = &addresses[sender_idx];
            let receiver = &addresses[receiver_idx];
            
            // Get sender balance
            let sender_balance = *balances.get(sender).unwrap_or(&0);
            
            // Only create transaction if sender has sufficient balance
            // (including the 1% transaction fee)
            if sender_balance > KARI_BASE {
                // Random amount between 0.1% and 10% of sender's balance
                let max_amount = sender_balance / 10;
                let min_amount = sender_balance / 1000;
                let amount = if max_amount > min_amount {
                    rng.gen_range(min_amount..=max_amount)
                } else {
                    min_amount
                };
                
                // Ensure sender has enough to cover amount + fee
                let fee = amount * TRANSACTION_FEE_PERCENT / 100;
                let total_cost = amount + fee;
                
                if sender_balance >= total_cost {
                    transactions.push(Transaction {
                        sender: sender.clone(),
                        receiver: receiver.clone(),
                        amount,  // The gross amount (fee will be deducted in mining function)
                        timestamp: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        signature: None,
                    });
                }
            }
        }
    } else {
        // If there aren't enough addresses yet, create a dummy transaction
        // from the genesis foundation to the miner (to bootstrap the system)
        if num_transactions > 0 && balances.contains_key("genesis_kari_foundation") {
            let genesis_balance = *balances.get("genesis_kari_foundation").unwrap();
            if genesis_balance > KARI_BASE * 100 {
                let amount = KARI_BASE * 100;  // Transfer 100 kari
                transactions.push(Transaction {
                    sender: "genesis_kari_foundation".to_string(),
                    receiver: miner_address.to_string(),
                    amount,
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    signature: None,
                });
            }
        }
    }
    
    transactions
}