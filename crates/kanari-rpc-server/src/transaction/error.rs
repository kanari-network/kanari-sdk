// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Structured transaction error classification for RPC responses.

use super::*;

pub(crate) fn classify_transaction_error_data(message: &str) -> Option<TransactionErrorData> {
    let data = if message.contains("must be Coin<") {
        TransactionErrorData::new(TransactionErrorReason::InvalidGasPaymentType)
    } else if message.contains("cannot overlap with a mutable object input") {
        TransactionErrorData::new(TransactionErrorReason::GasPaymentObjectOverlap)
    } else if message.contains("Gas payment object") && message.contains("does not exist") {
        TransactionErrorData::new(TransactionErrorReason::GasPaymentObjectNotFound)
    } else if (message.contains("Gas payment object") && message.contains("is not owned by sender"))
        || message.contains("Gas payment owner must match sender")
    {
        TransactionErrorData::new(TransactionErrorReason::GasPaymentOwnerMismatch)
    } else if message.contains("Gas payment version mismatch") {
        TransactionErrorData::new(TransactionErrorReason::GasPaymentVersionMismatch)
    } else if message.contains("Gas payment digest mismatch") {
        TransactionErrorData::new(TransactionErrorReason::GasPaymentDigestMismatch)
    } else if message.contains("requires at least two spendable Coin<") {
        TransactionErrorData::with_native_transfer_policy(
            TransactionErrorReason::InsufficientNativeCoinObjects,
        )
    } else if message.contains("could not reserve a separate gas coin") {
        TransactionErrorData::with_native_transfer_policy(
            TransactionErrorReason::NativeCoinConsolidationBlocked,
        )
    } else if message.contains("No single Coin<") && message.contains("can cover requested amount")
    {
        TransactionErrorData::with_native_transfer_policy(
            TransactionErrorReason::InsufficientTransferCoinBalance,
        )
    } else if message.contains("No spendable native gas coin object found with at least") {
        TransactionErrorData::with_native_transfer_policy(
            TransactionErrorReason::InsufficientGasCoinBalance,
        )
    } else if message.contains("Native transfer needs one Coin<") {
        TransactionErrorData::with_native_transfer_policy(
            TransactionErrorReason::NativeTransferPolicyNotSatisfied,
        )
    } else {
        return None;
    };

    Some(data)
}

pub(crate) fn transaction_error_with_reason(message: impl Into<String>) -> RpcError {
    let message = message.into();
    if let Some(data) = classify_transaction_error_data(&message) {
        RpcError::transaction_error_structured(message, data)
    } else {
        RpcError::transaction_error(message)
    }
}
