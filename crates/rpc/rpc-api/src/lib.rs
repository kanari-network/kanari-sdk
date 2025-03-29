use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, time::Duration};
use futures::FutureExt;
use jsonrpc_core::{IoHandler, Params, Result as JsonRpcResult, Error as RpcError, ErrorCode};
use jsonrpc_http_server::{ServerBuilder, AccessControlAllowOrigin, DomainsValidation};
use panorama::{blockchain::{BLOCKCHAIN_DATA, get_balance, load_blockchain_with_retry}, chain_id::CHAIN_ID, blockchain::BALANCES};
use panorama::simulation::process_transfer;
// Remove the missing utils import
use network::NetworkConfig;
use serde_json::{json, Value as JsonValue};
use mona_storage::file_storage::{FileStorage, StorageError2};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::Deserialize;
use tokio::sync::mpsc;
use key::{load_wallet, list_wallet_files};

// File upload parameters
#[derive(Deserialize)]
struct UploadParams {
    filename: String,
    data: String, // base64 encoded file content
}

// Blockchain API structures
#[derive(Deserialize)]
struct TransferParams {
    from: String,
    to: String,
    amount: f64,
    password: String,
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

// Add new RPC methods for file operations
fn upload_file(params: Params) -> JsonRpcResult<JsonValue> {
    let upload_params: UploadParams = params.parse()
        .map_err(|e| RpcError::invalid_params(format!("Invalid parameters: {}", e)))?;

    // Initialize storage
    FileStorage::init_storage()
        .map_err(|_| RpcError::internal_error())?;

    // Create temporary file from base64 data
    let file_data = BASE64.decode(upload_params.data)
        .map_err(|e| RpcError::invalid_params(format!("Invalid base64 data: {}", e)))?;
    
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(&upload_params.filename);
    std::fs::write(&temp_path, &file_data)
        .map_err(|_| RpcError::internal_error())?;

    // Use FileStorage::upload like CLI
    match FileStorage::upload(&temp_path, upload_params.filename) {
        Ok(storage) => {
            // Clean up temp file
            let _ = std::fs::remove_file(temp_path);
            
            let response = json!({
                "id": storage.id.to_string(),
                "filename": storage.metadata.filename,
                "location": storage.path.to_string_lossy(),
                "size": storage.metadata.size,
                "content_type": storage.metadata.content_type
            });
            Ok(response)
        },
        Err(_e) => {
            let _ = std::fs::remove_file(temp_path);
            Err(RpcError::internal_error())
        }
    }
}

fn get_file(params: Params) -> JsonRpcResult<JsonValue> {
    let file_id: String = params.parse()
        .map_err(|e| RpcError::invalid_params(format!("Invalid file ID: {}", e)))?;

    // Initialize storage
    FileStorage::init_storage()
        .map_err(|_| RpcError::internal_error())?;

    match FileStorage::get_by_id(&file_id) {
        Ok(storage) => {
            let file_data = std::fs::read(&storage.path)
                .map_err(|_| RpcError::internal_error())?;
            
            let response = json!({
                "id": storage.id.to_string(),
                "filename": storage.metadata.filename,
                "size": storage.metadata.size,
                "content_type": storage.metadata.content_type,
                "data": BASE64.encode(file_data),
                "location": storage.path.to_string_lossy()
            });
            
            Ok(response)
        },
        Err(StorageError2::NotFound) => Err(RpcError::invalid_params("File not found")),
        Err(_) => Err(RpcError::internal_error())
    }
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
        .into_iter() // Add into_iter to fix error
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

fn get_balance_by_address(params: Params) -> JsonRpcResult<JsonValue> {
    // Parse address from params
    let address: String = params.parse()
        .map_err(|e| RpcError::invalid_params(format!("Invalid address: {}", e)))?;
    
    // Load blockchain data if needed
    if let Err(e) = load_blockchain_with_retry() {
        return Err(RpcError {
            code: ErrorCode::InternalError,
            message: format!("Failed to load blockchain: {}", e),
            data: None,
        });
    }
    
    // Get balance
    match get_balance(&address) {
        Ok(balance) => {
            Ok(json!({
                "address": address,
                "balance_raw": balance,
                "balance_formatted": format_kari_amount(balance),
                "symbol": "KARI",
            }))
        },
        Err(e) => {
            Err(RpcError {
                code: ErrorCode::InternalError,
                message: format!("Failed to get balance: {}", e),
                data: None,
            })
        }
    }
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
    for (address, balance) in balances.iter() {
        // Count transactions for this account using fold instead of flat_map to avoid lifetime issues
        let tx_count = BLOCKCHAIN_DATA.iter()
            .into_iter()
            .fold(0, |acc, block| {
                // Count matching transactions in each block and accumulate
                acc + block.transactions.iter()
                    .filter(|tx| tx.sender == *address || tx.receiver == *address)
                    .count()
            });
            
        accounts.push(json!({
            "address": address,
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
            
            // Process transfer
            match process_transfer(&transfer_params.from, &transfer_params.to, amount_ka, &tx) {
                Ok(transaction) => {
                    // Generate a transaction ID
                    let tx_id = format!("tx_{}_{}_{}", 
                        transaction.sender, 
                        transaction.receiver,
                        transaction.timestamp);
                    
                    Ok(json!({
                        "transaction_id": tx_id,
                        "sender": transaction.sender,
                        "receiver": transaction.receiver,
                        "amount": transaction.amount,
                        "amount_formatted": format_kari_amount(transaction.amount),
                        "timestamp": transaction.timestamp,
                        "status": "pending",
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

// New API method to show information about all blocks
fn get_all_blocks(params: Params) -> JsonRpcResult<JsonValue> {
    // Parse optional limit parameter
    let limit: Option<usize> = match params.parse() {
        Ok(limit) => Some(limit),
        Err(_) => None, // If parsing fails, no limit will be applied
    };
    
    // Load blockchain data if needed
    if let Err(e) = load_blockchain_with_retry() {
        return Err(RpcError {
            code: ErrorCode::InternalError,
            message: format!("Failed to load blockchain: {}", e),
            data: None,
        });
    }
    
    let blocks = BLOCKCHAIN_DATA.iter();
    let block_count = blocks.len();
    
    // Apply the limit if provided
    let blocks_to_show = if let Some(limit) = limit {
        if limit < block_count {
            // Take the most recent blocks if a limit is specified
            blocks.into_iter().skip(block_count - limit).collect::<Vec<_>>()
        } else {
            blocks
        }
    } else {
        blocks
    };
    
    // Convert blocks to JSON format
    let blocks_json: Vec<JsonValue> = blocks_to_show
        .into_iter()
        .map(|block| {
            // Format transactions
            let transactions_json: Vec<JsonValue> = block.transactions
                .iter()
                .map(|tx| {
                    json!({
                        "sender": tx.sender,
                        "receiver": tx.receiver,
                        "amount": tx.amount,
                        "amount_formatted": format_kari_amount(tx.amount),
                        "timestamp": tx.timestamp
                    })
                })
                .collect();
                
            // Format the block
            json!({
                "index": block.index,
                "hash": block.hash,
                "prev_hash": block.prev_hash,
                "timestamp": block.timestamp,
                "datetime": chrono::DateTime::<chrono::Utc>::from_timestamp(block.timestamp as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown time".to_string()),
                "miner": block.address,
                "transactions": transactions_json,
                "transaction_count": block.transactions.len(),
                "tokens_minted": block.tokens
            })
        })
        .collect();
    
    // Create the response JSON
    let response = json!({
        "chain_id": CHAIN_ID.to_string(),
        "block_count": block_count,
        "blocks_returned": blocks_json.len(),
        "blocks": blocks_json
    });
    
    Ok(response)
}

/// Starts the RPC server for file operations
pub async fn start_rpc_server(network_config: NetworkConfig) {
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
    
    io.add_method("get_balance", |params| {
        futures::future::ready(get_balance_by_address(params)).boxed()
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

    io.add_method("get_all_blocks", |params| {
        futures::future::ready(get_all_blocks(params)).boxed()
    });

    // Configure socket address
    let local_addr = SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        network_config.port
    );

    // Create CORS settings that allow all origins during development
    let allowed_origins = vec![
        AccessControlAllowOrigin::Any, // Allow all during development
    ];

    match ServerBuilder::new(io)
        .cors(DomainsValidation::AllowOnly(allowed_origins))
        .start_http(&local_addr)
    {
        Ok(server) => {
            println!("RPC server running on http://127.0.0.1:{}", network_config.port);
            println!("Blockchain API is now available");
            
            // Create a non-blocking task to monitor for shutdown
            tokio::spawn(async move {
                // This will run in a separate task, allowing the server to be shut down properly
                server.wait();
            });
            
            // Sleep to keep the function running without blocking
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                // This allows the function to be cancelled when its task is cancelled
                tokio::task::yield_now().await;
            }
        }
        Err(e) => {
            eprintln!("Failed to start RPC server: {}", e);
            std::process::exit(1);
        }
    }
}