// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Consensus types for kanari-core.
//!
//! DAG block primitives are imported directly from `mysticeti`.
//! This module only defines kanari-specific types such as [`Checkpoint`].

use anyhow::Result;
use kanari_crypto::hash_data_blake3;
use kanari_types::transaction::{SignedTransaction, TransactionEffects};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A canonical checkpoint committed to the blockchain.
///
/// Checkpoints are the kanari-level abstraction that groups DAG vertices
/// (from mysticeti) with their executed state root. They form an
/// append-only chain rooted at genesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub sequence: u64,
    pub vertices: Vec<[u8; 32]>,
    pub transactions: Arc<[SignedTransaction]>,
    #[serde(default)]
    pub transaction_effects: Arc<[TransactionEffects]>,
    pub state_root: Vec<u8>,
    pub timestamp: u64,
    pub prev_checkpoint_hash: Vec<u8>,
}

impl Checkpoint {
    pub fn new<T>(
        sequence: u64,
        vertices: Vec<[u8; 32]>,
        transactions: T,
        state_root: Vec<u8>,
        timestamp: u64,
        prev_checkpoint_hash: Vec<u8>,
    ) -> Self
    where
        T: Into<Arc<[SignedTransaction]>>,
    {
        Self {
            sequence,
            vertices,
            transactions: transactions.into(),
            transaction_effects: Vec::new().into(),
            state_root,
            timestamp,
            prev_checkpoint_hash,
        }
    }

    pub fn with_transaction_effects<T>(mut self, effects: T) -> Self
    where
        T: Into<Arc<[TransactionEffects]>>,
    {
        self.transaction_effects = effects.into();
        self
    }

    pub fn hash(&self) -> Result<Vec<u8>> {
        let tx_hashes: Vec<Vec<u8>> = self
            .transactions
            .iter()
            .map(|tx| tx.transaction_hash().to_vec())
            .collect();
        let serialized = bcs::to_bytes(&(
            self.sequence,
            &tx_hashes,
            &self.state_root,
            &self.prev_checkpoint_hash,
        ))?;
        Ok(hash_data_blake3(&serialized))
    }

    pub fn genesis() -> Self {
        Self {
            sequence: 0,
            vertices: Vec::new(),
            transactions: Vec::new().into(),
            transaction_effects: Vec::new().into(),
            state_root: smt::default_hashes()[0].to_vec(),
            timestamp: 0,
            prev_checkpoint_hash: vec![0u8; 32],
        }
    }
}
