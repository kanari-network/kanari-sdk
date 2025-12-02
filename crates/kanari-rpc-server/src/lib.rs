//! Kanari RPC Server
//!
//! JSON-RPC server for Kanari blockchain using Axum framework

use anyhow::Result;
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::post};
use kanari_move_runtime::BlockchainEngine;
use kanari_rpc_api::*;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};

/// RPC server state
#[derive(Clone)]
pub struct RpcServerState {
    pub engine: Arc<BlockchainEngine>,
}

impl RpcServerState {
    pub fn new(engine: Arc<BlockchainEngine>) -> Self {
        Self { engine }
    }
}

/// Create RPC server router
pub fn create_router(state: RpcServerState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/", post(handle_rpc))
        .route("/rpc", post(handle_rpc))
        .layer(cors)
        .with_state(state)
}

/// Handle RPC request
async fn handle_rpc(
    State(state): State<RpcServerState>,
    Json(request): Json<RpcRequest>,
) -> impl IntoResponse {
    info!("RPC request: method={}, id={}", request.method, request.id);

    let response = match request.method.as_str() {
        // Account & Balance
        methods::GET_ACCOUNT => handle_get_account(&state, &request).await,
        methods::GET_BALANCE => handle_get_balance(&state, &request).await,
        methods::GET_TOKEN_BALANCE => handle_get_token_balance(&state, &request).await,
        methods::GET_ALL_BALANCES => handle_get_all_balances(&state, &request).await,
        
        // Blocks & Transactions
        methods::GET_BLOCK => handle_get_block(&state, &request).await,
        methods::GET_BLOCK_HEIGHT => handle_get_block_height(&state, &request).await,
        methods::GET_STATS => handle_get_stats(&state, &request).await,
        methods::SUBMIT_TRANSACTION => handle_submit_transaction(&state, &request).await,
        
        // Health
        methods::HEALTH => handle_health(&state, &request).await,
        
        // Module operations
        methods::PUBLISH_MODULE => handle_publish_module(&state, &request).await,
        methods::UPGRADE_MODULE => handle_upgrade_module(&state, &request).await,
        methods::GET_MODULE => handle_get_module(&state, &request).await,
        methods::LIST_MODULES => handle_list_modules(&state, &request).await,
        methods::VERIFY_MODULE => handle_verify_module(&state, &request).await,
        
        // Function calls
        methods::CALL_FUNCTION => handle_call_function(&state, &request).await,
        methods::SIMULATE_FUNCTION => handle_simulate_function(&state, &request).await,
        
        // Object operations
        methods::GET_OBJECT => handle_get_object(&state, &request).await,
        methods::GET_OWNED_OBJECTS => handle_get_owned_objects(&state, &request).await,
        methods::GET_OBJECTS_BY_TYPE => handle_get_objects_by_type(&state, &request).await,
        
        _ => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::method_not_found(&request.method)),
            id: request.id,
        },
    };

    (StatusCode::OK, Json(response))
}

/// Handle get account request
async fn handle_get_account(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let address: String = match serde_json::from_value(request.params.clone()) {
        Ok(addr) => addr,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    match state.engine.get_account_info(&address) {
        Some(info) => {
            let account_info = AccountInfo {
                address: info.address,
                balance: info.balance,
                sequence_number: info.sequence_number,
                modules: info.modules,
                token_balances: info.token_balances,
            };
            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::to_value(account_info).unwrap()),
                error: None,
                id: request.id,
            }
        }
        None => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::internal_error("Account not found")),
            id: request.id,
        },
    }
}

/// Handle get balance request
async fn handle_get_balance(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let address: String = match serde_json::from_value(request.params.clone()) {
        Ok(addr) => addr,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    match state.engine.get_account_info(&address) {
        Some(info) => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!(info.balance)),
            error: None,
            id: request.id,
        },
        None => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!(0)),
            error: None,
            id: request.id,
        },
    }
}

