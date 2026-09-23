// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{
    RpcRequest, RpcResponse, RpcServerState, internal_error_response, parse_params,
    respond_with_serialize,
};
use kanari_move_runtime_v1::state::StateManager;
use kanari_rpc_api::{
    FungibleAssetHolder, FungibleAssetHoldersResponse, FungibleAssetInfo,
    GetFungibleAssetHoldersRequest, GetFungibleAssetRequest, GetOwnerBalancesRequest,
    GetTokenBalanceRequest,
};
use kanari_types::coin::CoinModule;
use kanari_types::gas_coin::GAS_COIN;
use move_core_types::account_address::AccountAddress;
use serde_json;
use std::collections::BTreeSet;
use tracing::warn;

/// No fallback: returns on-chain `CoinMetadata` decimals only.
/// `GAS_COIN` (KANARI) is `Some(9)` as protocol constant, everything else
/// is `None` when metadata is not indexed — callers must not invent 9/6/0.
fn get_token_decimals(state_guard: &StateManager, token_type: &str) -> Option<u8> {
    if token_type == GAS_COIN {
        return Some(9);
    }
    state_guard.get_token_decimals(token_type).ok().flatten()
}

fn extract_symbol(token_type: &str) -> String {
    let inner = token_type
        .split('<')
        .next_back()
        .unwrap_or(token_type)
        .trim_end_matches('>');

    inner.split("::").last().unwrap_or(inner).to_string()
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

fn collect_fungible_asset_holders(
    state_guard: &StateManager,
    token_type: &str,
    limit: Option<usize>,
    compute_coin_count: bool,
) -> anyhow::Result<Vec<FungibleAssetHolder>> {
    let token_type = CoinModule::normalize_token_type(token_type);
    let mut balances: Vec<(AccountAddress, u64)> = Vec::new();

    for owner in state_guard.owner_addresses()? {
        if !is_public_asset_holder(&owner) {
            continue;
        }

        let balance = state_guard.resolve_owner_token_balance(owner, &token_type)?;
        if balance == 0 {
            continue;
        }
        balances.push((owner, balance));
    }

    balances.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    if let Some(limit) = limit {
        balances.truncate(limit);
    }

    // Hydrate coin counts only for survivors so a limit=10 query for a
    // rare token doesn't walk every owner's objects. Output identical to
    // before: truncated-away holders never appeared in the response.
    let mut holders = Vec::with_capacity(balances.len());
    for (owner, balance) in balances {
        let coin_object_count = if compute_coin_count {
            let mut count = 0usize;
            for object_id in state_guard.get_owned_objects(&owner)? {
                if let Some(object) = state_guard.get_object(&object_id)?
                    && CoinModule::is_coin_type_for(&object.type_, &token_type)
                {
                    count += 1;
                }
            }
            count
        } else {
            0
        };

        holders.push(FungibleAssetHolder {
            owner: owner.to_hex_literal(),
            balance,
            coin_object_count,
        });
    }
    Ok(holders)
}

fn is_public_asset_holder(owner: &AccountAddress) -> bool {
    *owner != AccountAddress::ZERO
}

/// Handle get owner request
pub async fn handle_get_owner(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let owner: String = match parse_params(request.id, &request.params) {
        Ok(owner) => owner,
        Err(response) => return *response,
    };

    match state.engine.get_owner_info(&owner) {
        Some(info) => respond_with_serialize(request.id, info),
        None => internal_error_response(request.id, "Owner not found"),
    }
}

/// Handle get token balance request
pub async fn handle_get_token_balance(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let req_data: GetTokenBalanceRequest = match parse_params(request.id, &request.params) {
        Ok(data) => data,
        Err(response) => return *response,
    };

    let owner_info = match state.engine.get_owner_info(&req_data.owner) {
        Some(info) => info,
        None => return internal_error_response(request.id, "Owner not found"),
    };

    let target_token = CoinModule::normalize_token_type(&req_data.token_type);
    let final_balance = owner_info.balances.get(&target_token).copied().unwrap_or(0);
    let state_guard = state.engine.state_read();
    let decimals = get_token_decimals(&state_guard, &target_token);

    RpcResponse {
        jsonrpc: "2.0".into(),
        result: Some(serde_json::json!({
            "token_type": req_data.token_type,
            "balance": final_balance,
            "decimals": decimals,
        })),
        error: None,
        id: request.id,
    }
}

/// Handle get owner balances request.
pub async fn handle_get_owner_balances(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let req_data: GetOwnerBalancesRequest = match parse_params(request.id, &request.params) {
        Ok(data) => data,
        Err(response) => return *response,
    };

    let owner_info = match state.engine.get_owner_info(&req_data.owner) {
        Some(info) => info,
        None => return internal_error_response(request.id, "Owner not found"),
    };

    let state_guard = state.engine.state_read();
    let balances: Vec<_> = owner_info
        .balances
        .into_iter()
        .filter(|(token_type, amount)| *amount > 0 || token_type == GAS_COIN)
        .map(|(token_type, balance)| build_balance_json(&state_guard, token_type, balance))
        .collect();

    RpcResponse {
        jsonrpc: "2.0".into(),
        result: Some(serde_json::json!({ "owner": req_data.owner, "balances": balances })),
        error: None,
        id: request.id,
    }
}

/// Handle list tokens request ( O(1) Optimized from Global Cache)
pub async fn handle_list_tokens(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let state_guard = state.engine.state_read();

    let mut token_types: BTreeSet<String> =
        state_guard.global_token_supplies.keys().cloned().collect();
    token_types.insert(GAS_COIN.to_string());

    if let Ok(Some(keys)) = state_guard.load_internal::<Vec<String>>(b"treasury_index") {
        for key in keys {
            let token_type = key.strip_prefix("treasury:").unwrap_or(&key);
            // Normalize so 0x02/0x2 spellings dedupe to one row.
            token_types.insert(CoinModule::normalize_token_type(token_type));
        }
    }
    // Global cache keys are normalized on write, but normalize defensively
    // so legacy raw-spelling entries can't produce duplicate rows.
    let token_types: BTreeSet<String> = token_types
        .into_iter()
        .map(|t| CoinModule::normalize_token_type(&t))
        .collect();

    // Batch: one object pass for all tokens instead of one full scan each.
    let ordered_types: Vec<String> = token_types.into_iter().collect();
    let summaries = match state_guard.token_supply_summaries(&ordered_types) {
        Ok(summaries) => summaries,
        Err(e) => {
            warn!("[RPC] Failed to build supply summaries: {}", e);
            return respond_with_serialize(request.id, Vec::<serde_json::Value>::new());
        }
    };
    let vals: Vec<serde_json::Value> = summaries
        .into_iter()
        .map(|summary| {
            let token_type = summary.token_type.clone();
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
            })
        })
        .collect();

    respond_with_serialize(request.id, vals)
}

