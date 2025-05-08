use jsonrpc_core::{Error, ErrorCode, Result as JsonRpcResult, Value};
use serde::{Deserialize, Serialize};

use mona_vm::{
    db,
    VM_STATE,
    VMTransaction,
    execute_vm_transaction
};
use move_core_types::account_address::AccountAddress;

const DEFAULT_LIMIT: usize = 10;
const MAX_LIMIT: usize = 100;

#[derive(Debug, Serialize, Deserialize)]
struct ListModulesParams {
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GetModuleParams {
    module_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GetModuleTransactionsParams {
    module_id: String,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExecuteFunctionParams {
    module_id: String,
    function: String,
    args: Option<Vec<Value>>,
    sender: Option<String>,
    gas_budget: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GetTransactionParams {
    tx_id: String,
}

/// List deployed Move modules with pagination
pub fn list_modules(params: jsonrpc_core::Params) -> JsonRpcResult<Value> {
    let params: ListModulesParams = parse_params(params)?;
    
    // Apply defaults and limits for pagination
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
    let offset = params.offset.unwrap_or(0);
    
    // Call the database function to get the actual modules
    match db::list_modules(limit, offset) {
        Ok(modules) => {
            let module_data: Vec<serde_json::Value> = modules.into_iter()
                .map(|module| {
                    // Parse the ABI JSON string to extract metadata
                    let abi = serde_json::from_str(&module.abi)
                        .unwrap_or_else(|_| serde_json::json!({}));
                    
                    // Extract public functions from ABI for easy access
                    let public_functions = if let Some(funcs) = abi.get("public_functions") {
                        funcs.clone()
                    } else {
                        serde_json::json!([])
                    };
                    
                    // Format each module as a JSON object with important fields
                    serde_json::json!({
                        "module_id": module.module_id,
                        "address": module.address,
                        "name": module.name,
                        "bytecode_size": module.bytecode.len(),
                        "deploy_tx_id": module.deploy_tx_id,
                        "deploy_block_height": module.deploy_block_height,
                        "deploy_time": module.deploy_time,
                        "public_functions": public_functions
                    })
                })
                .collect();
            
            // Return the formatted response with pagination info
            Ok(serde_json::json!({
                "modules": module_data,
                "total": module_data.len(),
                "limit": limit,
                "offset": offset
            }))
        },
        Err(e) => Err(internal_error(format!("Failed to list modules: {:?}", e)))
    }
}

/// Get details of a specific Move module
pub fn get_module(params: jsonrpc_core::Params) -> JsonRpcResult<Value> {
    let params: GetModuleParams = parse_params(params)?;
    
    match db::get_module(&params.module_id) {
        Ok(Some(module)) => {
            // Parse the ABI JSON string
            let abi = serde_json::from_str(&module.abi)
                .unwrap_or_else(|_| serde_json::json!({}));
            
            // Extract public functions
            let public_functions = if let Some(funcs) = abi.get("public_functions") {
                funcs.clone()
            } else {
                serde_json::json!([])
            };
            
            Ok(serde_json::json!({
                "module_id": module.module_id,
                "address": module.address,
                "name": module.name,
                "bytecode_size": module.bytecode.len(),
                "deploy_tx_id": module.deploy_tx_id,
                "deploy_block_height": module.deploy_block_height,
                "deploy_time": module.deploy_time,
                "abi": abi,
                "public_functions": public_functions
            }))
        },
        Ok(None) => Err(not_found_error(format!("Module not found: {}", params.module_id))),
        Err(e) => Err(internal_error(format!("Failed to get module: {:?}", e)))
    }
}

/// Get transactions related to a specific module
pub fn get_module_transactions(params: jsonrpc_core::Params) -> JsonRpcResult<Value> {
    let params: GetModuleTransactionsParams = parse_params(params)?;
    
    // Apply defaults and limits
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
    let offset = params.offset.unwrap_or(0);
    
    match db::get_module_transactions(&params.module_id, limit, offset) {
        Ok(transactions) => {
            let tx_data: Vec<serde_json::Value> = transactions.into_iter()
                .map(|tx| {
                    // Parse the result JSON string
                    let result = serde_json::from_str(&tx.result_data)
                        .unwrap_or_else(|_| serde_json::json!({}));
                    
                    // Format arguments for display
                    let formatted_args = tx.args.iter()
                        .map(|arg| format!("0x{}", hex::encode(arg)))
                        .collect::<Vec<String>>();
                    
                    serde_json::json!({
                        "tx_id": tx.tx_id,
                        "module_id": tx.module_id,
                        "function": tx.function,
                        "args": formatted_args,
                        "sender": tx.sender,
                        "gas_used": tx.gas_used,
                        "success": tx.success,
                        "block_height": tx.block_height,
                        "timestamp": tx.timestamp,
                        "result": result
                    })
                })
                .collect();
            
            Ok(serde_json::json!({
                "transactions": tx_data,
                "total": tx_data.len(),
                "limit": limit,
                "offset": offset,
                "module_id": params.module_id
            }))
        },
        Err(e) => Err(internal_error(format!("Failed to get module transactions: {:?}", e)))
    }
}

/// Execute a function in a Move module
pub fn execute_function(params: jsonrpc_core::Params) -> JsonRpcResult<Value> {
    let params: ExecuteFunctionParams = parse_params(params)?;
    
    // Parse arguments
    let args = match parse_function_args(params.args) {
        Ok(a) => a,
        Err(e) => return Err(invalid_params_error(format!("Invalid arguments: {}", e)))
    };
    
    // Use default sender address if not specified
    let sender = params.sender.unwrap_or_else(|| "0x1".to_string());
    
    // Use default gas budget if not specified
    let gas_budget = params.gas_budget.unwrap_or(1_000_000);
    
    // Create a VM transaction
    let vm_tx = VMTransaction::new(
        sender,
        params.module_id,
        params.function,
        args,
        gas_budget
    );
    
    // Execute the transaction
    match execute_vm_transaction(&vm_tx) {
        Ok(result) => Ok(result),
        Err(e) => Err(execution_error(format!("Function execution failed: {}", e)))
    }
}

/// Get details of a specific transaction
pub fn get_transaction(params: jsonrpc_core::Params) -> JsonRpcResult<Value> {
    let params: GetTransactionParams = parse_params(params)?;
    
    match db::get_transaction(&params.tx_id) {
        Ok(Some(tx)) => {
            // Parse the result JSON string
            let result = serde_json::from_str(&tx.result_data)
                .unwrap_or_else(|_| serde_json::json!({}));
            
            // Format arguments for display
            let formatted_args = tx.args.iter()
                .map(|arg| format!("0x{}", hex::encode(arg)))
                .collect::<Vec<String>>();
            
            Ok(serde_json::json!({
                "tx_id": tx.tx_id,
                "module_id": tx.module_id,
                "function": tx.function,
                "args": formatted_args,
                "sender": tx.sender,
                "gas_used": tx.gas_used,
                "success": tx.success,
                "block_height": tx.block_height,
                "timestamp": tx.timestamp,
                "result": result
            }))
        },
        Ok(None) => Err(not_found_error(format!("Transaction not found: {}", params.tx_id))),
        Err(e) => Err(internal_error(format!("Failed to get transaction: {:?}", e)))
    }
}

/// Get VM state information
pub fn get_vm_state(_params: jsonrpc_core::Params) -> JsonRpcResult<Value> {
    match VM_STATE.try_read() {
        Ok(state) => {
            let modules_count = state.modules.len();
            let registered_modules: Vec<String> = state.modules.keys()
                .take(10) // Limit to first 10 for display
                .cloned()
                .collect();
            
            Ok(serde_json::json!({
                "modules_count": modules_count, 
                "last_execution": state.last_execution,
                "execution_count": state.execution_count,
                "sample_modules": registered_modules,
                "last_signer": state.last_signer.clone().unwrap_or_default()
            }))
        },
        Err(e) => Err(internal_error(format!("Failed to access VM state: {}", e)))
    }
}

// Helper functions

fn parse_params<T>(params: jsonrpc_core::Params) -> Result<T, Error> 
where
    T: serde::de::DeserializeOwned,
{
    params.parse().map_err(|e| {
        Error {
            code: ErrorCode::InvalidParams,
            message: format!("Invalid parameters: {}", e),
            data: None,
        }
    })
}

fn parse_function_args(args_opt: Option<Vec<Value>>) -> Result<Vec<Vec<u8>>, String> {
    let mut parsed_args = Vec::new();
    
    // If no args provided, return empty vec
    let args = match args_opt {
        Some(a) => a,
        None => return Ok(Vec::new()),
    };
    
    for arg in args {
        match arg {
            // Handle address type (string)
            Value::String(s) if s.starts_with("0x") => {
                match AccountAddress::from_hex_literal(&s) {
                    Ok(addr) => parsed_args.push(addr.to_vec()),
                    Err(_) => return Err(format!("Invalid address format: {}", s))
                }
            },
            // Handle string type
            Value::String(s) => {
                parsed_args.push(s.as_bytes().to_vec());
            },
            // Handle integer types
            Value::Number(n) => {
                if let Some(i) = n.as_u64() {
                    parsed_args.push(i.to_le_bytes().to_vec());
                } else if let Some(i) = n.as_i64() {
                    parsed_args.push(i.to_le_bytes().to_vec());
                } else {
                    return Err(format!("Unsupported number format: {}", n));
                }
            },
            // Handle boolean
            Value::Bool(b) => {
                parsed_args.push(vec![if b { 1 } else { 0 }]);
            },
            // Handle null (empty vector)
            Value::Null => {
                parsed_args.push(Vec::new());
            },
            // Unsupported types
            _ => return Err(format!("Unsupported argument type: {:?}", arg))
        }
    }
    
    Ok(parsed_args)
}

fn internal_error(msg: String) -> Error {
    Error {
        code: ErrorCode::InternalError,
        message: msg,
        data: None,
    }
}

fn invalid_params_error(msg: String) -> Error {
    Error {
        code: ErrorCode::InvalidParams,
        message: msg,
        data: None,
    }
}

fn not_found_error(msg: String) -> Error {
    Error {
        code: ErrorCode::ServerError(-32004),
        message: msg,
        data: None,
    }
}

fn execution_error(msg: String) -> Error {
    Error {
        code: ErrorCode::ServerError(-32005),
        message: msg,
        data: None,
    }
}
