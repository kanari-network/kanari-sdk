use jsonrpc_core::{Error, ErrorCode, Result as JsonRpcResult, Value};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

use mona_vm::{
    VM_STATE, VMTransaction, execute_vm_transaction,
};
use mona_vm::db::{get_transaction, get_module_transactions, 
                  get_transactions_by_sender, list_modules, get_module_count, get_transaction_count};

use move_core_types::account_address::AccountAddress;
use log::{error, debug};

// =====================================
// Move API RPC Methods
// =====================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteParams {
    pub module_id: String,
    pub function: String,
    #[serde(default)]
    pub args: Vec<JsonValue>,
    #[serde(default = "default_gas_budget")]
    pub gas_budget: u64,
    #[serde(default)]
    pub sender: Option<String>,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub signer_address: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleParams {
    pub module_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListModulesParams {
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_gas_budget() -> u64 {
    1_000_000 // 1M gas units
}

fn default_limit() -> usize {
    100
}

/// Execute a function in a Move module
pub fn vm_execute(params: jsonrpc_core::Params) -> JsonRpcResult<Value> {
    let params: ExecuteParams = params.parse()?;

    // Parse arguments into correct format
    let mut parsed_args = Vec::new();
    for arg in params.args {
        match arg {
            JsonValue::String(s) => {
                if s.starts_with("0x") {
                    // Address argument
                    match AccountAddress::from_hex_literal(&s) {
                        Ok(addr) => parsed_args.push(addr.to_vec()),
                        Err(_) => {
                            match AccountAddress::from_hex(s.trim_start_matches("0x")) {
                                Ok(addr) => parsed_args.push(addr.to_vec()),
                                Err(_) => return Err(Error {
                                    code: ErrorCode::InvalidParams,
                                    message: "Invalid address argument".into(),
                                    data: None,
                                }),
                            }
                        }
                    }
                } else {
                    // Regular string
                    parsed_args.push(s.as_bytes().to_vec());
                }
            },
            JsonValue::Number(n) => {
                if let Some(n_u64) = n.as_u64() {
                    parsed_args.push(n_u64.to_le_bytes().to_vec());
                } else {
                    return Err(Error {
                        code: ErrorCode::InvalidParams,
                        message: "Unsupported number type".into(),
                        data: None,
                    });
                }
            },
            JsonValue::Bool(b) => {
                parsed_args.push(vec![if b { 1 } else { 0 }]);
            },
            _ => return Err(Error {
                code: ErrorCode::InvalidParams,
                message: "Unsupported argument type".into(),
                data: None,
            }),
        }
    }

    // Set up sender address
    let sender = params.sender.unwrap_or_else(|| "0x1".to_string());
    
    // Create VM transaction
    let mut vm_tx = VMTransaction::new(
        sender,
        params.module_id,
        params.function,
        parsed_args,
        params.gas_budget
    );
    
    // Add signature if provided
    if let (Some(sig_hex), Some(signer)) = (params.signature, params.signer_address) {
        match hex::decode(sig_hex) {
            Ok(signature) => {
                vm_tx = vm_tx.with_signature(signature, signer);
            },
            Err(_) => return Err(Error {
                code: ErrorCode::InvalidParams,
                message: "Invalid signature format".into(),
                data: None,
            }),
        }
    }

    debug!("Executing VM transaction: [module_id={}, function={}]", vm_tx.module_id, vm_tx.function);
    
    // Execute the transaction
    match execute_vm_transaction(&vm_tx) {
        Ok(result) => Ok(result.into()),
        Err(e) => {
            error!("VM execution error: {}", e);
            Err(Error {
                code: ErrorCode::InternalError,
                message: format!("VM execution failed: {}", e),
                data: None,
            })
        }
    }
}

/// Get information about a specific module
pub fn vm_get_module(params: jsonrpc_core::Params) -> JsonRpcResult<Value> {
    let params: ModuleParams = params.parse()?;
    
    // Get VM state
    let vm_state = match VM_STATE.read() {
        Ok(state) => state,
        Err(e) => {
            error!("Failed to read VM state: {}", e);
            return Err(Error {
                code: ErrorCode::InternalError,
                message: "Failed to access VM state".into(),
                data: None,
            });
        }
    };
    
    // Find the module
    let module = match vm_state.modules.get(&params.module_id) {
        Some(module) => module,
        None => {
            return Err(Error {
                code: ErrorCode::InvalidParams,
                message: format!("Module {} not found", params.module_id),
                data: None,
            });
        }
    };
    
    // Convert module to JSON response
    let response = json!({
        "status": "success",
        "module": {
            "module_id": module.module_id,
            "address": format!("0x{}", module.address.to_hex()),
            "name": module.name,
            "deploy_block_height": module.deploy_block_height,
            "bytecode_size": module.bytecode.len(),
            "public_functions": module.public_functions,
        }
    });
    
    Ok(response.into())
}

/// List all modules, optionally filtered by address
pub fn vm_list_modules(params: jsonrpc_core::Params) -> JsonRpcResult<Value> {
    let params: ListModulesParams = params.parse().unwrap_or_else(|_| ListModulesParams {
        address: None,
        limit: default_limit(),
        offset: 0,
    });
    
    // Get VM state
    let vm_state = match VM_STATE.read() {
        Ok(state) => state,
        Err(e) => {
            error!("Failed to read VM state: {}", e);
            return Err(Error {
                code: ErrorCode::InternalError,
                message: "Failed to access VM state".into(),
                data: None,
            });
        }
    };
    
    // Filter modules by address if provided
    let mut modules = Vec::new();
    let address_filter = params.address.as_deref();
    
    for module in vm_state.modules.values() {
        if let Some(addr) = address_filter {
            let module_addr = format!("0x{}", module.address.to_hex());
            if !module_addr.eq_ignore_ascii_case(addr) {
                continue;
            }
        }
        
        modules.push(json!({
            "module_id": module.module_id,
            "address": format!("0x{}", module.address.to_hex()),
            "name": module.name,
            "deploy_block_height": module.deploy_block_height,
            "public_functions": module.public_functions,
        }));
    }
    
    // Apply pagination
    let total = modules.len();
    let modules = modules
        .into_iter()
        .skip(params.offset)
        .take(params.limit)
        .collect::<Vec<_>>();
    
    // Build response
    let response = json!({
        "status": "success",
        "modules": modules,
        "total": total,
        "limit": params.limit,
        "offset": params.offset
    });
    
    Ok(response.into())
}

// Add new methods for database interaction
#[derive(Debug, Serialize, Deserialize)]
pub struct GetModuleHistoryParams {
    pub module_id: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetAddressHistoryParams {
    pub address: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTransactionParams {
    pub tx_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetModulesParams {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

// Get transaction history for a specific module
pub fn vm_get_module_history(params: jsonrpc_core::Params) -> JsonRpcResult<Value> {
    let params: GetModuleHistoryParams = params.parse()?;
    
    // Get transaction history from database
    match get_module_transactions(&params.module_id, params.limit, params.offset) {
        Ok(txs) => {
            // Transform database transactions to JSON response
            let tx_json: Vec<JsonValue> = txs.into_iter().map(|tx| {
                json!({
                    "tx_id": tx.tx_id,
                    "module_id": tx.module_id,
                    "function": tx.function,
                    "sender": tx.sender,
                    "timestamp": tx.timestamp,
                    "gas_used": tx.gas_used,
                    "success": tx.success,
                    "block_height": tx.block_height,
                    "result_data": serde_json::from_str::<JsonValue>(&tx.result_data)
                        .unwrap_or(json!({"error": "Invalid JSON data"}))
                })
            }).collect();
            
            let response = json!({
                "status": "success",
                "module_id": params.module_id,
                "transactions": tx_json,
                "limit": params.limit,
                "offset": params.offset
            });
            
            Ok(response.into())
        },
        Err(e) => {
            error!("Failed to get module history: {:?}", e);
            Err(Error {
                code: ErrorCode::InternalError,
                message: format!("Failed to get module history: {:?}", e),
                data: None,
            })
        }
    }
}

// Get transaction history for a specific address
pub fn vm_get_address_history(params: jsonrpc_core::Params) -> JsonRpcResult<Value> {
    let params: GetAddressHistoryParams = params.parse()?;
    
    // Ensure address starts with 0x
    let address = if params.address.starts_with("0x") {
        params.address.clone()
    } else {
        format!("0x{}", params.address)
    };
    
    // Get transaction history from database
    match get_transactions_by_sender(&address, params.limit, params.offset) {
        Ok(txs) => {
            // Transform database transactions to JSON response
            let tx_json: Vec<JsonValue> = txs.into_iter().map(|tx| {
                json!({
                    "tx_id": tx.tx_id,
                    "module_id": tx.module_id,
                    "function": tx.function,
                    "sender": tx.sender,
                    "timestamp": tx.timestamp,
                    "gas_used": tx.gas_used,
                    "success": tx.success,
                    "block_height": tx.block_height,
                    "result_data": serde_json::from_str::<JsonValue>(&tx.result_data)
                        .unwrap_or(json!({"error": "Invalid JSON data"}))
                })
            }).collect();
            
            let response = json!({
                "status": "success",
                "address": address,
                "transactions": tx_json,
                "limit": params.limit,
                "offset": params.offset
            });
            
            Ok(response.into())
        },
        Err(e) => {
            error!("Failed to get address history: {:?}", e);
            Err(Error {
                code: ErrorCode::InternalError,
                message: format!("Failed to get address history: {:?}", e),
                data: None,
            })
        }
    }
}

// Get specific transaction details
pub fn vm_get_transaction(params: jsonrpc_core::Params) -> JsonRpcResult<Value> {
    let params: GetTransactionParams = params.parse()?;
    
    // Get transaction from database
    match get_transaction(&params.tx_id) {
        Ok(Some(tx)) => {
            let result_data = serde_json::from_str::<JsonValue>(&tx.result_data)
                .unwrap_or(json!({"error": "Invalid JSON data"}));
            
            let response = json!({
                "status": "success",
                "transaction": {
                    "tx_id": tx.tx_id,
                    "module_id": tx.module_id,
                    "function": tx.function,
                    "sender": tx.sender,
                    "timestamp": tx.timestamp,
                    "gas_used": tx.gas_used,
                    "success": tx.success,
                    "block_height": tx.block_height,
                    "result": result_data
                }
            });
            
            Ok(response.into())
        },
        Ok(None) => {
            Err(Error {
                code: ErrorCode::InvalidParams,
                message: format!("Transaction {} not found", params.tx_id),
                data: None,
            })
        },
        Err(e) => {
            error!("Failed to get transaction: {:?}", e);
            Err(Error {
                code: ErrorCode::InternalError,
                message: format!("Failed to get transaction: {:?}", e),
                data: None,
            })
        }
    }
}

// Get all deployed modules
pub fn vm_get_all_modules(params: jsonrpc_core::Params) -> JsonRpcResult<Value> {
    let params: GetModulesParams = params.parse().unwrap_or_else(|_| GetModulesParams {
        limit: default_limit(),
        offset: 0,
    });
    
    // Get modules from database
    match list_modules(params.limit, params.offset) {
        Ok(modules) => {
            // Transform database modules to JSON response
            let modules_json: Vec<JsonValue> = modules.into_iter().map(|m| {
                let abi = serde_json::from_str::<JsonValue>(&m.abi)
                    .unwrap_or(json!({"error": "Invalid ABI JSON"}));
                
                json!({
                    "module_id": m.module_id,
                    "address": m.address,
                    "name": m.name,
                    "bytecode_size": m.bytecode.len(),
                    "deploy_time": m.deploy_time,
                    "deploy_tx_id": m.deploy_tx_id,
                    "deploy_block_height": m.deploy_block_height,
                    "abi": abi
                })
            }).collect();
            
            // Get total count
            let total = match get_module_count() {
                Ok(count) => count,
                Err(_) => modules_json.len(),
            };
            
            let response = json!({
                "status": "success",
                "modules": modules_json,
                "total": total,
                "limit": params.limit,
                "offset": params.offset
            });
            
            Ok(response.into())
        },
        Err(e) => {
            error!("Failed to get modules: {:?}", e);
            Err(Error {
                code: ErrorCode::InternalError,
                message: format!("Failed to get modules: {:?}", e),
                data: None,
            })
        }
    }
}

// Get stats about VM usage
pub fn vm_get_stats(_params: jsonrpc_core::Params) -> JsonRpcResult<Value> {
    let module_count = get_module_count().unwrap_or(0);
    let transaction_count = get_transaction_count().unwrap_or(0);
    
    // Get VM state information
    let vm_state = match VM_STATE.read() {
        Ok(state) => state,
        Err(e) => {
            error!("Failed to read VM state: {}", e);
            return Err(Error {
                code: ErrorCode::InternalError,
                message: "Failed to access VM state".into(),
                data: None,
            });
        }
    };
    
    let memory_module_count = vm_state.modules.len();
    
    let response = json!({
        "status": "success",
        "stats": {
            "module_count": module_count,
            "transaction_count": transaction_count,
            "memory_module_count": memory_module_count,
            "vm_state": {
                "execution_count": vm_state.execution_count,
                "last_execution": vm_state.last_execution
            }
        }
    });
    
    Ok(response.into())
}
