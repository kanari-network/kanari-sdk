// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! RPC request types.

use crate::args::deserialize_args;
use crate::fungible::FungibleAssetHolderCursor;
use crate::types::RpcObjectOwnerKindFilter;
use kanari_types::transaction::{GasPayment, ObjectInput, ObjectRef};
use serde::{Deserialize, Serialize};

/// Submit object transfer request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitObjectTransferRequest {
    pub transaction: ObjectTransferData,
}

/// Build native transfer request.
///
/// Native KANARI transfer follows the shared `NativeTransferPolicyContract`:
/// the backend chooses canonical object refs and prefers distinct transfer/gas
/// coins, while allowing one coin to cover both when its balance is sufficient.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildNativeTransferRequest {
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub gas_limit: u64,
    pub gas_price: u64,
    /// Object ids the caller already has in-flight locally and does not want
    /// the backend selector to reuse for this build. This is advisory and is
    /// combined with the node's pending access-key snapshot.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded_object_ids: Vec<String>,
    /// Preferred replay nonce field. If omitted, RPC generates one from OS randomness.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execute_immediate: Option<bool>,
}

/// Build one native coin consolidation step that joins another native coin into a primary coin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildNativeCoinConsolidationRequest {
    pub sender: String,
    pub required_amount: u64,
    pub gas_limit: u64,
    pub gas_price: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execute_immediate: Option<bool>,
}

/// Object transfer submission data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectTransferData {
    pub sender: String,
    pub coin_object_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coin_object_ref: Option<ObjectRef>,
    pub recipient: String,
    pub amount: u64,
    pub gas_limit: u64,
    pub gas_price: u64,
    /// Canonical replay nonce.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_payment: Option<GasPayment>,
    pub signature: Option<Vec<u8>>,
    /// Canonical BCS transaction hash returned by the build endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_hash: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execute_immediate: Option<bool>,
}

fn canonical_request_nonce(nonce: Option<u64>) -> Result<u64, String> {
    if let Some(nonce) = nonce {
        if nonce == 0 {
            return Err("Prepared request nonce must be non-zero".to_string());
        }
        return Ok(nonce);
    }

    Err("Prepared request is missing canonical nonce".to_string())
}

impl ObjectTransferData {
    pub fn canonical_nonce(&self) -> Result<u64, String> {
        canonical_request_nonce(self.nonce)
    }

    pub fn require_nonce(&mut self) -> Result<u64, String> {
        let nonce = self.canonical_nonce()?;
        self.nonce = Some(nonce);
        Ok(nonce)
    }
}

/// Publish module request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishModuleRequest {
    pub sender: String,
    pub module_bytes: Vec<u8>,
    pub module_name: String,
    pub gas_limit: u64,
    pub gas_price: u64,
    /// Canonical replay nonce.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_payment: Option<GasPayment>,
    pub signature: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execute_immediate: Option<bool>,
}

impl PublishModuleRequest {
    pub fn canonical_nonce(&self) -> Result<u64, String> {
        canonical_request_nonce(self.nonce)
    }

    pub fn require_nonce(&mut self) -> Result<u64, String> {
        let nonce = self.canonical_nonce()?;
        self.nonce = Some(nonce);
        Ok(nonce)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishPackageModule {
    pub module_name: String,
    pub module_bytes: Vec<u8>,
}

/// Publish package request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishPackageRequest {
    pub sender: String,
    pub modules: Vec<PublishPackageModule>,
    pub gas_limit: u64,
    pub gas_price: u64,
    /// Canonical replay nonce.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_payment: Option<GasPayment>,
    pub signature: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execute_immediate: Option<bool>,
}

impl PublishPackageRequest {
    pub fn canonical_nonce(&self) -> Result<u64, String> {
        canonical_request_nonce(self.nonce)
    }

    pub fn require_nonce(&mut self) -> Result<u64, String> {
        let nonce = self.canonical_nonce()?;
        self.nonce = Some(nonce);
        Ok(nonce)
    }
}

/// Build publish module request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildPublishModuleRequest {
    pub sender: String,
    pub module_bytes: Vec<u8>,
    pub module_name: String,
    pub gas_limit: u64,
    pub gas_price: u64,
    /// Preferred replay nonce field. If omitted, RPC generates one from OS randomness.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execute_immediate: Option<bool>,
}

/// Build publish package request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildPublishPackageRequest {
    pub sender: String,
    pub modules: Vec<PublishPackageModule>,
    pub gas_limit: u64,
    pub gas_price: u64,
    /// Preferred replay nonce field. If omitted, RPC generates one from OS randomness.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execute_immediate: Option<bool>,
}

