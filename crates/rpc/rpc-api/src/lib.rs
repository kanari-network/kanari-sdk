use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use futures::FutureExt;
use jsonrpc_core::{IoHandler, Params, Result as JsonRpcResult};
use jsonrpc_http_server::{ServerBuilder, AccessControlAllowOrigin, DomainsValidation};
use k2::{blockchain::BLOCKCHAIN, chain_id::CHAIN_ID};
use network::NetworkConfig;
use serde_json::Value as JsonValue;


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