/// Handle get block request
async fn handle_get_block(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let height: u64 = match serde_json::from_value(request.params.clone()) {
        Ok(h) => h,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    match state.engine.get_block(height) {
        Some(block) => {
            // Map runtime events to RPC events
            let rpc_events: Vec<RpcEvent> = block
                .events
                .into_iter()
                .map(|e| RpcEvent {
                    key: e.key,
                    sequence_number: e.sequence_number,
                    type_tag: e.type_tag,
                    event_data: e.event_data,
                })
                .collect();

            let block_info = BlockInfo {
                height: block.height,
                timestamp: block.timestamp,
                hash: block.hash.clone(),
                prev_hash: block.prev_hash,
                tx_count: block.tx_count,
                state_root: hex::encode(&block.hash), // Use block hash as state root placeholder
                events: rpc_events,
            };
            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::to_value(block_info).unwrap()),
                error: None,
                id: request.id,
            }
        }
        None => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::internal_error("Block not found")),
            id: request.id,
        },
    }
}

/// Handle get block height request
async fn handle_get_block_height(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let stats = state.engine.get_stats();
    RpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::json!(stats.height)),
        error: None,
        id: request.id,
    }
}

/// Handle get stats request
async fn handle_get_stats(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let stats = state.engine.get_stats();
    let blockchain_stats = BlockchainStats {
        height: stats.height,
        total_blocks: stats.total_blocks as u64,
        total_transactions: stats.total_transactions as u64,
        pending_transactions: stats.pending_transactions,
        total_accounts: stats.total_accounts,
        total_supply: stats.total_supply,
    };
    RpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::to_value(blockchain_stats).unwrap()),
        error: None,
        id: request.id,
    }
}

/// Handle submit transaction request
async fn handle_submit_transaction(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    use kanari_move_runtime::SignedTransaction;
    use kanari_types::address::Address;

    let tx_data: SignedTransactionData = match serde_json::from_value(request.params.clone()) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to parse transaction data: {}", e);
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(format!(
                    "Invalid transaction data: {}",
                    e
                ))),
                id: request.id,
            };
        }
    };

    // Parse sender address
    let sender = match Address::from_hex_literal(&tx_data.sender) {
        Ok(addr) => addr,
        Err(e) => {
            error!("Invalid sender address: {}", e);
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(format!(
                    "Invalid sender address: {}",
                    e
                ))),
                id: request.id,
            };
        }
    };

    // Parse recipient address if present
    let recipient = if let Some(ref recipient_str) = tx_data.recipient {
        match Address::from_hex_literal(recipient_str) {
            Ok(addr) => Some(addr),
            Err(e) => {
                error!("Invalid recipient address: {}", e);
                return RpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(RpcError::invalid_params(format!(
                        "Invalid recipient address: {}",
                        e
                    ))),
                    id: request.id,
                };
            }
        }
    } else {
        None
    };

    // Create Transaction based on type
    use kanari_move_runtime::Transaction;
    let transaction = if let (Some(recipient), Some(amount)) = (recipient, tx_data.amount) {
        // Regular transfer
        Transaction::Transfer {
            from: sender.to_string(),
            to: recipient.to_string(),
            amount,
            gas_limit: tx_data.gas_limit,
            gas_price: tx_data.gas_price,
            sequence_number: tx_data.sequence_number,
        }
    } else if recipient.is_none() && tx_data.amount.is_some() {
        // Burn transaction (no recipient, amount provided)
        // Restrict burns to system/admin addresses only
        let sender_hex = sender.to_hex_literal();
        let allowed = sender_hex == kanari_types::address::Address::KANARI_SYSTEM_ADDRESS
            || sender_hex == kanari_types::address::Address::DEV_ADDRESS;
        if !allowed {
            error!("Unauthorized burn attempt from {}", sender_hex);
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(
                    "Burn transactions are restricted to system administrators",
                )),
                id: request.id,
            };
        }

        Transaction::Burn {
            from: sender.to_string(),
            amount: tx_data.amount.unwrap(),
            gas_limit: tx_data.gas_limit,
            gas_price: tx_data.gas_price,
            sequence_number: tx_data.sequence_number,
        }
    } else {
        error!("Invalid transaction type - only transfers and burns supported currently");
        return RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::invalid_params(
                "Only transfer or burn transactions are supported",
            )),
            id: request.id,
        };
    };

    // Create SignedTransaction
    let mut signed_tx = SignedTransaction::new(transaction);

    // Set signature if present
    if let Some(sig) = tx_data.signature {
        signed_tx.signature = Some(sig);
    }

    // Submit transaction to blockchain
    match state.engine.submit_transaction(signed_tx) {
        Ok(tx_hash) => {
            let tx_hash_hex = hex::encode(&tx_hash);
            info!("Transaction submitted successfully: {}", tx_hash_hex);
            let result = serde_json::json!({
                "hash": tx_hash_hex,
                "status": "pending"
            });
            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(result),
                error: None,
                id: request.id,
            }
        }
        Err(e) => {
            error!("Failed to submit transaction: {}", e);
            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::internal_error(format!(
                    "Transaction submission failed: {}",
                    e
                ))),
                id: request.id,
            }
        }
    }
}