/// Call function request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallFunctionRequest {
    pub sender: String,
    pub package: String,
    pub module: String,
    pub function: String,
    pub type_args: Vec<String>,
    pub args: Vec<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_inputs: Option<Vec<ObjectInput>>,
    pub gas_limit: u64,
    pub gas_price: u64,
    /// Canonical replay nonce.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_payment: Option<GasPayment>,
    pub signature: Option<Vec<u8>>,
    pub execute_immediate: Option<bool>,
}

impl CallFunctionRequest {
    pub fn canonical_nonce(&self) -> Result<u64, String> {
        canonical_request_nonce(self.nonce)
    }

    pub fn require_nonce(&mut self) -> Result<u64, String> {
        let nonce = self.canonical_nonce()?;
        self.nonce = Some(nonce);
        Ok(nonce)
    }
}

/// Build call function request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildCallFunctionRequest {
    pub sender: String,
    pub package: String,
    pub module: String,
    pub function: String,
    pub type_args: Vec<String>,
    pub args: Vec<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_inputs: Option<Vec<ObjectInput>>,
    pub gas_limit: u64,
    pub gas_price: u64,
    /// Preferred replay nonce field. If omitted, RPC generates one from OS randomness.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execute_immediate: Option<bool>,
}

/// Claim 100 USD from the web faucet (`kanari_requestUsdFaucet`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestUsdFaucetRequest {
    /// Recipient address (`0x`-prefixed 64 hex characters).
    pub address: String,
}

/// Build token transfer request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildTokenTransferRequest {
    pub sender: String,
    pub recipient: String,
    pub token_type: String,
    pub amount: u64,
    pub gas_limit: u64,
    pub gas_price: u64,
    /// Preferred replay nonce field. If omitted, RPC generates one from OS randomness.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execute_immediate: Option<bool>,
}

/// Build KRC-1155 transfer request (`kanari_buildKrc1155Transfer`).
///
/// Exactly one selection mode is expected:
/// - `object_id` (legacy): transfer from that specific `Balance` object.
/// - `token_type`: the RPC selects up to three owned `Balance` objects of
///   that token type whose amounts cover `amount`, so callers do not need to
///   know how the holding is split across objects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildKrc1155TransferRequest {
    pub sender: String,
    pub recipient: String,
    /// Object id of the KRC-1155 Balance (`0x`-prefixed). Must be owned by `sender`.
    /// Ignored when `token_type` is provided.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub object_id: String,
    /// Token type address (`0x`-prefixed) whose `Balance` objects the RPC
    /// should select to cover `amount`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    /// Amount to transfer (in base units).
    pub amount: u64,
    pub gas_limit: u64,
    pub gas_price: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execute_immediate: Option<bool>,
}

/// Build NFT transfer request (`kanari_buildNftTransfer`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildNftTransferRequest {
    pub sender: String,
    pub recipient: String,
    /// Object id of the NFT (`0x`-prefixed). Must be owned by `sender`.
    pub object_id: String,
    pub gas_limit: u64,
    pub gas_price: u64,
    /// Preferred replay nonce field. If omitted, RPC generates one from OS randomness.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execute_immediate: Option<bool>,
}

/// Get object request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetObjectRequest {
    pub object_id: String,
}

/// Get owned objects request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOwnedObjectsRequest {
    pub owner: String,
    pub object_type: Option<String>,
}

/// Get objects by type request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetObjectsByTypeRequest {
    pub object_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetObjectsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_kind: Option<RpcObjectOwnerKindFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_version: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_version: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetObjectByRefRequest {
    pub object_ref: ObjectRef,
}

/// Get token balance request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTokenBalanceRequest {
    pub owner: String,
    pub token_type: String,
}

/// Get all balances for an owner request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOwnerBalancesRequest {
    pub owner: String,
}

/// Get fungible asset summary request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetFungibleAssetRequest {
    pub token_type: String,
}

/// Get fungible asset holder list request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetFungibleAssetHoldersRequest {
    pub token_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// Resume after this position. `None` starts from the top holder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<FungibleAssetHolderCursor>,
}

/// Get transactions that involve a fungible asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetFungibleAssetTransactionsRequest {
    pub token_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

/// View function request (read-only, no transaction submission)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewFunctionRequest {
    pub package: String,
    pub module: String,
    pub function: String,
    pub type_args: Vec<String>,
    #[serde(deserialize_with = "deserialize_args")]
    pub args: Vec<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_inputs: Option<Vec<ObjectInput>>,
}
