// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Canonical hash spellings shared by types, core and the Move runtime.
//!
//! These helpers only wrap existing primitives and serialization. They must
//! not change byte order, tuple layouts, or the presence of a `0x` prefix.

use anyhow::{Context, Result};
use kanari_crypto::hash_data_blake3_array;
use serde::Serialize;

/// BLAKE3 digest as raw bytes.
#[must_use]
pub fn blake3(data: &[u8]) -> [u8; 32] {
    hash_data_blake3_array(data)
}

/// BLAKE3 digest as lowercase hex without a prefix.
#[must_use]
pub fn blake3_hex(data: &[u8]) -> String {
    hex::encode(blake3(data))
}

/// BLAKE3 digest as lowercase hex with a `0x` prefix.
#[must_use]
pub fn blake3_hex_0x(data: &[u8]) -> String {
    format!("0x{}", blake3_hex(data))
}

/// BCS-serialize a value and hash the resulting bytes.
pub fn bcs_blake3<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>> {
    Ok(blake3(&bcs::to_bytes(value)?).to_vec())
}

/// BCS-serialize a value and return its lowercase hex digest.
pub fn bcs_blake3_hex<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    Ok(hex::encode(bcs_blake3(value)?))
}

/// Exact-length conversion for canonical 32-byte digests.
///
/// Never truncates, zero-pads, or re-hashes a short value.
pub fn bytes32_exact(bytes: &[u8], label: &str) -> Result<[u8; 32]> {
    bytes.try_into().with_context(|| format!("{label} must be 32 bytes"))
}

/// Exact-length conversion that fails closed by panicking.
#[must_use]
pub fn bytes32_expect(bytes: &[u8], label: &'static str) -> [u8; 32] {
    bytes.try_into().unwrap_or_else(|_| panic!("{label} must be 32 bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_spellings_match_existing_primitive() {
        let data = b"kanari shared hash";
        let expected = hex::encode(kanari_crypto::hash_data_blake3(data));
        assert_eq!(blake3_hex(data), expected);
        assert_eq!(blake3_hex_0x(data), format!("0x{expected}"));
    }

    #[test]
    fn bytes32_exact_rejects_wrong_lengths() {
        assert!(bytes32_exact(&[0; 31], "digest").is_err());
        assert!(bytes32_exact(&[0; 33], "digest").is_err());
        assert_eq!(bytes32_exact(&[7; 32], "digest").unwrap(), [7; 32]);
    }

    #[test]
    fn bcs_hash_is_stable() {
        let value = (1u64, "abc".to_string());
        assert_eq!(bcs_blake3(&value).unwrap(), bcs_blake3(&value).unwrap());
        assert_eq!(bcs_blake3_hex(&value).unwrap().len(), 64);
    }
}
