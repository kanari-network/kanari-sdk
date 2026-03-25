use super::{RpcError, RpcRequest, RpcResponse, RpcServerState, respond_with_serialize};
use kanari_rpc_api::{GetAllBalancesRequest, GetTokenBalanceRequest};
use move_core_types::language_storage::TypeTag;
use serde_json;
use std::str::FromStr;

/// Helper to get token decimals from engine state
fn get_token_decimals(engine: &kanari_core::engine::BlockchainEngine, token_type: &str) -> u8 {
    // Try to get decimals from state manager
    if let Ok(state) = engine.state.read()
        && let Ok(Some(decimals)) = state.get_token_decimals(token_type)
    {
        return decimals;
    }
    // Default to 9 for most tokens (including JAMES)
    9
}

fn normalize_token_type(token_type: &str) -> String {
    if let Ok(TypeTag::Struct(st)) = TypeTag::from_str(token_type) {
        return format!("{}", st);
    }
    token_type.to_string()
}

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

            use std::collections::BTreeMap;
            // Source of truth: account token balances tracked by runtime state.
            let mut token_sums: BTreeMap<String, u64> = info
                .token_balances
                .into_iter()
                .map(|(k, v)| (normalize_token_type(&k), v))
                .collect();

            // Backward-compatible fallback: also aggregate Coin objects and only fill
            // tokens that are currently missing from token_balances.
            let mut coin_sums: BTreeMap<String, u128> = BTreeMap::new();

            if let Some(ref objects) = info.owned_objects {
                for obj in objects {
                    if obj.type_.contains("::coin::Coin<") {
                        // Extract the token type from Coin<TokenType>
                        // e.g., "0x2::coin::Coin<james::james::JAMES>" -> "james::james::JAMES"
                        let token_type = if let Some(start) = obj.type_.find('<')
                            && let Some(end) = obj.type_.rfind('>')
                        {
                            normalize_token_type(&obj.type_[start + 1..end])
                        } else {
                            normalize_token_type(&obj.type_)
                        };

                        // Try to parse balance from last 8 bytes (standard Move Coin layout)
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

            // Add token balances to response (non-zero only).
            for (token_type, amount) in token_sums {
                // Skip zero-balance coins to avoid cluttering the response
                if amount == 0 {
                    continue;
                }

                let symbol = token_type.split("::").last().unwrap_or(&token_type);

                balances.push(serde_json::json!({
                    "token_type": token_type,
                    "balance": amount,
                    "decimals": get_token_decimals(&state.engine, &token_type),
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
            error: Some(RpcError::internal_error("Account not found")),
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
                "decimals": get_token_decimals(&state.engine, &token_type),
                "symbol": symbol,
            })
        })
        .collect();

    respond_with_serialize(request.id, vals)
}
