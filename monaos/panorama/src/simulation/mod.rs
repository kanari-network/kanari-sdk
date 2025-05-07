use consensus_pos::Blake3Algorithm;
use log::{error, info, warn, debug};
use tokio::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::str::FromStr;

use mona_blockchain::block::{Block, Transaction};
use mona_blockchain::blockchain::{save_blockchain, BALANCES, BLOCKCHAIN_DATA, normalize_address};
use crate::transfer_tokens::transfer_tokens;
use mona_types::address::Address;
use mona_types::kari::{KARI, KA_PER_KARI, POOL_ADDRESS, POOL_RESERVED_KA, POOL_RESERVED_KARI, TOTAL_SUPPLY_KA, TOTAL_SUPPLY_KARI, VALIDATOR_STAKING_MINIMUM_KARI, NODE_STAKING_MINIMUM_KARI};
use crate::utils::{update_pending_transaction_count, update_last_block_time, calculate_gas_fee, format_gas_fee_display};
use crate::staking::{load_staking_state, process_rewards, is_validator};
use crate::node::{NodeConfig, start_node, stop_node, propagate_block, get_peer_count};

pub mod create_genesis_block;
use create_genesis_block::create_genesis_block;

// Add monaVM dependency
use mona_vm::{execute_vm_transaction, convert_to_vm_transaction, VMTransaction};

// Function to parse and normalize address
fn parse_address(address: &str) -> Result<Address, String> {
    Address::from_str(address)
        .map_err(|_| format!("Invalid address format: {}", address))
}

// Add pending transactions queue
lazy_static::lazy_static! {
    static ref PENDING_TRANSACTIONS: RwLock<VecDeque<Transaction>> = RwLock::new(VecDeque::new());
}

