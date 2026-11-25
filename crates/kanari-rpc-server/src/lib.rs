// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Kanari RPC Server
//!
//! JSON-RPC server for Kanari blockchain using Axum framework

use anyhow::Result;
use kanari_core::BlockchainEngine;
use kanari_rpc_api::*;
use kanari_types::transaction::SignedTransaction;

use std::sync::Arc;
use std::time::Instant;

pub mod balance;
pub mod block;
pub mod krc_1155;
pub mod module;
pub mod nft;
mod rpc_runtime;
mod rpc_server;
pub mod transaction;

#[cfg(test)]
pub(crate) use rpc_runtime::run_rpc_dispatch;
#[cfg(test)]
pub(crate) use rpc_server::{
    RPC_RATE_LIMIT_PER_WINDOW, RPC_RATE_LIMIT_WINDOW, RPC_RATE_LIMITER_MAX_TRACKED_IPS,
    RpcRateLimiter,
};
pub use rpc_server::{
    create_router, create_router_with_anti_spam, start_server,
    start_server_with_transaction_broadcaster,
};

type TransactionBroadcaster = Arc<dyn Fn(SignedTransaction) -> Result<()> + Send + Sync>;

/// RPC server state
#[derive(Clone)]
pub struct RpcServerState {
    pub engine: Arc<BlockchainEngine>,
    transaction_broadcaster: Option<TransactionBroadcaster>,
    started_at: Instant,
}

impl RpcServerState {
    pub fn new(engine: Arc<BlockchainEngine>) -> Self {
        Self { engine, transaction_broadcaster: None, started_at: Instant::now() }
    }

    pub fn with_transaction_broadcaster(
        engine: Arc<BlockchainEngine>,
        broadcaster: impl Fn(SignedTransaction) -> Result<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            engine,
            transaction_broadcaster: Some(Arc::new(broadcaster)),
            started_at: Instant::now(),
        }
    }

    pub fn broadcast_submitted_transaction(&self, signed_tx: SignedTransaction) {
        if let Some(broadcaster) = &self.transaction_broadcaster
            && let Err(e) = broadcaster(signed_tx)
        {
            tracing::warn!("Failed to broadcast submitted transaction: {}", e);
        }
    }
}

fn error_response(id: u64, error: RpcError) -> RpcResponse {
    RpcResponse { jsonrpc: "2.0".to_string(), result: None, error: Some(error), id }
}

fn respond_with_value(id: u64, val: serde_json::Value) -> RpcResponse {
    RpcResponse { jsonrpc: "2.0".to_string(), result: Some(val), error: None, id }
}

pub(crate) fn invalid_params_response(id: u64, message: impl Into<String>) -> RpcResponse {
    error_response(id, RpcError::invalid_params(message.into()))
}

pub(crate) fn internal_error_response(id: u64, message: impl Into<String>) -> RpcResponse {
    error_response(id, RpcError::internal_error(message.into()))
}

pub(crate) fn parse_params<T: serde::de::DeserializeOwned>(
    id: u64,
    params: &serde_json::Value,
) -> Result<T, Box<RpcResponse>> {
    serde_json::from_value(params.clone())
        .map_err(|e| Box::new(invalid_params_response(id, e.to_string())))
}

pub(crate) fn parse_labeled_params<T: serde::de::DeserializeOwned>(
    id: u64,
    params: &serde_json::Value,
    label: &str,
) -> Result<T, Box<RpcResponse>> {
    serde_json::from_value(params.clone())
        .map_err(|e| Box::new(invalid_params_response(id, format!("Invalid {}: {}", label, e))))
}

pub(crate) fn first_array_param(
    id: u64,
    params: &serde_json::Value,
) -> Result<&serde_json::Value, Box<RpcResponse>> {
    let arr = params
        .as_array()
        .ok_or_else(|| Box::new(invalid_params_response(id, "Expected array params")))?;
    arr.first().ok_or_else(|| Box::new(invalid_params_response(id, "Empty params array")))
}

fn respond_with_serialize<T: serde::Serialize>(id: u64, v: T) -> RpcResponse {
    match serde_json::to_value(v) {
        Ok(val) => respond_with_value(id, val),
        Err(e) => internal_error_response(id, format!("Serialization failed: {}", e)),
    }
}

#[cfg(test)]
#[path = "../tests/unit/lib_tests.rs"]
mod tests;
