// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Structured RPC error types.

use serde::{Deserialize, Serialize};
use std::fmt;

/// RPC request wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: u64,
}

/// RPC response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
    pub id: u64,
}

/// Structured transaction error reasons that clients can rely on instead of parsing text.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransactionErrorReason {
    InvalidGasPaymentType,
    GasPaymentObjectOverlap,
    GasPaymentObjectNotFound,
    GasPaymentOwnerMismatch,
    GasPaymentVersionMismatch,
    GasPaymentDigestMismatch,
    InsufficientNativeCoinObjects,
    NativeCoinConsolidationBlocked,
    InsufficientTransferCoinBalance,
    InsufficientGasCoinBalance,
    NativeTransferPolicyNotSatisfied,
}

impl TransactionErrorReason {
    pub fn uses_native_transfer_policy(self) -> bool {
        matches!(
            self,
            Self::InsufficientNativeCoinObjects
                | Self::NativeCoinConsolidationBlocked
                | Self::InsufficientTransferCoinBalance
                | Self::InsufficientGasCoinBalance
                | Self::NativeTransferPolicyNotSatisfied
        )
    }
}

impl fmt::Display for TransactionErrorReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::InvalidGasPaymentType => "invalid_gas_payment_type",
            Self::GasPaymentObjectOverlap => "gas_payment_object_overlap",
            Self::GasPaymentObjectNotFound => "gas_payment_object_not_found",
            Self::GasPaymentOwnerMismatch => "gas_payment_owner_mismatch",
            Self::GasPaymentVersionMismatch => "gas_payment_version_mismatch",
            Self::GasPaymentDigestMismatch => "gas_payment_digest_mismatch",
            Self::InsufficientNativeCoinObjects => "insufficient_native_coin_objects",
            Self::NativeCoinConsolidationBlocked => "native_coin_consolidation_blocked",
            Self::InsufficientTransferCoinBalance => "insufficient_transfer_coin_balance",
            Self::InsufficientGasCoinBalance => "insufficient_gas_coin_balance",
            Self::NativeTransferPolicyNotSatisfied => "native_transfer_policy_not_satisfied",
        };
        f.write_str(label)
    }
}

/// Canonical native-transfer policy shared across RPC server, CLI, and faucet tooling.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NativeTransferPolicyContract {
    /// The backend selects canonical object refs and gas payment objects for clients.
    pub api_selects_objects: bool,
    /// Prepared objects must include canonical version and digest metadata.
    pub canonical_object_refs_required: bool,
    /// Gas payment objects must be native `Coin<0x2::kanari::KANARI>` objects.
    pub gas_payment_must_be_native_kanari: bool,
    /// A single native coin with enough balance may cover transfer and gas.
    pub allows_single_coin_for_transfer_and_gas: bool,
    /// Native transfer can use distinct transfer/gas objects when one coin is insufficient.
    pub allows_distinct_transfer_and_gas_objects: bool,
}

impl NativeTransferPolicyContract {
    pub fn kanari_native() -> Self {
        Self {
            api_selects_objects: true,
            canonical_object_refs_required: true,
            gas_payment_must_be_native_kanari: true,
            allows_single_coin_for_transfer_and_gas: true,
            allows_distinct_transfer_and_gas_objects: true,
        }
    }

    pub fn summary(&self) -> &'static str {
        "one Coin<0x2::kanari::KANARI> with enough balance for transfer plus gas, or two distinct coins with enough balance for each"
    }
}

/// Structured transaction error payload carried in `RpcError.data`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransactionErrorData {
    pub reason: TransactionErrorReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_transfer_policy: Option<NativeTransferPolicyContract>,
}

impl TransactionErrorData {
    pub fn new(reason: TransactionErrorReason) -> Self {
        Self {
            reason,
            native_transfer_policy: None,
        }
    }

    pub fn with_native_transfer_policy(reason: TransactionErrorReason) -> Self {
        Self {
            reason,
            native_transfer_policy: Some(NativeTransferPolicyContract::kanari_native()),
        }
    }
}

/// RPC error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl RpcError {
    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: msg.into(),
            data: None,
        }
    }

    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: msg.into(),
            data: None,
        }
    }

    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method.into()),
            data: None,
        }
    }

    pub fn parse_error(msg: impl Into<String>) -> Self {
        Self {
            code: -32700,
            message: msg.into(),
            data: None,
        }
    }

    pub fn invalid_request(msg: impl Into<String>) -> Self {
        Self {
            code: -32600,
            message: msg.into(),
            data: None,
        }
    }

    pub fn module_error(msg: impl Into<String>) -> Self {
        Self {
            code: -32001,
            message: format!("Module error: {}", msg.into()),
            data: None,
        }
    }

    pub fn transaction_error(msg: impl Into<String>) -> Self {
        Self {
            code: -32002,
            message: format!("Transaction error: {}", msg.into()),
            data: None,
        }
    }

    pub fn transaction_error_with_data(msg: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            code: -32002,
            message: format!("Transaction error: {}", msg.into()),
            data: Some(data),
        }
    }

    pub fn transaction_error_structured(
        msg: impl Into<String>,
        data: TransactionErrorData,
    ) -> Self {
        Self::transaction_error_with_data(
            msg,
            serde_json::to_value(data).unwrap_or_else(|_| serde_json::json!({})),
        )
    }

    pub fn transaction_error_details(&self) -> Option<TransactionErrorData> {
        self.data
            .clone()
            .and_then(|data| serde_json::from_value(data).ok())
    }

    pub fn transaction_error_reason(&self) -> Option<TransactionErrorReason> {
        self.transaction_error_details().map(|data| data.reason)
    }
}
