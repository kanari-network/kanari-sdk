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

/// Canonical hex form of `id`, or `id` unchanged when it is not a valid
/// hex-encoded Move address.
///
/// Used wherever an id must be normalized for comparison or storage but an
/// unparseable value keeps the handling it had before normalization was
/// introduced — falling back to the input instead of dropping it.
pub(crate) fn canonical_object_id_or_raw(id: &str) -> String {
    canonical_object_id(id).unwrap_or_else(|| id.to_owned())
}

/// Every spelling an object id may be stored under, in lookup order.
///
/// Object ids arrive in short (`0x2`) or full (`0x00..02`) form from different
/// callers, while storage keys preserve leading zeros. Resolve via the
/// canonical short form first, then fall back to the raw input and the
/// zero-padded full form so every spelling hits. Duplicates are dropped so each
/// storage key is probed once.
pub(crate) fn object_id_variants(id: &str) -> Vec<String> {
    let mut variants = Vec::with_capacity(3);
    variants.push(canonical_object_id_or_raw(id));
    if variants[0] != id {
        variants.push(id.to_owned());
    }
    if let Ok(address) = AccountAddress::from_hex_literal(id) {
        let full = format!("0x{}", hex::encode(address.as_ref()));
        if !variants.contains(&full) {
            variants.push(full);
        }
    }
    variants
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
