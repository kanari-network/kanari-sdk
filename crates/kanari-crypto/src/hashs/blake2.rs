// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// BLAKE2b (512-bit, quantum-resistant).
#[must_use]
pub fn hash_data_blake2b(data: &[u8]) -> Vec<u8> {
    hash_data_blake2b_chunks(&[data])
}

#[must_use]
pub fn hash_data_blake2b_chunks(chunks: &[&[u8]]) -> Vec<u8> {
    use blake2::{Blake2b512, Digest};
    let mut hasher = Blake2b512::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    hasher.finalize().to_vec()
}

/// BLAKE2s (256-bit, quantum-resistant).
#[must_use]
pub fn hash_data_blake2s(data: &[u8]) -> Vec<u8> {
    hash_data_blake2s_chunks(&[data])
}

#[must_use]
pub fn hash_data_blake2s_chunks(chunks: &[&[u8]]) -> Vec<u8> {
    use blake2::{Blake2s256, Digest};
    let mut hasher = Blake2s256::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    hasher.finalize().to_vec()
}
