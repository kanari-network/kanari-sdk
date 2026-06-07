// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Small hash abstraction layer using BLAKE3 (fast software hash).

/// Compute a 32-byte digest of `data` using BLAKE3.
pub fn digest(data: &[u8]) -> [u8; 32] {
    let h = blake3::hash(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(h.as_bytes());
    out
}

/// Hash a leaf: H(0x00 || key_hash || value)
pub fn hash_leaf(key_hash: &[u8; 32], value: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[0x00]);
    hasher.update(key_hash);
    hasher.update(value);

    let mut out = [0u8; 32];
    out.copy_from_slice(hasher.finalize().as_bytes());
    out
}

/// Hash an internal node: H(0x01 || left || right)
pub fn hash_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut input = [0u8; 65];
    input[0] = 0x01;
    input[1..33].copy_from_slice(left);
    input[33..65].copy_from_slice(right);
    digest(&input)
}
