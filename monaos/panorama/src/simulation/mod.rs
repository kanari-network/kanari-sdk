use consensus_pos::Blake3Algorithm;
use log::{error, info, warn, debug};
use tokio::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde_json::{json, Value};
use std::collections::VecDeque;

use crate::block::{Block, Transaction};
use crate::blockchain::{save_blockchain, BALANCES, BLOCKCHAIN_DATA};

// Constants for Kari token
/// The amount of KA per Kari token based on the the fact that KA is
/// 10^-9 of a Kari token
const KA_PER_KARI: u64 = 1_000_000_000;

/// The total supply of Kari denominated in whole Kari tokens (100 Million)
const TOTAL_SUPPLY_KARI: u64 = 100_000_000;

/// The total supply of Kari denominated in KA (100 Million * 10^9)
const TOTAL_SUPPLY_KA: u64 = 100_000_000_000_000_000;

// Enhanced Coin structure with additional properties
#[derive(Clone, Debug)]
pub struct Coin {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u64,
    pub max_supply: u64,      // Maximum supply that will ever exist
    pub block_reward: u64,    // Reward per block if applicable
    pub created_at: u64,      // Timestamp when coin was created
}

impl Default for Coin {
    fn default() -> Self {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        Coin {
            name: "Kanari".to_string(),
            symbol: "KARI".to_string(),
            decimals: 9, // 9 decimals for KA units
            total_supply: TOTAL_SUPPLY_KA,
            max_supply: TOTAL_SUPPLY_KA, // Same as total supply for fixed supply
            block_reward: 0,             // No mining rewards in this implementation
            created_at: current_time,    // Current timestamp
        }
    }
}

// Helper function to validate and normalize addresses
fn normalize_address(address: &str) -> String {
    // Ensure address starts with 0x prefix
    let address = if !address.starts_with("0x") {
        format!("0x{}", address)
    } else {
        address.to_string()
    };
    
    // Check address length and format (basic validation)
    if address.len() < 4 { // Minimum length check (0x + at least 2 chars)
        warn!("Address too short: {}", address);
    } else if address.len() > 66 { // Max length for 0x + 64 hex chars
        warn!("Address too long: {}", address);
    }
    
    // Return normalized address
    address
}

// Add pending transactions queue
lazy_static::lazy_static! {
    static ref PENDING_TRANSACTIONS: Mutex<VecDeque<Transaction>> = Mutex::new(VecDeque::new());
}

// Add transaction to the pending pool
pub fn add_pending_transaction(transaction: Transaction) -> bool {
    match PENDING_TRANSACTIONS.lock() {
        Ok(mut queue) => {
            queue.push_back(transaction);
            true
        },
        Err(_) => false
    }
}

// Improved function to ensure transactions are committed properly
pub fn process_transfer(
    from_address: &str,
    to_address: &str,
    amount: u64,
    tx: &mpsc::Sender<String>
) -> Result<Transaction, String> {
    // Normalize addresses
    let from = if !from_address.starts_with("0x") {
        format!("0x{}", from_address)
    } else {
        from_address.to_string()
    };
    
    let to = if !to_address.starts_with("0x") {
        format!("0x{}", to_address)
    } else {
        to_address.to_string()
    };
    
    // Execute transfer
    match crate::blockchain::transfer_tokens(&from, &to, amount) {
        Ok(transaction) => {
            // Add to pending transactions
            if add_pending_transaction(transaction.clone()) {
                // Notify about successful transaction submission
                let tx_json = json!({
                    "event": "transaction_created",
                    "transaction": {
                        "sender": from,
                        "receiver": to,
                        "amount": amount,
                        "timestamp": transaction.timestamp
                    },
                    "status": "pending"
                }).to_string();
                
                let _ = tx.try_send(tx_json);
                
                // Force save blockchain state to ensure transaction persistence
                match crate::blockchain::save_blockchain() {
                    Ok(_) => info!("Transaction recorded and blockchain state saved"),
                    Err(e) => warn!("Transaction recorded but failed to save state: {}", e),
                }
                
                // Return transaction
                Ok(transaction)
            } else {
                // Try to add the transaction directly to the next block
                match force_transaction_inclusion(&transaction) {
                    true => {
                        info!("Transaction bypassed queue and directly included in blockchain");
                        Ok(transaction)
                    },
                    false => Err("Failed to add transaction to blockchain".to_string())
                }
            }
        },
        Err(e) => {
            // Notify about transaction failure
            let error_json = json!({
                "event": "transaction_error",
                "error": format!("{}", e),
                "details": {
                    "sender": from,
                    "receiver": to,
                    "amount": amount
                }
            }).to_string();
            
            let _ = tx.try_send(error_json);
            
            // Return error
            Err(format!("{}", e))
        }
    }
}

