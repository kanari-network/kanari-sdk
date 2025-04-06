use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, str::FromStr};
use futures::FutureExt;
use jsonrpc_core::{IoHandler, Params, Result as JsonRpcResult, Error as RpcError, ErrorCode};
use jsonrpc_http_server::{ServerBuilder, AccessControlAllowOrigin, DomainsValidation};
use metadata::{get_file, upload_file};
use mona_types::address::Address;
use panorama::{blockchain::{BLOCKCHAIN_DATA, load_blockchain_with_retry}, chain_id::CHAIN_ID, blockchain::BALANCES};
use panorama::simulation::process_transfer;
use network::NetworkConfig;
use serde_json::{json, Value as JsonValue};

mod metadata;// Add this at the top with the other modules
mod stake;
mod get_block; // Add the new module

use serde::Deserialize;
use stake::{get_staking_info, get_staking_stats, stake_tokens, unstake_tokens};
use tokio::sync::mpsc;
use key::{load_wallet, list_wallet_files};

// Use functions from the get_block module
use get_block::{
    get_all_blocks, 
    get_account_details, 
    search_transactions, 
    get_transaction_by_id, 
    get_transaction_status, 
    get_gas_fee_info
};

// Blockchain API structures
#[derive(Deserialize)]
struct TransferParams {
    from: String,
    to: String,
    amount: f64,
    password: String,
    priority_boost: Option<u64>, // Optional priority boost for gas fee
}

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

// Blockchain API methods
fn get_blockchain_status(_params: Params) -> JsonRpcResult<JsonValue> {
    // Load blockchain data if needed
    if let Err(e) = load_blockchain_with_retry() {
        return Err(RpcError {
            code: ErrorCode::InternalError,
            message: format!("Failed to load blockchain: {}", e),
            data: None,
        });
    }
    
    let block_count = BLOCKCHAIN_DATA.len();
    
    // Get the latest block info
    let latest_block = if block_count > 0 {
        let block = BLOCKCHAIN_DATA.get_block(block_count - 1);
        block.map(|b| json!({
            "index": b.index,
            "hash": b.hash,
            "timestamp": b.timestamp,
            "transactions": b.transactions.len(),
            "miner": b.address,
        }))
    } else {
        None
    };
    
    // Get genesis timestamp if available
    let genesis_timestamp = if block_count > 0 {
        BLOCKCHAIN_DATA.get_block(0).map(|b| b.timestamp)
    } else {
        None
    };
    
    // Count transactions across all blocks
    let total_transactions = BLOCKCHAIN_DATA.iter()
        .into_iter()
        .fold(0, |acc, block| acc + block.transactions.len());
    
    let response = json!({
        "chain_id": CHAIN_ID.to_string(),
        "block_height": block_count,
        "block_count": block_count,
        "latest_block": latest_block,
        "total_transactions": total_transactions,
        "genesis_timestamp": genesis_timestamp,
    });
    
    Ok(response)
}

fn list_accounts(_params: Params) -> JsonRpcResult<JsonValue> {
    // Load blockchain data if needed
    if let Err(e) = load_blockchain_with_retry() {
        return Err(RpcError {
            code: ErrorCode::InternalError,
            message: format!("Failed to load blockchain: {}", e),
            data: None,
        });
    }
    
    let balances = match BALANCES.lock() {
        Ok(balances) => balances,
        Err(_) => return Err(RpcError {
            code: ErrorCode::InternalError,
            message: "Failed to access balances".to_string(),
            data: None,
        }),
    };
    
    let mut accounts = Vec::new();
    for (address_str, balance) in balances.iter() {
        // Parse the address string into Address type
        let address = match Address::from_str(address_str) {
            Ok(addr) => addr,
            Err(_) => continue, // Skip invalid addresses
        };

        // Count transactions for this account
        let tx_count = BLOCKCHAIN_DATA.iter()
            .into_iter()
            .fold(0, |acc, block| {
                // Count matching transactions in each block and accumulate
                acc + block.transactions.iter()
                    .filter(|tx| tx.sender == address || tx.receiver == address)
                    .count()
            });
            
        accounts.push(json!({
            "address": address_str,
            "balance": balance,
            "balance_formatted": format_kari_amount(*balance),
            "transaction_count": tx_count,
            "is_contract": false,
        }));
    }
    
    Ok(json!({
        "accounts": accounts,
        "total": accounts.len(),
    }))
}