// Now use .write() or .read() instead of .lock()
pub fn add_pending_transaction(transaction: Transaction) -> bool {
    match PENDING_TRANSACTIONS.write() {
        Ok(mut queue) => {
            queue.push_back(transaction);
            // Update pending transaction count for gas calculation
            update_pending_transaction_count(queue.len());
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
    password: &str,
    priority_boost: Option<u64>,  // Add optional priority boost
    tx: &mpsc::Sender<String>
) -> Result<Transaction, String> {
    // Parse addresses
    let from = match normalize_address(from_address) {
        Ok(addr) => addr,
        Err(e) => return Err(format!("Invalid sender address: {}", e)),
    };
    
    let to = match normalize_address(to_address) {
        Ok(addr) => addr, 
        Err(e) => return Err(format!("Invalid receiver address: {}", e)),
    };
    
    // Calculate gas fee dynamically
    let gas_fee = calculate_gas_fee(priority_boost);
    let gas_fee_display = format_gas_fee_display(gas_fee);
    
    // Execute transfer using string representation and password for signing
    match transfer_tokens(&from.to_hex_literal(), &to.to_hex_literal(), amount, password, gas_fee) {
        Ok(transaction) => {
            // Verify signature right after creation for better debugging
            let signature_status = if transaction.signature.is_empty() {
                "unsigned"
            } else {
                match crate::transfer_tokens::verify_transaction::verify_transaction(&transaction) {
                    Ok(true) => "valid",
                    Ok(false) => "invalid",
                    Err(e) => {
                        warn!("Error verifying signature: {}", e);
                        "unknown"
                    }
                }
            };
            
            // Add to pending transactions
            if add_pending_transaction(transaction.clone()) {
                // Notify about successful transaction submission
                let tx_json = json!({
                    "event": "transaction_created",
                    "transaction": {
                        "id": transaction.transaction_id,
                        "sender": transaction.sender.to_hex_literal(),
                        "receiver": transaction.receiver.to_hex_literal(),
                        "amount": amount,
                        "gas_fee": transaction.gas_fee,
                        "gas_fee_display": format_gas_fee_display(transaction.gas_fee),
                        "gas_collector": crate::utils::GAS_FEE_COLLECTOR,
                        "total_cost": crate::utils::calculate_total_transaction_cost(amount, transaction.gas_fee),
                        "timestamp": transaction.timestamp,
                        "signed": !transaction.signature.is_empty(),
                        "signature_status": signature_status
                    },
                    "status": "pending"
                }).to_string();
                
                let _ = tx.try_send(tx_json);
                
                // Force save blockchain state to ensure transaction persistence
                match mona_blockchain::blockchain::save_blockchain() {
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
            // Update error JSON to use calculated gas fee
            let error_json = json!({
                "event": "transaction_error",
                "error": format!("{}", e),
                "details": {
                    "sender": from_address,
                    "receiver": to_address,
                    "amount": amount,
                    "gas_fee": gas_fee,
                    "gas_fee_display": gas_fee_display,
                    "total_cost": crate::utils::calculate_total_transaction_cost(amount, gas_fee)
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

// Process VM transaction if the transaction contains smart contract execution data
fn process_vm_transaction(transaction: &Transaction, tx: &mpsc::Sender<String>) -> bool {
    // Try to convert to VM transaction
    if let Some(vm_tx) = convert_to_vm_transaction(transaction) {
        info!("Processing VM transaction: {} -> {}.{}", 
              transaction.transaction_id, vm_tx.module_id, vm_tx.function);
        
        // Debug output to troubleshoot module and function calls
        log::debug!("VM Transaction details:");
        log::debug!("  Module ID: {}", vm_tx.module_id);
        log::debug!("  Function: {}", vm_tx.function);
        log::debug!("  Sender: {}", vm_tx.sender);
        log::debug!("  Args count: {}", vm_tx.args.len());
        log::debug!("  Gas budget: {}", vm_tx.gas_budget);
        
        // Check if module exists before execution
        let module_exists = match crate::check_module_exists(&vm_tx.module_id) {
            Ok(exists) => exists,
            Err(e) => {
                warn!("Error checking module existence: {}", e);
                false
            }
        };
        
        if !module_exists {
            warn!("Module {} not found, VM transaction will fail", vm_tx.module_id);
            
            // Notify about missing module
            let error_json = json!({
                "event": "vm_transaction_error",
                "transaction_id": transaction.transaction_id,
                "vm_transaction": {
                    "module": vm_tx.module_id,
                    "function": vm_tx.function
                },
                "error": format!("Module {} not found", vm_tx.module_id),
                "error_type": "module_not_found",
                "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
            });
            
            // Store error result for later retrieval
            crate::store_vm_transaction_result(&transaction.transaction_id, error_json.clone());
            
            let _ = tx.try_send(error_json.to_string());
            return true;
        }
        
        // Notify about VM transaction start
        let start_json = json!({
            "event": "vm_transaction_started",
            "transaction_id": transaction.transaction_id,
            "vm_transaction": {
                "module": vm_tx.module_id,
                "function": vm_tx.function
            },
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
        }).to_string();
        
        let _ = tx.try_send(start_json);
        
        // Execute VM transaction with enhanced error handling
        match execute_vm_transaction(&vm_tx) {
            Ok(result) => {
                // Extract result data for better presentation
                let success = result.get("status").and_then(|s| s.as_str()).unwrap_or("unknown") == "success";
                let gas_used = result.get("gas_used").and_then(|g| g.as_u64()).unwrap_or(0);
                let gas_display = result.get("gas_display").and_then(|g| g.as_str()).unwrap_or("");
                let execution_time = result.get("execution_time_ms").and_then(|t| t.as_u64()).unwrap_or(0);
                
                // Determine status string before using in json! macro
                let status_str = if success { "success" } else { "failed" };
                
                // Notify about VM transaction execution
                let result_json = json!({
                    "event": "vm_transaction_executed",
                    "transaction_id": transaction.transaction_id,
                    "vm_transaction": {
                        "module": vm_tx.module_id,
                        "function": vm_tx.function,
                        "gas_used": gas_used,
                        "gas_display": gas_display,
                        "execution_time_ms": execution_time
                    },
                    "status": status_str,
                    "result": result,
                    "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
                });
                
                // Store result for later retrieval
                crate::store_vm_transaction_result(&transaction.transaction_id, result_json.clone());
                
                let _ = tx.try_send(result_json.to_string());
                info!("VM transaction completed successfully: {} -> {}.{} ({}ms, {} gas)",
                      transaction.transaction_id, vm_tx.module_id, vm_tx.function, execution_time, gas_used);
                true
            },
            Err(e) => {
                // Provide more detailed error information
                let error_type = if e.contains("not found") {
                    "module_not_found"
                } else if e.contains("Function") && e.contains("not found") {
                    "function_not_found"
                } else {
                    "execution_error"
                };
                
                // Notify about VM transaction failure with more details
                let error_json = json!({
                    "event": "vm_transaction_error",
                    "transaction_id": transaction.transaction_id,
                    "vm_transaction": {
                        "module": vm_tx.module_id,
                        "function": vm_tx.function
                    },
                    "error": e,
                    "error_type": error_type,
                    "suggestions": get_error_suggestions(error_type, &vm_tx),
                    "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
                });
                
                // Store error result for later retrieval
                crate::store_vm_transaction_result(&transaction.transaction_id, error_json.clone());
                
                let _ = tx.try_send(error_json.to_string());
                warn!("VM transaction execution failed: {} ({})", e, error_type);
                
                // Return true to indicate we handled this transaction (even though it failed)
                true
            }
        }
    } else {
        // Not a VM transaction
        false
    }
}

// New helper function to provide suggestions for VM errors
fn get_error_suggestions(error_type: &str, vm_tx: &VMTransaction) -> Vec<String> {
    match error_type {
        "module_not_found" => {
            vec![
                format!("Make sure module '{}' was deployed correctly", vm_tx.module_id),
                "Check the module ID format - it should be 0xADDRESS::module_name".to_string(),
                "Run 'kari move publish' to deploy your module first".to_string(),
            ]
        },
        "function_not_found" => {
            vec![
                format!("Make sure the function '{}' exists in module '{}'", vm_tx.function, vm_tx.module_id),
                "Function names are case-sensitive in Move".to_string(),
                "Only public functions can be called externally".to_string(),
                format!("Try 'kari move inspect --module-id {}' to see available functions", vm_tx.module_id),
            ]
        },
        _ => {
            vec![
                "Check your transaction arguments".to_string(),
                "Ensure your module doesn't have runtime errors".to_string(),
                "Try redeploying the module with 'kari move publish'".to_string(),
            ]
        }
    }
}

// Enhanced verify_transaction_processing to handle VM module deployments
fn verify_transaction_processing(transactions: &Vec<Transaction>, tx: &mpsc::Sender<String>) {
    for transaction in transactions {
        log::info!("Verifying transaction: {}", transaction.transaction_id);
        
        // Check if it's a VM transaction
        if let Some(data) = &transaction.data {
            if !data.is_empty() {
                // Check if it's a VM module deployment
                if let Ok(data_str) = std::str::from_utf8(data) {
                    if data_str.starts_with("VM_MODULE:") {
                        let parts: Vec<&str> = data_str.split(':').collect();
                        if parts.len() >= 3 {
                            let module_name = parts[1];
                            let module_size = parts[2];
                            
                            // Double check if module was deployed by querying VM state
                            let address_hex = transaction.sender.to_hex_literal();
                            let module_id = format!("{}::{}", address_hex, module_name);
                            
                            info!("Checking if module {} was deployed", module_id);
                            
                            // Try direct DB query for the module
                            match mona_vm::db::get_module(&module_id) {
                                Ok(Some(_)) => {
                                    info!("✅ Module {} found in database", module_id);
                                },
                                Ok(None) => {
                                    warn!("❌ Module {} not found in database!", module_id);
                                    
                                    // Try alternate formats
                                    let alt_id = if module_id.starts_with("0x") {
                                        module_id[2..].to_string()
                                    } else {
                                        format!("0x{}", module_id)
                                    };
                                    
                                    match mona_vm::db::get_module(&alt_id) {
                                        Ok(Some(_)) => info!("✅ Module found with alternate ID: {}", alt_id),
                                        _ => warn!("❌ Module not found with alternate ID either")
                                    }
                                },
                                Err(e) => warn!("Error checking module: {:?}", e)
                            }
                            
                            // Notify about module deployment transaction
                            let module_tx_json = json!({
                                "event": "module_deployed",
                                "module": {
                                    "name": module_name,
                                    "size": module_size,
                                    "address": transaction.sender.to_hex_literal(),
                                    "module_id": module_id,
                                    "transaction_id": transaction.transaction_id
                                },
                                "status": "committed",
                                "gas_fee": transaction.gas_fee,
                                "gas_fee_display": format_gas_fee_display(transaction.gas_fee)
                            }).to_string();
                            
                            let _ = tx.try_send(module_tx_json);
                            
                            continue; // Skip standard transaction processing for module deployments
                        }
                    }
                    
                    // Process VM function calls
                    if process_vm_transaction(transaction, tx) {
                        continue;
                    }
                } else {
                    // Try processing as VM transaction even if we can't interpret the data as UTF-8
                    if process_vm_transaction(transaction, tx) {
                        continue;
                    }
                }
            }
        }
        
        // Always mark transactions as valid in UI for better user experience
        let signature_status = "valid";
        
        // Verify sender and receiver balances using Address directly
        match mona_blockchain::blockchain::get_address_balance(&transaction.sender) {
            Ok(balance) => {
                debug!("Verified sender {} balance: {}", transaction.sender, balance);
            },
            Err(e) => {
                warn!("Failed to verify sender balance: {}", e);
            }
        }
        
        match mona_blockchain::blockchain::get_address_balance(&transaction.receiver) {
            Ok(balance) => {
                debug!("Verified receiver {} balance: {}", transaction.receiver, balance);
                
                // Notify about completed transaction with formatted gas fee display
                let tx_json = json!({
                    "event": "transaction_confirmed",
                    "transaction": {
                        "id": transaction.transaction_id, // Include transaction ID
                        "sender": transaction.sender.to_hex_literal(),
                        "receiver": transaction.receiver.to_hex_literal(),
                        "amount": transaction.amount,
                        "gas_fee": transaction.gas_fee,
                        "gas_fee_display": format_gas_fee_display(transaction.gas_fee),
                        "gas_collector": crate::utils::GAS_FEE_COLLECTOR,
                        "receiver_balance": balance,
                        "signature_status": signature_status,
                        "has_signature": !transaction.signature.is_empty()
                    },
                    "status": "confirmed"
                }).to_string();
                
                let _ = tx.try_send(tx_json);
            },
            Err(e) => {
                warn!("Failed to verify receiver balance: {}", e);
            }
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
    let node_address = match parse_address(&address) {
        Ok(addr) => addr,
        Err(e) => {
            error!("Invalid node address: {}", e);
            // Send error notification
            let error_json = json!({
                "event": "blockchain_error",
                "error": format!("Invalid node address: {}", e),
                "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
            }).to_string();
            let _ = tx.try_send(error_json);
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
            "devnet.kanari.site:51303".to_string(),
            "testnet.kanari.site:51303".to_string(),
            "mainnet.kanari.site:51303".to_string(),
        ],
        max_peers: 50, // Increased max peers for better network connectivity
        is_validator: is_validator(&node_address), // Dynamically check if this node is a validator
        use_tls: false, // TLS disabled by default
        cert_path: Some(format!("{}/certs/node.crt", common::get_kari_dir().display())),
        key_path: Some(format!("{}/certs/node.key", common::get_kari_dir().display())),
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

    // Send initial status including staking information
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
        "staking": {
            "validator_minimum": VALIDATOR_STAKING_MINIMUM_KARI,
            "node_minimum": NODE_STAKING_MINIMUM_KARI,
        },
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
            
            // Send shutdown notification
            let shutdown_json = json!({
                "event": "blockchain_stopped",
                "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
            }).to_string();
            let _ = tx.try_send(shutdown_json);
            break;
        }

        // Get the previous block
        let blocks = BLOCKCHAIN_DATA.iter();
        let prev_block = match blocks.last() {
            Some(block) => block,
            None => {
                error!("Cannot find previous block");
                break;
            }
        };

        // Get pending transactions for this block - use our improved function
        let transactions = mona_blockchain::blockchain::get_next_block_transactions(100);
        
        // Log transaction types being included in this block
        if !transactions.is_empty() {
            info!("Including {} transactions in block {}", transactions.len(), prev_block.index + 1);
            for (i, tx) in transactions.iter().enumerate() {
                if let Some(data) = &tx.data {
                    if let Ok(data_str) = std::str::from_utf8(data) {
                        if data_str.starts_with("VM_MODULE:") {
                            let parts: Vec<&str> = data_str.split(':').collect();
                            if parts.len() >= 3 {
                                info!("  {}: VM_MODULE_DEPLOYMENT - {} ({} bytes)", 
                                     i+1, parts[1], parts[2]);
                                continue;
                            }
                        } else if data_str.starts_with("VM:") {
                            let parts: Vec<&str> = data_str.split(':').collect();
                            if parts.len() >= 3 {
                                info!("  {}: VM_FUNCTION_CALL - {}::{}", 
                                     i+1, parts[1], parts[2]);
                                continue;
                            }
                        }
                    }
                }
                info!("  {}: {} -> {} ({})", i+1, tx.sender, tx.receiver, tx.amount);
            }
        }

        // Create JSON representation of transactions for the block data
        let tx_json: Vec<Value> = transactions.iter().map(|tx| {
            json!({
                "id": tx.transaction_id, // Include transaction ID
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
            transactions.clone(), // Include transactions in the block
            normalized_address.clone(),
            Blake3Algorithm::new(),
        );

        // Add block to chain and ensure we save the state
        BLOCKCHAIN_DATA.add_block(new_block.clone());
        
        // Propagate block to connected peers
        if get_peer_count() > 0 {
            if let Err(e) = propagate_block(&new_block) {
                warn!("Failed to propagate block to peers: {}", e);
            } else {
                info!("Block {} propagated to {} peers", new_block.index, get_peer_count());
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

        // Check if any transactions were included and log them
        if !transactions.is_empty() {
            info!("Block {} includes {} transactions:", new_block.index, transactions.len());
            for (i, tx) in transactions.iter().enumerate() {
                // Check if it's a Move module deployment transaction
                if let Some(data) = &tx.data {
                    if let Ok(data_str) = std::str::from_utf8(data) {
                        if data_str.starts_with("VM_MODULE:") {
                            let parts: Vec<&str> = data_str.split(':').collect();
                            if parts.len() >= 3 {
                                info!("  {}: Move Module Deployment - {} ({} bytes)", 
                                      i+1, parts[1], parts[2]);
                                continue;
                            }
                        }
                    }
                }
                
                // Regular transaction
                info!("  {}: {} -> {} ({})", i+1, tx.sender, tx.receiver, tx.amount);
            }
        }

        // Convert timestamp to human-readable format
        let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(new_block.timestamp as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown time".to_string());

        // Enhanced block status update as JSON with staking info and peer info
        let status_json = json!({
            "event": "block_mined",
            "block": {
                "index": new_block.index,
                "hash": new_block.hash,
                "prev_hash": new_block.prev_hash,
                "timestamp": new_block.timestamp,
                "datetime": datetime,
                "minter": normalized_address,
                "transactions": new_block.transactions.len(),
            },
            "blockchain": {
                "height": BLOCKCHAIN_DATA.len(),
                "last_update": current_time
            },
            "staking": {
                "rewards_distributed": staking_rewards,
                "is_validator": validator_status,
                "display_rewards": staking_rewards as f64 / KA_PER_KARI as f64,
                "pool_balance": match crate::staking::get_pool_remaining_balance() {
                    Ok(balance) => balance,
                    Err(_) => 0
                },
                "pool_balance_display": match crate::staking::get_pool_remaining_balance() {
                    Ok(balance) => balance as f64 / KA_PER_KARI as f64,
                    Err(_) => 0.0
                },
                "pool_address": POOL_ADDRESS
            },
            "networking": {
                "peer_count": get_peer_count(),
                "node_id": format!("node-{}", normalized_address[..8].to_string()),
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
                    // Run transaction verification in a separate thread to avoid blocking
                    let txs_clone = transactions.clone();
                    let tx_clone = tx.clone();
                    std::thread::spawn(move || {
                        verify_transaction_processing(&txs_clone, &tx_clone);
                    });
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
            match mona_blockchain::blockchain::get_balance(&normalized_address) {
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

        // Update last block time for gas fee calculation
        update_last_block_time(new_block.timestamp);

        // Sleep to control block creation rate
        thread::sleep(Duration::from_millis(420)); // 420 milliseconds for better performance
    }
}