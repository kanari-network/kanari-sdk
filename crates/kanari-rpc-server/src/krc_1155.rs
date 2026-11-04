// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{RpcServerState, invalid_params_response, respond_with_serialize};
use kanari_rpc_api::{RpcRequest, RpcResponse};
use kanari_types::address::Address;
use kanari_types::krc_1155::Krc1155Module;
use kanari_types::transaction::ObjectOwnerKind;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct MoveString {
    bytes: Vec<u8>,
}

#[derive(Deserialize)]
struct MoveUrl {
    url: MoveString,
}

#[derive(Deserialize)]
struct Krc1155TokenType {
    _id: [u8; 32],
    name: MoveString,
    description: MoveString,
    image_url: MoveUrl,
    max_supply: u64,
    total_supply: u64,
}

fn move_str(s: &MoveString) -> String {
    String::from_utf8_lossy(&s.bytes).to_string()
}

fn parse_token_type(obj_data: &[u8]) -> Option<serde_json::Value> {
    let parsed = bcs::from_bytes::<Krc1155TokenType>(obj_data).ok()?;
    Some(json!({
        "name": move_str(&parsed.name),
        "description": move_str(&parsed.description),
        "image_url": move_str(&parsed.image_url.url),
        "max_supply": parsed.max_supply,
        "total_supply": parsed.total_supply,
    }))
}

struct LabeledSingleParam {
    value: String,
}

impl<'de> Deserialize<'de> for LabeledSingleParam {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map = serde_json::Map::<String, serde_json::Value>::deserialize(deserializer)?;
        let value = map
            .values()
            .next()
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                serde::de::Error::custom(
                    "expected a single string param or { \"field\": \"value\" }",
                )
            })?
            .to_string();
        Ok(LabeledSingleParam { value })
    }
}

fn parse_single_string_param(
    id: u64,
    params: &serde_json::Value,
    field: &str,
) -> Result<String, Box<RpcResponse>> {
    if let serde_json::Value::String(s) = params {
        return Ok(s.clone());
    }
    match crate::parse_labeled_params::<LabeledSingleParam>(id, params, field) {
        Ok(labeled) => Ok(labeled.value),
        Err(response) => Err(response),
    }
}

pub async fn handle_get_owned_krc1155_balances(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let owner: String = match parse_single_string_param(request.id, &request.params, "owner") {
        Ok(owner) => owner,
        Err(response) => return *response,
    };

    let addr = match Address::parse_to_account_address(&owner) {
        Ok(a) => a,
        Err(_) => return invalid_params_response(request.id, "Invalid owner address format"),
    };

    let state_guard = state.engine.state_read();
    let owned_ids = match state_guard.get_owned_objects(&addr) {
        Ok(ids) => ids,
        Err(_) => return respond_with_serialize(request.id, Vec::<serde_json::Value>::new()),
    };

    let mut grouped: std::collections::BTreeMap<String, serde_json::Value> =
        std::collections::BTreeMap::new();
    for id in owned_ids {
        if let Ok(Some(obj)) = state_guard.get_object(&id)
            && matches!(&obj.owner_kind, ObjectOwnerKind::AddressOwner(o)
                if Address::parse_to_account_address(o).ok() == Some(addr))
            && Krc1155Module::is_krc1155_object_type(&obj.type_)
        {
            let token_type =
                Krc1155Module::token_type_from_balance_bcs(&obj.data).map(|a| format!("{:#x}", a));
            let amount = Krc1155Module::amount_from_balance_bcs(&obj.data).unwrap_or(0);
            let token_type_key = token_type.clone().unwrap_or_default();
            let token_details = token_type
                .as_ref()
                .and_then(|tt| state_guard.get_object(tt).ok().flatten())
                .and_then(|o| parse_token_type(&o.data));

            let entry = grouped.entry(token_type_key.clone()).or_insert_with(|| {
                let mut base = json!({
                    "owner": owner,
                    "token_type": token_type,
                    "amount": 0u64,
                    "balances": Vec::<serde_json::Value>::new(),
                });
                if let Some(details) = &token_details
                    && let Some(obj_map) = base.as_object_mut()
                    && let Some(d) = details.as_object()
                {
                    for (k, v) in d {
                        obj_map.insert(k.clone(), v.clone());
                    }
                }
                base
            });
            if let Some(obj_map) = entry.as_object_mut() {
                let current = obj_map.get("amount").and_then(|v| v.as_u64()).unwrap_or(0);
                obj_map.insert("amount".to_string(), json!(current + amount));
                if let Some(arr) = obj_map.get_mut("balances").and_then(|v| v.as_array_mut()) {
                    arr.push(json!({"object_id": id, "amount": amount}));
                }
            }
        }
    }

    let balances: Vec<serde_json::Value> = grouped.into_values().collect();
    respond_with_serialize(request.id, balances)
}

pub async fn handle_get_krc1155_token_type(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let id: String = match parse_single_string_param(request.id, &request.params, "id") {
        Ok(id) => id,
        Err(response) => return *response,
    };

    let state_guard = state.engine.state_read();
    match state_guard.get_object(&id) {
        Ok(Some(obj)) if Krc1155Module::is_token_type_object_type(&obj.type_) => {
            let mut result = json!({
                "object_id": id,
                "type": obj.type_,
                "data": obj.data,
            });
            if let Some(details) = parse_token_type(&obj.data)
                && let (Some(r), Some(d)) = (result.as_object_mut(), details.as_object())
            {
                for (k, v) in d {
                    r.insert(k.clone(), v.clone());
                }
            }
            respond_with_serialize(request.id, result)
        }
        _ => invalid_params_response(request.id, "KRC-1155 token type not found"),
    }
}
