// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Mapping of engine transactions and effects to API transaction details.

use super::*;

pub(crate) fn base_transaction_details(
    hash: String,
    status: String,
    block_height: Option<u64>,
    tx_type: &str,
    sender: String,
    sender_address: String,
    nonce: u64,
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
        nonce: Some(nonce),
        gas_limit,
        gas_price,
        gas_fee: None,
        transfers: None,
        object_inputs: None,
        gas_payment: None,
        effects: None,
        module: None,
        function: None,
        module_functions: None,
    }
}

pub(crate) fn derive_transaction_state_flags(status: &str) -> (bool, bool, bool, bool) {
    match status {
        "failed" => (false, true, false, false),
        "simulated_pending" => (true, true, true, false),
        "pending" => (true, false, true, false),
        "committed" => (true, false, true, true),
        "executed" => (true, true, true, false),
        _ => (false, false, false, false),
    }
}

pub(crate) fn pending_status(record: &PendingTransactionRecord) -> &'static str {
    if record.metadata.previewed {
        "simulated_pending"
    } else {
        "pending"
    }
}

pub(crate) fn apply_pending_preview_metadata(
    state: Option<&RpcServerState>,
    record: &PendingTransactionRecord,
    details: &mut TransactionDetails,
) {
    if let Some(gas_used) = record.metadata.preview_gas_used {
        details.gas_used = Some(gas_used);
        details.gas_fee = Some(gas_used.saturating_mul(effective_gas_price(details.gas_price)));
    }
    if let Some(effects) = &record.metadata.preview_effects {
        details.effects = Some(effects.clone());
        enrich_transfer_from_effect(state, details, effects);
    }
}

pub(crate) fn apply_committed_effect(
    state: Option<&RpcServerState>,
    details: &mut TransactionDetails,
    effect: Option<&kanari_types::transaction::TransactionEffects>,
) {
    let Some(effect) = effect else {
        return;
    };
    let execution_succeeded = effect.status == "success";
    details.status = if execution_succeeded {
        "committed".to_string()
    } else {
        "failed".to_string()
    };
    details.success = execution_succeeded;
    details.previewed = false;
    details.submitted = true;
    // A failed Move execution is still final once its Mysticeti sub-DAG commits.
    details.committed = true;
    details.gas_used = Some(effect.gas_used);
    details.gas_fee = Some(
        effect
            .gas_used
            .saturating_mul(effective_gas_price(details.gas_price)),
    );
    details.effects = Some(effect.clone());
    enrich_transfer_from_effect(state, details, effect);
}

// =========================================================================
// HELPER: Map Transaction to API TransactionDetails
// =========================================================================
pub(crate) fn map_transaction_to_details(
    state: &RpcServerState,
    tx: &Transaction,
    tx_hash_hex: &str,
    status: &str,
    block_height: Option<u64>,
    _state_root_hex: Option<String>,
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
            nonce,
            gas_payment,
            gas_limit,
            gas_price,
            ..
        }
        | Transaction::UpgradeModule {
            sender,
            module_bytes,
            module_name,
            nonce,
            gas_payment,
            gas_limit,
            gas_price,
            ..
        } => {
            let tx_type = tx.tx_type_label();
            let module_prefix = sender_address.clone();
            let module_funcs = extract_functions_from_bytes(module_bytes).map(|fns| {
                fns.into_iter()
                    .map(|f| format!("{}::{}::{}", module_prefix, module_name, f))
                    .collect()
            });

            let mut details = base_transaction_details(
                hash,
                status_str,
                block_height,
                tx_type,
                sender.clone(),
                sender_address.clone(),
                *nonce,
                *gas_limit,
                *gas_price,
            );
            details.module = Some(format!("{}::{}", sender_address, module_name));
            details.module_functions = module_funcs;
            details.gas_payment = gas_payment.clone();
            details
        }
        Transaction::PublishPackage {
            sender,
            modules,
            nonce,
            gas_payment,
            gas_limit,
            gas_price,
            ..
        }
        | Transaction::UpgradePackage {
            sender,
            modules,
            nonce,
            gas_payment,
            gas_limit,
            gas_price,
            ..
        } => {
            let tx_type = tx.tx_type_label();
            let published_modules = modules
                .iter()
                .map(|module| format!("{}::{}", sender_address, module.module_name))
                .collect::<Vec<_>>();
            let module_functions = modules
                .iter()
                .flat_map(|module| {
                    extract_functions_from_bytes(&module.module_bytes)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|function| {
                            format!("{}::{}::{}", sender_address, module.module_name, function)
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();

            let mut details = base_transaction_details(
                hash,
                status_str,
                block_height,
                tx_type,
                sender.clone(),
                sender_address.clone(),
                *nonce,
                *gas_limit,
                *gas_price,
            );
            details.module = Some(published_modules.join(", "));
            details.module_functions = Some(module_functions);
            details.gas_payment = gas_payment.clone();
            details
        }
        Transaction::ExecuteFunction {
            sender,
            module,
            function,
            nonce,
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
                *nonce,
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
            if let Some((recipient, amount, token_type, coin_object_id)) =
                token_transfer_details(state, tx)
            {
                let transfer_decimals = lookup_token_decimals(Some(state), token_type.as_deref());
                push_transfer_entry(
                    &mut details,
                    kanari_rpc_api::TransferEntry {
                        recipient: Some(recipient),
                        transfer_amount: Some(amount),
                        transfer_token_type: token_type,
                        transfer_decimals,
                        previous_owner: None,
                        coin_object_id: Some(coin_object_id),
                    },
                );
            }
            if let Some(native_call) = tx.native_call() {
                match native_call {
                    NativeCall::Transfer {
                        coin_object_id,
                        recipient,
                        amount,
                    } => {
                        push_transfer_entry(
                            &mut details,
                            kanari_rpc_api::TransferEntry {
                                recipient: Some(recipient),
                                transfer_amount: Some(amount),
                                transfer_token_type: Some(GAS_COIN.to_string()),
                                transfer_decimals: Some(9),
                                previous_owner: None,
                                coin_object_id: Some(coin_object_id),
                            },
                        );
                    }
                    NativeCall::Burn { .. } => {}
                }
            }
            details
        }
    }
}

// =========================================================================
// HELPER: Format ChangeSet JSON for API responses
// =========================================================================
pub(crate) fn format_changeset_json(
    state: &RpcServerState,
    changeset: &ChangeSet,
) -> serde_json::Value {
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
