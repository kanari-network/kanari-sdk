use crate::respond_with_serialize;

use super::{RpcError, RpcRequest, RpcResponse, RpcServerState};
use kanari_core::{SignedTransaction, Transaction};
use kanari_rpc_api::{
    CallFunctionRequest, PublishModuleRequest, SignedTransactionData, UpgradeModuleRequest,
};
use kanari_types::address::Address;
use move_binary_format::CompiledModule;
use serde_json;
use tracing::{error, info};

// Extract function names from module bytecode (returns None on error)
fn extract_functions_from_bytes(bytes: &[u8]) -> Option<Vec<String>> {
    match CompiledModule::deserialize_with_defaults(bytes) {
        Ok(module) => {
            let mut names = Vec::new();
            for func_def in module.function_defs() {
                let fh = module.function_handle_at(func_def.function);
                let ident = module.identifier_at(fh.name);
                names.push(ident.as_str().to_string());
            }
            Some(names)
        }
        Err(_) => None,
    }
}

// Best-effort lookup of module functions using the engine registry
fn lookup_module_functions(state: &RpcServerState, module_str: &str) -> Option<Vec<String>> {
    // If given as "address::Name" try direct lookup
    if let Some(idx) = module_str.find("::") {
        let addr = &module_str[..idx];
        let name = &module_str[idx + 2..];
        if let Some(bytes) = state.engine.get_module_bytecode(addr, name) {
            return extract_functions_from_bytes(&bytes);
        }
    }

    // Search published modules by name or address
    let mut results: Vec<String> = Vec::new();
    let modules = state.engine.list_all_modules();
    for (addr, name) in modules.iter() {
        if name == module_str || addr == module_str {
            if let Some(bytes) = state.engine.get_module_bytecode(addr, name) {
                if let Some(fns) = extract_functions_from_bytes(&bytes) {
                    for f in fns.into_iter() {
                        results.push(format!("{}::{}::{}", addr, name, f));
                    }
                }
            }
        }
    }
    if results.is_empty() {
        None
    } else {
        Some(results)
    }
}

