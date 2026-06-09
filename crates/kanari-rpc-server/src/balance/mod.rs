use super::{
    RpcRequest, RpcResponse, RpcServerState, internal_error_response, parse_params,
    respond_with_serialize,
};
use crate::module::aggregate_owned_objects;
use kanari_move_runtime_v1::state::StateManager;
use kanari_rpc_api::{GetAllBalancesRequest, GetTokenBalanceRequest};
use kanari_types::kanari::KANARI_TOKEN_TYPE;
use move_core_types::language_storage::TypeTag;
use serde_json;
use std::collections::BTreeSet;
use std::str::FromStr;
use tracing::warn;

fn get_token_decimals(state_guard: &StateManager, token_type: &str) -> u8 {
    if token_type == KANARI_TOKEN_TYPE {
        return 9;
    }
    if let Ok(Some(decimals)) = state_guard.get_token_decimals(token_type) {
        return decimals;
    }
    9
}

fn extract_symbol(token_type: &str) -> String {
    let inner = token_type
        .split('<')
        .next_back()
        .unwrap_or(token_type)
        .trim_end_matches('>');

    inner.split("::").last().unwrap_or(inner).to_string()
}

fn normalize_token_type(token_type: &str) -> String {
    if let Ok(TypeTag::Struct(st)) = TypeTag::from_str(token_type) {
        return format!("{}", st);
    }
    token_type.to_string()
}

fn build_balance_json(
    state_guard: &StateManager,
    token_type: String,
    balance: u64,
) -> serde_json::Value {
    let db_symbol = state_guard.get_token_symbol(&token_type).unwrap_or(None);
    let symbol = db_symbol.unwrap_or_else(|| extract_symbol(&token_type));

    let name = state_guard
        .get_token_name(&token_type)
        .unwrap_or(None)
        .unwrap_or_else(|| symbol.clone());

    let description = state_guard
        .get_token_description(&token_type)
        .unwrap_or(None);
    let icon_url = state_guard.get_token_icon_url(&token_type).unwrap_or(None);

    serde_json::json!({
        "token_type": token_type,
        "balance": balance,
        "decimals": get_token_decimals(state_guard, &token_type),
        "symbol": symbol,
        "name": name,
        "description": description,
        "icon_url": icon_url
    })
}

/// Handle get account request
pub async fn handle_get_account(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let address: String = match parse_params(request.id, &request.params) {
        Ok(addr) => addr,
        Err(response) => return *response,
    };

    match state.engine.get_account_info(&address) {
        Some(mut info) => {
            if let Some(objects) = info.owned_objects.take() {
                info.owned_objects = Some(aggregate_owned_objects(objects));
            }
            respond_with_serialize(request.id, info)
        }
        None => internal_error_response(request.id, "Account not found"),
    }
}

/// Handle get token balance request
pub async fn handle_get_token_balance(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let req_data: GetTokenBalanceRequest = match parse_params(request.id, &request.params) {
        Ok(data) => data,
        Err(response) => return *response,
    };

    let account_info = match state.engine.get_account_info(&req_data.address) {
        Some(info) => info,
        None => return internal_error_response(request.id, "Account not found"),
    };

    let target_token = normalize_token_type(&req_data.token_type);
    let final_balance = account_info
        .token_balances
        .get(&target_token)
        .copied()
        .unwrap_or(0);

    RpcResponse {
        jsonrpc: "2.0".into(),
        result: Some(serde_json::json!({
            "token_type": req_data.token_type,
            "balance": final_balance
        })),
        error: None,
        id: request.id,
    }
}

/// Handle get all balances request
pub async fn handle_get_all_balances(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let req_data: GetAllBalancesRequest = match parse_params(request.id, &request.params) {
        Ok(data) => data,
        Err(response) => return *response,
    };

    let account_info = match state.engine.get_account_info(&req_data.address) {
        Some(info) => info,
        None => return internal_error_response(request.id, "Account not found"),
    };

    let state_guard = state.engine.state_read();
    let balances: Vec<_> = account_info
        .token_balances
        .into_iter()
        .filter(|(token_type, amount)| *amount > 0 || token_type == KANARI_TOKEN_TYPE)
        .map(|(token_type, balance)| build_balance_json(&state_guard, token_type, balance))
        .collect();

    RpcResponse {
        jsonrpc: "2.0".into(),
        result: Some(serde_json::json!({ "address": req_data.address, "balances": balances })),
        error: None,
        id: request.id,
    }
}

/// Handle list tokens request ( O(1) Optimized from Global Cache)
pub async fn handle_list_tokens(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let state_guard = state.engine.state_read();

    let mut token_types: BTreeSet<String> =
        state_guard.global_token_supplies.keys().cloned().collect();
    token_types.insert(KANARI_TOKEN_TYPE.to_string());

    if let Ok(Some(keys)) = state_guard.store.load::<Vec<String>>(b"treasury_index") {
        for key in keys {
            let token_type = key.strip_prefix("treasury:").unwrap_or(&key).to_string();
            token_types.insert(token_type);
        }
    }

    let vals: Vec<serde_json::Value> = token_types
        .into_iter()
        .filter_map(|token_type| {
            let summary = match state_guard.token_supply_summary(&token_type) {
                Ok(summary) => summary,
                Err(e) => {
                    warn!(
                        "[RPC] Failed to build supply summary for token {}: {}",
                        token_type, e
                    );
                    return None;
                }
            };
            let db_symbol = state_guard.get_token_symbol(&token_type).unwrap_or(None);
            let symbol = db_symbol.unwrap_or_else(|| extract_symbol(&token_type));

            let name = state_guard
                .get_token_name(&token_type)
                .unwrap_or(None)
                .unwrap_or_else(|| symbol.clone());

            let description = state_guard
                .get_token_description(&token_type)
                .unwrap_or(None);

            let icon_url = state_guard.get_token_icon_url(&token_type).unwrap_or(None);

            Some(serde_json::json!({
                "token_type": summary.token_type,
                "total_supply": summary.total_supply,
                "wallet_visible_supply": summary.wallet_visible_supply,
                "circulating_supply": summary.wallet_visible_supply,
                "object_locked_supply": summary.object_locked_supply,
                "accounted_supply": summary.accounted_supply,
                "untracked_supply": summary.untracked_supply,
                "decimals": get_token_decimals(&state_guard, &token_type),
                "symbol": symbol,
                "name": name,
                "description": description,
                "icon_url": icon_url
            }))
        })
        .collect();

    respond_with_serialize(request.id, vals)
}