/// Handle publish module request
async fn handle_publish_module(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    use kanari_move_runtime::{SignedTransaction, Transaction};
    use kanari_types::address::Address;

    let module_data: PublishModuleRequest = match serde_json::from_value(request.params.clone()) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to parse module data: {}", e);
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(format!(
                    "Invalid module data: {}",
                    e
                ))),
                id: request.id,
            };
        }
    };

    // Validate sender address
    if let Err(e) = Address::from_hex_literal(&module_data.sender) {
        error!("Invalid sender address: {}", e);
        return RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::invalid_params(format!(
                "Invalid sender address: {}",
                e
            ))),
            id: request.id,
        };
    }

    // Create transaction
    let transaction = Transaction::PublishModule {
        sender: module_data.sender.clone(),
        module_bytes: module_data.module_bytes,
        module_name: module_data.module_name,
        gas_limit: module_data.gas_limit,
        gas_price: module_data.gas_price,
        sequence_number: module_data.sequence_number,
    };

    let mut signed_tx = SignedTransaction::new(transaction);
    if let Some(sig) = module_data.signature {
        signed_tx.signature = Some(sig);
    }

    // Submit to blockchain
    match state.engine.execute_transaction_immediate(signed_tx) {
        Ok((tx_hash, changeset)) => {
            let tx_hash_hex = hex::encode(&tx_hash);
            info!("Module published successfully: {}", tx_hash_hex);
            
            // Extract created objects from state
            let created_objects = {
                let state_guard = state.engine.state.read().unwrap();
                let objects = &state_guard.objects;
                
                // Get recently created objects (simplification: get all objects owned by sender)
                let sender_addr = kanari_types::address::Address::from_hex_literal(&module_data.sender)
                    .unwrap();
                let sender_bytes = sender_addr.to_bytes();
                let mut addr_array = [0u8; 32];
                addr_array.copy_from_slice(&sender_bytes[0..32]);
                let sender_account_addr = move_core_types::account_address::AccountAddress::new(addr_array);
                
                objects.get_objects_by_owner(&sender_account_addr)
                    .into_iter()
                    .map(|obj| {
                        use kanari_rpc_api::ObjectInfo;
                        ObjectInfo {
                            id: hex::encode(obj.id.to_vec()),
                            owner: format!("{:?}", obj.owner),
                            type_: obj.type_.clone(),
                            data: obj.data.clone(),
                            version: obj.version,
                        }
                    })
                    .collect::<Vec<_>>()
            };
            
            use kanari_rpc_api::TransactionResult;
            let result = TransactionResult {
                hash: tx_hash_hex,
                status: if changeset.success { "success".to_string() } else { "failed".to_string() },
                gas_used: changeset.gas_used,
                created_objects,
            };
            
            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::to_value(result).unwrap()),
                error: None,
                id: request.id,
            }
        }
        Err(e) => {
            error!("Failed to publish module: {}", e);
            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::internal_error(format!(
                    "Module publication failed: {}",
                    e
                ))),
                id: request.id,
            }
        }
    }
}

