use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use futures::FutureExt;
use jsonrpc_core::IoHandler;
// Add Axum imports
use axum::{
    routing::{get, post},
    Router, Json, Extension, extract::DefaultBodyLimit,
    http::{Method, StatusCode, header},
    response::IntoResponse,
    serve,
};
use tokio::net::TcpListener; // Add TcpListener import
use tower_http::cors::{CorsLayer, Any};
use serde_json::{json, Value};
use std::sync::Arc;
use metadata::{get_file, upload_file};
use network::NetworkConfig;
mod metadata;
mod stake;
mod get_block;
mod accounts;
mod move_api; // Add the new module

use stake::{get_staking_info, get_staking_stats, stake_tokens, unstake_tokens};
use get_block::{
    get_all_blocks, 
    get_account_details, 
    search_transactions, 
    get_transaction_by_id, 
    get_transaction_status, 
    get_gas_fee_info
};
use accounts::{get_blockchain_status, get_wallets, list_accounts, transfer_tokens};
use move_api::{
    list_modules,
    get_module,
    execute_function,
    get_transaction,
    get_vm_state
};

// Format function locally since panorama::utils is not available
fn format_kari_amount(ka_amount: u64) -> String {
    const KA_PER_KARI: u64 = 1_000_000_000;
    
    // Calculate whole and fractional parts
    let whole_kari = ka_amount / KA_PER_KARI;
    let fractional_ka = ka_amount % KA_PER_KARI;
    
    // Format with thousands separators and 9 decimal places
    let whole_formatted = format!("{}", whole_kari)
        .chars()
        .rev()
        .collect::<Vec<_>>()
        .chunks(3)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(",")
        .chars()
        .rev()
        .collect::<String>();
    
    format!("{}.{:09}", whole_formatted, fractional_ka)
}

/// Starts the RPC server for file operations
pub async fn start_rpc_server(network_config: NetworkConfig) -> Result<(), tokio::io::Error> {
    // Load kanari config to keep track of node connection details
    let mut kanari_config = match common::load_kanari_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Warning: Failed to load kanari configuration: {}", e);
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
        }
    };

    // Create a shared state that can be accessed by all handlers
    let state = Arc::new(AppState {
        io_handler: create_io_handler(),
        network_config: network_config.clone(),
    });

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    // Build the Axum router
    let app = Router::new()
        .route("/", post(handle_rpc_request))
        .route("/health", get(health_check))
        .route("/ready", get(ready_check))
        .layer(cors)
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB max request size
        .layer(Extension(state));

    // Configure socket address - bind only to localhost if in localhost_only mode
    let bind_addr = if network_config.localhost_only {
        println!("Localhost-only mode: Binding to 127.0.0.1");
        SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), // Bind only to localhost
            network_config.port
        )
    } else {
        SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), // Bind to all interfaces
            network_config.port
        )
    };

    // First create a TCP listener
    let listener = TcpListener::bind(bind_addr).await?;
    println!("Starting Axum HTTP server on {}", bind_addr);
    
    // Then serve the app with the listener
    match serve(listener, app.into_make_service()).await {
        Ok(_) => {
            // Display basic server information
            if network_config.localhost_only {
                println!("Running in LOCALHOST-ONLY mode");
                println!("HTTP server running on http://127.0.0.1:{}", network_config.port);
                println!("Note: The node will not connect to or accept connections from other nodes");
            } else {
                println!("HTTP server running on http://{}:{}", network_config.node_address, network_config.port);
                
                // Update kanari config with the actual running port
                update_kanari_config(&mut kanari_config, network_config.port);
                
                if !network_config.peers.is_empty() {
                    println!("Connected to peers:");
                    for peer in &network_config.peers {
                        println!("  - {}", peer);
                    }
                }
            }
            
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to start HTTP server: {}", e);
            Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        }
    }
}

