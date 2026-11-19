// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Cryptographic hash using SHA2-256.
#[must_use]
pub fn hash_data_sha2_256(data: &[u8]) -> Vec<u8> {
    hash_data_sha2_256_chunks(&[data])
}

/// Cryptographic hash using SHA2-256 over multiple input chunks.
#[must_use]
pub fn hash_data_sha2_256_chunks(chunks: &[&[u8]]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    hasher.finalize().to_vec()
}

/// Cryptographic hash using SHA2-512.
#[must_use]
pub fn hash_data_sha2_512(data: &[u8]) -> Vec<u8> {
    hash_data_sha2_512_chunks(&[data])
}

/// Cryptographic hash using SHA2-512 over multiple input chunks.
#[must_use]
pub fn hash_data_sha2_512_chunks(chunks: &[&[u8]]) -> Vec<u8> {
    use sha2::{Digest, Sha512};
    let mut hasher = Sha512::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    hasher.finalize().to_vec()
}

/// Cryptographic hash using SHA2-384 (quantum-resistant, 192-bit security).
#[must_use]
pub fn hash_data_sha384(data: &[u8]) -> Vec<u8> {
    hash_data_sha384_chunks(&[data])
}

/// Cryptographic hash using SHA2-384 over multiple input chunks.
#[must_use]
pub fn hash_data_sha384_chunks(chunks: &[&[u8]]) -> Vec<u8> {
    use sha2::{Digest, Sha384};
    let mut hasher = Sha384::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    hasher.finalize().to_vec()
}

/// Cryptographic hash using SHA2-224.
#[must_use]
pub fn hash_data_sha2_224(data: &[u8]) -> Vec<u8> {
    hash_data_sha2_224_chunks(&[data])
}

#[must_use]
pub fn hash_data_sha2_224_chunks(chunks: &[&[u8]]) -> Vec<u8> {
    use sha2::{Digest, Sha224};
    let mut hasher = Sha224::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    hasher.finalize().to_vec()
}

/// Cryptographic hash using SHA2-512/224.
#[must_use]
pub fn hash_data_sha2_512_224(data: &[u8]) -> Vec<u8> {
    hash_data_sha2_512_224_chunks(&[data])
}

#[must_use]
pub fn hash_data_sha2_512_224_chunks(chunks: &[&[u8]]) -> Vec<u8> {
    use sha2::{Digest, Sha512_224};
    let mut hasher = Sha512_224::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    hasher.finalize().to_vec()
}

/// Cryptographic hash using SHA2-512/256.
#[must_use]
pub fn hash_data_sha2_512_256(data: &[u8]) -> Vec<u8> {
    hash_data_sha2_512_256_chunks(&[data])
}

#[must_use]
pub fn hash_data_sha2_512_256_chunks(chunks: &[&[u8]]) -> Vec<u8> {
    use sha2::{Digest, Sha512_256};
    let mut hasher = Sha512_256::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    hasher.finalize().to_vec()
}
