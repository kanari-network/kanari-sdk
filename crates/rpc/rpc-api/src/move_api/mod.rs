use jsonrpc_core::{Error as RpcError, ErrorCode, Params, Result as JsonRpcResult};
use mona_blockchain::blockchain::load_blockchain_with_retry;
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use std::time::{SystemTime, UNIX_EPOCH};
use mona_vm::{VMTransaction, execute_vm_transaction, VM_STATE};
use move_core_types::account_address::AccountAddress;
use mona_blockchain::blockchain::BLOCKCHAIN_DATA;

// VM function execution parameters
#[derive(Deserialize)]
pub struct VmExecuteParams {
    pub module_id: String,
    pub function: String,
    pub args: Vec<JsonValue>,
    pub sender: Option<String>,
    pub gas_budget: Option<u64>,
}

// VM module info parameters
#[derive(Deserialize)]
pub struct VmModuleParams {
    pub module_id: String,
}

// Execute a Move VM function
pub fn vm_execute(params: Params) -> JsonRpcResult<JsonValue> {
    // Parse execute params
    let execute_params: VmExecuteParams = params.parse()
        .map_err(|e| RpcError::invalid_params(format!("Invalid parameters: {}", e)))?;
    
    // Load blockchain data
    if let Err(e) = load_blockchain_with_retry() {
        return Err(RpcError {
            code: ErrorCode::InternalError,
            message: format!("Failed to load blockchain: {}", e),
            data: None,
        });
    }
    
    // Create VM transaction
    let vm_tx = VMTransaction {
        tx_id: format!("vm_tx_{}", generate_tx_id()),
        sender: execute_params.sender.unwrap_or_else(|| "system".to_string()),
        module_id: execute_params.module_id,
        function: execute_params.function,
        args: convert_args_to_bytes(&execute_params.args),
        gas_budget: execute_params.gas_budget.unwrap_or(1000000),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };
    
    // Execute VM transaction
    match execute_vm_transaction(&vm_tx) {
        Ok(result) => Ok(result),
        Err(e) => Err(RpcError {
            code: ErrorCode::InternalError,
            message: format!("Failed to execute VM function: {}", e),
            data: None,
        })
    }
}

// Get information about a deployed module
pub fn vm_get_module(params: Params) -> JsonRpcResult<JsonValue> {
    // Parse module params
    let module_params: VmModuleParams = params.parse()
        .map_err(|e| RpcError::invalid_params(format!("Invalid parameters: {}", e)))?;
    
    // Load blockchain data
    if let Err(e) = load_blockchain_with_retry() {
        return Err(RpcError {
            code: ErrorCode::InternalError,
            message: format!("Failed to load blockchain: {}", e),
            data: None,
        });
    }
    
    // Get VM state
    let vm_state = match VM_STATE.read() {
        Ok(state) => state,
        Err(_) => return Err(RpcError {
            code: ErrorCode::InternalError,
            message: "Failed to access VM state".to_string(),
            data: None,
        }),
    };
    
    // Find module
    let module = match vm_state.modules.get(&module_params.module_id) {
        Some(module) => module,
        None => return Err(RpcError {
            code: ErrorCode::InvalidParams,
            message: format!("Module not found: {}", module_params.module_id),
            data: None,
        }),
    };
    
    // Get current block height
    let blockchain = BLOCKCHAIN_DATA.iter();
    let current_block_height = match blockchain.last() {
        Some(block) => block.index,
        None => 0,
    };
    
    // Return module info
    Ok(json!({
        "module_id": module.module_id,
        "name": module.name,
        "address": format!("0x{}", module.address.to_hex()),
        "public_functions": module.public_functions,
        "size_bytes": module.bytecode.len(),
        "deploy_block_height": module.deploy_block_height,
        "current_block_height": current_block_height,
        "blocks_since_deploy": current_block_height.saturating_sub(module.deploy_block_height)
    }))
}

// List all deployed modules
pub fn vm_list_modules(_params: Params) -> JsonRpcResult<JsonValue> {
    // Load blockchain data
    if let Err(e) = load_blockchain_with_retry() {
        return Err(RpcError {
            code: ErrorCode::InternalError,
            message: format!("Failed to load blockchain: {}", e),
            data: None,
        });
    }
    
    // Get VM state
    let vm_state = match VM_STATE.read() {
        Ok(state) => state,
        Err(_) => return Err(RpcError {
            code: ErrorCode::InternalError,
            message: "Failed to access VM state".to_string(),
            data: None,
        }),
    };
    
    // Convert modules to JSON
    let modules: Vec<JsonValue> = vm_state.modules.values().map(|module| {
        json!({
            "module_id": module.module_id,
            "name": module.name,
            "address": format!("0x{}", module.address.to_hex()),
            "public_functions": module.public_functions,
            "size_bytes": module.bytecode.len(),
            "deploy_block_height": module.deploy_block_height
        })
    }).collect();
    
    // Return modules list
    Ok(json!({
        "modules": modules,
        "count": modules.len(),
        "last_execution": vm_state.last_execution,
        "execution_count": vm_state.execution_count,
    }))
}

// Generate a unique transaction ID
fn generate_tx_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    
    format!("{:x}", timestamp)
}

// Convert JSON arguments to byte arrays
fn convert_args_to_bytes(args: &[JsonValue]) -> Vec<Vec<u8>> {
    args.iter().map(|arg| {
        match arg {
            JsonValue::String(s) => s.as_bytes().to_vec(),
            _ => serde_json::to_string(arg).unwrap_or_default().into_bytes(),
        }
    }).collect()
}
