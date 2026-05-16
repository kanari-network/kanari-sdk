use super::{RpcError, RpcRequest, RpcResponse, RpcServerState, respond_with_serialize};
use kanari_rpc_api::{BlockInfo, RpcEvent};
use serde_json;

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

fn parse_height(id: u64, params: &serde_json::Value) -> Result<u64, RpcResponse> {
    serde_json::from_value(params.clone()).map_err(|e| invalid_params_response(id, e.to_string()))
}

fn to_rpc_block_info(block: kanari_rpc_api::BlockData) -> BlockInfo {
    let events = block
        .events
        .into_iter()
        .map(|e| RpcEvent {
            key: e.key,
            sequence_number: e.sequence_number,
            type_tag: e.type_tag,
            event_data: e.event_data,
        })
        .collect();

    BlockInfo {
        height: block.height,
        timestamp: block.timestamp,
        hash: block.hash,
        prev_hash: block.prev_hash,
        tx_count: block.tx_count,
        state_root: block.state_root,
        events,
    }
}

/// Handle get block request
pub async fn handle_get_block(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let height = match parse_height(request.id, &request.params) {
        Ok(height) => height,
        Err(response) => return response,
    };

    match state.engine.get_block(height) {
        Some(block) => respond_with_serialize(request.id, to_rpc_block_info(block)),
        None => internal_error_response(request.id, "Block not found"),
    }
}

/// Handle get full block request
pub async fn handle_get_full_block(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let height = match parse_height(request.id, &request.params) {
        Ok(height) => height,
        Err(response) => return response,
    };

    match state.engine.get_full_block(height) {
        Some(block) => respond_with_serialize(request.id, block),
        None => internal_error_response(request.id, "Block not found"),
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
        Ok(info) => respond_with_serialize(request.id, info),
        Err(e) => internal_error_response(request.id, format!("Produce block failed: {}", e)),
    }
}