fn transfer_tokens(params: Params) -> JsonRpcResult<JsonValue> {
    // Parse transfer params
    let transfer_params: TransferParams = params.parse()
        .map_err(|e| RpcError::invalid_params(format!("Invalid parameters: {}", e)))?;
    
    // Load blockchain data if needed
    if let Err(e) = load_blockchain_with_retry() {
        return Err(RpcError {
            code: ErrorCode::InternalError,
            message: format!("Failed to load blockchain: {}", e),
            data: None,
        });
    }
    
    // Validate sender's address and password by loading the wallet
    match load_wallet(&transfer_params.from, &transfer_params.password) {
        Ok(_) => {
            // Calculate amount in KA units (1 KARI = 10^9 KA)
            const KA_PER_KARI: u64 = 1_000_000_000;
            let amount_ka = (transfer_params.amount * KA_PER_KARI as f64) as u64;
            
            // Create a channel for transaction notifications
            let (tx, _rx) = mpsc::channel::<String>(10);
            
            // Process transfer with password for signing and priority boost
            match process_transfer(
                &transfer_params.from, 
                &transfer_params.to, 
                amount_ka, 
                &transfer_params.password,
                transfer_params.priority_boost,
                &tx
            ) {
                Ok(transaction) => {
                    // Check if the transaction was signed correctly
                    let signature_status = if transaction.signature.is_empty() {
                        "unsigned"
                    } else {
                        match panorama::transfer_tokens::verify_transaction::verify_transaction(&transaction) {
                            Ok(true) => "valid",
                            Ok(false) => "invalid",
                            Err(_) => "unknown"
                        }
                    };
                    
                    // Include gas fee information in the response
                    Ok(json!({
                        "transaction_id": transaction.transaction_id,
                        "sender": transaction.sender,
                        "receiver": transaction.receiver,
                        "amount": transaction.amount,
                        "amount_formatted": format_kari_amount(transaction.amount),
                        "gas_fee": transaction.gas_fee,
                        "gas_fee_formatted": format_kari_amount(transaction.gas_fee),
                        "gas_collector": panorama::utils::GAS_FEE_COLLECTOR,
                        "total_cost": panorama::utils::calculate_total_transaction_cost(transaction.amount, transaction.gas_fee),
                        "total_cost_formatted": format_kari_amount(panorama::utils::calculate_total_transaction_cost(transaction.amount, transaction.gas_fee)),
                        "timestamp": transaction.timestamp,
                        "status": "pending", // Status is pending until included in a block
                        "signed": !transaction.signature.is_empty(),
                        "signature_status": signature_status  // Add signature verification status
                    }))
                },
                Err(e) => {
                    Err(RpcError {
                        code: ErrorCode::InternalError,
                        message: format!("Transfer failed: {}", e),
                        data: None,
                    })
                }
            }
        },
        Err(_) => {
            Err(RpcError {
                code: ErrorCode::InvalidParams,
                message: "Invalid wallet password".to_string(),
                data: None,
            })
        }
    }
}

fn get_wallets(_params: Params) -> JsonRpcResult<JsonValue> {
    match list_wallet_files() {
        Ok(wallets) => {
            let wallet_list = wallets.into_iter()
                .map(|(name, is_selected)| {
                    let address = name.trim_end_matches(".enc");
                    json!({
                        "address": address,
                        "selected": is_selected
                    })
                })
                .collect::<Vec<_>>();
                
            Ok(json!({
                "wallets": wallet_list,
                "count": wallet_list.len()
            }))
        },
        Err(e) => {
            Err(RpcError {
                code: ErrorCode::InternalError,
                message: format!("Failed to list wallets: {}", e),
                data: None,
            })
        }
    }
}

// The functions moved to get_block.rs have been removed from here

/// Starts the RPC server for file operations
pub async fn start_rpc_server(network_config: NetworkConfig) -> Result<(), tokio::io::Error> {
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

    // Configure socket address to bind to all interfaces
    let bind_addr = SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), // Bind to all interfaces
        network_config.port
    );

    // Create CORS settings for production
    let mut allowed_origins = vec![
        AccessControlAllowOrigin::Any, // Allow all origins for testing
        AccessControlAllowOrigin::Value(network_config.domain.clone().into()),
        AccessControlAllowOrigin::Value(format!("https://{}", network_config.domain).into()),
        AccessControlAllowOrigin::Value(format!("http://{}", network_config.domain).into()),
    ];
    
    // Add support for kanari.network subdomains
    if network_config.domain.ends_with(".kanari.network") {
        allowed_origins.push(AccessControlAllowOrigin::Value("https://*.kanari.network".into()));
        allowed_origins.push(AccessControlAllowOrigin::Value("http://*.kanari.network".into()));
    }
    
    // Check if TLS certificates are available
    let cert_path = std::env::current_dir().unwrap().join("cert.pem");
    let key_path = std::env::current_dir().unwrap().join("key.pem");
    
    let use_tls = cert_path.exists() && key_path.exists();
    
    if use_tls {
        println!("TLS certificates found, but HTTPS is not supported in this version of jsonrpc-http-server.");
        println!("Starting HTTP server instead. For secure connections, consider using a reverse proxy like Nginx or Caddy.");
    }
    
    // Start HTTP server
    match ServerBuilder::new(io)
        .cors(DomainsValidation::AllowOnly(allowed_origins))
        .threads(4) // Increase thread count for better performance
        .max_request_body_size(10 * 1024 * 1024) // 10MB max request size
        .health_api(("health", "ready")) // Add health check endpoints
        .start_http(&bind_addr)
    {
        Ok(server) => {
            println!("HTTP server running on http://{}:{}", network_config.node_address, network_config.port);
            if !network_config.peers.is_empty() {
                println!("Connected to peers:");
                for peer in &network_config.peers {
                    println!("  - {}", peer);
                }
            }
            
            // Create a channel for shutdown coordination
            let (shutdown_complete_tx, _shutdown_complete_rx) = tokio::sync::oneshot::channel();
            
            // Spawn a task to wait for the server in the background
            // We don't need to clone the server, just move it into the task
            tokio::spawn(async move {
                // This will block until the server is shut down
                server.wait();
                
                // Once server.wait() returns, signal completion
                let _ = shutdown_complete_tx.send(());
            });
            
            // Return immediately, allowing the server to run in the background
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to start HTTP server: {}", e);
            Err(e.into())
        }
    }
}