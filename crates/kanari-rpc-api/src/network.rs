// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! System health and network status types.

use serde::{Deserialize, Serialize};

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub sync_status: String,
    pub network: String,
    pub supply_invariants_ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supply_invariant_error: Option<String>,
    pub fail_fast_enabled: bool,
    pub strict_persistence_required: bool,
    pub strict_checkpoint_roots: bool,
    pub persistent_storage_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAuthorityStatus {
    pub authority_id: String,
    pub local: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub local_authority_id: String,
    pub authority_count: usize,
    pub authorities: Vec<NetworkAuthorityStatus>,
}
