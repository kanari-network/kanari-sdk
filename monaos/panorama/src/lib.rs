pub mod utils;
pub mod simulation;
pub mod transfer_tokens;
pub mod config;
pub mod staking;
pub mod node;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use mona_blockchain::block::Transaction;
use lazy_static::lazy_static;
use log::{debug, error, info};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

// Transaction result cache
lazy_static! {
    static ref VM_TRANSACTION_RESULTS: Arc<Mutex<HashMap<String, Value>>> = 
        Arc::new(Mutex::new(HashMap::new()));
    
    // Add a module result cache for faster lookups
    static ref VM_MODULE_CACHE: Arc<Mutex<HashMap<String, Value>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

// Store VM transaction result
pub fn store_vm_transaction_result(tx_id: &str, result: Value) {
    if let Ok(mut results) = VM_TRANSACTION_RESULTS.lock() {
        results.insert(tx_id.to_string(), result);
        
        // Prune old results - keep only the last 100
        if results.len() > 100 {
            let oldest_keys: Vec<String> = results.keys()
                .take(results.len() - 100)
                .cloned()
                .collect();
            
            for key in oldest_keys {
                results.remove(&key);
            }
        }
        
        debug!("VM transaction result stored for {}", tx_id);
    } else {
        error!("Failed to store VM transaction result: mutex error");
    }
}

// Store VM module info for faster retrieval
pub fn cache_vm_module_info(module_id: &str, module_info: Value) {
    if let Ok(mut cache) = VM_MODULE_CACHE.lock() {
        cache.insert(module_id.to_string(), module_info);
        debug!("VM module info cached for {}", module_id);
    }
}

// Get VM module info from cache or database
pub fn get_vm_module_info(module_id: &str) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    // First check cache
    if let Ok(cache) = VM_MODULE_CACHE.lock() {
        if let Some(info) = cache.get(module_id) {
            return Ok(Some(info.clone()));
        }
    }
    
    // If not in cache, query database
    match mona_vm::db::get_module(module_id) {
        Ok(Some(module)) => {
            let abi = serde_json::from_str::<Value>(&module.abi)
                .unwrap_or_else(|_| json!({}));
            
            let info = json!({
                "module_id": module.module_id,
                "address": module.address,
                "name": module.name,
                "bytecode_size": module.bytecode.len(),
                "deploy_time": module.deploy_time,
                "deploy_block_height": module.deploy_block_height,
                "functions": abi.get("public_functions").cloned().unwrap_or(json!([])),
                "abi": abi
            });
            
            // Cache for future use
            cache_vm_module_info(&module.module_id, info.clone());
            
            Ok(Some(info))
        },
        Ok(None) => Ok(None),
        Err(e) => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other, 
            format!("Database error: {:?}", e)
        )))
    }
}

// Get result of a VM transaction
pub fn get_transaction_result(tx_id: &str) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    // First check our in-memory cache
    if let Ok(results) = VM_TRANSACTION_RESULTS.lock() {
        if let Some(result) = results.get(tx_id) {
            debug!("Found transaction result in memory cache: {}", tx_id);
            return Ok(Some(result.clone()));
        }
    }
    
    // If not in memory, try to query database
    match mona_vm::db::get_transaction(tx_id) {
        Ok(Some(tx)) => {
            info!("Found transaction in database: {}", tx_id);
            let result = match serde_json::from_str(&tx.result_data) {
                Ok(v) => v,
                Err(_) => json!({
                    "status": tx.success,
                    "raw_result": tx.result_data,
                })
            };
            
            // Store in cache for future queries
            store_vm_transaction_result(tx_id, result.clone());
            
            Ok(Some(result))
        },
        Ok(None) => {
            debug!("Transaction not found: {}", tx_id);
            Ok(None)
        },
        Err(e) => Err(format!("Failed to query transaction: {:?}", e).into())
    }
}

// Forward transaction to blockchain
pub fn submit_transaction(tx: Transaction) -> Result<(), Box<dyn std::error::Error>> {
    info!("Submitting transaction to blockchain: {}", tx.transaction_id);
    
    // Log transaction type for better debugging
    if let Some(data) = &tx.data {
        if let Ok(data_str) = std::str::from_utf8(data) {
            if data_str.starts_with("VM_MODULE:") {
                info!("Transaction type: Module deployment");
            } else if data_str.starts_with("VM:") {
                info!("Transaction type: VM function call");
            } else {
                info!("Transaction type: Regular data transaction");
            }
        } else {
            info!("Transaction type: Binary data transaction");
        }
    } else {
        info!("Transaction type: Regular transfer");
    }
    
    // Forward to mona_blockchain
    match mona_blockchain::blockchain::submit_transaction(tx) {
        Ok(_) => {
            info!("Transaction submitted successfully");
            Ok(())
        },
        Err(e) => {
            error!("Failed to submit transaction: {:?}", e);
            Err(format!("Failed to submit transaction: {:?}", e).into())
        }
    }
}