pub async fn handle_get_fungible_asset(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let req_data: GetFungibleAssetRequest = match parse_params(request.id, &request.params) {
        Ok(data) => data,
        Err(response) => return *response,
    };

    let state_guard = state.engine.state_read();
    let token_type = CoinModule::normalize_token_type(&req_data.token_type);
    let summary = match state_guard.token_supply_summary(&token_type) {
        Ok(summary) => summary,
        Err(e) => return internal_error_response(request.id, e.to_string()),
    };
    let holders_count = match collect_fungible_asset_holders(&state_guard, &token_type, None, false)
        .map(|h| h.len())
    {
        Ok(count) => count,
        Err(e) => return internal_error_response(request.id, e.to_string()),
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

    respond_with_serialize(
        request.id,
        FungibleAssetInfo {
            token_type: summary.token_type,
            name,
            symbol,
            decimals: get_token_decimals(&state_guard, &token_type),
            description,
            icon_url,
            total_supply: summary.total_supply,
            wallet_visible_supply: summary.wallet_visible_supply,
            circulating_supply: summary.wallet_visible_supply,
            object_locked_supply: summary.object_locked_supply,
            accounted_supply: summary.accounted_supply,
            untracked_supply: summary.untracked_supply,
            holders_count,
            verified: token_type == GAS_COIN,
        },
    )
}

pub async fn handle_get_fungible_asset_holders(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let req_data: GetFungibleAssetHoldersRequest = match parse_params(request.id, &request.params) {
        Ok(data) => data,
        Err(response) => return *response,
    };

    let state_guard = state.engine.state_read();
    let token_type = CoinModule::normalize_token_type(&req_data.token_type);
    let holder_limit = req_data.limit.unwrap_or(100).min(500);
    let holders =
        match collect_fungible_asset_holders(&state_guard, &token_type, Some(holder_limit), true) {
            Ok(holders) => holders,
            Err(e) => return internal_error_response(request.id, e.to_string()),
        };

    respond_with_serialize(
        request.id,
        FungibleAssetHoldersResponse {
            token_type,
            holders,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_asset_holders_exclude_zero_system_address() {
        assert!(!is_public_asset_holder(&AccountAddress::ZERO));
    }

    #[test]
    fn public_asset_holders_include_regular_wallet_address() {
        let owner = AccountAddress::from_hex_literal(
            "0x3141a487d7a5382bb435c0ad39a6060067765e60e45b50953a0050bcf24b03a3",
        )
        .expect("valid wallet address");

        assert!(is_public_asset_holder(&owner));
    }
}
