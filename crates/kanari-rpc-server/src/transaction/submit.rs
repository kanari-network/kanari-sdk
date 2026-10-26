// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! RPC submission endpoints for package publishing and Move calls.

use super::*;

pub async fn handle_publish_module(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let is_upgrade = request.method == methods::UPGRADE_MODULE;
    let action = if is_upgrade {
        "upgrade_module"
    } else {
        "publish"
    };
    let failure = if is_upgrade {
        "Module upgrade failed"
    } else {
        "Module publication failed"
    };
    let mut module_data: PublishModuleRequest =
        match parse_labeled_params(request.id, &request.params, "module data") {
            Ok(data) => data,
            Err(response) => return *response,
        };
    if let Err(response) = parse_hex_address(request.id, &module_data.sender, "sender address") {
        return *response;
    }
    if let Err(response) =
        validate_object_inputs_and_gas(request.id, &[], module_data.gas_payment.as_ref())
    {
        return *response;
    }
    if let Err(message) = module_data.require_nonce() {
        return invalid_params_response(request.id, &message);
    }
    let immediate = module_data.execute_immediate.unwrap_or(false);
    execute_or_submit_response(
        state,
        request.id,
        build_publish_signed_tx(module_data, is_upgrade),
        immediate,
        action,
        failure,
    )
    .await
}

pub async fn handle_publish_package(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let is_upgrade = request.method == methods::UPGRADE_PACKAGE;
    let action = if is_upgrade {
        "upgrade_package"
    } else {
        "publish_package"
    };
    let empty = if is_upgrade {
        "Package upgrade requires at least one module"
    } else {
        "Package publish requires at least one module"
    };
    let failure = if is_upgrade {
        "Package upgrade failed"
    } else {
        "Package publication failed"
    };
    let mut package_data: PublishPackageRequest =
        match parse_labeled_params(request.id, &request.params, "package data") {
            Ok(data) => data,
            Err(response) => return *response,
        };
    if let Err(response) = parse_hex_address(request.id, &package_data.sender, "sender address") {
        return *response;
    }
    if package_data.modules.is_empty() {
        return invalid_params_response(request.id, empty);
    }
    if let Err(response) =
        validate_object_inputs_and_gas(request.id, &[], package_data.gas_payment.as_ref())
    {
        return *response;
    }
    if let Err(message) = package_data.require_nonce() {
        return invalid_params_response(request.id, &message);
    }
    let immediate = package_data.execute_immediate.unwrap_or(false);
    execute_or_submit_response(
        state,
        request.id,
        build_publish_package_signed_tx(package_data, is_upgrade),
        immediate,
        action,
        failure,
    )
    .await
}

pub async fn handle_call_function(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let mut call_data: CallFunctionRequest =
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
        return *response;
    }
    if let Err(message) = call_data.require_nonce() {
        return invalid_params_response(request.id, &message);
    }
    let immediate = call_data.execute_immediate.unwrap_or(false);
    execute_or_submit_response(
        state,
        request.id,
        build_call_signed_tx(call_data),
        immediate,
        "call",
        "Call submission failed",
    )
    .await
}

fn build_publish_signed_tx(module_data: PublishModuleRequest, upgrade: bool) -> SignedTransaction {
    let nonce = module_data
        .canonical_nonce()
        .expect("publish request canonical nonce must be validated before signing");
    let transaction = if upgrade {
        Transaction::UpgradeModule {
            sender: module_data.sender,
            module_bytes: module_data.module_bytes,
            module_name: module_data.module_name,
            gas_payment: module_data.gas_payment,
            gas_limit: module_data.gas_limit,
            gas_price: module_data.gas_price,
            nonce,
        }
    } else {
        Transaction::PublishModule {
            sender: module_data.sender,
            module_bytes: module_data.module_bytes,
            module_name: module_data.module_name,
            gas_payment: module_data.gas_payment,
            gas_limit: module_data.gas_limit,
            gas_price: module_data.gas_price,
            nonce,
        }
    };
    let mut signed_tx = SignedTransaction::new(transaction);
    maybe_attach_signature(&mut signed_tx, module_data.signature);
    signed_tx
}

