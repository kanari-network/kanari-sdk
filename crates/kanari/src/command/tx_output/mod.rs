// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use kanari_rpc_api::{RpcError, TransactionResult, TransactionStatus};
use log::error;

pub fn rpc_error_reason(error: &RpcError) -> Option<&str> {
    error
        .data
        .as_ref()
        .and_then(|data| data.get("reason"))
        .and_then(|value| value.as_str())
}

pub fn print_rpc_error(prefix: &str, error: &RpcError) {
    error!("{}RPC error: {} (code {})", prefix, error.message, error.code);
    if let Some(reason) = rpc_error_reason(error) {
        eprintln!("{}RPC reason: {}", prefix, reason);
    }
}

pub fn print_transaction_status(prefix: &str, status: &TransactionStatus) {
    eprintln!("{}Transaction submitted successfully", prefix);
    eprintln!("{}Transaction hash: {}", prefix, status.hash);
    eprintln!("{}Status: {}", prefix, status.status);
    eprintln!(
        "{}Outcome: success={} previewed={} submitted={} committed={}",
        prefix, status.success, status.previewed, status.submitted, status.committed
    );
}

pub fn print_transaction_result(prefix: &str, result: &TransactionResult) {
    eprintln!("{}Transaction: {}", prefix, result.hash);
    eprintln!("{}Status: {}", prefix, result.status);
    eprintln!("{}Gas used: {} Mist", prefix, result.gas_used);

    if let Some(ref error_message) = result.error_message {
        error!("{}Transaction failed: {}", prefix, error_message);
    }
}

pub fn print_json_value(prefix: &str, label: &str, value: &serde_json::Value) {
    match serde_json::to_string_pretty(value) {
        Ok(pretty) => eprintln!("{}{}:\n{}", prefix, label, pretty),
        Err(_) => eprintln!("{}{}: {}", prefix, label, value),
    }
}
