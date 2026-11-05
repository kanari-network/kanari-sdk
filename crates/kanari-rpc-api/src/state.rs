// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! State and SMT diagnostic types.

use serde::{Deserialize, Serialize};

/// Request for state root (optional height). If `height` is None, latest root is returned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStateRootRequest {
    pub height: Option<u64>,
}

/// Owner state proof returned to light clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerProof {
    pub state_root: String,
    pub is_member: bool,
    pub leaf_hash: Vec<u8>,
    pub siblings: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOwnerProofRequest {
    pub address: String,
    pub height: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GetSmtStatusRequest {
    /// Run a full persisted-leaf and canonical-root audit. This can be
    /// expensive on a large database and defaults to false.
    #[serde(default)]
    pub audit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SmtStatusResponse {
    pub height: u64,
    pub checkpoint_state_root: String,
    pub enabled: bool,
    pub persisted_root: Option<String>,
    pub effective_root: String,
    pub overlay_entries: usize,
    pub overlay_updates: usize,
    pub overlay_deletes: usize,
    pub canonical_membership_changed: bool,
    pub runtime_schema_version: Option<u32>,
    pub expected_runtime_schema_version: u32,
    pub wallet_supply_index_version: Option<u32>,
    pub expected_wallet_supply_index_version: u32,
    pub audit_requested: bool,
    pub audit_performed: bool,
    pub persisted_leaf_count: Option<usize>,
    pub consistent: Option<bool>,
    pub consistency_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CanonicalStateEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GetCanonicalStateSnapshotRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CanonicalStateSnapshotResponse {
    pub height: u64,
    pub state_root: String,
    pub entry_count: usize,
    pub entries: Vec<CanonicalStateEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompareCanonicalStateSnapshotRequest {
    pub entries: Vec<CanonicalStateEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CanonicalStateDiffResponse {
    pub height: u64,
    pub state_root: String,
    pub local_entry_count: usize,
    pub remote_entry_count: usize,
    pub first_divergence: Option<String>,
}
