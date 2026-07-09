// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Data models for the indexer

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Indexed block information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedBlock {
    pub height: u64,
    pub hash: String,
    pub prev_hash: String,
    pub state_root: String,
    pub merkle_root: String,
    pub timestamp: u64,
    pub tx_count: usize,
    pub created_at: DateTime<Utc>,
}

/// Indexed transaction information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedTransaction {
    pub id: i64,
    pub tx_hash: String,
    pub block_height: u64,
    pub sender: String,
    pub tx_type: String,
    pub sequence_number: u64,
    pub gas_limit: u64,
    pub gas_price: u64,
    pub gas_used: u64,
    pub status: String,
    pub signature: String,
    pub raw_data: Option<Vec<u8>>,
    pub timestamp: u64,
    pub created_at: DateTime<Utc>,
}

/// Transaction argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionArg {
    pub id: i64,
    pub tx_hash: String,
    pub arg_index: u32,
    pub arg_value: Vec<u8>,
}

/// Indexed event information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedEvent {
    pub id: i64,
    pub event_key: String,
    pub sequence_number: u64,
    pub type_tag: String,
    pub event_data: Option<Vec<u8>>,
    pub tx_hash: String,
    pub block_height: u64,
    pub created_at: DateTime<Utc>,
}

/// Indexed coin information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedCoin {
    pub id: String,
    pub owner: String,
    pub coin_type: String,
    pub balance: u64,
    pub is_frozen: bool,
    pub created_tx_hash: Option<String>,
    pub last_updated_tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Owner balance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerBalance {
    pub address: String,
    pub coin_type: String,
    pub total_balance: u64,
    pub coin_count: u32,
    pub last_updated: DateTime<Utc>,
}

pub type AccountBalance = OwnerBalance;

/// Indexer metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerMetadata {
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
}