/// Handle submit transaction request
pub async fn handle_submit_transaction(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
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
        // Compare Address types directly instead of string hex to avoid formatting issues
        let system_addr =
            Address::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS).unwrap_or(Address::ZERO);
        let dev_addr = Address::from_hex_literal(Address::DEV_ADDRESS).unwrap_or(Address::ZERO);
        let allowed = sender == system_addr || sender == dev_addr;
        if !allowed {
            error!("Unauthorized burn attempt from {}", sender.to_hex_literal());
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

/// Handle get transaction by hash request
pub async fn handle_get_transaction(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    // Accept either a plain string param (hash) or an object like { "hash": "..." }
    let hash_param: String = match serde_json::from_value(request.params.clone()) {
        Ok(h) => h,
        Err(_) => {
            // try parsing object
            match serde_json::from_value::<serde_json::Value>(request.params.clone()) {
                Ok(v) => match v.get("hash") {
                    Some(hv) => match hv.as_str() {
                        Some(s) => s.to_string(),
                        None => {
                            return RpcResponse {
                                jsonrpc: "2.0".to_string(),
                                result: None,
                                error: Some(RpcError::invalid_params("Invalid hash parameter")),
                                id: request.id,
                            };
                        }
                    },
                    None => {
                        return RpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: Some(RpcError::invalid_params("Missing 'hash' parameter")),
                            id: request.id,
                        };
                    }
                },
                Err(e) => {
                    return RpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(RpcError::invalid_params(format!("Invalid params: {}", e))),
                        id: request.id,
                    };
                }
            }
        }
    };

    let normalized = hash_param.trim_start_matches("0x").to_lowercase();

    // Search blockchain for the transaction
    let chain = state.engine.blockchain.read().unwrap();
    for block in chain.blocks.iter() {
        for tx in block.transactions.iter() {
            let st = SignedTransaction::new(tx.clone());
            let tx_hash = hex::encode(&st.hash());
            if tx_hash.to_lowercase() == normalized {
                // Build detailed transaction info from the Transaction
                let details = match tx {
                    Transaction::PublishModule {
                        sender,
                        sequence_number,
                        gas_limit,
                        gas_price,
                        ..
                    } => kanari_rpc_api::TransactionDetails {
                        hash: format!("0x{}", tx_hash),
                        status: "committed".to_string(),
                        block_height: Some(block.header.height),
                        gas_used: None,
                        tx_type: "publish_module".to_string(),
                        sender: sender.clone(),
                        sequence_number: *sequence_number,
                        gas_limit: *gas_limit,
                        gas_price: *gas_price,
                        module: None,
                        function: None,
                        module_functions: None,
                    },
                    Transaction::ExecuteFunction {
                        sender,
                        module,
                        function,
                        sequence_number,
                        gas_limit,
                        gas_price,
                        ..
                    } => kanari_rpc_api::TransactionDetails {
                        hash: format!("0x{}", tx_hash),
                        status: "committed".to_string(),
                        block_height: Some(block.header.height),
                        gas_used: None,
                        tx_type: "call".to_string(),
                        sender: sender.clone(),
                        sequence_number: *sequence_number,
                        gas_limit: *gas_limit,
                        gas_price: *gas_price,
                        module: Some(module.clone()),
                        function: Some(function.clone()),
                        module_functions: lookup_module_functions(state, module).map(|v| v),
                    },
                    Transaction::Transfer {
                        from,
                        to: _,
                        amount: _,
                        sequence_number,
                        gas_limit,
                        gas_price,
                        ..
                    } => kanari_rpc_api::TransactionDetails {
                        hash: format!("0x{}", tx_hash),
                        status: "committed".to_string(),
                        block_height: Some(block.header.height),
                        gas_used: None,
                        tx_type: "transfer".to_string(),
                        sender: from.clone(),
                        sequence_number: *sequence_number,
                        gas_limit: *gas_limit,
                        gas_price: *gas_price,
                        module: None,
                        function: None,
                        module_functions: None,
                    },
                    Transaction::Burn {
                        from,
                        amount: _,
                        sequence_number,
                        gas_limit,
                        gas_price,
                        ..
                    } => kanari_rpc_api::TransactionDetails {
                        hash: format!("0x{}", tx_hash),
                        status: "committed".to_string(),
                        block_height: Some(block.header.height),
                        gas_used: None,
                        tx_type: "burn".to_string(),
                        sender: from.clone(),
                        sequence_number: *sequence_number,
                        gas_limit: *gas_limit,
                        gas_price: *gas_price,
                        module: None,
                        function: None,
                        module_functions: None,
                    },
                };
                return respond_with_serialize(request.id, details);
            }
        }
    }

    // Not found in committed blocks — check pending transactions pool
    let pending = state.engine.pending_txs.read().unwrap();
    for tx in pending.iter() {
        let st = SignedTransaction::new(tx.clone());
        let tx_hash = hex::encode(&st.hash());
        if tx_hash.to_lowercase() == normalized {
            // Build details for pending tx
            let details = match tx {
                Transaction::PublishModule {
                    sender,
                    module_bytes,
                    module_name,
                    sequence_number,
                    gas_limit,
                    gas_price,
                    ..
                } => {
                    let module_funcs = extract_functions_from_bytes(module_bytes).map(|fns| {
                        fns.into_iter()
                            .map(|f| format!("{}::{}::{}", "pending", module_name, f))
                            .collect()
                    });
                    kanari_rpc_api::TransactionDetails {
                        hash: format!("0x{}", tx_hash),
                        status: "pending".to_string(),
                        block_height: None,
                        gas_used: None,
                        tx_type: "publish_module".to_string(),
                        sender: sender.clone(),
                        sequence_number: *sequence_number,
                        gas_limit: *gas_limit,
                        gas_price: *gas_price,
                        module: Some(module_name.clone()),
                        function: None,
                        module_functions: module_funcs,
                    }
                }
                Transaction::ExecuteFunction {
                    sender,
                    module,
                    function,
                    sequence_number,
                    gas_limit,
                    gas_price,
                    ..
                } => kanari_rpc_api::TransactionDetails {
                    hash: format!("0x{}", tx_hash),
                    status: "pending".to_string(),
                    block_height: None,
                    gas_used: None,
                    tx_type: "call".to_string(),
                    sender: sender.clone(),
                    sequence_number: *sequence_number,
                    gas_limit: *gas_limit,
                    gas_price: *gas_price,
                    module: Some(module.clone()),
                    function: Some(function.clone()),
                    module_functions: lookup_module_functions(state, module).map(|v| v),
                },
                Transaction::Transfer {
                    from,
                    to: _,
                    amount: _,
                    sequence_number,
                    gas_limit,
                    gas_price,
                    ..
                } => kanari_rpc_api::TransactionDetails {
                    hash: format!("0x{}", tx_hash),
                    status: "pending".to_string(),
                    block_height: None,
                    gas_used: None,
                    tx_type: "transfer".to_string(),
                    sender: from.clone(),
                    sequence_number: *sequence_number,
                    gas_limit: *gas_limit,
                    gas_price: *gas_price,
                    module: None,
                    function: None,
                    module_functions: None,
                },
                Transaction::Burn {
                    from,
                    amount: _,
                    sequence_number,
                    gas_limit,
                    gas_price,
                    ..
                } => kanari_rpc_api::TransactionDetails {
                    hash: format!("0x{}", tx_hash),
                    status: "pending".to_string(),
                    block_height: None,
                    gas_used: None,
                    tx_type: "burn".to_string(),
                    sender: from.clone(),
                    sequence_number: *sequence_number,
                    gas_limit: *gas_limit,
                    gas_price: *gas_price,
                    module: None,
                    function: None,
                    module_functions: None,
                },
            };
            return respond_with_serialize(request.id, details);
        }
    }

    // Not found
    RpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(RpcError::internal_error("Transaction not found")),
        id: request.id,
    }
}