/// Handle call function request
async fn handle_call_function(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    use kanari_move_runtime::{SignedTransaction, Transaction};
    use kanari_types::address::Address;

    let call_data: CallFunctionRequest = match serde_json::from_value(request.params.clone()) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to parse call data: {}", e);
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(format!(
                    "Invalid call data: {}",
                    e
                ))),
                id: request.id,
            };
        }
    };

    // Validate addresses
    if let Err(e) = Address::from_hex_literal(&call_data.sender) {
        error!("Invalid sender address: {}", e);
        return RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::invalid_params(format!(
                "Invalid sender address: {}",
                e
            ))),
            id: request.id,
        };
    }

    if let Err(e) = Address::from_hex_literal(&call_data.package) {
        error!("Invalid package address: {}", e);
        return RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::invalid_params(format!(
                "Invalid package address: {}",
                e
            ))),
            id: request.id,
        };
    }

    // Create transaction
    let transaction = Transaction::ExecuteFunction {
        sender: call_data.sender.clone(),
        module: call_data.module.clone(),
        function: call_data.function,
        type_args: call_data.type_args,
        args: call_data.args,
        gas_limit: call_data.gas_limit,
        gas_price: call_data.gas_price,
        sequence_number: call_data.sequence_number,
    };

    let mut signed_tx = SignedTransaction::new(transaction);
    if let Some(sig) = call_data.signature {
        signed_tx.signature = Some(sig);
    }

    // Submit to blockchain
    match state.engine.submit_transaction(signed_tx) {
        Ok(tx_hash) => {
            let tx_hash_hex = hex::encode(&tx_hash);
            info!("Function called successfully: {}", tx_hash_hex);
            let result = serde_json::json!({
                "hash": tx_hash_hex,
                "status": "pending"
            });
            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(result),
                error: None,
                id: request.id,
            }
        }
        Err(e) => {
            error!("Failed to call function: {}", e);
            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::internal_error(format!(
                    "Function call failed: {}",
                    e
                ))),
                id: request.id,
            }
        }
    }
}

/// Start RPC server
pub async fn start_server(engine: Arc<BlockchainEngine>, addr: &str) -> Result<()> {
    let state = RpcServerState::new(engine);
    let app = create_router(state);

    info!("Starting RPC server on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Handle health check
async fn handle_health(_state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    use kanari_rpc_api::HealthStatus;
    
    let health = HealthStatus {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // TODO: Track actual uptime
        sync_status: "synced".to_string(),
    };
    
    RpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::to_value(health).unwrap()),
        error: None,
        id: request.id,
    }
}

/// Handle upgrade module (same as publish but with upgrade flag)
async fn handle_upgrade_module(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    use kanari_move_runtime::{SignedTransaction, Transaction};
    use kanari_rpc_api::UpgradeModuleRequest;
    use kanari_types::address::Address;

    let module_data: UpgradeModuleRequest = match serde_json::from_value(request.params.clone()) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to parse module upgrade data: {}", e);
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::module_error(format!("Invalid module data: {}", e))),
                id: request.id,
            };
        }
    };

    // Validate sender
    if let Err(e) = Address::from_hex_literal(&module_data.sender) {
        return RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::invalid_params(format!("Invalid sender: {}", e))),
            id: request.id,
        };
    }

    // Create transaction (same as publish - runtime handles upgrade)
    let transaction = Transaction::PublishModule {
        sender: module_data.sender.clone(),
        module_bytes: module_data.module_bytes,
        module_name: module_data.module_name,
        gas_limit: module_data.gas_limit,
        gas_price: module_data.gas_price,
        sequence_number: module_data.sequence_number,
    };

    let mut signed_tx = SignedTransaction::new(transaction);
    if let Some(sig) = module_data.signature {
        signed_tx.signature = Some(sig);
    }

    match state.engine.submit_transaction(signed_tx) {
        Ok(tx_hash) => {
            let tx_hash_hex = hex::encode(&tx_hash);
            info!("Module upgraded successfully: {}", tx_hash_hex);
            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::json!({
                    "hash": tx_hash_hex,
                    "status": "pending",
                    "action": "upgrade"
                })),
                error: None,
                id: request.id,
            }
        }
        Err(e) => {
            error!("Module upgrade failed: {}", e);
            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::module_error(e.to_string())),
                id: request.id,
            }
        }
    }
}

