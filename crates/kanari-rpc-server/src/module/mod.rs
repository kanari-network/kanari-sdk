use kanari_rpc_api::{ModuleInfo, RpcError, RpcRequest, RpcResponse};
use move_binary_format::CompiledModule;

use crate::{RpcServerState, respond_with_serialize};

/// Handle get module
pub async fn handle_get_module(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    #[derive(serde::Deserialize)]
    struct GetModuleParams {
        address: String,
        name: String,
    }

    let params: GetModuleParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
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
        None => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::module_error("Module not found")),
            id: request.id,
        },
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

    let params: VerifyParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
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
    let req: kanari_rpc_api::GetObjectRequest = match serde_json::from_value(request.params.clone())
    {
        Ok(r) => r,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    // Try to look up object in engine state
    let id = req.object_id;
    let state_guard = state.engine.state.read().unwrap();

    // Try both with and without 0x prefix
    let candidates = vec![id.clone(), id.trim_start_matches("0x").to_string()];
    for cid in candidates {
        if let Some(obj) = state_guard.objects.get(&cid) {
            let info = kanari_rpc_api::ObjectInfo {
                id: obj.id.clone(),
                owner: format!("{:#x}", obj.owner),
                type_: obj.type_.clone(),
                data: obj.data.clone(),
                version: obj.version,
            };
            return respond_with_serialize(request.id, info);
        }
    }

    RpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(RpcError::internal_error("Object not found")),
        id: request.id,
    }
}
