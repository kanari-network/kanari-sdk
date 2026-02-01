use super::{RpcError, RpcRequest, RpcResponse, RpcServerState, respond_with_serialize};
use kanari_rpc_api::{BlockInfo, RpcEvent};
use serde_json;

/// Handle get block request
pub async fn handle_get_block(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let height: u64 = match serde_json::from_value(request.params.clone()) {
        Ok(h) => h,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    match state.engine.get_block(height) {
        Some(block) => {
            // Map runtime events to RPC events
            let rpc_events: Vec<RpcEvent> = block
                .events
                .into_iter()
                .map(|e| RpcEvent {
                    key: e.key,
                    sequence_number: e.sequence_number,
                    type_tag: e.type_tag,
                    event_data: e.event_data,
                })
                .collect();

            let block_info = BlockInfo {
                height: block.height,
                timestamp: block.timestamp,
                hash: block.hash.clone(),
                prev_hash: block.prev_hash,
                tx_count: block.tx_count,
                // `block.hash` is already a hex string; avoid double-encoding
                state_root: block.state_root.clone(),
                events: rpc_events,
            };
            respond_with_serialize(request.id, block_info)
        }
        None => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::internal_error("Block not found")),
            id: request.id,
        },
    }
}

/// Handle get full block request
pub async fn handle_get_full_block(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let height: u64 = match serde_json::from_value(request.params.clone()) {
        Ok(h) => h,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    match state.engine.get_full_block(height) {
        Some(block) => respond_with_serialize(request.id, block),
        None => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::internal_error("Block not found")),
            id: request.id,
        },
    }
}

/// Handle get state root request
pub async fn handle_get_state_root(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let req: kanari_rpc_api::GetStateRootRequest =
        match serde_json::from_value(request.params.clone()) {
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

    let root = state.engine.get_state_root(req.height);
    match root {
        Some(r) => respond_with_serialize(request.id, serde_json::json!(r)),
        None => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::internal_error("State root not available")),
            id: request.id,
        },
    }
}

/// Handle get block height request
pub async fn handle_get_block_height(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let stats = state.engine.get_stats();
    RpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::json!(stats.height)),
        error: None,
        id: request.id,
    }
}

/// Handle get stats request
pub async fn handle_get_stats(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let stats = state.engine.get_stats();
    let blockchain_stats = kanari_rpc_api::BlockchainStats {
        height: stats.height,
        total_blocks: stats.total_blocks,
        total_transactions: stats.total_transactions,
        pending_transactions: stats.pending_transactions,
        total_accounts: stats.total_accounts,
        total_supply: stats.total_supply,
    };
    respond_with_serialize(request.id, blockchain_stats)
}

/// Handle produce block request (force block production now)
pub async fn handle_produce_block(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    match state.engine.produce_block() {
        Ok(info) => {
            // Serialize the core BlockInfo returned from engine
            respond_with_serialize(request.id, info)
        }
        Err(e) => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::internal_error(format!(
                "Produce block failed: {}",
                e
            ))),
            id: request.id,
        },
    }
}
