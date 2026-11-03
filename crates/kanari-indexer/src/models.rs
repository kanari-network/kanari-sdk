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
    pub nonce: u64,
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
    pub tx_hash: Option<String>,
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

/// Indexed object locked supply change event (from escrow ObjectLockedSupplyChanged)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedObjectLockedSupplyEvent {
    pub id: i64,
    pub tx_hash: Option<String>,
    pub block_height: u64,
    pub object_id: String,
    pub token_type: String,
    pub amount: u64,
    pub action: u8,
    pub action_label: String,
    pub actor: String,
    pub timestamp: u64,
    pub created_at: DateTime<Utc>,
}

impl IndexedObjectLockedSupplyEvent {
    pub fn action_label(action: u8) -> &'static str {
        match action {
            1 => "locked",
            2 => "released",
            3 => "refunded",
            _ => "unknown",
        }
    }
}

/// Indexer metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerMetadata {
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
}

/// KRC-1155 semi-fungible token event action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Krc1155Action {
    Minted,
    Burned,
    Transferred,
    Unknown,
}

impl Krc1155Action {
    pub fn from_u8(action: u8) -> Self {
        match action {
            1 => Krc1155Action::Minted,
            2 => Krc1155Action::Burned,
            3 => Krc1155Action::Transferred,
            _ => Krc1155Action::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Krc1155Action::Minted => "minted",
            Krc1155Action::Burned => "burned",
            Krc1155Action::Transferred => "transferred",
            Krc1155Action::Unknown => "unknown",
        }
    }
}

/// Indexed KRC-1155 event (Minted / Burned / Transferred)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedKrc1155Event {
    pub id: i64,
    pub tx_hash: Option<String>,
    pub block_height: u64,
    pub token_type: String,
    pub action: Krc1155Action,
    pub amount: u64,
    pub sender: Option<String>,
    pub recipient: Option<String>,
    pub owner: Option<String>,
    pub timestamp: u64,
    pub created_at: DateTime<Utc>,
}

/// Indexed KRC-721 event (MintLog)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedKrc721Event {
    pub id: i64,
    pub tx_hash: Option<String>,
    pub block_height: u64,
    pub object_id: String,
    pub creator: String,
    pub collection_id: String,
    pub timestamp: u64,
    pub created_at: DateTime<Utc>,
}
