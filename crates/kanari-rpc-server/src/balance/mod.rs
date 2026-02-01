use super::{RpcError, RpcRequest, RpcResponse, RpcServerState, respond_with_serialize};
use kanari_rpc_api::{GetAllBalancesRequest, GetTokenBalanceRequest};
use serde_json;

/// Handle get account request
pub async fn handle_get_account(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let address: String = match serde_json::from_value(request.params.clone()) {
        Ok(addr) => addr,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    match state.engine.get_account_info(&address) {
        Some(info) => respond_with_serialize(request.id, info),
        None => RpcResponse {
            jsonrpc: "2.0".to_string(),
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
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    match state.engine.get_account_info(&address) {
        Some(info) => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!(info.balance)),
            error: None,
            id: request.id,
        },
        None => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!(0)),
            error: None,
            id: request.id,
        },
    }
}

/// Handle get token balance request
pub async fn handle_get_token_balance(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let req_data: GetTokenBalanceRequest = match serde_json::from_value(request.params.clone()) {
        Ok(data) => data,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
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
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::json!({
            "token_type": req_data.token_type,
            "balance": balance
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
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    let account_info = state.engine.get_account_info(&req_data.address);

    match account_info {
        Some(info) => {
            let mut balances = vec![serde_json::json!({
                "token_type": "KANARI",
                "balance": info.balance,
                "decimals": 9,
                "symbol": "KANARI"
            })];

            // Start with recorded token_balances (from state.account.token_balances)
            for (token_type, amount) in info.token_balances.iter() {
                let symbol = token_type.split("::").last().unwrap_or(token_type);

                balances.push(serde_json::json!({
                    "token_type": token_type,
                    "balance": amount,
                    "decimals": 9,
                    "symbol": symbol
                }));
            }

            // Best-effort: inspect owned objects for coin objects and sum their values.
            // Many Move coin implementations store the coin value as a u64 in the last 8 bytes.
            use std::collections::BTreeMap;
            let mut coin_sums: BTreeMap<String, u128> = BTreeMap::new();
            if let Some(ref objects) = info.owned_objects {
                for obj in objects {
                    if obj.type_.contains("::coin::Coin<") {
                        // Try to parse last 8 bytes from obj.data (which is Vec<u8> in RPC types)
                        if obj.data.len() >= 8 {
                            let n = obj.data.len();
                            if let Ok(bytes) = obj.data[n - 8..].try_into() {
                                let amount = u64::from_le_bytes(bytes) as u128;
                                // token_type = the generic type inside Coin<...>
                                // We'll use the full object type as token_type for uniqueness
                                let token_type = obj.type_.clone();
                                *coin_sums.entry(token_type).or_insert(0) += amount;
                            }
                        }
                    }
                }
            }

            for (token_type, amount128) in coin_sums.into_iter() {
                // convert to u64 if safe, else cap
                let amount = if amount128 > (u128::from(u64::MAX)) {
                    u64::MAX
                } else {
                    amount128 as u64
                };
                let mut symbol = token_type
                    .split("::")
                    .last()
                    .unwrap_or(&token_type)
                    .to_string();
                // Trim possible generic angle-brackets from the last segment (e.g. "JAMES>")
                symbol = symbol
                    .trim_start_matches('<')
                    .trim_end_matches('>')
                    .to_string();
                balances.push(serde_json::json!({
                    "token_type": token_type,
                    "balance": amount,
                    "decimals": 9,
                    "symbol": symbol
                }));
            }

            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::json!({
                    "address": req_data.address,
                    "balances": balances
                })),
                error: None,
                id: request.id,
            }
        }
        None => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::invalid_params("Account not found")),
            id: request.id,
        },
    }
}

/// Handle list tokens request
pub async fn handle_list_tokens(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    // No params expected; ignore request.params
    let tokens = state.engine.list_tokens();

    // Map into serializable objects { token_type, total_supply, symbol }
    let vals: Vec<serde_json::Value> = tokens
        .into_iter()
        .map(|(token_type, supply)| {
            let mut symbol = token_type
                .split("::")
                .last()
                .unwrap_or(&token_type)
                .to_string();
            // strip generics if present
            symbol = symbol
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_string();
            serde_json::json!({
                "token_type": token_type,
                "total_supply": supply,
                "symbol": symbol,
            })
        })
        .collect();

    respond_with_serialize(request.id, vals)
}
