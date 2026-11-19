// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Keccak-256 (Ethereum hash, quantum-resistant).
#[must_use]
pub fn hash_data_keccak256(data: &[u8]) -> Vec<u8> {
    hash_data_keccak256_chunks(&[data])
}

#[must_use]
pub fn hash_data_keccak256_chunks(chunks: &[&[u8]]) -> Vec<u8> {
    use sha3::{Digest, Keccak256};
    let mut hasher = Keccak256::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    hasher.finalize().to_vec()
}

/// Keccak-384.
#[must_use]
pub fn hash_data_keccak384(data: &[u8]) -> Vec<u8> {
    hash_data_keccak384_chunks(&[data])
}

#[must_use]
pub fn hash_data_keccak384_chunks(chunks: &[&[u8]]) -> Vec<u8> {
    use sha3::{Digest, Keccak384};
    let mut hasher = Keccak384::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    hasher.finalize().to_vec()
}

/// Keccak-512.
#[must_use]
pub fn hash_data_keccak512(data: &[u8]) -> Vec<u8> {
    hash_data_keccak512_chunks(&[data])
}

#[must_use]
pub fn hash_data_keccak512_chunks(chunks: &[&[u8]]) -> Vec<u8> {
    use sha3::{Digest, Keccak512};
    let mut hasher = Keccak512::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    hasher.finalize().to_vec()
}