// List deployed modules
pub fn list_deployed_modules(limit: usize, offset: usize) -> Result<Value, Box<dyn std::error::Error>> {
    match mona_vm::db::list_modules(limit, offset) {
        Ok(modules) => {
            let module_list = modules.iter().map(|m| {
                json!({
                    "module_id": m.module_id,
                    "address": m.address,
                    "name": m.name,
                    "bytecode_size": m.bytecode.len(),
                    "deploy_time": m.deploy_time,
                    "deploy_block_height": m.deploy_block_height,
                    "deploy_tx_id": m.deploy_tx_id
                })
            }).collect::<Vec<_>>();
            
            let result = json!({
                "count": modules.len(),
                "modules": module_list,
                "has_more": modules.len() >= limit,
                "offset": offset,
                "limit": limit
            });
            
            Ok(result)
        },
        Err(e) => Err(format!("Failed to list modules: {:?}", e).into()),
    }
}

// Get module transactions by module ID
pub fn list_module_transactions(
    module_id: &str, 
    limit: usize, 
    offset: usize
) -> Result<Value, Box<dyn std::error::Error>> {
    match mona_vm::db::get_module_transactions(module_id, limit, offset) {
        Ok(transactions) => {
            let tx_list = transactions.iter().map(|tx| {
                let result = match serde_json::from_str::<Value>(&tx.result_data) {
                    Ok(v) => v,
                    Err(_) => json!({"status": tx.success})
                };
                
                json!({
                    "tx_id": tx.tx_id,
                    "module_id": tx.module_id,
                    "function": tx.function,
                    "sender": tx.sender,
                    "timestamp": tx.timestamp,
                    "gas_used": tx.gas_used, 
                    "success": tx.success,
                    "result": result,
                    "block_height": tx.block_height
                })
            }).collect::<Vec<_>>();
            
            let result = json!({
                "count": transactions.len(),
                "transactions": tx_list,
                "has_more": transactions.len() >= limit,
                "module_id": module_id,
                "offset": offset,
                "limit": limit
            });
            
            Ok(result)
        },
        Err(e) => Err(format!("Failed to list module transactions: {:?}", e).into()),
    }
}

// Check if a module is available
pub fn check_module_exists(module_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
    // First try direct database query
    match mona_vm::db::get_module(module_id) {
        Ok(Some(_)) => {
            info!("Module found in database: {}", module_id);
            return Ok(true);
        },
        Ok(None) => {
            // Try alternate module format (with/without 0x prefix)
            let alt_module_id = if module_id.starts_with("0x") {
                module_id[2..].to_string()
            } else {
                format!("0x{}", module_id)
            };
            
            match mona_vm::db::get_module(&alt_module_id) {
                Ok(Some(_)) => {
                    info!("Module found with alternate ID: {}", alt_module_id);
                    return Ok(true);
                },
                Ok(None) => {
                    debug!("Module not found with ID {} or {}", module_id, alt_module_id);
                    return Ok(false);
                },
                Err(e) => return Err(format!("Failed to query module with alternate ID: {:?}", e).into()),
            }
        },
        Err(e) => return Err(format!("Failed to query module: {:?}", e).into()),
    }
}

// Add a function for debugging
pub fn debug_pending_transactions() -> Result<Value, Box<dyn std::error::Error>> {
    let mut debug_info = Vec::new();
    
    // Check pending transactions in the blockchain
    if let Ok(lock) = mona_blockchain::blockchain::PENDING_TRANSACTIONS.lock() {
        for (idx, tx) in lock.iter().enumerate() {
            let tx_type = if let Some(data) = &tx.data {
                if let Ok(data_str) = std::str::from_utf8(data) {
                    if data_str.starts_with("VM_MODULE:") {
                        "VM_MODULE_DEPLOYMENT"
                    } else if data_str.starts_with("VM:") {
                        "VM_FUNCTION_CALL"
                    } else {
                        "DATA_TRANSACTION"
                    }
                } else {
                    "BINARY_DATA"
                }
            } else {
                "TRANSFER"
            };
            
            debug_info.push(json!({
                "index": idx,
                "transaction_id": tx.transaction_id,
                "type": tx_type,
                "sender": tx.sender.to_hex_literal(),
                "receiver": tx.receiver.to_hex_literal(),
                "amount": tx.amount,
                "gas_fee": tx.gas_fee,
                "timestamp": tx.timestamp,
            }));
        }
    }
    
    Ok(json!({
        "pending_transactions": debug_info,
        "count": debug_info.len(),
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }))
}