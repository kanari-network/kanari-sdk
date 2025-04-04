use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, str::FromStr, time::Duration};
use futures::FutureExt;
use jsonrpc_core::{IoHandler, Params, Result as JsonRpcResult, Error as RpcError, ErrorCode};
use jsonrpc_http_server::{ServerBuilder, AccessControlAllowOrigin, DomainsValidation};
use mona_types::address::Address;
use panorama::{blockchain::{BLOCKCHAIN_DATA, get_balance, load_blockchain_with_retry}, chain_id::CHAIN_ID, blockchain::BALANCES};
use panorama::simulation::process_transfer;
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
    priority_boost: Option<u64>, // Optional priority boost for gas fee
}

// Account API structures
#[derive(Deserialize)]
struct AccountParams {
    address: String,
}

// Search transaction parameters
#[derive(Deserialize)]
struct SearchTransactionsParams {
    address: String,
    limit: Option<usize>,
    offset: Option<usize>,
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
                        "id": tx.transaction_id,
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

fn get_account_details(params: Params) -> JsonRpcResult<JsonValue> {
    // Parse account address
    let account_params: AccountParams = params.parse()
        .map_err(|e| RpcError::invalid_params(format!("Invalid parameters: {}", e)))?;
    
    // Load blockchain data if needed
    if let Err(e) = load_blockchain_with_retry() {
        return Err(RpcError {
            code: ErrorCode::InternalError,
            message: format!("Failed to load blockchain: {}", e),
            data: None,
        });
    }
    
    // Parse the address string into Address type
    let address = match Address::from_str(&account_params.address) {
        Ok(addr) => addr,
        Err(_) => return Err(RpcError::invalid_params("Invalid address format")),
    };

    // Get account balance
    let balance = match get_balance(&account_params.address) {
        Ok(balance) => balance,
        Err(_) => return Err(RpcError::invalid_params("Account not found")),
    };
    
    // Find all transactions involving this account
    let transactions = BLOCKCHAIN_DATA.iter()
        .into_iter()
        .flat_map(|block| {
            block.transactions.iter()
                .filter(|tx| tx.sender == address || tx.receiver == address)
                .map(|tx| {
                    // Determine if this is incoming or outgoing for this address
                    let tx_type = if tx.receiver == address { "incoming" } else { "outgoing" };
                    
                    json!({
                        "id": tx.transaction_id,
                        "type": tx_type,
                        "sender": tx.sender.to_string(),
                        "receiver": tx.receiver.to_string(),
                        "amount": tx.amount,
                        "amount_formatted": format_kari_amount(tx.amount),
                        "timestamp": tx.timestamp,
                        "datetime": chrono::DateTime::<chrono::Utc>::from_timestamp(tx.timestamp as i64, 0)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_else(|| "Unknown time".to_string()),
                        "block_index": block.index,
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    
    // Determine if this is a contract address (simplified check, can be expanded)
    let is_contract = false; // Add contract detection logic if available
    let account_type = if is_contract { "contract" } else { "wallet" };
    
    // Create response
    Ok(json!({
        "address": account_params.address,
        "balance": balance,
        "balance_formatted": format_kari_amount(balance),
        "account_type": account_type,
        "is_contract": is_contract,
        "transaction_count": transactions.len(),
        "transactions": transactions
    }))
}

fn search_transactions(params: Params) -> JsonRpcResult<JsonValue> {
    // Parse search parameters
    let search_params: SearchTransactionsParams = params.parse()
        .map_err(|e| RpcError::invalid_params(format!("Invalid parameters: {}", e)))?;
    
    // Load blockchain data if needed
    if let Err(e) = load_blockchain_with_retry() {
        return Err(RpcError {
            code: ErrorCode::InternalError,
            message: format!("Failed to load blockchain: {}", e),
            data: None,
        });
    }
    
    // Parse the address string into Address type
    let search_address = match Address::from_str(&search_params.address) {
        Ok(addr) => addr,
        Err(_) => return Err(RpcError::invalid_params("Invalid address format")),
    };

    // Find all transactions involving this address (either as sender or receiver)
    let mut transactions = BLOCKCHAIN_DATA.iter()
        .into_iter()
        .flat_map(|block| {
            block.transactions.iter()
                .filter(|tx| tx.sender == search_address || tx.receiver == search_address)
                .map(|tx| {
                    // Determine if this is incoming or outgoing for the search address
                    let tx_type = if tx.receiver == search_address { "incoming" } else { "outgoing" };
                    
                    json!({
                        "id": tx.transaction_id,
                        "type": tx_type,
                        "sender": tx.sender.to_string(),
                        "receiver": tx.receiver.to_string(),
                        "amount": tx.amount,
                        "amount_formatted": format_kari_amount(tx.amount),
                        "timestamp": tx.timestamp,
                        "datetime": chrono::DateTime::<chrono::Utc>::from_timestamp(tx.timestamp as i64, 0)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_else(|| "Unknown time".to_string()),
                        "block_index": block.index,
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    
    // Sort transactions by timestamp, most recent first
    transactions.sort_by(|a, b| {
        let a_time = a["timestamp"].as_u64().unwrap_or(0);
        let b_time = b["timestamp"].as_u64().unwrap_or(0);
        b_time.cmp(&a_time)
    });
    
    // Apply pagination if provided
    let total_count = transactions.len();
    let offset = search_params.offset.unwrap_or(0);
    
    // Apply offset and limit
    let transactions = if offset < transactions.len() {
        let paginated = &transactions[offset..];
        if let Some(limit) = search_params.limit {
            if limit < paginated.len() {
                paginated[..limit].to_vec()
            } else {
                paginated.to_vec()
            }
        } else {
            paginated.to_vec()
        }
    } else {
        vec![]
    };
    
    // Create response
    Ok(json!({
        "address": search_params.address,
        "total_transactions": total_count,
        "returned_transactions": transactions.len(),
        "offset": offset,
        "transactions": transactions
    }))
}

// New API method to search for a transaction by its ID
fn get_transaction_by_id(params: Params) -> JsonRpcResult<JsonValue> {
    // Parse transaction ID parameter
    let tx_id: String = params.parse()
        .map_err(|e| RpcError::invalid_params(format!("Invalid transaction ID: {}", e)))?;
    
    // Load blockchain data if needed
    if let Err(e) = load_blockchain_with_retry() {
        return Err(RpcError {
            code: ErrorCode::InternalError,
            message: format!("Failed to load blockchain: {}", e),
            data: None,
        });
    }
    
    // Search for the transaction in all blocks
    for block in BLOCKCHAIN_DATA.iter() {
        for tx in &block.transactions {
            if tx.transaction_id == tx_id {
                // Found the transaction
                let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(tx.timestamp as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown time".to_string());
                
                // Get sender and receiver balances
                let sender_balance = match get_balance(&tx.sender.to_hex_literal()) {
                    Ok(balance) => balance,
                    Err(_) => 0,
                };
                
                let receiver_balance = match get_balance(&tx.receiver.to_hex_literal()) {
                    Ok(balance) => balance,
                    Err(_) => 0,
                };
                
                // Add gas fee information
                return Ok(json!({
                    "transaction": {
                        "id": tx.transaction_id,
                        "sender": tx.sender.to_hex_literal(),
                        "receiver": tx.receiver.to_hex_literal(),
                        "amount": tx.amount,
                        "amount_formatted": format_kari_amount(tx.amount),
                        "gas_fee": tx.gas_fee,
                        "gas_fee_formatted": format_kari_amount(tx.gas_fee),
                        "gas_collector": panorama::utils::GAS_FEE_COLLECTOR,
                        "total_cost": panorama::utils::calculate_total_transaction_cost(tx.amount, tx.gas_fee),
                        "total_cost_formatted": format_kari_amount(panorama::utils::calculate_total_transaction_cost(tx.amount, tx.gas_fee)),
                        "timestamp": tx.timestamp,
                        "datetime": datetime,
                        "signature": tx.signature
                    },
                    "block": {
                        "index": block.index,
                        "hash": block.hash,
                        "timestamp": block.timestamp
                    },
                    "balances": {
                        "sender": {
                            "address": tx.sender.to_hex_literal(),
                            "balance": sender_balance,
                            "formatted": format_kari_amount(sender_balance)
                        },
                        "receiver": {
                            "address": tx.receiver.to_hex_literal(),
                            "balance": receiver_balance,
                            "formatted": format_kari_amount(receiver_balance)
                        }
                    }
                }));
            }
        }
    }
    
    // Transaction not found
    Err(RpcError {
        code: ErrorCode::InvalidParams,
        message: format!("Transaction with ID {} not found", tx_id),
        data: None,
    })
}

// New API method to get transaction status
fn get_transaction_status(params: Params) -> JsonRpcResult<JsonValue> {
    // Parse transaction ID parameter
    let tx_id: String = params.parse()
        .map_err(|e| RpcError::invalid_params(format!("Invalid transaction ID: {}", e)))?;
    
    // Load blockchain data if needed
    if let Err(e) = load_blockchain_with_retry() {
        return Err(RpcError {
            code: ErrorCode::InternalError,
            message: format!("Failed to load blockchain: {}", e),
            data: None,
        });
    }
    
    // First check if the transaction is in a block (confirmed)
    for block in BLOCKCHAIN_DATA.iter() {
        for tx in &block.transactions {
            if tx.transaction_id == tx_id {
                return Ok(json!({
                    "transaction_id": tx_id,
                    "status": "confirmed",
                    "block_index": block.index,
                    "block_hash": block.hash,
                    "timestamp": tx.timestamp,
                    "confirmation_time": block.timestamp
                }));
            }
        }
    }
    
    // If not found in blocks, it might be pending
    // In a more advanced implementation, we would check the pending transaction queue
    
    // Not found at all
    Ok(json!({
        "transaction_id": tx_id,
        "status": "unknown",
        "message": "Transaction not found in blockchain"
    }))
}

// Update get_gas_fee_info to provide dynamic gas fee information
fn get_gas_fee_info(_params: Params) -> JsonRpcResult<JsonValue> {
    const KA_PER_KARI: u64 = 1_000_000_000;
    
    // Get current network stats
    let network_stats = panorama::utils::get_network_stats();
    
    // Calculate sample gas fees for different priority levels
    let base_fee = panorama::utils::calculate_gas_fee(None);
    let medium_fee = panorama::utils::calculate_gas_fee(Some(5));
    let high_fee = panorama::utils::calculate_gas_fee(Some(10));
    
    Ok(json!({
        "gas_fee": {
            "current": base_fee,
            "current_formatted": format_kari_amount(base_fee),
            "medium_priority": medium_fee,
            "medium_priority_formatted": format_kari_amount(medium_fee),
            "high_priority": high_fee,
            "high_priority_formatted": format_kari_amount(high_fee),
            "min_fee": panorama::utils::MIN_GAS_FEE,
            "max_fee": panorama::utils::MAX_GAS_FEE
        },
        "gas_collector": {
            "address": panorama::utils::GAS_FEE_COLLECTOR,
            "balance": get_balance(panorama::utils::GAS_FEE_COLLECTOR).unwrap_or(0)
        },
        "network_stats": {
            "pending_transactions": network_stats.pending_transactions,
            "last_block_time": network_stats.last_block_time,
            "congestion_level": if network_stats.pending_transactions < 10 {
                "low"
            } else if network_stats.pending_transactions < 50 {
                "medium"
            } else {
                "high"
            },
        },
        "gas_enabled": true,
        "system_info": {
            "description": "Dynamic gas fee system",
            "unit": "KA"
        }
    }))
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
    
    io.add_method("get_account_details", |params| {
        futures::future::ready(get_account_details(params)).boxed()
    });
    
    // Add the new search transactions method
    io.add_method("search_transactions", |params| {
        futures::future::ready(search_transactions(params)).boxed()
    });

    // Add the new get transaction by ID method
    io.add_method("get_transaction_by_id", |params| {
        futures::future::ready(get_transaction_by_id(params)).boxed()
    });

    // Add the new transaction status method
    io.add_method("get_transaction_status", |params| {
        futures::future::ready(get_transaction_status(params)).boxed()
    });

    // Add the gas fee info endpoint
    io.add_method("get_gas_fee_info", |params| {
        futures::future::ready(get_gas_fee_info(params)).boxed()
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