// Function to handle transactions when queue fails
fn force_transaction_inclusion(transaction: &Transaction) -> bool {
    // Get the current blockchain state
    let blocks = BLOCKCHAIN_DATA.iter();
    let last_block = match blocks.last() {
        Some(block) => block,
        None => {
            error!("Cannot find previous block");
            return false;
        }
    };

    // Create transaction list
    let mut transactions = Vec::new();
    transactions.push(transaction.clone());

    // Create a forced block with this transaction
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
        
    // Create block data
    let block_data = json!({
        "block_type": "forced_transaction",
        "timestamp": timestamp,
        "transactions": [{
            "sender": transaction.sender,
            "receiver": transaction.receiver,
            "amount": transaction.amount,
            "timestamp": transaction.timestamp
        }]
    }).to_string().into_bytes();

    // Create emergency block to include the transaction
    let emergency_block = Block::new(
        last_block.index + 1,
        block_data,
        last_block.hash.clone(),
        0,
        transactions,
        "system".to_string(), // Use system as the minter for forced blocks
        Blake3Algorithm::new(),
    );

    // Add block to chain
    BLOCKCHAIN_DATA.add_block(emergency_block);
    
    // Save the blockchain immediately
    match save_blockchain() {
        Ok(_) => {
            info!("Emergency transaction block created and saved");
            true
        },
        Err(e) => {
            error!("Failed to save emergency transaction block: {}", e);
            false
        }
    }
}