/// Handle get module
async fn handle_get_module(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    use kanari_rpc_api::ModuleInfo;
    
    #[derive(serde::Deserialize)]
    struct GetModuleParams {
        address: String,
        name: String,
    }

    let params: GetModuleParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    // Get module bytecode from Move storage
    match state.engine.get_module_bytecode(&params.address, &params.name) {
        Some(bytecode) => {
            let module_info = ModuleInfo {
                address: params.address,
                name: params.name,
                bytecode_hash: hex::encode(&blake3::hash(&bytecode).as_bytes()[..]),
                size: bytecode.len(),
                dependencies: vec![], // TODO: Extract dependencies from bytecode
            };
            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::to_value(module_info).unwrap()),
                error: None,
                id: request.id,
            }
        }
        None => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::module_error("Module not found")),
            id: request.id,
        },
    }
}

/// Handle list modules
async fn handle_list_modules(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    use kanari_rpc_api::ModuleInfo;
    
    // Get modules from Move storage instead of contracts
    let modules_data = state.engine.list_all_modules();
    let modules: Vec<ModuleInfo> = modules_data
        .iter()
        .map(|(address, name)| {
            let bytecode_opt = state.engine.get_module_bytecode(address, name);
            ModuleInfo {
                address: address.clone(),
                name: name.clone(),
                bytecode_hash: bytecode_opt
                    .as_ref()
                    .map(|b| hex::encode(&blake3::hash(b).as_bytes()[..]))
                    .unwrap_or_else(|| "unknown".to_string()),
                size: bytecode_opt.as_ref().map(|b| b.len()).unwrap_or(0),
                dependencies: vec![],
            }
        })
        .collect();

    RpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::to_value(modules).unwrap()),
        error: None,
        id: request.id,
    }
}

/// Handle verify module
async fn handle_verify_module(_state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    #[derive(serde::Deserialize)]
    struct VerifyParams {
        module_bytes: Vec<u8>,
    }

    let params: VerifyParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    // Try to deserialize module
    use move_binary_format::file_format::CompiledModule;
    match CompiledModule::deserialize_with_defaults(&params.module_bytes) {
        Ok(module) => {
            let module_id = module.self_id();
            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::json!({
                    "valid": true,
                    "address": module_id.address().to_hex_literal(),
                    "name": module_id.name().as_str()
                })),
                error: None,
                id: request.id,
            }
        }
        Err(e) => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "valid": false,
                "error": e.to_string()
            })),
            error: None,
            id: request.id,
        },
    }
}

/// Handle simulate function call
async fn handle_simulate_function(_state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    use kanari_rpc_api::CallFunctionRequest;

    let call_data: CallFunctionRequest = match serde_json::from_value(request.params.clone()) {
        Ok(data) => data,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    // TODO: Implement actual simulation using MoveRuntime
    RpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::json!({
            "success": true,
            "gas_used": 1000,
            "return_values": []
        })),
        error: None,
        id: request.id,
    }
}

/// Handle get object request
async fn handle_get_object(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    use move_core_types::account_address::AccountAddress;
    
    let object_id: String = match serde_json::from_value(request.params.clone()) {
        Ok(id) => id,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    // Parse object ID
    let obj_id = match AccountAddress::from_hex_literal(&object_id) {
        Ok(id) => id,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(format!("Invalid object ID: {}", e))),
                id: request.id,
            };
        }
    };

    let state_manager = state.engine.state.read().unwrap();
    let object = state_manager.objects.get(&obj_id);

    match object {
        Some(obj) => {
            let obj_info = ObjectInfo {
                id: format!("{:#x}", obj.id),
                owner: match &obj.owner {
                    kanari_move_runtime::Owner::AddressOwner(addr) => format!("{:#x}", addr),
                    kanari_move_runtime::Owner::Shared => "shared".to_string(),
                    kanari_move_runtime::Owner::Immutable => "immutable".to_string(),
                },
                type_: obj.type_.clone(),
                data: obj.data.clone(),
                version: obj.version,
            };

            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::to_value(obj_info).unwrap()),
                error: None,
                id: request.id,
            }
        }
        None => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::internal_error("Object not found")),
            id: request.id,
        },
    }
}

