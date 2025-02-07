use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use futures::FutureExt;
use jsonrpc_core::{IoHandler, Params, Result as JsonRpcResult, Error as RpcError};
use jsonrpc_http_server::{ServerBuilder, AccessControlAllowOrigin, DomainsValidation};
use k2::{blockchain::BLOCKCHAIN, chain_id::CHAIN_ID};
use network::NetworkConfig;
use serde_json::{json, Value as JsonValue};
use mona_storage::file_storage::{FileStorage, StorageError2};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};



#[derive(Serialize)]
struct FileInfo {
    id: String,
    filename: String,
    size: u64,
    content_type: String,
}

#[derive(Deserialize)]
struct UploadParams {
    filename: String,
    data: String, // base64 encoded file content
}

// Add new RPC methods for file operations
fn upload_file(params: Params) -> JsonRpcResult<JsonValue> {
    let upload_params: UploadParams = params.parse()
        .map_err(|e| RpcError::invalid_params(format!("Invalid parameters: {}", e)))?;

    // Decode base64 data
    let file_data = BASE64.decode(upload_params.data)
        .map_err(|e| RpcError::invalid_params(format!("Invalid base64 data: {}", e)))?;

    // Initialize storage
    FileStorage::init_storage()
        .map_err(|e| RpcError::internal_error())?;

    // Create new storage instance
    let storage = FileStorage::new()
        .map_err(|e| RpcError::internal_error())?;

    // Store the file
    match storage.store(&upload_params.filename, &file_data) {
        Ok(stored) => {
            let file_info = FileInfo {
                id: stored.id.to_string(),
                filename: stored.metadata.filename,
                size: stored.metadata.size,
                content_type: stored.metadata.content_type,
            };
            Ok(serde_json::to_value(file_info).unwrap())
        },
        Err(e) => Err(RpcError::internal_error())
    }
}

fn get_file(params: Params) -> JsonRpcResult<JsonValue> {
    let file_id: String = params.parse()
        .map_err(|e| RpcError::invalid_params(format!("Invalid file ID: {}", e)))?;

    match FileStorage::get_by_id(&file_id) {
        Ok(storage) => {
            let file_data = std::fs::read(&storage.path)
                .map_err(|_| RpcError::internal_error())?;
            
            let response = json!({
                "id": storage.id.to_string(),
                "filename": storage.metadata.filename,
                "size": storage.metadata.size,
                "content_type": storage.metadata.content_type,
                "data": BASE64.encode(file_data)
            });
            
            Ok(response)
        },
        Err(StorageError2::NotFound) => Err(RpcError::invalid_params("File not found")),
        Err(_) => Err(RpcError::internal_error())
    }
}


// RPC server
fn get_latest_block(_params: Params) -> JsonRpcResult<JsonValue> {
    unsafe {
        if let Some(block) = BLOCKCHAIN.back() {
            Ok(serde_json::to_value(block).unwrap())
        } else {
            Ok(JsonValue::Null)
        }
    }
}

fn get_chain_id(_params: Params) -> JsonRpcResult<JsonValue> {
    Ok(JsonValue::String(CHAIN_ID.to_string()))
}

fn get_block_by_index(params: Params) -> JsonRpcResult<JsonValue> {
    let index: u32 = params.parse().map_err(|e| jsonrpc_core::Error::invalid_params(format!("Invalid index parameter: {}", e)))?;

    unsafe {
        if let Some(block) = BLOCKCHAIN.iter().find(|b| b.index == index) {
            Ok(serde_json::to_value(block).unwrap())
        } else {
            Ok(JsonValue::Null)
        }
    }
}

pub async fn start_rpc_server(network_config: NetworkConfig) {
    let mut io = IoHandler::new();

    // Add file operations
    io.add_method("upload_file", |params| {
        futures::future::ready(upload_file(params)).boxed()
    });

    io.add_method("get_file", |params| {
        futures::future::ready(get_file(params)).boxed()
    });


    // Add RPC methods
    io.add_method("get_latest_block", |params| {
        futures::future::ready(get_latest_block(params)).boxed()
    });

    io.add_method("get_chain_id", |params| {
        futures::future::ready(get_chain_id(params)).boxed()
    });

    io.add_method("get_block_by_index", |params| {
        futures::future::ready(get_block_by_index(params)).boxed()
    });

    // Configure socket address
    let local_addr = SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        network_config.port
    );

    // Create CORS settings that allow the domain pattern
    let allowed_origins = vec![
        AccessControlAllowOrigin::Any, // Allow all during development
        AccessControlAllowOrigin::Value(format!("https://{}", network_config.domain).into()),
    ];

    match ServerBuilder::new(io)
        .cors(DomainsValidation::AllowOnly(allowed_origins))
        .start_http(&local_addr)
    {
        Ok(server) => {
            println!("RPC server running on http://127.0.0.1:{}", network_config.port);
            println!("Accessible via https://{}", network_config.domain);
            server.wait();
        }
        Err(e) => {
            eprintln!("Failed to start RPC server: {}", e);
            std::process::exit(1);
        }
    }
}