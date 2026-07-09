use crate::{
    first_array_param, internal_error_response, invalid_params_response, parse_labeled_params,
    respond_with_serialize,
};

use super::{RpcError, RpcRequest, RpcResponse, RpcServerState};
use kanari_core::engine::{PendingTransactionMetadata, PendingTransactionRecord};
use kanari_move_runtime_v1::changeset::ChangeSet;
use kanari_rpc_api::{
    CallFunctionRequest, ObjectTransferData, PublishModuleRequest, TransactionDetails,
    ViewFunctionRequest,
};
use kanari_types::address::Address;
use kanari_types::transaction::{NativeCall, ObjectRef, SignedTransaction, Transaction};
use move_binary_format::CompiledModule;
use std::collections::HashSet;
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info};

// Extract function names from module bytecode (returns None on error)
fn extract_functions_from_bytes(bytes: &[u8]) -> Option<Vec<String>> {
    CompiledModule::deserialize_with_defaults(bytes)
        .ok()
        .map(|module| {
            module
                .function_defs()
                .iter()
                .map(|func_def| {
                    let fh = module.function_handle_at(func_def.function);
                    module.identifier_at(fh.name).as_str().to_string()
                })
                .collect()
        })
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
    let modules = state.engine.list_all_modules();
    let results: Vec<String> = modules
        .into_iter()
        .filter(|(addr, name)| name == module_str || addr == module_str)
        .filter_map(|(addr, name)| {
            state
                .engine
                .get_module_bytecode(&addr, &name)
                .and_then(|bytes| extract_functions_from_bytes(&bytes))
                .map(|fns| {
                    fns.into_iter()
                        .map(move |f| format!("{}::{}::{}", addr, name, f))
                })
        })
        .flatten()
        .collect();

    if results.is_empty() {
        None
    } else {
        Some(results)
    }
}

// Normalize address strings for comparison (converts to raw hex address)
fn normalize_addr(s: &str) -> String {
    use std::str::FromStr;
    Address::from_str(s)
        .map(|addr| addr.to_hex())
        .unwrap_or_else(|_| s.trim_start_matches("0x").to_lowercase())
}

fn tx_matches_owner(tx: &Transaction, owner_norm: Option<&str>) -> bool {
    let Some(owner) = owner_norm else {
        return true;
    };

    if let Some(native_call) = tx.native_call() {
        match native_call {
            NativeCall::Transfer { recipient, .. } => {
                return normalize_addr(tx.sender()) == owner
                    || normalize_addr(&recipient) == owner;
            }
            NativeCall::BurnAmount { .. } => {}
        }
    }

    normalize_addr(tx.sender()) == owner
}

fn push_unique_tx_details(
    results: &mut Vec<TransactionDetails>,
    seen_hashes: &mut HashSet<String>,
    limit: usize,
    details: TransactionDetails,
) -> bool {
    if results.len() >= limit {
        return false;
    }

    if !seen_hashes.insert(details.hash.to_lowercase()) {
        return true;
    }

    results.push(details);
    true
}

fn parse_hex_address(id: u64, raw: &str, field: &str) -> Result<Address, Box<RpcResponse>> {
    Address::from_hex_literal(raw).map_err(|e| {
        Box::new(invalid_params_response(
            id,
            format!("Invalid {}: {}", field, e),
        ))
    })
}

