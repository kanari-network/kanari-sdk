// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Core RPC data types.

use kanari_types::event::Event;
use kanari_types::transaction::{
    ObjectChange, ObjectGraphEdge, ObjectOwnerKind, SignedTransaction, TransactionEffects,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Owner info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerInfo {
    pub owner: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub nonce: Option<u64>,
    pub modules: Vec<String>,
    pub balances: BTreeMap<String, u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned_object_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned_objects: Option<Vec<ObjectInfo>>,
}

/// Token balance info.
/// `decimals` is optional: `None` means on-chain metadata unknown —
/// callers must show unknown, never invent 9/6/0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub token_type: String,
    pub amount: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
    pub symbol: String,
}

/// Checkpoint-backed block view response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub height: u64,
    pub timestamp: u64,
    pub hash: String,
    pub prev_hash: String,
    pub tx_count: usize,
    pub state_root: String,
    pub events: Vec<RpcEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainStats {
    pub height: u64,
    pub total_blocks: usize,
    pub total_transactions: usize,
    pub pending_transactions: usize,
    pub total_owners: usize,
    pub total_supply: u64,
    pub state_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInfo {
    pub id: String,
    pub owner: String,
    pub owner_kind: ObjectOwnerKind,
    pub type_: String,
    pub data: Vec<u8>,
    pub version: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RpcObjectOwnerKindFilter {
    Address,
    Shared,
    Immutable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockData {
    pub height: u64,
    pub timestamp: u64,
    pub hash: String,
    pub prev_hash: String,
    pub state_root: String,
    pub tx_count: usize,
    pub events: Vec<Event>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub transaction_effects: Vec<TransactionEffects>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub object_changes: Vec<ObjectChange>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub object_graph_edges: Vec<ObjectGraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullBlockData {
    pub height: u64,
    pub timestamp: u64,
    pub hash: String,
    pub prev_hash: String,
    pub state_root: String,
    pub tx_count: usize,
    pub events: Vec<Event>,
    pub transactions: Vec<SignedTransaction>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub transaction_effects: Vec<TransactionEffects>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub object_changes: Vec<ObjectChange>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub object_graph_edges: Vec<ObjectGraphEdge>,
    pub vertices: Vec<String>,
}

/// Event emitted by Move runtime (RPC representation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcEvent {
    pub key: Vec<u8>,
    pub sequence_number: u64,
    pub type_tag: String,
    pub event_data: Vec<u8>,
}