// Update the kanari config with the new port
fn update_kanari_config(kanari_config: &mut serde_yaml::Value, port: u16) {
    if let Some(kanari_mapping) = kanari_config.as_mapping_mut() {
        // Extract active_env as a String to avoid immutable borrow persisting
        let active_env = kanari_mapping.get("active_env")
                         .and_then(|v| v.as_str())
                         .unwrap_or("local")
                         .to_string();
                         
        if let Some(envs) = kanari_mapping.get_mut("envs").and_then(|v| v.as_sequence_mut()) {
            for env in envs {
                if let Some(alias) = env.get("alias").and_then(|v| v.as_str()) {
                    if alias == active_env {
                        // Update RPC URL with the new port
                        env["rpc"] = serde_yaml::Value::String(format!("http://127.0.0.1:{}", port));
                        break;
                    }
                }
            }
        }
        
        // Save the updated kanari config
        if let Err(e) = common::save_kanari_config(&kanari_config) {
            eprintln!("Warning: Failed to save updated kanari config: {}", e);
        }
    }
}

// Application state that will be shared across handlers
struct AppState {
    io_handler: IoHandler,
    #[allow(dead_code)]
    network_config: NetworkConfig,
}

// Create the IoHandler with all the RPC methods
fn create_io_handler() -> IoHandler {
    let mut io = IoHandler::new();

    // Add file operations
    io.add_method("upload_file", |params| {
        futures::future::ready(upload_file(params)).boxed()
    });

    io.add_method("get_file", |params| {
        futures::future::ready(get_file(params)).boxed()
    });

    // Add blockchain operations
    io.add_method("blockchain_status", |params| {
        futures::future::ready(get_blockchain_status(params)).boxed()
    });
    
    io.add_method("list_accounts", |params| {
        futures::future::ready(list_accounts(params)).boxed()
    });
    
    io.add_method("transfer", |params| {
        futures::future::ready(transfer_tokens(params)).boxed()
    });
    
    io.add_method("get_wallets", |params| {
        futures::future::ready(get_wallets(params)).boxed()
    });

    // Use methods from get_block.rs
    io.add_method("get_all_blocks", |params| {
        futures::future::ready(get_all_blocks(params)).boxed()
    });
    
    io.add_method("get_account_details", |params| {
        futures::future::ready(get_account_details(params)).boxed()
    });
    
    io.add_method("search_transactions", |params| {
        futures::future::ready(search_transactions(params)).boxed()
    });

    io.add_method("get_transaction_by_id", |params| {
        futures::future::ready(get_transaction_by_id(params)).boxed()
    });

    io.add_method("get_transaction_status", |params| {
        futures::future::ready(get_transaction_status(params)).boxed()
    });

    io.add_method("get_gas_fee_info", |params| {
        futures::future::ready(get_gas_fee_info(params)).boxed()
    });

    // Add staking operations
    io.add_method("stake_tokens", |params| {
        futures::future::ready(stake_tokens(params)).boxed()
    });

    io.add_method("unstake_tokens", |params| {
        futures::future::ready(unstake_tokens(params)).boxed()
    });

    io.add_method("get_staking_info", |params| {
        futures::future::ready(get_staking_info(params)).boxed()
    });

    io.add_method("get_staking_stats", |params| {
        futures::future::ready(get_staking_stats(params)).boxed()
    });

    // Add Move VM operations
    io.add_method("list_modules", |params| {
        futures::future::ready(list_modules(params)).boxed()
    });

    io.add_method("get_module", |params| {
        futures::future::ready(get_module(params)).boxed()
    });

    io.add_method("execute_function", |params| {
        futures::future::ready(execute_function(params)).boxed()
    });

    io.add_method("get_transaction", |params| {
        futures::future::ready(get_transaction(params)).boxed()
    });

    io.add_method("get_vm_state", |params| {
        futures::future::ready(get_vm_state(params)).boxed()
    });

    io
}

// Handle RPC requests by forwarding them to the IoHandler
async fn handle_rpc_request(
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let response = state.io_handler.handle_request_sync(&serde_json::to_string(&payload).unwrap());
    
    match response {
        Some(result) => {
            let json_result: Value = serde_json::from_str(&result).unwrap_or_else(|_| {
                json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32603,
                        "message": "Internal error",
                    },
                    "id": null
                })
            });
            
            (StatusCode::OK, Json(json_result))
        },
        None => {
            let error_response = json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32600,
                    "message": "Invalid Request",
                },
                "id": null
            });
            
            (StatusCode::BAD_REQUEST, Json(error_response))
        }
    }
}

// Health check endpoint
async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

// Readiness check endpoint
async fn ready_check() -> impl IntoResponse {
    StatusCode::OK
}