fn extract_hash_param(params: &serde_json::Value) -> Option<String> {
    serde_json::from_value::<String>(params.clone())
        .ok()
        .or_else(|| {
            params
                .get("hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
}

fn build_publish_signed_tx(module_data: PublishModuleRequest) -> SignedTransaction {
    let mut signed_tx = SignedTransaction::new(Transaction::PublishModule {
        sender: module_data.sender,
        module_bytes: module_data.module_bytes,
        module_name: module_data.module_name,
        gas_limit: module_data.gas_limit,
        gas_price: module_data.gas_price,
        sequence_number: module_data.sequence_number,
    });
    maybe_attach_signature(&mut signed_tx, module_data.signature);
    signed_tx
}

fn build_call_signed_tx(call_data: CallFunctionRequest) -> SignedTransaction {
    let mut signed_tx = SignedTransaction::new(Transaction::ExecuteFunction {
        sender: call_data.sender.clone(),
        module: format!("{}::{}", call_data.package, call_data.module),
        function: call_data.function,
        type_args: call_data.type_args,
        args: call_data.args,
        object_inputs: call_data.object_inputs.unwrap_or_default(),
        gas_payment: call_data.gas_payment,
        gas_limit: call_data.gas_limit,
        gas_price: call_data.gas_price,
        sequence_number: call_data.sequence_number,
    });
    maybe_attach_signature(&mut signed_tx, call_data.signature);
    signed_tx
}

fn base_transaction_details(
    hash: String,
    status: String,
    block_height: Option<u64>,
    tx_type: &str,
    sender: String,
    sender_address: String,
    sequence_number: u64,
    gas_limit: u64,
    gas_price: u64,
) -> TransactionDetails {
    let (success, previewed, submitted, committed) = derive_transaction_state_flags(&status);
    TransactionDetails {
        hash,
        status,
        block_height,
        gas_used: None,
        success,
        previewed,
        submitted,
        committed,
        tx_type: tx_type.to_string(),
        sender,
        sender_address: Some(sender_address),
        sequence_number,
        gas_limit,
        gas_price,
        object_inputs: None,
        gas_payment: None,
        effects: None,
        module: None,
        function: None,
        module_functions: None,
    }
}

fn derive_transaction_state_flags(status: &str) -> (bool, bool, bool, bool) {
    match status {
        "failed" => (false, true, false, false),
        "simulated_pending" => (true, true, true, false),
        "pending" => (true, false, true, false),
        "committed" => (true, false, true, true),
        "executed" => (true, true, true, false),
        _ => (false, false, false, false),
    }
}

fn pending_status(record: &PendingTransactionRecord) -> &'static str {
    if record.metadata.previewed {
        "simulated_pending"
    } else {
        "pending"
    }
}

fn apply_pending_preview_metadata(
    record: &PendingTransactionRecord,
    details: &mut TransactionDetails,
) {
    if let Some(gas_used) = record.metadata.preview_gas_used {
        details.gas_used = Some(gas_used);
    }
    if let Some(effects) = &record.metadata.preview_effects {
        details.effects = Some(effects.clone());
    }
}

// =========================================================================
// HELPER: Map Transaction to API TransactionDetails
// =========================================================================
fn map_transaction_to_details(
    state: &RpcServerState,
    tx: &Transaction,
    tx_hash_hex: &str,
    status: &str,
    block_height: Option<u64>,
    state_root_hex: Option<String>,
) -> TransactionDetails {
    let hash = format!("0x{}", tx_hash_hex);
    let status_str = status.to_string();
    let sender_address = Address::parse_to_account_address(tx.sender_address())
        .map(|addr| addr.to_hex_literal())
        .unwrap_or_else(|_| tx.sender_address().to_string());

    match tx {
        Transaction::PublishModule {
            sender,
            module_bytes,
            module_name,
            sequence_number,
            gas_limit,
            gas_price,
            ..
        } => {
            let prefix = state_root_hex
                .map(|sr| format!("0x{}", sr))
                .unwrap_or_else(|| status_str.clone());
            let module_funcs = extract_functions_from_bytes(module_bytes).map(|fns| {
                fns.into_iter()
                    .map(|f| format!("{}::{}::{}", prefix, module_name, f))
                    .collect()
            });

            let mut details = base_transaction_details(
                hash,
                status_str,
                block_height,
                "publish_module",
                sender.clone(),
                sender_address.clone(),
                *sequence_number,
                *gas_limit,
                *gas_price,
            );
            details.module = Some(module_name.clone());
            details.module_functions = module_funcs;
            details
        }
        Transaction::ExecuteFunction {
            sender,
            module,
            function,
            sequence_number,
            gas_limit,
            gas_price,
            ..
        } => {
            let mut details = base_transaction_details(
                hash,
                status_str,
                block_height,
                tx.tx_type_label(),
                sender.clone(),
                sender_address.clone(),
                *sequence_number,
                *gas_limit,
                *gas_price,
            );
            details.module = Some(module.clone());
            details.function = Some(function.clone());
            details.module_functions = lookup_module_functions(state, module);
            let object_inputs = tx.object_inputs();
            if !object_inputs.is_empty() {
                details.object_inputs = Some(object_inputs);
            }
            details.gas_payment = tx.gas_payment();
            if let Some(native_call) = tx.native_call() {
                match native_call {
                    NativeCall::Transfer {
                        coin_object_id,
                        recipient,
                        ..
                    } => {
                        details.module = Some(format!("To: {} via {}", recipient, coin_object_id));
                    }
                    NativeCall::BurnAmount { .. } => {}
                }
            }
            details
        }
    }
}

// =========================================================================
// HELPER: Format ChangeSet JSON for API responses
// =========================================================================
fn format_changeset_json(state: &RpcServerState, changeset: &ChangeSet) -> serde_json::Value {
    let mut cs_value = serde_json::to_value(changeset).unwrap_or(serde_json::json!(null));

    let state_guard = state.engine.state_read();

    if let Some(map) = cs_value.as_object_mut()
        && let Some(created_val) = map.get_mut("created_objects")
        && let Some(arr) = created_val.as_array_mut()
    {
        for entry in arr.iter_mut() {
            if let Some(obj) = entry.get_mut(1).and_then(|o| o.as_object_mut()) {
                // 1. Truncate Data payload for cleaner JSON output
                if let Some(data_val) = obj.get_mut("data")
                    && let Some(data_arr) = data_val.as_array_mut()
                {
                    let original_len = data_arr.len();
                    let max_len = 32usize;
                    let bytes: Vec<u8> = data_arr
                        .iter()
                        .take(max_len)
                        .filter_map(|v| v.as_u64().map(|n| n as u8))
                        .collect();

                    *data_val = serde_json::Value::String(format!("0x{}", hex::encode(bytes)));
                    if original_len > max_len {
                        obj.insert("data_truncated".to_string(), serde_json::Value::Bool(true));
                        obj.insert("data_len".to_string(), serde_json::json!(original_len));
                    }
                }

                // 2. Normalize noisy Move-VM Types to clean persisted types
                if let Some(type_val) = obj.get("type").and_then(|v| v.as_str())
                    && (type_val.contains("StructInstantiation")
                        || type_val.contains("CachedStructIndex"))
                    && let Some(id) = obj.get("id").and_then(|v| v.as_str())
                {
                    let id_norm = id.trim_start_matches("0x");
                    // Try direct or normalized ID
                    if let Ok(Some(stored)) = state_guard
                        .get_object(id)
                        .or_else(|_| state_guard.get_object(id_norm))
                    {
                        obj.insert(
                            "type".to_string(),
                            serde_json::Value::String(stored.type_.clone()),
                        );
                    }
                }
            }
        }
    }
    if let Some(map) = cs_value.as_object_mut() {
        map.insert(
            "effects".to_string(),
            serde_json::to_value(changeset.effects(None)).unwrap_or(serde_json::json!(null)),
        );
    }
    cs_value
}

fn validate_object_ref_completeness(
    id: u64,
    field: &str,
    object_ref: &ObjectRef,
) -> Result<(), RpcResponse> {
    if object_ref.version.is_some() ^ object_ref.digest.is_some() {
        return Err(invalid_params_response(
            id,
            format!("{field} must include both version and digest when either is provided"),
        ));
    }
    Ok(())
}

fn validate_object_inputs_and_gas(
    id: u64,
    object_inputs: &[kanari_types::transaction::ObjectInput],
    gas_payment: Option<&kanari_types::transaction::GasPayment>,
) -> Result<(), RpcResponse> {
    for (index, input) in object_inputs.iter().enumerate() {
        validate_object_ref_completeness(
            id,
            &format!("object_inputs[{index}].object_ref"),
            &input.object_ref,
        )?;
    }

    if let Some(gas_payment) = gas_payment {
        for (index, payment) in gas_payment.payment_objects.iter().enumerate() {
            validate_object_ref_completeness(
                id,
                &format!("gas_payment.payment_objects[{index}]"),
                payment,
            )?;
        }
    }

    Ok(())
}

fn maybe_attach_signature(signed_tx: &mut SignedTransaction, signature: Option<Vec<u8>>) {
    if let Some(sig) = signature {
        signed_tx.signature = sig;
    }
}

fn classify_transaction_error_data(message: &str) -> Option<serde_json::Value> {
    let reason = if message.contains("must be Coin<") {
        "invalid_gas_payment_type"
    } else if message.contains("cannot overlap with a mutable object input") {
        "gas_payment_object_overlap"
    } else if message.contains("Gas payment object")
        && message.contains("does not exist")
    {
        "gas_payment_object_not_found"
    } else if message.contains("Gas payment object")
        && message.contains("is not owned by sender")
    {
        "gas_payment_owner_mismatch"
    } else if message.contains("Gas payment owner must match sender") {
        "gas_payment_owner_mismatch"
    } else if message.contains("Gas payment version mismatch") {
        "gas_payment_version_mismatch"
    } else if message.contains("Gas payment digest mismatch") {
        "gas_payment_digest_mismatch"
    } else {
        return None;
    };

    Some(serde_json::json!({ "reason": reason }))
}

fn transaction_error_with_reason(message: impl Into<String>) -> RpcError {
    let message = message.into();
    if let Some(data) = classify_transaction_error_data(&message) {
        RpcError::transaction_error_with_data(message, data)
    } else {
        RpcError::transaction_error(message)
    }
}

fn submit_pending_response(
    state: &RpcServerState,
    request_id: u64,
    signed_tx: SignedTransaction,
    action: &str,
    submit_error: &str,
) -> RpcResponse {
    let tx_for_broadcast = signed_tx.clone();
    match state.engine.submit_transactions_batch(vec![signed_tx]) {
        Ok(tx_hashes) => {
            let tx_hash = hex::encode(&tx_hashes[0]);
            debug!("{} accepted into mempool: {}", action, tx_hash);
            state.broadcast_submitted_transaction(tx_for_broadcast);

            respond_with_serialize(
                request_id,
                serde_json::json!({
                    "hash": tx_hash,
                    "status": "pending",
                    "action": action,
                    "success": true,
                    "previewed": false,
                    "submitted": true,
                    "committed": false
                }),
            )
        }
        Err(e) => RpcResponse {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(transaction_error_with_reason(format!(
                "{}: {}",
                submit_error, e
            ))),
            id: request_id,
        },
    }
}

async fn submit_after_immediate_execution(
    state: &RpcServerState,
    signed_tx: SignedTransaction,
    metadata: PendingTransactionMetadata,
) -> anyhow::Result<()> {
    const MAX_RETRIES: usize = 5;
    const RETRY_DELAY_MS: u64 = 50;

    let mut last_error = None;

    for attempt in 0..MAX_RETRIES {
        match state
            .engine
            .submit_transactions_batch_with_metadata(vec![signed_tx.clone()], metadata.clone())
        {
            Ok(_) => return Ok(()),
            Err(e) => {
                let error_text = e.to_string();
                let retryable = error_text.contains("Sequence number too high");
                last_error = Some(e);

                if !retryable || attempt + 1 == MAX_RETRIES {
                    break;
                }

                sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
            }
        }
    }

    Err(last_error.expect("retry loop should capture a submission error"))
}

async fn execute_or_submit_response(
    state: &RpcServerState,
    request_id: u64,
    signed_tx: SignedTransaction,
    execute_immediate: bool,
    action: &str,
    pending_submit_error: &str,
) -> RpcResponse {
    if signed_tx.signature.is_empty() {
        return invalid_params_response(request_id, "Missing or empty signature");
    }

    if !execute_immediate {
        return submit_pending_response(state, request_id, signed_tx, action, pending_submit_error);
    }

    let exec_tx = signed_tx.clone();
    match state.engine.execute_transaction_immediate(exec_tx) {
        Ok((tx_hash, changeset)) => {
            let tx_hash_hex = hex::encode(&tx_hash);
            let _cs_value = format_changeset_json(state, &changeset);

            if !changeset.success {
                return respond_with_serialize(
                    request_id,
                    kanari_rpc_api::TransactionResult {
                        hash: tx_hash_hex,
                        status: "failed".to_string(),
                        gas_used: changeset.gas_used,
                        success: false,
                        previewed: true,
                        submitted: false,
                        committed: false,
                        effects: Some(changeset.effects(None)),
                        error_message: changeset.error_message.clone(),
                    },
                );
            }

            let preview_effects = changeset.effects(None);
            let tx_for_broadcast = signed_tx.clone();
            if let Err(e) = submit_after_immediate_execution(
                state,
                signed_tx,
                PendingTransactionMetadata {
                    previewed: true,
                    preview_gas_used: Some(changeset.gas_used),
                    preview_effects: Some(preview_effects.clone()),
                },
            )
            .await
            {
                error!("Failed to submit executed transaction: {}", e);
                return RpcResponse {
                    jsonrpc: "2.0".into(),
                    result: None,
                    error: Some(transaction_error_with_reason(format!(
                        "Simulation successful, but mempool submission failed: {}",
                        e
                    ))),
                    id: request_id,
                };
            }
            state.broadcast_submitted_transaction(tx_for_broadcast);

            info!(
                "{} previewed immediately & submitted pending: {}",
                action, tx_hash_hex
            );
            respond_with_serialize(
                request_id,
                kanari_rpc_api::TransactionResult {
                    hash: tx_hash_hex,
                    status: "simulated_pending".to_string(),
                    gas_used: changeset.gas_used,
                    success: true,
                    previewed: true,
                    submitted: true,
                    committed: false,
                    effects: Some(preview_effects),
                    error_message: None,
                },
            )
        }
        Err(e) => RpcResponse {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(transaction_error_with_reason(format!(
                "Immediate execution failed: {}",
                e
            ))),
            id: request_id,
        },
    }
}

// =========================================================================
// HANDLERS
// =========================================================================

/// Handle submit object transfer request
pub async fn handle_submit_object_transfer(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let tx_data: ObjectTransferData = match serde_json::from_value(request.params.clone()) {
        Ok(data) => data,
        Err(e) => {
            return invalid_params_response(request.id, format!("Invalid transaction data: {}", e));
        }
    };

    let _sender = match parse_hex_address(request.id, &tx_data.sender, "sender address") {
        Ok(addr) => addr,
        Err(response) => return *response,
    };

    let recipient = match parse_hex_address(request.id, &tx_data.recipient, "recipient") {
        Ok(addr) => addr,
        Err(response) => return *response,
    };

    let coin_object_ref = tx_data
        .coin_object_ref
        .clone()
        .unwrap_or_else(|| ObjectRef::new(tx_data.coin_object_id.clone(), None, None));
    if let Err(response) =
        validate_object_ref_completeness(request.id, "coin_object_ref", &coin_object_ref)
    {
        return response;
    }
    if let Err(response) = validate_object_inputs_and_gas(request.id, &[], tx_data.gas_payment.as_ref()) {
        return response;
    }

    let mut transaction = Transaction::new_transfer_with_object_ref_and_gas(
        tx_data.sender.clone(),
        coin_object_ref,
        recipient.to_hex_literal(),
        tx_data.amount,
        tx_data.sequence_number,
        tx_data.gas_limit,
        tx_data.gas_price,
    );
    if let Transaction::ExecuteFunction { gas_payment, .. } = &mut transaction
        && tx_data.gas_payment.is_some()
    {
        *gas_payment = tx_data.gas_payment.clone();
    }

    let mut signed_tx = SignedTransaction::new(transaction);
    maybe_attach_signature(&mut signed_tx, tx_data.signature);

    execute_or_submit_response(
        state,
        request.id,
        signed_tx,
        tx_data.execute_immediate.unwrap_or(false),
        "submit",
        "Submission failed",
    )
    .await
}

/// Handle get transaction by hash request
pub async fn handle_get_transaction(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let hash_param = extract_hash_param(&request.params).unwrap_or_default();

    if hash_param.is_empty() {
        return invalid_params_response(request.id, "Invalid or missing 'hash' parameter");
    }

    let normalized = hash_param.trim_start_matches("0x").to_lowercase();
    let tx_hash_bytes = match hex::decode(&normalized) {
        Ok(bytes) => bytes,
        Err(_) => return invalid_params_response(request.id, "Invalid transaction hash hex"),
    };

    let chain = state.engine.blockchain.read().unwrap_or_else(|p| {
        error!("blockchain lock poisoned; recovering");
        p.into_inner()
    });

    if let Some((tx, height, state_root)) = chain.get_transaction_location(&tx_hash_bytes) {
        let details = map_transaction_to_details(
            state,
            &tx.transaction,
            &hex::encode(tx.transaction_hash()),
            "committed",
            Some(height),
            Some(hex::encode(state_root)),
        );
        return respond_with_serialize(request.id, details);
    }
    drop(chain);

    if let Some((tx, height, state_root)) = state
        .engine
        .get_committed_transaction_from_history(&tx_hash_bytes)
    {
        let details = map_transaction_to_details(
            state,
            &tx.transaction,
            &hex::encode(tx.transaction_hash()),
            "committed",
            Some(height),
            Some(hex::encode(state_root)),
        );
        return respond_with_serialize(request.id, details);
    }

    let pending = state.engine.pending_transaction_records_snapshot();

    for tx in pending.iter() {
        let tx_hash = hex::encode(tx.signed_tx.transaction_hash());
        if tx_hash.to_lowercase() == normalized {
            let mut details = map_transaction_to_details(
                state,
                &tx.signed_tx.transaction,
                &tx_hash,
                pending_status(tx),
                None,
                None,
            );
            apply_pending_preview_metadata(tx, &mut details);
            return respond_with_serialize(request.id, details);
        }
    }

    internal_error_response(request.id, "Transaction not found")
}

/// Handle request to list all transactions (committed + pending)
/// Optimized: Fetches latest transactions first and implements pagination (Limit)
pub async fn handle_get_all_transactions(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let limit = request
        .params
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;

    let owner_norm = request
        .params
        .as_str()
        .or_else(|| {
            request
                .params
                .as_object()
                .and_then(|obj| obj.get("owner").and_then(|v| v.as_str()))
        })
        .map(|a| a.trim_start_matches("0x").to_lowercase());

    let mut results: Vec<TransactionDetails> = Vec::new();
    let mut seen_hashes = HashSet::new();
    let pending = state.engine.pending_transaction_records_snapshot();

    for tx in pending.iter().rev() {
        if !tx_matches_owner(&tx.signed_tx.transaction, owner_norm.as_deref()) {
            continue;
        }

        if !push_unique_tx_details(
            &mut results,
            &mut seen_hashes,
            limit,
            {
                let mut details = map_transaction_to_details(
                state,
                &tx.signed_tx.transaction,
                &hex::encode(tx.signed_tx.transaction_hash()),
                pending_status(tx),
                None,
                None,
                );
                apply_pending_preview_metadata(tx, &mut details);
                details
            },
        ) {
            break;
        }
    }

    if results.len() < limit {
        let chain = state.engine.blockchain.read().unwrap_or_else(|p| {
            error!("blockchain lock poisoned while listing transactions; recovering");
            p.into_inner()
        });

        for checkpoint in chain.dag_checkpoints.iter().rev() {
            if results.len() >= limit {
                break;
            }

            for tx in checkpoint.transactions.iter().rev() {
                if !tx_matches_owner(&tx.transaction, owner_norm.as_deref()) {
                    continue;
                }

                if !push_unique_tx_details(
                    &mut results,
                    &mut seen_hashes,
                    limit,
                    map_transaction_to_details(
                        state,
                        &tx.transaction,
                        &hex::encode(tx.transaction_hash()),
                        "committed",
                        Some(checkpoint.sequence),
                        Some(hex::encode(&checkpoint.state_root)),
                    ),
                ) {
                    break;
                }
            }
        }
    }

    if results.len() < limit {
        for (tx, height, state_root) in state
            .engine
            .list_committed_transactions_from_history(limit, |tx| {
                tx_matches_owner(tx, owner_norm.as_deref())
            })
        {
            if !push_unique_tx_details(
                &mut results,
                &mut seen_hashes,
                limit,
                map_transaction_to_details(
                    state,
                    &tx.transaction,
                    &hex::encode(tx.transaction_hash()),
                    "committed",
                    Some(height),
                    Some(hex::encode(state_root)),
                ),
            ) {
                break;
            }
        }
    }

    respond_with_serialize(request.id, results)
}

/// Handle publish module request
pub async fn handle_publish_module(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let module_data: PublishModuleRequest =
        match parse_labeled_params(request.id, &request.params, "module data") {
            Ok(data) => data,
            Err(response) => return *response,
        };

    if let Err(response) = parse_hex_address(request.id, &module_data.sender, "sender address") {
        return *response;
    }

    let execute_immediate = module_data.execute_immediate.unwrap_or(false);
    let signed_tx = build_publish_signed_tx(module_data);

    execute_or_submit_response(
        state,
        request.id,
        signed_tx,
        execute_immediate,
        "publish",
        "Module publication failed",
    )
    .await
}

/// Handle call function request
pub async fn handle_call_function(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let call_data: CallFunctionRequest =
        match parse_labeled_params(request.id, &request.params, "call data") {
            Ok(data) => data,
            Err(response) => return *response,
        };

    if let Err(response) = parse_hex_address(request.id, &call_data.sender, "sender address") {
        return *response;
    }
    if let Err(response) = parse_hex_address(request.id, &call_data.package, "package address") {
        return *response;
    }
    if let Err(response) = validate_object_inputs_and_gas(
        request.id,
        call_data.object_inputs.as_deref().unwrap_or(&[]),
        call_data.gas_payment.as_ref(),
    ) {
        return response;
    }

    let execute_immediate = call_data.execute_immediate.unwrap_or(false);
    let signed_tx = build_call_signed_tx(call_data);

    execute_or_submit_response(
        state,
        request.id,
        signed_tx,
        execute_immediate,
        "call",
        "Call submission failed",
    )
    .await
}

/// Handle view function request (read-only, no transaction submission)
pub async fn handle_view_function(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let first_param = match first_array_param(request.id, &request.params) {
        Ok(param) => param,
        Err(response) => return *response,
    };

    let view_data: ViewFunctionRequest =
        match parse_labeled_params(request.id, first_param, "view function data") {
            Ok(data) => data,
            Err(response) => return *response,
        };

    if let Err(response) = parse_hex_address(request.id, &view_data.package, "package address") {
        return *response;
    }

    info!(
        "Executing view function: {}::{}::{}",
        view_data.package, view_data.module, view_data.function
    );

    match state.engine.execute_view_function(
        &view_data.package,
        &view_data.module,
        &view_data.function,
        &view_data.type_args,
        &view_data.args,
    ) {
        Ok(result) => {
            info!("View function executed successfully");
            respond_with_serialize(
                request.id,
                serde_json::json!({
                    "status": "success",
                    "action": "view",
                    "result": result
                }),
            )
        }
        Err(e) => {
            error!("View function execution failed: {}", e);
            internal_error_response(request.id, format!("View function execution failed: {}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{classify_transaction_error_data, derive_transaction_state_flags, transaction_error_with_reason};

    #[test]
    fn transaction_state_flags_match_pending_status() {
        let (success, previewed, submitted, committed) = derive_transaction_state_flags("pending");
        assert!(success);
        assert!(!previewed);
        assert!(submitted);
        assert!(!committed);
    }

    #[test]
    fn transaction_state_flags_match_committed_status() {
        let (success, previewed, submitted, committed) =
            derive_transaction_state_flags("committed");
        assert!(success);
        assert!(!previewed);
        assert!(submitted);
        assert!(committed);
    }

    #[test]
    fn transaction_state_flags_match_simulated_pending_status() {
        let (success, previewed, submitted, committed) =
            derive_transaction_state_flags("simulated_pending");
        assert!(success);
        assert!(previewed);
        assert!(submitted);
        assert!(!committed);
    }

    #[test]
    fn classifies_invalid_gas_payment_type_error() {
        let data = classify_transaction_error_data(
            "Immediate execution failed: Gas payment object 0xabc must be Coin<0x2::kanari::KANARI>, found 0x2::coin::Coin<0x2::foo::BAR>",
        )
        .expect("classification should exist");
        assert_eq!(data["reason"], "invalid_gas_payment_type");
    }

    #[test]
    fn classifies_gas_payment_overlap_error() {
        let data = classify_transaction_error_data(
            "Submission failed: Gas payment object 0xabc cannot overlap with a mutable object input",
        )
        .expect("classification should exist");
        assert_eq!(data["reason"], "gas_payment_object_overlap");
    }

    #[test]
    fn structured_transaction_error_sets_reason_data() {
        let error = transaction_error_with_reason(
            "Immediate execution failed: Gas payment object 0xabc cannot overlap with a mutable object input",
        );
        assert_eq!(error.code, -32002);
        assert_eq!(
            error.data.as_ref().and_then(|data| data.get("reason")),
            Some(&serde_json::json!("gas_payment_object_overlap"))
        );
    }
}
