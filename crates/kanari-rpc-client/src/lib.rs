// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Kanari RPC Client
//!
//! HTTP client for interacting with Kanari RPC server

use anyhow::{Context, Result};
use kanari_rpc_api::*;
use kanari_types::error::KanariUnwrapExt;
use kanari_types::transaction::ObjectRef;
use reqwest::Client;
use std::sync::atomic::{AtomicU64, Ordering};

/// RPC client
pub struct RpcClient {
    client: Client,
    url: String,
    request_id: AtomicU64,
}

impl RpcClient {
    /// Create new RPC client
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            url: url.into(),
            request_id: AtomicU64::new(1),
        }
    }

    /// Get next request ID
    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Send RPC request
    async fn request(&self, method: &str, params: serde_json::Value) -> Result<RpcResponse> {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: self.next_id(),
        };

        let response = self
            .client
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request")?;

        let rpc_response: RpcResponse =
            response.json().await.context("Failed to parse response")?;

        if let Some(error) = rpc_response.error {
            let reason = error
                .data
                .as_ref()
                .and_then(|data| data.get("reason"))
                .and_then(|value| value.as_str());
            if let Some(reason) = reason {
                anyhow::bail!(
                    "RPC error: {} (code: {}, reason: {})",
                    error.message,
                    error.code,
                    reason
                );
            }
            anyhow::bail!("RPC error: {} (code: {})", error.message, error.code);
        }

        Ok(rpc_response)
    }

    /// Get owner information
    pub async fn get_owner(&self, owner: &str) -> Result<OwnerInfo> {
        let response = self
            .request(methods::GET_OWNER, serde_json::json!(owner))
            .await?;

        let result = response.result.context("No result in response")?;
        serde_json::from_value(result).context("Failed to parse owner info")
    }

    /// Get all visible balances for an owner.
    pub async fn get_owner_balances(&self, owner: &str) -> Result<serde_json::Value> {
        let response = self
            .request(
                methods::GET_OWNER_BALANCES,
                serde_json::to_value(GetOwnerBalancesRequest {
                    owner: owner.to_string(),
                })?,
            )
            .await?;

        response.result.context("No result in response")
    }

    pub async fn get_object_by_ref(&self, object_ref: ObjectRef) -> Result<ObjectInfo> {
        let response = self
            .request(
                methods::GET_OBJECT_BY_REF,
                serde_json::to_value(GetObjectByRefRequest { object_ref })?,
            )
            .await?;

        let result = response.result.context("No result in response")?;
        serde_json::from_value(result).context("Failed to parse object info")
    }

    pub async fn get_object(&self, object_id: &str) -> Result<ObjectInfo> {
        let response = self
            .request(
                methods::GET_OBJECT,
                serde_json::to_value(GetObjectRequest {
                    object_id: object_id.to_string(),
                })?,
            )
            .await?;

        let result = response.result.context("No result in response")?;
        serde_json::from_value(result).context("Failed to parse object info")
    }

    pub async fn get_objects(&self, request: GetObjectsRequest) -> Result<OwnedObjectsResponse> {
        let response = self
            .request(methods::GET_OBJECTS, serde_json::to_value(request)?)
            .await?;

        let result = response.result.context("No result in response")?;
        serde_json::from_value(result).context("Failed to parse object query response")
    }

    /// Get block by height
    pub async fn get_block(&self, height: u64) -> Result<BlockInfo> {
        let response = self
            .request(methods::GET_BLOCK, serde_json::json!(height))
            .await?;

        let result = response.result.context("No result in response")?;
        serde_json::from_value(result).context("Failed to parse block info")
    }

    /// Get full block by height (including transactions and vertices)
    pub async fn get_full_block(&self, height: u64) -> Result<FullBlockData> {
        let response = self
            .request(methods::GET_FULL_BLOCK, serde_json::json!(height))
            .await?;

        let result = response.result.context("No result in response")?;
        serde_json::from_value(result).context("Failed to parse full block data")
    }

    /// Get current block height
    pub async fn get_block_height(&self) -> Result<u64> {
        let response = self
            .request(methods::GET_BLOCK_HEIGHT, serde_json::json!(null))
            .await?;

        let result = response.result.context("No result in response")?;
        serde_json::from_value(result).context("Failed to parse block height")
    }

    /// Get transaction details by hash.
    pub async fn get_transaction(&self, hash: &str) -> Result<TransactionDetails> {
        let response = self
            .request(
                methods::GET_TRANSACTION,
                serde_json::json!({ "hash": hash }),
            )
            .await?;

        let result = response.result.context("No result in response")?;
        serde_json::from_value(result).context("Failed to parse transaction details")
    }

    /// Get blockchain statistics
    pub async fn get_stats(&self) -> Result<BlockchainStats> {
        let response = self
            .request(methods::GET_STATS, serde_json::json!(null))
            .await?;

        let result = response.result.context("No result in response")?;
        serde_json::from_value(result).context("Failed to parse stats")
    }

    /// Submit an object transfer request.
    pub async fn submit_object_transfer(
        &self,
        tx: kanari_rpc_api::ObjectTransferData,
    ) -> Result<TransactionStatus> {
        let response = self
            .request(methods::SUBMIT_OBJECT_TRANSFER, serde_json::to_value(tx)?)
            .await?;

        let result = response.result.context("No result in response")?;

        let status = TransactionStatus {
            hash: result["hash"]
                .as_str()
                .require("missing transaction hash")?
                .to_string(),
            status: result["status"]
                .as_str()
                .require("missing transaction status")?
                .to_string(),
            block_height: None,
            gas_used: result["gas_used"].as_u64(),
            success: result["success"].as_bool().unwrap_or(matches!(
                result["status"].as_str().unwrap_or_default(),
                "pending" | "executed" | "committed" | "simulated_pending"
            )),
            previewed: result["previewed"].as_bool().unwrap_or(false),
            submitted: result["submitted"].as_bool().unwrap_or(false),
            committed: result["committed"].as_bool().unwrap_or(false),
        };

        Ok(status)
    }

    pub async fn build_native_transfer(
        &self,
        request: BuildNativeTransferRequest,
    ) -> Result<ObjectTransferData> {
        let response = self
            .request(
                methods::BUILD_NATIVE_TRANSFER,
                serde_json::to_value(request)?,
            )
            .await?;

        let result = response.result.context("No result in response")?;
        serde_json::from_value(result).context("Failed to parse prepared native transfer")
    }

    /// Publish Move module
    pub async fn publish_module(&self, request: PublishModuleRequest) -> Result<TransactionStatus> {
        let response = self
            .request(methods::PUBLISH_MODULE, serde_json::to_value(request)?)
            .await?;

        let result = response.result.context("No result in response")?;

        let status = TransactionStatus {
            hash: result["hash"]
                .as_str()
                .require("missing transaction hash")?
                .to_string(),
            status: result["status"]
                .as_str()
                .require("missing transaction status")?
                .to_string(),
            block_height: None,
            gas_used: result["gas_used"].as_u64(),
            success: result["success"].as_bool().unwrap_or(matches!(
                result["status"].as_str().unwrap_or_default(),
                "pending" | "executed" | "committed" | "simulated_pending"
            )),
            previewed: result["previewed"].as_bool().unwrap_or(false),
            submitted: result["submitted"].as_bool().unwrap_or(false),
            committed: result["committed"].as_bool().unwrap_or(false),
        };

        Ok(status)
    }

    pub async fn build_publish_module(
        &self,
        request: BuildPublishModuleRequest,
    ) -> Result<PublishModuleRequest> {
        let response = self
            .request(
                methods::BUILD_PUBLISH_MODULE,
                serde_json::to_value(request)?,
            )
            .await?;

        let result = response.result.context("No result in response")?;
        serde_json::from_value(result).context("Failed to parse prepared publish request")
    }

    /// Call Move function
    pub async fn call_function(&self, request: CallFunctionRequest) -> Result<TransactionStatus> {
        let response = self
            .request(methods::CALL_FUNCTION, serde_json::to_value(request)?)
            .await?;

        let result = response.result.context("No result in response")?;

        let status = TransactionStatus {
            hash: result["hash"]
                .as_str()
                .require("missing transaction hash")?
                .to_string(),
            status: result["status"]
                .as_str()
                .require("missing transaction status")?
                .to_string(),
            block_height: None,
            gas_used: result["gas_used"].as_u64(),
            success: result["success"].as_bool().unwrap_or(matches!(
                result["status"].as_str().unwrap_or_default(),
                "pending" | "executed" | "committed" | "simulated_pending"
            )),
            previewed: result["previewed"].as_bool().unwrap_or(false),
            submitted: result["submitted"].as_bool().unwrap_or(false),
            committed: result["committed"].as_bool().unwrap_or(false),
        };

        Ok(status)
    }

    pub async fn build_call_function(
        &self,
        request: BuildCallFunctionRequest,
    ) -> Result<CallFunctionRequest> {
        let response = self
            .request(methods::BUILD_CALL_FUNCTION, serde_json::to_value(request)?)
            .await?;

        let result = response.result.context("No result in response")?;
        serde_json::from_value(result).context("Failed to parse prepared call request")
    }

    pub async fn build_token_transfer(
        &self,
        request: BuildTokenTransferRequest,
    ) -> Result<CallFunctionRequest> {
        let response = self
            .request(
                methods::BUILD_TOKEN_TRANSFER,
                serde_json::to_value(request)?,
            )
            .await?;

        let result = response.result.context("No result in response")?;
        serde_json::from_value(result).context("Failed to parse prepared token transfer request")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = RpcClient::new("http://localhost:19001");
        assert_eq!(client.url, "http://localhost:19001");
    }
}
