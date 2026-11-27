// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Small hash abstraction layer using Kanari's canonical crypto crate.

use kanari_crypto::{hash_data_blake3_array, hash_data_blake3_chunks_array};

/// Compute a 32-byte digest of `data` using BLAKE3.
pub fn digest(data: &[u8]) -> [u8; 32] {
    hash_data_blake3_array(data)
}

/// Hash a leaf: H(0x00 || key_hash || value)
#[inline]
pub fn hash_leaf(key_hash: &[u8; 32], value: &[u8]) -> [u8; 32] {
    // Domain-separated bytes are identical to the chunked form; writing them
    // into one stack buffer avoids the per-chunk loop for the small values that
    // make up the canonical state root; larger values keep the generic path.
    const MAX_VALUE_LEN: usize = 256 - 1 - 32;
    if value.len() <= MAX_VALUE_LEN {
        let mut buffer = [0u8; 256];
        buffer[0] = 0x00;
        buffer[1..33].copy_from_slice(key_hash);
        buffer[33..33 + value.len()].copy_from_slice(value);
        return hash_data_blake3_array(&buffer[..33 + value.len()]);
    }
    hash_data_blake3_chunks_array(&[&[0x00], key_hash, value])
}

/// Hash an internal node: H(0x01 || left || right)
#[inline]
pub fn hash_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut buffer = [0u8; 1 + 32 + 32];
    buffer[0] = 0x01;
    buffer[1..33].copy_from_slice(left);
    buffer[33..65].copy_from_slice(right);
    hash_data_blake3_array(&buffer)
}