/// Handle get owned objects request
async fn handle_get_owned_objects(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    use move_core_types::account_address::AccountAddress;
    
    let req_data: GetOwnedObjectsRequest = match serde_json::from_value(request.params.clone()) {
        Ok(data) => data,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    // Parse owner address
    let owner = match AccountAddress::from_hex_literal(&req_data.owner) {
        Ok(addr) => addr,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(format!("Invalid owner address: {}", e))),
                id: request.id,
            };
        }
    };

    let state_manager = state.engine.state.read().unwrap();
    
    let objects = if let Some(obj_type) = req_data.object_type {
        state_manager.objects.get_owned_objects_by_type(&owner, &obj_type)
    } else {
        state_manager.objects.get_owned_objects(&owner)
    };

    let obj_infos: Vec<ObjectInfo> = objects
        .iter()
        .map(|obj| ObjectInfo {
            id: format!("{:#x}", obj.id),
            owner: match &obj.owner {
                kanari_move_runtime::Owner::AddressOwner(addr) => format!("{:#x}", addr),
                kanari_move_runtime::Owner::Shared => "shared".to_string(),
                kanari_move_runtime::Owner::Immutable => "immutable".to_string(),
            },
            type_: obj.type_.clone(),
            data: obj.data.clone(),
            version: obj.version,
        })
        .collect();

    RpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::to_value(obj_infos).unwrap()),
        error: None,
        id: request.id,
    }
}

/// Handle get objects by type request
async fn handle_get_objects_by_type(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let object_type: String = match serde_json::from_value(request.params.clone()) {
        Ok(t) => t,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    let state_manager = state.engine.state.read().unwrap();
    let objects = state_manager.objects.get_objects_by_type(&object_type);

    let obj_infos: Vec<ObjectInfo> = objects
        .iter()
        .map(|obj| ObjectInfo {
            id: format!("{:#x}", obj.id),
            owner: match &obj.owner {
                kanari_move_runtime::Owner::AddressOwner(addr) => format!("{:#x}", addr),
                kanari_move_runtime::Owner::Shared => "shared".to_string(),
                kanari_move_runtime::Owner::Immutable => "immutable".to_string(),
            },
            type_: obj.type_.clone(),
            data: obj.data.clone(),
            version: obj.version,
        })
        .collect();

    RpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::to_value(obj_infos).unwrap()),
        error: None,
        id: request.id,
    }
}


/// Handle get token balance request
async fn handle_get_token_balance(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    use kanari_rpc_api::GetTokenBalanceRequest;

    let req_data: GetTokenBalanceRequest = match serde_json::from_value(request.params.clone()) {
        Ok(data) => data,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    let balance = state.engine.get_token_balance(&req_data.address, &req_data.token_type);

    RpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::json!({
            "token_type": req_data.token_type,
            "balance": balance
        })),
        error: None,
        id: request.id,
    }
}

/// Handle get all balances request
async fn handle_get_all_balances(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    use kanari_rpc_api::GetAllBalancesRequest;

    let req_data: GetAllBalancesRequest = match serde_json::from_value(request.params.clone()) {
        Ok(data) => data,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    let account_info = state.engine.get_account_info(&req_data.address);
    
    match account_info {
        Some(info) => {
            let mut balances = vec![
                serde_json::json!({
                    "token_type": "KANARI",
                    "balance": info.balance,
                    "decimals": 9,
                    "symbol": "KANARI"
                })
            ];
            
            for (token_type, amount) in info.token_balances.iter() {
                let symbol = token_type.split("::").last().unwrap_or(token_type);
                
                balances.push(serde_json::json!({
                    "token_type": token_type,
                    "balance": amount,
                    "decimals": 9,
                    "symbol": symbol
                }));
            }
            
            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::json!({
                    "address": req_data.address,
                    "balances": balances
                })),
                error: None,
                id: request.id,
            }
        },
        None => {
            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params("Account not found")),
                id: request.id,
            }
        }
    }
}
