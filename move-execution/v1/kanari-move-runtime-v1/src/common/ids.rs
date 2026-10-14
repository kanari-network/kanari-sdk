// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Shared identifier utilities for object and module IDs.

use move_core_types::account_address::AccountAddress;

/// Canonicalize an object ID to its full 32-byte hex literal form
/// (`0x` + 64 lowercase hex chars).
///
/// Every subsystem (transfers, storage keys, access keys, harnesses) must key
/// the same object identically. Short forms previously collided with
/// full-form lookups and broke downstream hex decoding, so leading zeros are
/// preserved, never stripped.
/// Returns `None` if the ID is not a valid hex-encoded Move address.
pub(crate) fn canonical_object_id(id: &str) -> Option<String> {
    let trimmed = id.trim();
    let hex_part = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))?;
    if hex_part.is_empty()
        || hex_part.len() > AccountAddress::LENGTH * 2
        || !hex_part.chars().all(|c| c.is_ascii_hexdigit())
    {
        return None;
    }
    let mut out = String::with_capacity(2 + AccountAddress::LENGTH * 2);
    out.push_str("0x");
    for _ in 0..AccountAddress::LENGTH * 2 - hex_part.len() {
        out.push('0');
    }
    out.push_str(&hex_part.to_ascii_lowercase());
    Some(out)
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
    const HEXDIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(2 + AccountAddress::LENGTH * 2);
    out.push_str("0x");
    for byte in arr {
        out.push(HEXDIGITS[(byte >> 4) as usize] as char);
        out.push(HEXDIGITS[(byte & 0x0F) as usize] as char);
    }
    Some(out)
}