/// Handle request to list all transactions (committed + pending)
pub async fn handle_get_all_transactions(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    // Collect committed transactions from blockchain
    let mut results: Vec<kanari_rpc_api::TransactionDetails> = Vec::new();

    let chain = state.engine.blockchain.read().unwrap();
    for block in chain.blocks.iter() {
        for tx in block.transactions.iter() {
            let st = SignedTransaction::new(tx.clone());
            let tx_hash = hex::encode(&st.hash());
            // build details
            let details = match tx {
                Transaction::PublishModule {
                    sender,
                    module_bytes,
                    module_name,
                    sequence_number,
                    gas_limit,
                    gas_price,
                    ..
                } => kanari_rpc_api::TransactionDetails {
                    hash: format!("0x{}", tx_hash),
                    status: "committed".to_string(),
                    block_height: Some(block.header.height),
                    gas_used: None,
                    tx_type: "publish_module".to_string(),
                    sender: sender.clone(),
                    sequence_number: *sequence_number,
                    gas_limit: *gas_limit,
                    gas_price: *gas_price,
                    module: Some(module_name.clone()),
                    function: None,
                    module_functions: extract_functions_from_bytes(module_bytes).map(|fns| {
                        fns.into_iter()
                            .map(|f| {
                                format!(
                                    "{}::{}::{}",
                                    format!("0x{}", hex::encode(&block.header.state_root)),
                                    module_name,
                                    f
                                )
                            })
                            .collect()
                    }),
                },
                Transaction::ExecuteFunction {
                    sender,
                    module,
                    function,
                    sequence_number,
                    gas_limit,
                    gas_price,
                    ..
                } => kanari_rpc_api::TransactionDetails {
                    hash: format!("0x{}", tx_hash),
                    status: "committed".to_string(),
                    block_height: Some(block.header.height),
                    gas_used: None,
                    tx_type: "call".to_string(),
                    sender: sender.clone(),
                    sequence_number: *sequence_number,
                    gas_limit: *gas_limit,
                    gas_price: *gas_price,
                    module: Some(module.clone()),
                    function: Some(function.clone()),
                    module_functions: lookup_module_functions(state, module).map(|v| v),
                },
                Transaction::Transfer {
                    from,
                    to: _,
                    amount: _,
                    sequence_number,
                    gas_limit,
                    gas_price,
                    ..
                } => kanari_rpc_api::TransactionDetails {
                    hash: format!("0x{}", tx_hash),
                    status: "committed".to_string(),
                    block_height: Some(block.header.height),
                    gas_used: None,
                    tx_type: "transfer".to_string(),
                    sender: from.clone(),
                    sequence_number: *sequence_number,
                    gas_limit: *gas_limit,
                    gas_price: *gas_price,
                    module: None,
                    function: None,
                    module_functions: None,
                },
                Transaction::Burn {
                    from,
                    amount: _,
                    sequence_number,
                    gas_limit,
                    gas_price,
                    ..
                } => kanari_rpc_api::TransactionDetails {
                    hash: format!("0x{}", tx_hash),
                    status: "committed".to_string(),
                    block_height: Some(block.header.height),
                    gas_used: None,
                    tx_type: "burn".to_string(),
                    sender: from.clone(),
                    sequence_number: *sequence_number,
                    gas_limit: *gas_limit,
                    gas_price: *gas_price,
                    module: None,
                    function: None,
                    module_functions: None,
                },
            };
            results.push(details);
        }
    }

    // Append pending transactions
    let pending = state.engine.pending_txs.read().unwrap();
    for tx in pending.iter() {
        let st = SignedTransaction::new(tx.clone());
        let tx_hash = hex::encode(&st.hash());
        let details = match tx {
            Transaction::PublishModule {
                sender,
                module_bytes,
                module_name,
                sequence_number,
                gas_limit,
                gas_price,
                ..
            } => kanari_rpc_api::TransactionDetails {
                hash: format!("0x{}", tx_hash),
                status: "pending".to_string(),
                block_height: None,
                gas_used: None,
                tx_type: "publish_module".to_string(),
                sender: sender.clone(),
                sequence_number: *sequence_number,
                gas_limit: *gas_limit,
                gas_price: *gas_price,
                module: Some(module_name.clone()),
                function: None,
                module_functions: extract_functions_from_bytes(module_bytes).map(|fns| {
                    fns.into_iter()
                        .map(|f| format!("pending::{}::{}", module_name, f))
                        .collect()
                }),
            },
            Transaction::ExecuteFunction {
                sender,
                module,
                function,
                sequence_number,
                gas_limit,
                gas_price,
                ..
            } => kanari_rpc_api::TransactionDetails {
                hash: format!("0x{}", tx_hash),
                status: "pending".to_string(),
                block_height: None,
                gas_used: None,
                tx_type: "call".to_string(),
                sender: sender.clone(),
                sequence_number: *sequence_number,
                gas_limit: *gas_limit,
                gas_price: *gas_price,
                module: Some(module.clone()),
                function: Some(function.clone()),
                module_functions: lookup_module_functions(state, module).map(|v| v),
            },
            Transaction::Transfer {
                from,
                to: _,
                amount: _,
                sequence_number,
                gas_limit,
                gas_price,
                ..
            } => kanari_rpc_api::TransactionDetails {
                hash: format!("0x{}", tx_hash),
                status: "pending".to_string(),
                block_height: None,
                gas_used: None,
                tx_type: "transfer".to_string(),
                sender: from.clone(),
                sequence_number: *sequence_number,
                gas_limit: *gas_limit,
                gas_price: *gas_price,
                module: None,
                function: None,
                module_functions: None,
            },
            Transaction::Burn {
                from,
                amount: _,
                sequence_number,
                gas_limit,
                gas_price,
                ..
            } => kanari_rpc_api::TransactionDetails {
                hash: format!("0x{}", tx_hash),
                status: "pending".to_string(),
                block_height: None,
                gas_used: None,
                tx_type: "burn".to_string(),
                sender: from.clone(),
                sequence_number: *sequence_number,
                gas_limit: *gas_limit,
                gas_price: *gas_price,
                module: None,
                function: None,
                module_functions: None,
            },
        };
        results.push(details);
    }

    respond_with_serialize(request.id, results)
}

