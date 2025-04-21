use log::{debug, error, info, warn};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc;
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::ModuleId;
use move_core_types::identifier::Identifier;

use crate::blockchain::{normalize_address, BLOCKCHAIN_DATA};
use crate::block::{Block, Transaction};
use mona_types::address::Address;

/// Represents a Move module that can be deployed to the blockchain
pub struct MoveModule {
    pub id: String,
    pub name: String,
    pub bytecode: Vec<u8>,
    pub address: Address,
    pub gas_budget: u64,
}

/// Represents a Move function call that can be executed on the blockchain
pub struct MoveFunctionCall {
    pub module_id: String,
    pub function_name: String,
    pub args: Vec<Value>,
    pub sender: Address,
    pub gas_budget: u64,
}

/// Result of a Move module deployment
pub struct DeploymentResult {
    pub transaction_id: String,
    pub status: String,
    pub gas_used: u64,
    pub block_height: u64,
}

/// Result of a Move function call
pub struct FunctionCallResult {
    pub transaction_id: String,
    pub status: String,
    pub gas_used: u64,
    pub result: Value,
    pub block_height: u64,
}

/// Deploys a Move module to the blockchain
pub fn deploy_move_module(
    module: MoveModule,
    tx: &mpsc::Sender<String>,
) -> Result<DeploymentResult, String> {
    info!(
        "Deploying Move module '{}' to address {}",
        module.name, module.address
    );

    // Create transaction data
    let module_data = json!({
        "transaction_type": "module_publish",
        "module_name": module.name,
        "module_id": module.id,
        "bytecode_length": module.bytecode.len(),
        "gas_budget": module.gas_budget,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    });

    // Create a special transaction for the module deployment
    let transaction = Transaction {
        transaction_id: format!("move_publish_{}", generate_random_id()),
        sender: module.address.clone(),
        receiver: module.address.clone(), // Module publisher is also the receiver
        amount: 0, // No tokens transferred in module publish
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        gas_fee: module.gas_budget,
        signature: Vec::new(), // No signature for system transactions
        data: Some(serde_json::to_vec(&module_data).unwrap_or_default()),
    };

    // Get the previous block
    let blocks = BLOCKCHAIN_DATA.iter();
    let prev_block = match blocks.last() {
        Some(block) => block,
        None => {
            error!("Cannot find previous block for module deployment");
            return Err("Blockchain not initialized".to_string());
        }
    };

    // Create new block with the module transaction
    let mut transactions = Vec::new();
    transactions.push(transaction.clone());

    // Add module bytecode to block data for verification
    let block_data = json!({
        "block_type": "move_module_publish",
        "timestamp": transaction.timestamp,
        "module": {
            "id": module.id,
            "name": module.name,
            "address": module.address.to_hex_literal(),
            "bytecode_length": module.bytecode.len(),
        },
        "transaction": {
            "id": transaction.transaction_id,
            "gas_fee": transaction.gas_fee,
            "gas_fee_display": crate::utils::format_gas_fee_display(transaction.gas_fee),
        }
    })
    .to_string()
    .into_bytes();

    // Create the block
    let new_block = Block::new(
        prev_block.index + 1,
        block_data,
        prev_block.hash.clone(),
        0, // No new tokens
        transactions,
        "system".to_string(),
        consensus_pos::Blake3Algorithm::new(),
    );

    // Add block to chain
    BLOCKCHAIN_DATA.add_block(new_block.clone());

    // Save blockchain state
    match crate::blockchain::save_blockchain() {
        Ok(_) => {
            info!("Move module deployment block created and saved");
            
            // Notify about the deployment
            let deployment_json = json!({
                "event": "move_module_deployed",
                "module": {
                    "id": module.id,
                    "name": module.name,
                    "address": module.address.to_hex_literal(),
                    "bytecode_length": module.bytecode.len(),
                },
                "transaction": {
                    "id": transaction.transaction_id,
                    "gas_fee": transaction.gas_fee,
                    "gas_fee_display": crate::utils::format_gas_fee_display(transaction.gas_fee),
                },
                "block": {
                    "index": new_block.index,
                    "hash": new_block.hash,
                }
            }).to_string();
            
            let _ = tx.try_send(deployment_json);
            
            Ok(DeploymentResult {
                transaction_id: transaction.transaction_id,
                status: "COMMITTED".to_string(),
                gas_used: transaction.gas_fee,
                block_height: new_block.index as u64, // Convert u32 to u64
            })
        },
        Err(e) => {
            error!("Failed to save blockchain after module deployment: {}", e);
            Err(format!("Failed to save blockchain: {}", e))
        }
    }
}

