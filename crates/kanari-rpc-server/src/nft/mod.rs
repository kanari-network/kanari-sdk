use crate::{RpcServerState, respond_with_serialize};
use kanari_rpc_api::{RpcError, RpcRequest, RpcResponse};
use kanari_types::address::Address;
use serde_json::json;

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

    // ดึงรายการ Object ID ทั้งหมดที่ Address นี้เป็นเจ้าของ
    let owned_ids = match state_guard.get_owned_objects(&addr) {
        Ok(ids) => ids,
        Err(_) => return respond_with_serialize(request.id, Vec::<serde_json::Value>::new()),
    };

    let mut nfts = Vec::new();

    for id in owned_ids {
        if let Ok(Some(obj)) = state_guard.get_object(&id) {
            // กรองเฉพาะ Object ที่มาจาก module nft (เช่น '::nft::KariKid')
            if obj.type_.contains("::nft::") {
                nfts.push(json!({
                    "object_id": id,
                    "type": obj.type_,
                    "owner": address_str,
                    "version": obj.version,
                    // ข้อมูลดิบจาก Move Storage (สามารถนำไป parse ต่อใน frontend หรือ backend ได้)
                    "data": obj.data,
                }));
            }
        }
    }

    respond_with_serialize(request.id, nfts)
}

pub async fn handle_list_collections(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let state_guard = state.engine.state.read().unwrap_or_else(|p| p.into_inner());

    // ดึงจาก Index (O(1)) แทนการวนลูป All Objects
    let ids = state_guard.get_all_collection_ids();
    let mut collections = Vec::new();

    for id in ids {
        if let Ok(Some(obj)) = state_guard.get_object(&id) {
            collections.push(serde_json::json!({
                "id": id,
                "type": obj.type_,
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
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params("Invalid collection id")),
                id: request.id,
            };
        }
    };

    let state_guard = state.engine.state.read().unwrap_or_else(|p| p.into_inner());

    // ดึงจาก Index (O(1))
    let nft_ids = state_guard.get_collection_nft_ids(&coll_id);
    let mut nfts = Vec::new();

    for id in nft_ids {
        if let Ok(Some(obj)) = state_guard.get_object(&id) {
            nfts.push(serde_json::json!({
                "object_id": id,
                "type": obj.type_,
                "data": obj.data,
            }));
        }
    }
    respond_with_serialize(request.id, nfts)
}
