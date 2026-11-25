// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Named metadata snapshot for mempool admission.
//!
//! Admission used to carry a ten-element tuple through sorting, replay checks,
//! lane accounting and pending-record construction. Positional indexes made
//! the hot path hard to change safely. This module keeps the same values with
//! field names so each stage states exactly which metadata it consumes.

use super::{PendingTransactionMetadata, PendingTransactionRecord, SignedTransaction};

/// Fully derived admission data for one signed transaction.
///
/// All conflict/access keys are computed once during signature verification
/// and reused by later selection and congestion stages.
#[derive(Debug, Clone)]
pub(crate) struct VerifiedMempoolTx {
    pub(crate) signed_tx: SignedTransaction,
    pub(crate) tx_hash: Vec<u8>,
    pub(crate) normalized_sender: String,
    pub(crate) sender_address: String,
    pub(crate) nonce: u64,
    pub(crate) primary_access_key: String,
    pub(crate) congestion_access_key: String,
    pub(crate) access_keys: Vec<String>,
    pub(crate) conflict_keys: Vec<String>,
    pub(crate) size_bytes: usize,
}

impl VerifiedMempoolTx {
    pub(crate) fn into_pending_record(
        self,
        metadata: PendingTransactionMetadata,
    ) -> PendingTransactionRecord {
        PendingTransactionRecord {
            signed_tx: self.signed_tx,
            metadata,
            tx_hash: self.tx_hash,
            normalized_sender: self.normalized_sender,
            sender_address: self.sender_address,
            nonce: self.nonce,
            primary_access_key: self.primary_access_key,
            congestion_access_key: self.congestion_access_key,
            access_keys: self.access_keys,
            conflict_keys: self.conflict_keys,
            size_bytes: self.size_bytes,
        }
    }
}