/// Executes a Move function on the blockchain
pub fn execute_move_function(
    function_call: MoveFunctionCall,
    tx: &mpsc::Sender<String>,
) -> Result<FunctionCallResult, String> {
    info!(
        "Executing Move function '{}' from module '{}'",
        function_call.function_name, function_call.module_id
    );
    
    // Create transaction data
    let function_data = json!({
        "transaction_type": "move_function_call",
        "module_id": function_call.module_id,
        "function_name": function_call.function_name,
        "args": function_call.args,
        "gas_budget": function_call.gas_budget,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    });
    
    // Create a special transaction for the function call
    let transaction = Transaction {
        transaction_id: format!("move_call_{}", generate_random_id()),
        sender: function_call.sender.clone(),
        receiver: Address::from_str("0x1").unwrap(), // System address for function calls
        amount: 0, // No tokens transferred in function calls
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        gas_fee: function_call.gas_budget,
        signature: Vec::new(), // No signature for system transactions
        data: Some(serde_json::to_vec(&function_data).unwrap_or_default()),
    };
    
    // Get the previous block
    let blocks = BLOCKCHAIN_DATA.iter();
    let prev_block = match blocks.last() {
        Some(block) => block,
        None => {
            error!("Cannot find previous block for function execution");
            return Err("Blockchain not initialized".to_string());
        }
    };
    
    // Create new block with the function call transaction
    let mut transactions = Vec::new();
    transactions.push(transaction.clone());
    
    // Add function call details to block data
    let block_data = json!({
        "block_type": "move_function_call",
        "timestamp": transaction.timestamp,
        "function_call": {
            "module_id": function_call.module_id,
            "function_name": function_call.function_name,
            "sender": function_call.sender.to_hex_literal(),
        },
        "transaction": {
            "id": transaction.transaction_id,
            "gas_fee": transaction.gas_fee,
            "gas_fee_display": crate::utils::format_gas_fee_display(transaction.gas_fee),
        }
    })
    .to_string()
    .into_bytes();
    
    // Create the block
    let new_block = Block::new(
        prev_block.index + 1,
        block_data,
        prev_block.hash.clone(),
        0, // No new tokens
        transactions,
        "system".to_string(),
        consensus_pos::Blake3Algorithm::new(),
    );
    
    // Add block to chain
    BLOCKCHAIN_DATA.add_block(new_block.clone());
    
    // Mock function result (in a real system this would be the actual execution result)
    let result = json!({
        "status": "success",
        "return_values": ["example return value"],
        "gas_used": function_call.gas_budget / 2,
    });
    
    // Save blockchain state
    match crate::blockchain::save_blockchain() {
        Ok(_) => {
            info!("Move function execution block created and saved");
            
            // Notify about the function call
            let function_json = json!({
                "event": "move_function_executed",
                "function": {
                    "module_id": function_call.module_id,
                    "function_name": function_call.function_name,
                    "sender": function_call.sender.to_hex_literal(),
                },
                "transaction": {
                    "id": transaction.transaction_id,
                    "gas_fee": transaction.gas_fee,
                    "gas_fee_display": crate::utils::format_gas_fee_display(transaction.gas_fee),
                },
                "block": {
                    "index": new_block.index,
                    "hash": new_block.hash,
                },
                "result": result
            }).to_string();
            
            let _ = tx.try_send(function_json);
            
            Ok(FunctionCallResult {
                transaction_id: transaction.transaction_id,
                status: "COMMITTED".to_string(),
                gas_used: function_call.gas_budget / 2, // Simulate partial gas usage
                result,
                block_height: new_block.index as u64, // Convert u32 to u64
            })
        },
        Err(e) => {
            error!("Failed to save blockchain after function execution: {}", e);
            Err(format!("Failed to save blockchain: {}", e))
        }
    }
}

/// Convert between Move VM and Mona blockchain address types
pub fn convert_address(address: &AccountAddress) -> Result<Address, String> {
    let address_str = format!("0x{}", address.to_hex());
    normalize_address(&address_str).map_err(|e| e.to_string()) // Convert error type to String
}

/// Utility function to generate a random ID for transactions
fn generate_random_id() -> String {
    use rand::{thread_rng, Rng};
    
    let mut rng = thread_rng();
    const CHARSET: &[u8] = b"0123456789abcdef";
    
    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
