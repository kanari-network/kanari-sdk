use super::{RpcError, RpcRequest, RpcResponse, RpcServerState, respond_with_serialize};
use kanari_move_runtime::state::StateManager;
use kanari_rpc_api::{GetAllBalancesRequest, GetTokenBalanceRequest};
use move_core_types::language_storage::TypeTag;
use serde_json;
use std::collections::BTreeMap;
use std::str::FromStr;

// =========================================================================
// HELPERS (Optimized for performance and correctness)
// =========================================================================

/// Helper to get token decimals from engine state.
/// OPTIMIZED: Requires an already acquired state guard to prevent lock contention in loops.
fn get_token_decimals(state_guard: &StateManager, token_type: &str) -> u8 {
    if token_type == "KANARI" {
        return 9;
    }
    if let Ok(Some(decimals)) = state_guard.get_token_decimals(token_type) {
        return decimals;
    }
    // Default to 9 for most tokens
    9
}

/// Extracts the pure symbol name from a complex Move Type string.
/// e.g., "0x2::coin::Coin<0x2::james::JAMES>" -> "JAMES"
/// e.g., "0x2::james::JAMES" -> "JAMES"
fn extract_symbol(token_type: &str) -> String {
    // 1. Extract innermost generic if present
    let inner = token_type
        .split('<')
        .next_back() // <--- เปลี่ยนจาก .last() เป็น .next_back()
        .unwrap_or(token_type)
        .trim_end_matches('>');
    
    // 2. Extract the final module/struct name
    inner.split("::").last().unwrap_or(inner).to_string()
}

/// Normalizes TypeTag into a clean string representation
fn normalize_token_type(token_type: &str) -> String {
    if let Ok(TypeTag::Struct(st)) = TypeTag::from_str(token_type) {
        return format!("{}", st);
    }
    token_type.to_string()
}

// =========================================================================
// HANDLERS
// =========================================================================

/// Handle get account request
pub async fn handle_get_account(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let address: String = match serde_json::from_value(request.params.clone()) {
        Ok(addr) => addr,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    match state.engine.get_account_info(&address) {
        Some(info) => respond_with_serialize(request.id, info),
        None => RpcResponse {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(RpcError::internal_error("Account not found")),
            id: request.id,
        },
    }
}

/// Handle get balance request
pub async fn handle_get_balance(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let address: String = match serde_json::from_value(request.params.clone()) {
        Ok(addr) => addr,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    let balance = state
        .engine
        .get_account_info(&address)
        .map(|info| info.balance)
        .unwrap_or(0);

    RpcResponse {
        jsonrpc: "2.0".into(),
        result: Some(serde_json::json!(balance)),
        error: None,
        id: request.id,
    }
}

/// Handle get token balance request
pub async fn handle_get_token_balance(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let req_data: GetTokenBalanceRequest = match serde_json::from_value(request.params.clone()) {
        Ok(data) => data,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    let balance = state
        .engine
        .get_token_balance(&req_data.address, &req_data.token_type);

    RpcResponse {
        jsonrpc: "2.0".into(),
        result: Some(serde_json::json!({ "token_type": req_data.token_type, "balance": balance })),
        error: None,
        id: request.id,
    }
}

/// Handle get all balances request
pub async fn handle_get_all_balances(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let req_data: GetAllBalancesRequest = match serde_json::from_value(request.params.clone()) {
        Ok(data) => data,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    let account_info = match state.engine.get_account_info(&req_data.address) {
        Some(info) => info,
        None => {
            return RpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(RpcError::internal_error("Account not found")),
                id: request.id,
            };
        }
    };

    let mut balances = vec![serde_json::json!({
        "token_type": "KANARI",
        "balance": account_info.balance,
        "decimals": 9,
        "symbol": "KANARI"
    })];

    // Source of truth: account token balances tracked by runtime state.
    let mut token_sums: BTreeMap<String, u64> = account_info
        .token_balances
        .into_iter()
        .map(|(k, v)| (normalize_token_type(&k), v))
        .collect();

    // Backward-compatible fallback: aggregate Coin objects
    let mut coin_sums: BTreeMap<String, u128> = BTreeMap::new();

    if let Some(ref objects) = account_info.owned_objects {
        for obj in objects {
            if obj.type_.contains("::coin::Coin<") {
                let token_type = if let Some(start) = obj.type_.find('<')
                    && let Some(end) = obj.type_.rfind('>')
                {
                    normalize_token_type(&obj.type_[start + 1..end])
                } else {
                    normalize_token_type(&obj.type_)
                };

                // Try to parse balance from last 8 bytes
                if obj.data.len() >= 8 {
                    let n = obj.data.len();
                    if let Ok(bytes) = obj.data[n - 8..].try_into() {
                        let amount = u64::from_le_bytes(bytes) as u128;
                        *coin_sums.entry(token_type).or_insert(0) += amount;
                    }
                }
            }
        }
    }

    // Fill missing tokens from aggregated coin objects.
    for (token_type, amount128) in coin_sums {
        let amount = if amount128 > u128::from(u64::MAX) {
            u64::MAX
        } else {
            amount128 as u64
        };
        token_sums.entry(token_type).or_insert(amount);
    }

    // OPTIMIZATION: Acquire lock exactly ONCE before the loop
    let state_guard = state.engine.state.read().unwrap_or_else(|p| p.into_inner());

    // Add token balances to response (non-zero only).
    for (token_type, amount) in token_sums {
        if amount == 0 {
            continue;
        }

        let symbol = extract_symbol(&token_type);

        balances.push(serde_json::json!({
            "token_type": token_type,
            "balance": amount,
            "decimals": get_token_decimals(&state_guard, &token_type),
            "symbol": symbol
        }));
    }

    RpcResponse {
        jsonrpc: "2.0".into(),
        result: Some(serde_json::json!({ "address": req_data.address, "balances": balances })),
        error: None,
        id: request.id,
    }
}

/// Handle list tokens request
pub async fn handle_list_tokens(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let tokens = state.engine.list_tokens();

    // OPTIMIZATION: Acquire lock exactly ONCE to fetch decimals for all tokens
    let state_guard = state.engine.state.read().unwrap_or_else(|p| p.into_inner());

    // Map into serializable objects { token_type, total_supply, decimals, symbol }
    let vals: Vec<serde_json::Value> = tokens
        .into_iter()
        .map(|(token_type, supply)| {
            let symbol = extract_symbol(&token_type);

            serde_json::json!({
                "token_type": token_type,
                "total_supply": supply,
                "decimals": get_token_decimals(&state_guard, &token_type),
                "symbol": symbol,
            })
        })
        .collect();

    respond_with_serialize(request.id, vals)
}
