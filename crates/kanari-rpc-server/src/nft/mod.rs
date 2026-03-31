use crate::{RpcServerState, respond_with_serialize};
use kanari_rpc_api::{RpcError, RpcRequest, RpcResponse};
use kanari_types::address::Address;
use serde::Deserialize;
use serde_json::json;

#[allow(dead_code)]
#[derive(Deserialize)]
struct MoveString {
    bytes: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct MoveUrl {
    inner: MoveString,
}

// ==========================================
// Universal Standard Structure
// Match exactly with nft.move to prevent Trailing Bytes Error
// ==========================================
#[allow(dead_code)]
#[derive(Deserialize)]
struct UniversalStandardNft {
    _id: [u8; 32],                     // 1. UID
    name: MoveString,                  // 2. Name
    image_url: MoveUrl,                // 3. URL
    description: MoveString,           // 4. Description
    attribute_keys: Vec<MoveString>,   // 5. Keys
    attribute_values: Vec<MoveString>, // 6. Values

    // 🚨 Include all remaining fields to make BCS read data completely 🚨
    number: MoveString,      // 7. Number
    collection_id: [u8; 32], // 8. Collection ID (Address)
    creator: [u8; 32],       // 9. Creator (Address)
}

// Collection structure matching collection.move
#[allow(dead_code)]
#[derive(Deserialize)]
struct ParsedCollection {
    _id: [u8; 32],
    name: MoveString,
    description: MoveString,
    banner_url: MoveUrl,  // 🚨 Added to match Move
    website_url: MoveUrl, // 🚨 Added to match Move
    creator: [u8; 32],
    max_supply: u64,
}

// ==========================================
// Improved parse_any_nft function to provide more complete data
// ==========================================
fn parse_any_nft(_type_tag: &str, data: &[u8]) -> Option<serde_json::Value> {
    if let Ok(parsed) = bcs::from_bytes::<UniversalStandardNft>(data) {
        return Some(json!({
            "name": move_str_to_string(&parsed.name),
            "image_url": move_str_to_string(&parsed.image_url.inner),
            "description": move_str_to_string(&parsed.description),
            "number": move_str_to_string(&parsed.number),
            "creator": format!("0x{}", hex::encode(parsed.creator)), // 🚨 Added creator info
            "attributes": {
                "keys": move_str_vec_to_strings(&parsed.attribute_keys),
                "values": move_str_vec_to_strings(&parsed.attribute_values),
            }
        }));
    }
    None
}

// ==========================================
// Helper functions
// ==========================================
fn move_str_to_string(m: &MoveString) -> String {
    String::from_utf8_lossy(&m.bytes).to_string()
}

fn move_str_vec_to_strings(vec: &[MoveString]) -> Vec<String> {
    vec.iter().map(move_str_to_string).collect()
}

pub async fn handle_get_owned_nfts(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let address_str: String = match serde_json::from_value(request.params.clone()) {
        Ok(addr) => addr,
        Err(_) => {
            return RpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(RpcError::invalid_params("Invalid address")),
                id: request.id,
            };
        }
    };

    let addr = match Address::parse_to_account_address(&address_str) {
        Ok(a) => a,
        Err(_) => {
            return RpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(RpcError::invalid_params("Invalid address format")),
                id: request.id,
            };
        }
    };

    let state_guard = state.engine.state.read().unwrap_or_else(|p| p.into_inner());
    let owned_ids = match state_guard.get_owned_objects(&addr) {
        Ok(ids) => ids,
        Err(_) => return respond_with_serialize(request.id, Vec::<serde_json::Value>::new()),
    };

    let mut nfts = Vec::new();

    for id in owned_ids {
        if let Ok(Some(obj)) = state_guard.get_object(&id) {
            // Allow if it's ::nft:: or contains the word NFT
            if obj.type_.contains("::nft::") || obj.type_.to_uppercase().contains("NFT") {
                nfts.push(json!({
                    "object_id": id,
                    "type": obj.type_,
                    "owner": address_str,
                    "version": obj.version,
                    "parsed": parse_any_nft(&obj.type_, &obj.data),
                    "data": obj.data,
                }));
            }
        }
    }

    respond_with_serialize(request.id, nfts)
}

// Function to retrieve all collections and attempt to convert data completely according to the structure defined in Move
pub async fn handle_list_collections(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let state_guard = state.engine.state.read().unwrap_or_else(|p| p.into_inner());
    let ids = state_guard.get_all_collection_ids();
    let mut collections = Vec::new();

    for id in ids {
        if let Ok(Some(obj)) = state_guard.get_object(&id) {
            // 🚨 Create default values first
            let mut name = "Unknown Collection".to_string();
            let mut description = "".to_string();
            let mut banner_url = "".to_string();
            let mut website_url = "".to_string();
            let mut max_supply = 0u64;

            if obj.type_.contains("::collection::Collection")
                && let Ok(parsed) = bcs::from_bytes::<ParsedCollection>(&obj.data) {
                name = move_str_to_string(&parsed.name);
                description = move_str_to_string(&parsed.description);
                banner_url = move_str_to_string(&parsed.banner_url.inner);
                website_url = move_str_to_string(&parsed.website_url.inner);
                max_supply = parsed.max_supply;
            }

            collections.push(serde_json::json!({
                "id": id,
                "type": obj.type_,
                "name": name,
                "description": description,
                "banner_url": banner_url,
                "website_url": website_url,
                "max_supply": max_supply,
                "owner": format!("{:#x}", obj.owner),
                "data": obj.data,
            }));
        }
    }
    respond_with_serialize(request.id, collections)
}

pub async fn handle_get_nfts_by_collection(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let coll_id: String = match serde_json::from_value(request.params.clone()) {
        Ok(id) => id,
        Err(_) => {
            return RpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(RpcError::invalid_params("Invalid id")),
                id: request.id,
            };
        }
    };

    let state_guard = state.engine.state.read().unwrap_or_else(|p| p.into_inner());
    let nft_ids = state_guard.get_collection_nft_ids(&coll_id);
    let mut nfts = Vec::new();

    for id in nft_ids {
        if let Ok(Some(obj)) = state_guard.get_object(&id) {
            nfts.push(serde_json::json!({
                "object_id": id,
                "type": obj.type_,
                "parsed": parse_any_nft(&obj.type_, &obj.data),
                "data": obj.data,
            }));
        }
    }
    respond_with_serialize(request.id, nfts)
}