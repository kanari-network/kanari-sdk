// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! RPC response types.

use crate::types::ObjectInfo;
use kanari_types::transaction::ObjectChange;
use serde::{Deserialize, Serialize};

/// Module query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub address: String,
    pub name: String,
    pub bytecode_hash: String,
    pub size: usize,
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub functions: Vec<String>,
    pub function_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnedObjectsResponse {
    pub objects: Vec<ObjectInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<OwnerObjectSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerObjectSummary {
    pub owner: String,
    pub total_objects: usize,
    pub object_changes_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectChangeSet {
    pub changes: Vec<ObjectChange>,
}

/// Result of a USD faucet claim (`kanari_requestUsdFaucet`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestUsdFaucetResponse {
    pub hash: String,
    pub status: String,
    pub leg_hashes: Vec<String>,
    pub amount_usd: String,
    pub amount_units: u64,
    pub usd_type: String,
    pub recipient: String,
}

/// USD faucet configuration (`kanari_getUsdFaucetStatus`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsdFaucetStatusResponse {
    pub enabled: bool,
    pub usd_type: String,
    pub usd_decimals: u32,
    pub default_drip_usd: String,
    pub max_per_request_usd: String,
    pub cooldown_secs: u64,
}
