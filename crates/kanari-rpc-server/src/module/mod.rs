use kanari_rpc_api::{ModuleInfo, RpcError, RpcRequest, RpcResponse};
use move_binary_format::CompiledModule;

use crate::{RpcServerState, respond_with_serialize};

fn invalid_params_response(id: u64, message: impl Into<String>) -> RpcResponse {
    RpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(RpcError::invalid_params(message.into())),
        id,
    }
}

fn internal_error_response(id: u64, message: impl Into<String>) -> RpcResponse {
    RpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(RpcError::internal_error(message.into())),
        id,
    }
}

fn parse_params<T: serde::de::DeserializeOwned>(
    id: u64,
    params: &serde_json::Value,
) -> Result<T, RpcResponse> {
    serde_json::from_value(params.clone()).map_err(|e| invalid_params_response(id, e.to_string()))
}

/// Handle get module
pub async fn handle_get_module(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    #[derive(serde::Deserialize)]
    struct GetModuleParams {
        address: String,
        name: String,
    }

    let params: GetModuleParams = match parse_params(request.id, &request.params) {
        Ok(p) => p,
        Err(response) => return response,
    };

    // Get module bytecode from Move storage
    match state
        .engine
        .get_module_bytecode(&params.address, &params.name)
    {
        Some(bytecode) => {
            let module_info = ModuleInfo {
                address: params.address,
                name: params.name,
                bytecode_hash: hex::encode(&blake3::hash(&bytecode).as_bytes()[..]),
                size: bytecode.len(),
                dependencies: vec![], // TODO: Extract dependencies from bytecode
            };
            respond_with_serialize(request.id, module_info)
        }
        None => internal_error_response(request.id, "Module not found"),
    }
}

/// Handle list modules
pub async fn handle_list_modules(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    // Get modules from Move storage instead of contracts
    let modules_data = state.engine.list_all_modules();
    let modules: Vec<ModuleInfo> = modules_data
        .iter()
        .map(|(address, name)| {
            let bytecode_opt = state.engine.get_module_bytecode(address, name);
            ModuleInfo {
                address: address.clone(),
                name: name.clone(),
                bytecode_hash: bytecode_opt
                    .as_ref()
                    .map(|b| hex::encode(&blake3::hash(b).as_bytes()[..]))
                    .unwrap_or_else(|| "unknown".to_string()),
                size: bytecode_opt.as_ref().map(|b| b.len()).unwrap_or(0),
                dependencies: vec![],
            }
        })
        .collect();

    respond_with_serialize(request.id, modules)
}

/// Handle verify module
pub async fn handle_verify_module(_state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    #[derive(serde::Deserialize)]
    struct VerifyParams {
        module_bytes: Vec<u8>,
    }

    let params: VerifyParams = match parse_params(request.id, &request.params) {
        Ok(p) => p,
        Err(response) => return response,
    };

    // Try to deserialize module
    match CompiledModule::deserialize_with_defaults(&params.module_bytes) {
        Ok(module) => {
            let module_id = module.self_id();
            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::json!({
                    "valid": true,
                    "address": module_id.address().to_hex_literal(),
                    "name": module_id.name().as_str()
                })),
                error: None,
                id: request.id,
            }
        }
        Err(e) => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "valid": false,
                "error": e.to_string()
            })),
            error: None,
            id: request.id,
        },
    }
}

/// Handle get object request
pub async fn handle_get_object(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let req: kanari_rpc_api::GetObjectRequest = match parse_params(request.id, &request.params) {
        Ok(r) => r,
        Err(response) => return response,
    };

    // Try to look up object in engine state
    let id = req.object_id;
    let state_guard = state.engine.state.read().unwrap_or_else(|e| e.into_inner());

    // Try both with and without 0x prefix
    let candidates = vec![id.clone(), id.trim_start_matches("0x").to_string()];
    for uid in candidates {
        if let Ok(Some(obj)) = state_guard.get_object(&uid) {
            let info = kanari_rpc_api::ObjectInfo {
                id: uid.clone(),
                owner: format!("{:#x}", obj.owner),
                type_: obj.type_.clone(),
                data: obj.data.clone(),
                version: obj.version,
            };
            return respond_with_serialize(request.id, info);
        }
    }

    internal_error_response(request.id, "Object not found")
}

/// Handle get owned objects request
pub async fn handle_get_owned_objects(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let req: kanari_rpc_api::GetOwnedObjectsRequest =
        match parse_params(request.id, &request.params) {
            Ok(r) => r,
            Err(response) => return response,
        };

    // Parse owner address using kanari_types::address::Address::parse_to_account_address
    let owner_addr = match kanari_types::address::Address::parse_to_account_address(&req.owner) {
        Ok(addr) => addr,
        Err(e) => {
            return invalid_params_response(request.id, format!("Invalid owner address: {}", e));
        }
    };

    let state_guard = state.engine.state.read().unwrap_or_else(|e| e.into_inner());

    // Get all owned object IDs for the owner
    let owned_ids = match state_guard.get_owned_objects(&owner_addr) {
        Ok(ids) => ids,
        Err(e) => {
            return internal_error_response(
                request.id,
                format!("Failed to get owned objects: {}", e),
            );
        }
    };

    // Filter objects by type if specified
    let mut objects = Vec::new();
    for uid in owned_ids {
        if let Ok(Some(obj)) = state_guard.get_object(&uid) {
            // If object_type filter is specified, check if it matches
            if let Some(ref filter_type) = req.object_type
                && !obj.type_.contains(filter_type)
            {
                continue;
            }

            let info = kanari_rpc_api::ObjectInfo {
                id: uid.clone(),
                owner: format!("{:#x}", obj.owner),
                type_: obj.type_.clone(),
                data: obj.data.clone(),
                version: obj.version,
            };
            objects.push(info);
        }
    }

    // Return the filtered list of objects
    RpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::json!({
            "objects": objects
        })),
        error: None,
        id: request.id,
    }
}
