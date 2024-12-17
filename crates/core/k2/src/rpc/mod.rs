use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use futures::FutureExt;
use jsonrpc_core::{IoHandler, Params, Result as JsonRpcResult};
use jsonrpc_http_server::{ServerBuilder, AccessControlAllowOrigin, DomainsValidation};
use network::NetworkConfig;
use serde_json::Value as JsonValue;

use crate::{blockchain::BLOCKCHAIN, chain_id::CHAIN_ID};


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

    // Existing methods
    io.add_method("get_latest_block", |params| {
        futures::future::ready(get_latest_block(params)).boxed()
    });

    io.add_method("get_chain_id", |params| {
        futures::future::ready(get_chain_id(params)).boxed()
    });

    io.add_method("get_block_by_index", |params| {
        futures::future::ready(get_block_by_index(params)).boxed()
    });

    // Create socket address from network config
    let addr = SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        network_config.port
    );

    match ServerBuilder::new(io)
        .cors(DomainsValidation::AllowOnly(vec![AccessControlAllowOrigin::Any]))
        .start_http(&addr)
    {
        Ok(server) => {
            println!("RPC server running on http://127.0.0.1:{}", network_config.port);
            server.wait();
        }
        Err(e) => {
            eprintln!("Failed to start RPC server: {}", e);
            std::process::exit(1);
        }
    }
}