fn build_publish_package_signed_tx(
    package_data: PublishPackageRequest,
    upgrade: bool,
) -> SignedTransaction {
    let nonce = package_data
        .canonical_nonce()
        .expect("publish package request canonical nonce must be validated before signing");
    let modules = package_data
        .modules
        .into_iter()
        .map(|module| PublishedModule {
            module_name: module.module_name,
            module_bytes: module.module_bytes,
        })
        .collect();
    let transaction = if upgrade {
        Transaction::UpgradePackage {
            sender: package_data.sender,
            modules,
            gas_payment: package_data.gas_payment,
            gas_limit: package_data.gas_limit,
            gas_price: package_data.gas_price,
            nonce,
        }
    } else {
        Transaction::PublishPackage {
            sender: package_data.sender,
            modules,
            gas_payment: package_data.gas_payment,
            gas_limit: package_data.gas_limit,
            gas_price: package_data.gas_price,
            nonce,
        }
    };
    let mut signed_tx = SignedTransaction::new(transaction);
    maybe_attach_signature(&mut signed_tx, package_data.signature);
    signed_tx
}

pub(crate) fn build_call_signed_tx(call_data: CallFunctionRequest) -> SignedTransaction {
    let nonce = call_data
        .canonical_nonce()
        .expect("call request canonical nonce must be validated before signing");
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
        nonce,
    });
    maybe_attach_signature(&mut signed_tx, call_data.signature);
    signed_tx
}

fn maybe_attach_signature(signed_tx: &mut SignedTransaction, signature: Option<Vec<u8>>) {
    if let Some(sig) = signature {
        signed_tx.signature = sig;
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
    state
        .engine
        .submit_transactions_batch_with_metadata(vec![signed_tx], metadata)
        .map(|_| ())
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
                        gas_used: Some(changeset.gas_used),
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
                    gas_used: Some(changeset.gas_used),
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

/// Handle submit object transfer request
pub async fn handle_submit_object_transfer(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let mut tx_data: ObjectTransferData = match serde_json::from_value(request.params.clone()) {
        Ok(data) => data,
        Err(e) => {
            return invalid_params_response(request.id, format!("Invalid transaction data: {}", e));
        }
    };

    let _sender = match parse_hex_address(request.id, &tx_data.sender, "sender address") {
        Ok(addr) => addr,
        Err(response) => return *response,
    };

    if tx_data
        .signature
        .as_ref()
        .map(|signature| signature.is_empty())
        .unwrap_or(true)
    {
        return invalid_params_response(request.id, "Missing or empty signature");
    }

    let recipient = match parse_hex_address(request.id, &tx_data.recipient, "recipient") {
        Ok(addr) => addr,
        Err(response) => return *response,
    };

    let coin_object_ref = tx_data.coin_object_ref.clone().ok_or_else(|| {
        invalid_params_response(
            request.id,
            "coin_object_ref is required and must include (object_id, version, digest)",
        )
    });
    let coin_object_ref = match coin_object_ref {
        Ok(object_ref) => object_ref,
        Err(response) => return response,
    };
    if let Err(response) =
        validate_object_ref_completeness(request.id, "coin_object_ref", &coin_object_ref)
    {
        return *response;
    }
    if !(coin_object_ref.version.is_some() && coin_object_ref.digest.is_some()) {
        return invalid_params_response(
            request.id,
            "coin_object_ref must include (object_id, version, digest)",
        );
    }
    if let Err(response) =
        validate_object_inputs_and_gas(request.id, &[], tx_data.gas_payment.as_ref())
    {
        return *response;
    }
    let canonical_nonce = match tx_data.require_nonce() {
        Ok(nonce) => nonce,
        Err(message) => return invalid_params_response(request.id, &message),
    };

    // Reconstruct the transaction exactly as the client signed it: the build
    // side already hands out canonical id spellings (see `canonical_object_id`
    // in the build handlers), so the payload must never be re-spelled here —
    // a client may legitimately sign any id form it received.
    let mut transaction = Transaction::new_transfer_with_object_ref_and_gas(
        tx_data.sender.clone(),
        coin_object_ref,
        recipient.to_hex_literal(),
        tx_data.amount,
        canonical_nonce,
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
