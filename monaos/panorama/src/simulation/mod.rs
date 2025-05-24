use consensus_pos::Blake3Algorithm;
use log::{error, info, warn, debug};
use tokio::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde_json::{json, Value};

use mona_blockchain::block::{Block, Transaction};
use mona_blockchain::blockchain::{save_blockchain, BALANCES, BLOCKCHAIN_DATA, normalize_address};
use mona_types::kari::{KARI, KA_PER_KARI, POOL_ADDRESS, POOL_RESERVED_KA, POOL_RESERVED_KARI, TOTAL_SUPPLY_KA, TOTAL_SUPPLY_KARI};
use crate::utils::update_last_block_time;
use crate::staking::{load_staking_state, process_rewards, is_validator, get_pool_remaining_balance};
use crate::node::{NodeConfig, start_node, stop_node, get_peer_count};
use crate::node::coordinator; // Add import for the coordinator module

pub mod create_genesis_block;
pub mod transaction;
pub mod blockchain_display;

use create_genesis_block::create_genesis_block;

// Re-export the functions from transaction module
pub use transaction::{add_pending_transaction, process_transfer, PENDING_TRANSACTIONS};

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
        
    // Create block data - properly serialize Address types via serde JSON
    let block_data = json!({
        "block_type": "forced_transaction",
        "timestamp": timestamp,
        "transactions": [{
            "id": transaction.transaction_id, // Include transaction ID
            "sender": transaction.sender.to_hex_literal(),
            "receiver": transaction.receiver.to_hex_literal(),
            "amount": transaction.amount,
            "gas_fee": transaction.gas_fee, // Fix: Use transaction.gas_fee instead of crate::utils::GAS_FEE_AMOUNT
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

// Update run_blockchain's block creation to consistently include pending transactions
pub fn run_blockchain(
    running: Arc<Mutex<bool>>, 
    address: String,
    tx: mpsc::Sender<String>
) {
    // System password for automated transactions - could be set via config
    let _system_password = "kanari_system";

    let coin = KARI::default();

    info!("Initializing blockchain with {} coin", coin.name);
    info!(
        "Total supply: {} {} ({} {})",
        TOTAL_SUPPLY_KARI,
        coin.symbol,
        TOTAL_SUPPLY_KA,
        format!("{}A", coin.symbol)
    );

    // Parse address
    let node_address = match normalize_address(&address) {
        Ok(addr) => addr,
        Err(e) => {
            error!("Invalid node address: {}", e);
            // Send error notification
            blockchain_display::send_error_notification(
                &tx, 
                "blockchain", 
                &format!("Invalid node address: {}", e), 
                None
            );
            return;
        }
    };

    // Get string representation of address for display
    let normalized_address = node_address.to_hex_literal();
    debug!("Using normalized address: {}", normalized_address);

    // Initialize staking system
    if let Err(e) = load_staking_state() {
        warn!("Failed to load staking state: {}", e);
    } else {
        info!("Staking system initialized");
    }
    
    // Initialize node networking if multiple nodes are supported
    let node_config = NodeConfig {
        node_id: format!("node-{}", normalized_address[..8].to_string()),
        blockchain_address: normalized_address.clone(),
        listen_ip: "0.0.0.0".to_string(), // Listen on all interfaces, not just localhost
        listen_port: 51303, // Use fixed default port instead of dynamic calculation
        discovery_nodes: vec![
            // List of discovery nodes for peer discovery
        ],
        max_peers: 50, // Increased max peers for better network connectivity
        is_validator: is_validator(&node_address), // Dynamically check if this node is a validator
        use_tls: false, // Default to false for backward compatibility
        localhost_only: false, // Default to allow external connections
    };
    
    // Log node network configuration
    info!("Node network configuration: {}:{} (validator: {})", 
          node_config.listen_ip, node_config.listen_port, node_config.is_validator);
    
    // Start node networking
    if let Err(e) = start_node(node_config, tx.clone()) {
        warn!("Failed to start node networking: {}", e);
    } else {
        info!("Node networking started");
    }

    // Send initialization status using the display module
    blockchain_display::send_initialization_status(
        &tx,
        &coin.name,
        &coin.symbol,
        coin.decimals,
        coin.total_supply,
        TOTAL_SUPPLY_KARI as f64,
        &normalized_address
    );

    // Check if blockchain is already initialized
    if !BLOCKCHAIN_DATA.is_empty() {
        info!(
            "Blockchain already initialized with {} blocks",
            BLOCKCHAIN_DATA.len()
        );

        // Enhanced blockchain status through display module
        let blocks = BLOCKCHAIN_DATA.iter();
        let last_block = blocks.last().unwrap();
        
        blockchain_display::send_blockchain_loaded_status(&tx, BLOCKCHAIN_DATA.len(), last_block);

        // Check balance using both original and normalized address for troubleshooting
        match mona_blockchain::blockchain::get_balance(&normalized_address) {
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
            match mona_blockchain::blockchain::get_balance(&address) {
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
        let genesis_block = create_genesis_block(&node_address, &coin);
        BLOCKCHAIN_DATA.add_block(genesis_block.clone());

        // Send genesis block creation notification
        blockchain_display::send_genesis_created_notification(
            &tx,
            &genesis_block,
            &coin.name,
            &coin.symbol,
            coin.decimals,
            &normalized_address,
            TOTAL_SUPPLY_KA,
            TOTAL_SUPPLY_KARI as f64
        );

        // Update balances with normalized address
        {
            let mut balances = BALANCES.lock().unwrap();
            balances.insert(normalized_address.clone(), coin.total_supply);
            
            // Update pool balance if exists
            if let Ok(pool_addr) = normalize_address(POOL_ADDRESS) {
                let pool_addr_str = pool_addr.to_hex_literal();
                balances.insert(pool_addr_str.clone(), POOL_RESERVED_KA);
                // Decrease genesis address balance by the pool amount
                if let Some(balance) = balances.get_mut(&normalized_address) {
                    *balance -= POOL_RESERVED_KA;
                }
                info!("Reserved {} KARI for pool address: {}", POOL_RESERVED_KARI, pool_addr_str);
            }
            
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
            
            // Stop node networking
            if let Err(e) = stop_node() {
                warn!("Error stopping node: {}", e);
            }
            
            // Send shutdown notification using display module
            blockchain_display::send_blockchain_stopped_notification(&tx);
            break;
        }

        // Get the previous block
        let blocks = BLOCKCHAIN_DATA.iter();
        let prev_block = match blocks.last() {
            Some(block) => block,
            None => {
                error!("Cannot find previous block");
                blockchain_display::send_error_notification(
                    &tx,
                    "blockchain",
                    "Cannot find previous block",
                    None
                );
                break;
            }
        };

        // Get pending transactions for this block
        let transactions = mona_blockchain::blockchain::get_next_block_transactions(100);
        
        // Create JSON representation of transactions for the block data
        let tx_json: Vec<Value> = transactions.iter().map(|tx| {
            json!({
                "id": tx.transaction_id,
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
            transactions.clone(), // Include transactions in the block
            normalized_address.clone(),
            Blake3Algorithm::new(),
        );

        // Add block to chain and ensure we save the state
        BLOCKCHAIN_DATA.add_block(new_block.clone());
        
        // Propagate block to connected peers using the coordinator module
        if get_peer_count() > 0 {
            if let Err(e) = coordinator::broadcast_block(&new_block, &tx) {
                warn!("Failed to broadcast block to peers: {}", e);
            } else {
                info!("Block {} broadcasted to {} peers", new_block.index, get_peer_count());
            }
        }
        
        // Process staking rewards
        let staking_rewards = match process_rewards(new_block.index) {
            Ok(rewards) => rewards,
            Err(e) => {
                warn!("Failed to process staking rewards: {}", e);
                0
            }
        };
        
        // Check if the node operator is staking as validator
        let validator_status = is_validator(&node_address);
        
        // Get pool remaining balance
        let pool_balance = get_pool_remaining_balance().ok();

        // Send block mining notification using display module
        blockchain_display::send_block_mined_notification(
            &tx,
            &new_block,
            BLOCKCHAIN_DATA.len(),
            &normalized_address,
            staking_rewards,
            validator_status,
            pool_balance,
            get_peer_count()
        );

        // Save blockchain state - make sure it happens reliably after transaction block
        match save_blockchain() {
            Ok(_) => {
                info!(
                    "Block {} created successfully. Hash: {}...",
                    new_block.index,
                    &new_block.hash[..16]
                );
            },
            Err(e) => {
                error!("Failed to save blockchain: {}", e);
                // Notify clients of save error using display module
                blockchain_display::send_error_notification(
                    &tx,
                    "blockchain",
                    &format!("Failed to save blockchain: {}", e),
                    Some(json!({ "block_index": new_block.index }))
                );
            },
        }

        // Enhanced balance checking
        if new_block.index % 5 == 0 {
            match mona_blockchain::blockchain::get_balance(&normalized_address) {
                Ok(balance) => {
                    // Send balance update through display module
                    blockchain_display::send_balance_update(
                        &tx,
                        &normalized_address,
                        balance,
                        &coin.symbol,
                        new_block.index.into()
                    );
                    
                    // Log balance info
                    debug!(
                        "Current balance for {} is {} {}A",
                        normalized_address, balance, coin.symbol
                    );
                },
                Err(e) => {
                    warn!("Failed to get balance: {}", e);
                    // Notify of balance error through display module
                    blockchain_display::send_error_notification(
                        &tx,
                        "balance",
                        &format!("Failed to get balance: {}", e),
                        Some(json!({ "address": normalized_address }))
                    );
                },
            }
            
            // Debug: Enhanced log of all balances
            if let Ok(balances) = BALANCES.lock() {
                debug!("Current balances in system ({} accounts):", balances.len());
                for (addr, bal) in balances.iter() {
                    debug!("  {} => {}", addr, bal);
                }
                
                // Send system-wide balance report through display module
                blockchain_display::send_system_balances_report(&tx, balances.len());
            }
        }

        // Update last block time for gas fee calculation
        update_last_block_time(new_block.timestamp);

        // Sleep to control block creation rate
        thread::sleep(Duration::from_millis(420)); // 420 milliseconds for better performance
    }
}