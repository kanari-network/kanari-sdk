// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Shared identifier utilities for object and module IDs.

use move_core_types::account_address::AccountAddress;

/// Canonicalize an object ID to its normalized hex literal form (e.g., `0x1`).
/// Returns `None` if the ID is not a valid hex-encoded Move address.
pub(crate) fn canonical_object_id(id: &str) -> Option<String> {
    let trimmed = id.trim();
    // Reject non-hex characters early so the error is caught here instead of
    // producing a misleading canonical form.
    if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
        let hex_part = &trimmed[2..];
        if hex_part.is_empty() || !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
    } else if !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    AccountAddress::from_hex_literal(trimmed)
        .ok()
        .map(|addr| addr.to_hex_literal())
}

pub(crate) fn object_id_from_bytes(bytes: &[u8]) -> Option<String> {
    if bytes.len() < AccountAddress::LENGTH {
        return None;
    }

    let mut arr = [0u8; AccountAddress::LENGTH];
    arr.copy_from_slice(&bytes[..AccountAddress::LENGTH]);
    if arr.iter().all(|byte| *byte == 0) {
        return None;
    }
    Some(AccountAddress::new(arr).to_hex_literal())
}
