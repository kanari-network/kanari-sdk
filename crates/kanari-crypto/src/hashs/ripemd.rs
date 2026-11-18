// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// RIPEMD-160 (160-bit, used in Bitcoin address derivation).
#[must_use]
pub fn hash_data_ripemd160(data: &[u8]) -> Vec<u8> {
    hash_data_ripemd160_chunks(&[data])
}

#[must_use]
pub fn hash_data_ripemd160_chunks(chunks: &[&[u8]]) -> Vec<u8> {
    use ripemd::{Digest, Ripemd160};
    let mut hasher = Ripemd160::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    hasher.finalize().to_vec()
}