/// Handle publish module request
pub async fn handle_publish_module(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
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

    // If caller requested immediate execution (or omitted it), execute and return the changeset
    // If caller explicitly sets `execute_immediate: false`, submit the transaction to the
    // pending pool instead (do not execute immediately) and return a pending response.
    if module_data.execute_immediate.unwrap_or(true) {
        // Execute immediately but also submit a copy so it gets committed later.
        let exec_tx = signed_tx.clone();
        let submit_tx = signed_tx.clone();

        match state.engine.execute_transaction_immediate(exec_tx) {
            Ok((tx_hash, changeset)) => {
                let tx_hash_hex = hex::encode(&tx_hash);
                info!("Module publish executed immediately: {}", tx_hash_hex);

                // Try to submit for eventual commitment; log any failure but
                // still return the execution result to the caller.
                match state.engine.submit_transaction(submit_tx) {
                    Ok(sub_hash) => info!(
                        "Also submitted transaction for commitment: {}",
                        hex::encode(&sub_hash)
                    ),
                    Err(e) => error!("Failed to submit executed transaction: {}", e),
                }

                let cs_value = serde_json::to_value(&changeset).unwrap_or(serde_json::json!(null));
                return RpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(serde_json::json!({
                        "hash": tx_hash_hex,
                        "status": "executed",
                        "action": "publish",
                        "changeset": cs_value
                    })),
                    error: None,
                    id: request.id,
                };
            }
            Err(e) => {
                error!("Failed to execute publish immediately: {}", e);
                return RpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(RpcError::internal_error(format!(
                        "Immediate execution failed: {}",
                        e
                    ))),
                    id: request.id,
                };
            }
        }
    }

    // Otherwise, submit to blockchain pending pool (do not execute immediately)
    match state.engine.submit_transaction(signed_tx) {
        Ok(tx_hash) => {
            let tx_hash_hex = hex::encode(&tx_hash);
            info!("Module publish transaction submitted: {}", tx_hash_hex);
            RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::json!({
                    "hash": tx_hash_hex,
                    "status": "pending",
                    "action": "publish"
                })),
                error: None,
                id: request.id,
            }
        }
        Err(e) => {
            error!("Failed to submit publish transaction: {}", e);
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
/// Handle upgrade module (same as publish but with upgrade flag)
pub async fn handle_upgrade_module(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let module_data: UpgradeModuleRequest = match serde_json::from_value(request.params.clone()) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to parse module upgrade data: {}", e);
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::module_error(format!(
                    "Invalid module data: {}",
                    e
                ))),
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

/// Handle call function request
pub async fn handle_call_function(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
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

    // If caller requested immediate execution (or omitted it), execute and return the changeset
    // If caller explicitly sets `execute_immediate: false`, submit the transaction to the
    // pending pool instead (do not execute immediately) and return a pending response.
    if call_data.execute_immediate.unwrap_or(true) {
        // Execute immediately but also submit a copy so it gets included in a block later.
        let exec_tx = signed_tx.clone();
        let submit_tx = signed_tx.clone();

        match state.engine.execute_transaction_immediate(exec_tx) {
            Ok((tx_hash, changeset)) => {
                let tx_hash_hex = hex::encode(&tx_hash);
                info!("Function executed immediately: {}", tx_hash_hex);
                // Serialize the ChangeSet first
                let mut cs_value =
                    serde_json::to_value(&changeset).unwrap_or(serde_json::json!(null));

                // Normalize created object `type` fields: replace Move-VM debug strings
                // like `StructInstantiation((CachedStructIndex...)` with a nicer stored
                // type string when possible (look up persisted object info in state).
                {
                    let state_guard = state.engine.state.read().unwrap();
                    if let Some(map) = cs_value.as_object_mut() {
                        if let Some(created_val) = map.get_mut("created_objects") {
                            if let Some(arr) = created_val.as_array_mut() {
                                for obj in arr.iter_mut() {
                                    if let Some(obj_map) = obj.as_object_mut() {
                                        if let Some(type_val) =
                                            obj_map.get("type").and_then(|v| v.as_str())
                                        {
                                            let is_noisy = type_val.contains("StructInstantiation")
                                                || type_val.contains("CachedStructIndex");
                                            if is_noisy {
                                                if let Some(id) =
                                                    obj_map.get("id").and_then(|v| v.as_str())
                                                {
                                                    // try direct lookup in persisted objects
                                                    if let Some(stored) =
                                                        state_guard.objects.get(id)
                                                    {
                                                        obj_map.insert(
                                                            "type".to_string(),
                                                            serde_json::Value::String(
                                                                stored.type_.clone(),
                                                            ),
                                                        );
                                                    } else {
                                                        // try without 0x prefix
                                                        let id_norm = id.trim_start_matches("0x");
                                                        if let Some(stored2) =
                                                            state_guard.objects.get(id_norm)
                                                        {
                                                            obj_map.insert(
                                                                "type".to_string(),
                                                                serde_json::Value::String(
                                                                    stored2.type_.clone(),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // If Move ChangeSet reported no created objects, try to fetch persisted
                // objects from engine state (StateManager) for the sender address to
                // provide better CLI feedback.
                if let Some(obj_arr) = cs_value.get("created_objects").and_then(|v| v.as_array()) {
                    if obj_arr.is_empty() {
                        // attempt to look up owned objects from state using sender in request params
                        if let Ok(call_req) = serde_json::from_value::<
                            kanari_rpc_api::CallFunctionRequest,
                        >(request.params.clone())
                        {
                            if let Ok(addr) =
                                kanari_types::address::Address::from_hex_literal(&call_req.sender)
                            {
                                if let Ok(a) = move_core_types::account_address::AccountAddress::from_hex_literal(&addr.to_string()) {
                                    let state_guard = state.engine.state.read().unwrap();
                                    if let Some(ids) = state_guard.owned_objects.get(&a) {
                                        // Build array of created objects from state.objects
                                        let mut objs = Vec::new();
                                        for uid in ids.iter().rev().take(10) {
                                            if let Some(co) = state_guard.objects.get(uid) {
                                                        let o = serde_json::json!({
                                                            "id": uid.clone(),
                                                            "type": co.type_.clone(),
                                                            "owner": format!("0x{}", hex::encode(co.owner.as_ref())),
                                                        });
                                                objs.push(o);
                                            }
                                        }
                                        if !objs.is_empty() {
                                            // replace created_objects field
                                            if let Some(map) = cs_value.as_object_mut() {
                                                map.insert("created_objects".to_string(), serde_json::Value::Array(objs));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Attempt to submit executed transaction for commitment as well.
                match state.engine.submit_transaction(submit_tx) {
                    Ok(sub_hash) => info!(
                        "Also submitted executed transaction for commitment: {}",
                        hex::encode(&sub_hash)
                    ),
                    Err(e) => error!("Failed to submit executed transaction: {}", e),
                }

                return RpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(serde_json::json!({
                        "hash": tx_hash_hex,
                        "status": if changeset.success { "executed" } else { "failed" },
                        "action": "call",
                        "changeset": cs_value
                    })),
                    error: None,
                    id: request.id,
                };
            }
            Err(e) => {
                error!("Failed to execute function immediately: {}", e);
                return RpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(RpcError::internal_error(format!(
                        "Immediate execution failed: {}",
                        e
                    ))),
                    id: request.id,
                };
            }
        }
    } else {
        // Submit transaction to pending pool (do not execute immediately)
        match state.engine.submit_transaction(signed_tx) {
            Ok(tx_hash) => {
                let tx_hash_hex = hex::encode(&tx_hash);
                info!("Function call transaction submitted: {}", tx_hash_hex);
                return RpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(serde_json::json!({
                        "hash": tx_hash_hex,
                        "status": "pending",
                        "action": "call"
                    })),
                    error: None,
                    id: request.id,
                };
            }
            Err(e) => {
                error!("Failed to submit call transaction: {}", e);
                return RpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(RpcError::transaction_error(format!(
                        "Call submission failed: {}",
                        e
                    ))),
                    id: request.id,
                };
            }
        }
    }
}