pub fn run_blockchain(
    running: Arc<Mutex<bool>>, 
    address: String,
    tx: mpsc::Sender<String>
) {
    let coin = Coin::default();

    info!("Initializing blockchain with {} coin", coin.name);
    info!(
        "Total supply: {} {} ({} {})",
        TOTAL_SUPPLY_KARI,
        coin.symbol,
        TOTAL_SUPPLY_KA,
        format!("{}A", coin.symbol)
    );

    // Enhanced address normalization
    let normalized_address = normalize_address(&address);
    debug!("Using normalized address: {}", normalized_address);

    // Send initial status regardless of initialization state
    let init_status = json!({
        "event": "blockchain_initializing",
        "coin": {
            "name": coin.name,
            "symbol": coin.symbol,
            "decimals": coin.decimals,
            "total_supply": coin.total_supply,
            "display_supply": TOTAL_SUPPLY_KARI
        },
        "node_address": normalized_address,
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
    }).to_string();
    let _ = tx.try_send(init_status);

    // Check if blockchain is already initialized
    if !BLOCKCHAIN_DATA.is_empty() {
        info!(
            "Blockchain already initialized with {} blocks",
            BLOCKCHAIN_DATA.len()
        );

        // Enhanced blockchain status
        // Fix: Store the iterator result in a variable first
        let blocks = BLOCKCHAIN_DATA.iter();
        let last_block = blocks.last().unwrap();
        
        let status_json = json!({
            "event": "blockchain_loaded",
            "blocks": BLOCKCHAIN_DATA.len(),
            "last_block": {
                "index": last_block.index,
                "hash": last_block.hash,
                "timestamp": last_block.timestamp
            },
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
        }).to_string();
        
        let _ = tx.try_send(status_json);

        // Check balance using both original and normalized address for troubleshooting
        match crate::blockchain::get_balance(&normalized_address) {
            Ok(balance) => {
                let balance_in_kari = balance as f64 / KA_PER_KARI as f64;
                info!(
                    "Address {} has {:.9} {} ({} {}A)",
                    normalized_address, balance_in_kari, coin.symbol, balance, coin.symbol
                );
            }
            Err(e) => warn!("Failed to get balance for address {}: {}", normalized_address, e),
        }

        // Debug: Also check with original address if they're different
        if normalized_address != address {
            match crate::blockchain::get_balance(&address) {
                Ok(balance) => {
                    debug!(
                        "Original address {} has {} {}A",
                        address, balance, coin.symbol
                    );
                }
                Err(e) => debug!("Failed to get balance for original address {}: {}", address, e),
            }
        }
    } else {
        // Create genesis block with enhanced coin info as JSON
        let genesis_block = create_genesis_block(&normalized_address, &coin);
        BLOCKCHAIN_DATA.add_block(genesis_block.clone());

        // Enhanced genesis block info
        let genesis_json = json!({
            "event": "genesis_created",
            "block": {
                "index": genesis_block.index,
                "hash": genesis_block.hash,
                "timestamp": genesis_block.timestamp,
                "datetime": chrono::DateTime::<chrono::Utc>::from_timestamp(genesis_block.timestamp as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown time".to_string())
            },
            "coin": {
                "name": coin.name,
                "symbol": coin.symbol,
                "decimals": coin.decimals
            },
            "minter": normalized_address,
            "total_supply": {
                "amount": TOTAL_SUPPLY_KA,
                "display": TOTAL_SUPPLY_KARI,
                "symbol": coin.symbol
            }
        }).to_string();
        
        let _ = tx.try_send(genesis_json);

        // Update balances with normalized address
        {
            let mut balances = BALANCES.lock().unwrap();
            balances.insert(normalized_address.clone(), coin.total_supply);
            
            // Debug: Output all balances
            debug!("Initial balances after genesis:");
            for (addr, bal) in balances.iter() {
                debug!("  {} => {}", addr, bal);
            }
        }

        match save_blockchain() {
            Ok(_) => info!("Genesis block created successfully"),
            Err(e) => error!("Failed to save blockchain: {}", e),
        }

        info!(
            "Total supply of {} {} ({} {}A) minted to: {}",
            TOTAL_SUPPLY_KARI, coin.symbol, TOTAL_SUPPLY_KA, coin.symbol, normalized_address
        );
    }

    // Start block production loop
    info!("Starting block production with node address: {}", normalized_address);
    
    // Block production loop
    loop {
        if !*running.lock().unwrap() {
            info!("Blockchain simulation stopped");
            // Send shutdown notification
            let shutdown_json = json!({
                "event": "blockchain_stopped",
                "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
            }).to_string();
            let _ = tx.try_send(shutdown_json);
            break;
        }

        // Get the previous block - fix to properly store the blocks first
        let blocks = BLOCKCHAIN_DATA.iter();
        let prev_block = match blocks.last() {
            Some(block) => block,
            None => {
                error!("Cannot find previous block");
                break;
            }
        };

        // Get pending transactions for this block - improve transaction handling
        let transactions = {
            match PENDING_TRANSACTIONS.lock() {
                Ok(mut queue) => {
                    // Take up to 10 transactions for this block
                    let mut block_txs = Vec::new();
                    
                    // Log transaction queue status
                    info!("Processing transaction queue with {} pending transactions", queue.len());
                    
                    while let Some(tx) = queue.pop_front() {
                        info!("Including transaction: {} -> {}, amount: {}", 
                            tx.sender, tx.receiver, tx.amount);
                        block_txs.push(tx);
                        if block_txs.len() >= 10 {
                            break;
                        }
                    }
                    
                    if !block_txs.is_empty() {
                        info!("Added {} transactions to current block", block_txs.len());
                    }
                    
                    block_txs
                },
                Err(_) => {
                    error!("Failed to lock pending transactions queue");
                    Vec::new() // Empty vector on error
                }
            }
        };

        // Create JSON representation of transactions for the block data
        let tx_json: Vec<Value> = transactions.iter().map(|tx| {
            json!({
                "sender": tx.sender,
                "receiver": tx.receiver,
                "amount": tx.amount,
                "timestamp": tx.timestamp
            })
        }).collect();
        
        // Create new block data with transactions included
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        let block_data = json!({
            "block_type": "transaction",
            "index": prev_block.index + 1,
            "coin": coin.symbol,
            "timestamp": current_time,
            "miner": normalized_address,
            "transactions": tx_json,
            "metadata": {
                "network": "testnet",
                "client_version": env!("CARGO_PKG_VERSION"),
                "previous_block_hash": prev_block.hash
            }
        }).to_string().into_bytes();

        // Create new block with transactions
        let new_block = Block::new(
            prev_block.index + 1,
            block_data,
            prev_block.hash.clone(),
            0,          // No new tokens in regular blocks
            transactions.clone(), // Include transactions in the block - explicitly clone
            normalized_address.clone(),
            Blake3Algorithm::new(),
        );

        // Add block to chain and ensure we save the state
        BLOCKCHAIN_DATA.add_block(new_block.clone());
        
        // If we included transactions, provide detailed logs
        if !transactions.is_empty() {
            info!("Block {} includes {} transactions:", new_block.index, transactions.len());
            for (i, tx) in transactions.iter().enumerate() {
                info!("  {}: {} -> {} ({})", i+1, tx.sender, tx.receiver, tx.amount);
            }
        }

        // Convert timestamp to human-readable format
        let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(new_block.timestamp as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown time".to_string());

        // Enhanced block status update as JSON
        let status_json = json!({
            "event": "block_mined",
            "block": {
                "index": new_block.index,
                "hash": new_block.hash,
                "prev_hash": new_block.prev_hash,
                "timestamp": new_block.timestamp,
                "datetime": datetime,
                "minter": normalized_address,
                "transactions": new_block.transactions.len()
            },
            "blockchain": {
                "height": BLOCKCHAIN_DATA.len(),
                "last_update": current_time
            }
        }).to_string();
        
        let _ = tx.try_send(status_json);

        // Save blockchain state - make sure it happens reliably after transaction block
        match save_blockchain() {
            Ok(_) => {
                info!(
                    "Block {} created successfully. Hash: {}...",
                    new_block.index,
                    &new_block.hash[..16]
                );
                
                // After saving, run a verification step to ensure transactions were processed
                if !transactions.is_empty() {
                    verify_transaction_processing(&transactions, &tx);
                }
            },
            Err(e) => {
                error!("Failed to save blockchain: {}", e);
                // Notify clients of save error
                let error_json = json!({
                    "event": "blockchain_error",
                    "error": format!("Failed to save blockchain: {}", e),
                    "block_index": new_block.index,
                    "timestamp": current_time
                }).to_string();
                let _ = tx.try_send(error_json);
            },
        }

        // Enhanced balance checking
        if new_block.index % 5 == 0 {
            match crate::blockchain::get_balance(&normalized_address) {
                Ok(balance) => {
                    // Enhanced balance update with more details
                    let balance_json = json!({
                        "event": "balance_update",
                        "address": normalized_address,
                        "balance": {
                            "amount": balance,
                            "display": balance as f64 / KA_PER_KARI as f64,
                            "symbol": coin.symbol,
                            "formatted": format!("{:.9} {}", balance as f64 / KA_PER_KARI as f64, coin.symbol)
                        },
                        "block_height": new_block.index,
                        "timestamp": current_time
                    }).to_string();
                    
                    let _ = tx.try_send(balance_json);
                    
                    // Log balance info
                    debug!(
                        "Current balance for {} is {} {}A",
                        normalized_address, balance, coin.symbol
                    );
                },
                Err(e) => {
                    warn!("Failed to get balance: {}", e);
                    // Notify of balance error
                    let error_json = json!({
                        "event": "balance_error",
                        "error": format!("Failed to get balance: {}", e),
                        "address": normalized_address,
                        "timestamp": current_time
                    }).to_string();
                    let _ = tx.try_send(error_json);
                },
            }
            
            // Debug: Enhanced log of all balances
            if let Ok(balances) = BALANCES.lock() {
                debug!("Current balances in system ({} accounts):", balances.len());
                for (addr, bal) in balances.iter() {
                    debug!("  {} => {}", addr, bal);
                }
                
                // Send system-wide balance report
                let balance_report = json!({
                    "event": "system_balances",
                    "account_count": balances.len(),
                    "timestamp": current_time
                }).to_string();
                let _ = tx.try_send(balance_report);
            }
        }

        // Sleep to control block creation rate
        thread::sleep(Duration::from_secs(10));
    }
}

// New function to verify transactions were properly processed
fn verify_transaction_processing(transactions: &Vec<Transaction>, tx: &mpsc::Sender<String>) {
    for transaction in transactions {
        // Verify sender and receiver balances
        match crate::blockchain::get_balance(&transaction.sender) {
            Ok(balance) => {
                debug!("Verified sender {} balance: {}", transaction.sender, balance);
            },
            Err(e) => {
                warn!("Failed to verify sender balance: {}", e);
            }
        }
        
        match crate::blockchain::get_balance(&transaction.receiver) {
            Ok(balance) => {
                debug!("Verified receiver {} balance: {}", transaction.receiver, balance);
                
                // Notify about completed transaction
                let tx_json = json!({
                    "event": "transaction_confirmed",
                    "transaction": {
                        "sender": transaction.sender,
                        "receiver": transaction.receiver,
                        "amount": transaction.amount,
                        "receiver_balance": balance
                    }
                }).to_string();
                
                let _ = tx.try_send(tx_json);
            },
            Err(e) => {
                warn!("Failed to verify receiver balance: {}", e);
            }
        }
    }
}

/// Create a genesis block containing the total supply of Kari tokens
fn create_genesis_block(address: &str, coin: &Coin) -> Block<Blake3Algorithm> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs();

    // Enhanced genesis data with more comprehensive information
    let genesis_data = json!({
        "block_type": "genesis",
        "coin": {
            "name": coin.name,
            "symbol": coin.symbol,
            "decimals": coin.decimals,
            "total_supply": {
                "amount": coin.total_supply,
                "display": TOTAL_SUPPLY_KARI,
                "symbol": coin.symbol
            },
            "block_reward": coin.block_reward,
            "max_supply": coin.max_supply
        },
        "network": {
            "name": "Kanari Testnet",
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": timestamp,
            "datetime": chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown time".to_string())
        },
        "genesis_address": address
    }).to_string().into_bytes();

    info!("Creating genesis block for {} with {} total supply", coin.name, coin.total_supply);
    
    Block::new(
        0,
        genesis_data,
        "0".repeat(64),
        coin.total_supply,
        Vec::new(),
        address.to_string(),
        Blake3Algorithm::new(),
    )
}

