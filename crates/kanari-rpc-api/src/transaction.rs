// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Transaction-related RPC types.

use kanari_types::transaction::{GasPayment, ObjectInput, TransactionEffects};
use serde::{Deserialize, Serialize};

/// Single recognized coin movement within a transaction.
///
/// A transaction can move multiple coins (batch transfers, splits, merges).
/// `transfers` lists every movement the RPC layer could identify from
/// transaction args plus execution effects.
///
/// `transfer_decimals` is the on-chain `CoinMetadata` decimals for
/// `transfer_token_type` (e.g. 6 for THB, 9 for KANARI). Explorer must use
/// it to scale `transfer_amount` (raw units) into display units:
/// `display = raw / 10^decimals`. It is `None` only when metadata is
/// not yet indexed for that token.
///
/// `transfer_amount` is `Some` when parsed from transaction args or when the
/// effect records the post-move coin balance (what the new owner holds, for
/// `Created` / ownership-changing `Transferred` entries). It is `None` for
/// history written before amounts were recorded — clients must render those
/// as amount unknown, never as zero.
///
/// `previous_owner` is the pre-execution owner when the effect records one;
/// it lets clients distinguish a real ownership change from sender
/// change-back noise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_amount: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_decimals: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coin_object_id: Option<String>,
}

/// Transaction status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStatus {
    pub hash: String,
    pub status: String,
    pub block_height: Option<u64>,
    pub gas_used: Option<u64>,
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub previewed: bool,
    #[serde(default)]
    pub submitted: bool,
    #[serde(default)]
    pub committed: bool,
}

/// Detailed transaction information returned by `getTransaction` and `getAllTransactions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionDetails {
    pub hash: String,
    pub status: String,
    pub block_height: Option<u64>,
    pub gas_used: Option<u64>,
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub previewed: bool,
    #[serde(default)]
    pub submitted: bool,
    #[serde(default)]
    pub committed: bool,
    pub tx_type: String,
    pub sender: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u64>,
    pub gas_limit: u64,
    pub gas_price: u64,
    /// Actual fee charged after execution: `gas_used * effective_gas_price`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_fee: Option<u64>,
    /// Every recognized coin movement. `None` when nothing identifiable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfers: Option<Vec<TransferEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_inputs: Option<Vec<ObjectInput>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_payment: Option<GasPayment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effects: Option<TransactionEffects>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_functions: Option<Vec<String>>,
}

/// Transaction result with created objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResult {
    pub hash: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_used: Option<u64>,
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub previewed: bool,
    #[serde(default)]
    pub submitted: bool,
    #[serde(default)]
    pub committed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effects: Option<TransactionEffects>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}
