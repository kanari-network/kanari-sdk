use super::{RpcError, RpcRequest, RpcResponse, RpcServerState, respond_with_serialize};
use kanari_move_runtime_v1::state::StateManager;
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
    if token_type.to_uppercase().contains("KANARI") {
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
        .next_back()
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

    let target_token = normalize_token_type(&req_data.token_type);
    let mut final_balance = 0;

    if target_token.to_uppercase() == "KANARI" || target_token.contains("::kanari::KANARI") {
        final_balance = account_info.balance;
    } else if let Some(record) = account_info.token_balances.get(&target_token) {
        final_balance = *record;
    }

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

    let mut coin_sums: BTreeMap<String, u128> = BTreeMap::new();

    // 1. Use Native Balance as initial KANARI token balance
    coin_sums.insert("KANARI".to_string(), account_info.balance as u128);

    // 🚨 2. Loop through token_balances for non-KANARI tokens only to prevent double counting
    for (token_type, amount) in account_info.token_balances {
        // Skip KANARI token since we already use Native Balance as primary source in step 1
        if token_type.to_uppercase().contains("KANARI") {
            continue;
        }

        *coin_sums.entry(token_type).or_insert(0) += amount as u128;
    }

    let state_guard = state.engine.state.read().unwrap_or_else(|p| p.into_inner());
    let mut balances = Vec::new();

    for (token_type, amount128) in coin_sums {
        if amount128 == 0 && token_type != "KANARI" {
            continue;
        }

        let amount = if amount128 > u128::from(u64::MAX) {
            u64::MAX
        } else {
            amount128 as u64
        };

        let db_symbol = state_guard.get_token_symbol(&token_type).unwrap_or(None);
        let symbol = db_symbol.unwrap_or_else(|| extract_symbol(&token_type));

        let name = state_guard
            .get_token_name(&token_type)
            .unwrap_or(None)
            .unwrap_or_else(|| symbol.clone());

        let mut description = state_guard
            .get_token_description(&token_type)
            .unwrap_or(None);

        let is_kanari = token_type.to_uppercase().contains("KANARI");

        let (final_name, final_symbol, icon_url) = if is_kanari {
            // Enforce exact KANARI token metadata
            if description.is_none() {
                description = Some("The native token of Kanari Network".to_string());
            }
            (
                "Kanari Network Coin".to_string(),
                "KANARI".to_string(),
                Some("https://avatars.githubusercontent.com/u/127471673?s=200&v=4".to_string()),
            )
        } else {
            (
                name,
                symbol,
                state_guard.get_token_icon_url(&token_type).unwrap_or(None),
            )
        };

        balances.push(serde_json::json!({
            "token_type": token_type,
            "balance": amount,
            "decimals": get_token_decimals(&state_guard, &token_type),
            "symbol": final_symbol,
            "name": final_name,
            "description": description,
            "icon_url": icon_url
        }));
    }

    RpcResponse {
        jsonrpc: "2.0".into(),
        result: Some(serde_json::json!({ "address": req_data.address, "balances": balances })),
        error: None,
        id: request.id,
    }
}

/// Handle list tokens request (⚡ O(1) Optimized from Global Cache)
pub async fn handle_list_tokens(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let state_guard = state.engine.state.read().unwrap_or_else(|p| p.into_inner());

    // 🚨 1. Fetch cached Global Token Supplies from RAM immediately (no Deep Scan needed!)
    let mut global_tokens = state_guard.global_token_supplies.clone();

    // 🚨 2. Always include Native Token (KANARI) balance
    global_tokens.insert("KANARI".to_string(), state_guard.total_supply);

    // 🚨 3. Check for Treasury-enabled tokens with zero mint (display 0 balance)
    if let Ok(Some(keys)) = state_guard.store.load::<Vec<String>>(b"treasury_index") {
        for key in keys {
            let token_type = key.strip_prefix("treasury:").unwrap_or(&key).to_string();
            if !token_type.to_uppercase().contains("KANARI") {
                global_tokens.entry(token_type).or_insert(0);
            }
        }
    }

    let tokens: Vec<(String, u64)> = global_tokens.into_iter().collect();

    // =====================================================================
    // STEP 2: METADATA & FORMATTING
    // =====================================================================

    let vals: Vec<serde_json::Value> = tokens
        .into_iter()
        .map(|(token_type, supply)| {
            let db_symbol = state_guard.get_token_symbol(&token_type).unwrap_or(None);
            let symbol = db_symbol.unwrap_or_else(|| extract_symbol(&token_type));

            let name = state_guard
                .get_token_name(&token_type)
                .unwrap_or(None)
                .unwrap_or_else(|| symbol.clone());

            let mut description = state_guard
                .get_token_description(&token_type)
                .unwrap_or(None);

            let is_kanari = token_type.to_uppercase().contains("KANARI");

            let (final_name, final_symbol, icon_url) = if is_kanari {
                if description.is_none() {
                    description = Some("The native token of Kanari Network".to_string());
                }
                (
                    "Kanari Network Coin".to_string(),
                    "KANARI".to_string(),
                    Some("https://avatars.githubusercontent.com/u/127471673?s=200&v=4".to_string()),
                )
            } else {
                (
                    name,
                    symbol,
                    state_guard.get_token_icon_url(&token_type).unwrap_or(None),
                )
            };

            serde_json::json!({
                "token_type": token_type,
                "total_supply": supply,
                "decimals": get_token_decimals(&state_guard, &token_type),
                "symbol": final_symbol,
                "name": final_name,
                "description": description,
                "icon_url": icon_url
            })
        })
        .collect();

    respond_with_serialize(request.id, vals)
}
