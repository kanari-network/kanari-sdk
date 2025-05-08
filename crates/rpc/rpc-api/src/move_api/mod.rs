use jsonrpc_core::{Error, ErrorCode, Result as JsonRpcResult, Value};
use serde::{Deserialize, Serialize};

use mona_vm::{
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
    
    // Get modules directly from VM_STATE
    match VM_STATE.try_read() {
        Ok(state) => {
            let modules = state.modules.values()
                .skip(offset)
                .take(limit)
                .collect::<Vec<_>>();
            
            let module_data: Vec<serde_json::Value> = modules.iter()
                .map(|module| {
                    // Format each module as a JSON object with important fields
                    serde_json::json!({
                        "module_id": module.module_id,
                        "address": format!("0x{}", module.address.to_hex()),
                        "name": module.name,
                        "bytecode_size": module.bytecode.len(),
                        "deploy_block_height": module.deploy_block_height,
                        "public_functions": module.public_functions
                    })
                })
                .collect();
            
            // Return the formatted response with pagination info
            Ok(serde_json::json!({
                "modules": module_data,
                "total": state.modules.len(),
                "limit": limit,
                "offset": offset
            }))
        },
        Err(e) => Err(internal_error(format!("Failed to access VM state: {}", e)))
    }
}

/// Get details of a specific Move module
pub fn get_module(params: jsonrpc_core::Params) -> JsonRpcResult<Value> {
    let params: GetModuleParams = parse_params(params)?;
    
    match VM_STATE.try_read() {
        Ok(state) => {
            match state.modules.get(&params.module_id) {
                Some(module) => {
                    Ok(serde_json::json!({
                        "module_id": module.module_id,
                        "address": format!("0x{}", module.address.to_hex()),
                        "name": module.name,
                        "bytecode_size": module.bytecode.len(),
                        "deploy_block_height": module.deploy_block_height,
                        "public_functions": module.public_functions
                    }))
                },
                None => Err(not_found_error(format!("Module not found: {}", params.module_id)))
            }
        },
        Err(e) => Err(internal_error(format!("Failed to access VM state: {}", e)))
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
pub fn get_transaction(_params: jsonrpc_core::Params) -> JsonRpcResult<Value> {
    // Transaction history is no longer available in the in-memory implementation
    Err(not_found_error("Transaction history is not available in the in-memory VM".to_string()))
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
