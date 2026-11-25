// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Result, ensure};
use kanari_crypto::keys::CurveType;
use kanari_crypto::verify_signature;
use kanari_crypto::{hash_data_blake3, signatures::sign_message};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tracing::error;

use super::Transaction;

/// Signed transaction wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub signature: Vec<u8>,
    #[serde(skip, default)]
    cached_transaction_hash: OnceLock<Vec<u8>>,
    #[serde(skip, default)]
    cached_verified_signature: OnceLock<(Vec<u8>, Vec<u8>)>,
}

/// Immutable transaction wrapper for internal paths that have already verified
/// the signature against the exact transaction bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedSignedTransaction {
    signed: SignedTransaction,
    tx_hash: Vec<u8>,
}

impl SignedTransaction {
    pub fn new(transaction: Transaction) -> Self {
        Self {
            transaction,
            signature: vec![],
            cached_transaction_hash: OnceLock::new(),
            cached_verified_signature: OnceLock::new(),
        }
    }

    pub fn sign(&mut self, private_key: &str, curve_type: CurveType) -> Result<()> {
        let tx_hash = self.transaction_hash().to_vec();
        let signature = sign_message(private_key, &tx_hash, curve_type)
            .map_err(|e| anyhow::anyhow!("Failed to sign transaction: {}", e))?;
        self.signature = signature;
        // NOTE: do NOT populate `cached_verified_signature` here. The cache is
        // an optimization for *verified* signatures only; populating it from
        // `sign()` would let an attacker bypass sender/public-key verification
        // by calling sign() with an arbitrary key and then into_verified().
        Ok(())
    }

    pub fn verify_signature(&self) -> Result<bool> {
        let tx_hash = self.transaction_hash();
        self.verify_signature_for_hash(tx_hash)
    }

    pub fn transaction_hash(&self) -> &[u8] {
        self.cached_transaction_hash.get_or_init(|| self.transaction.hash()).as_slice()
    }

    pub fn verify_signature_for_hash(&self, tx_hash: &[u8]) -> Result<bool> {
        if self.signature.is_empty() {
            anyhow::bail!("Transaction not signed");
        }

        let current_hash = self.transaction.hash();
        if tx_hash != current_hash.as_slice() {
            return Ok(false);
        }

        self.verify_signature_against_hash(tx_hash)
    }

    /// Verify a signature against a hash that the caller derived from
    /// `bcs::to_bytes(&self.transaction)`. This skips the defensive
    /// re-serialization in `verify_signature_for_hash`; callers must guarantee
    /// the bytes came from this exact transaction.
    pub fn verify_signature_against_hash(&self, tx_hash: &[u8]) -> Result<bool> {
        if self.signature.is_empty() {
            anyhow::bail!("Transaction not signed");
        }

        let signature = &self.signature;
        if self
            .cached_verified_signature
            .get()
            .map(|(cached_hash, cached_signature)| {
                cached_hash.as_slice() == tx_hash && cached_signature == signature
            })
            .unwrap_or(false)
        {
            return Ok(true);
        }

        let sender = self.transaction.sender();

        let verified = verify_signature(sender, tx_hash, signature)
            .map_err(|e| anyhow::anyhow!("Signature verification failed: {}", e))?;
        if !verified {
            return Ok(false);
        }
        let _ = self.cached_verified_signature.set((tx_hash.to_vec(), signature.clone()));
        Ok(true)
    }

    pub fn verified_transaction_hash(&self) -> Result<Vec<u8>> {
        if self.signature.is_empty() {
            anyhow::bail!("Transaction not signed");
        }

        let current_hash = self.transaction.hash();
        let signature = &self.signature;
        if self
            .cached_verified_signature
            .get()
            .map(|(cached_hash, cached_signature)| {
                cached_hash.as_slice() == current_hash.as_slice() && cached_signature == signature
            })
            .unwrap_or(false)
        {
            return Ok(current_hash);
        }

        let sender = self.transaction.sender();
        let verified = verify_signature(sender, &current_hash, signature)
            .map_err(|e| anyhow::anyhow!("Signature verification failed: {}", e))?;
        if !verified {
            anyhow::bail!("Invalid transaction signature");
        }
        let _ = self.cached_verified_signature.set((current_hash.clone(), signature.clone()));
        Ok(current_hash)
    }

    pub fn into_verified(self) -> Result<VerifiedSignedTransaction> {
        let tx_hash = self.verified_transaction_hash()?;
        Ok(VerifiedSignedTransaction { signed: self, tx_hash })
    }

    /// Build a verified wrapper using transaction bytes the caller already
    /// serialized from `self.transaction`. The hash is derived once from those
    /// bytes and cached, so downstream code never re-serializes the transaction.
    pub fn into_verified_from_serialized_transaction(
        self,
        serialized_transaction: &[u8],
    ) -> Result<VerifiedSignedTransaction> {
        let tx_hash = Transaction::hash_from_serialized(serialized_transaction);
        ensure!(self.verify_signature_against_hash(&tx_hash)?, "Invalid transaction signature");
        let _ = self.cached_transaction_hash.set(tx_hash.clone());
        Ok(VerifiedSignedTransaction { signed: self, tx_hash })
    }

    pub fn hash(&self) -> Vec<u8> {
        let serialized = match bcs::to_bytes(self) {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to serialize SignedTransaction for hashing: {}", e);
                Vec::new()
            }
        };
        hash_data_blake3(&serialized)
    }
}

impl VerifiedSignedTransaction {
    pub fn hash(&self) -> &[u8] {
        &self.tx_hash
    }

    pub fn transaction(&self) -> &Transaction {
        &self.signed.transaction
    }

    pub fn into_signed_transaction(self) -> SignedTransaction {
        self.signed
    }
}
