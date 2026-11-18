// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod argon2;
pub mod blake2;
pub mod blake3;
pub mod keccak;
pub mod ripemd;
pub mod sha2;
pub mod sha3;
pub mod shake128;
pub mod shake256;

pub use argon2::{hash_password, verify_password};
pub use blake2::{
    hash_data_blake2b, hash_data_blake2b_chunks, hash_data_blake2s, hash_data_blake2s_chunks,
};
pub use blake3::{
    hash_data_blake3, hash_data_blake3_array, hash_data_blake3_chunks,
    hash_data_blake3_chunks_array, hash_data_blake3_derive_key, hash_data_blake3_keyed,
    hash_data_blake3_keyed_chunks, hash_data_blake3_xof, hash_data_blake3_xof_chunks,
};
pub use keccak::{
    hash_data_keccak256, hash_data_keccak256_chunks, hash_data_keccak384,
    hash_data_keccak384_chunks, hash_data_keccak512, hash_data_keccak512_chunks,
};
pub use ripemd::{hash_data_ripemd160, hash_data_ripemd160_chunks};
pub use sha2::{
    hash_data_sha2_224, hash_data_sha2_224_chunks, hash_data_sha2_256, hash_data_sha2_256_chunks,
    hash_data_sha2_512, hash_data_sha2_512_224, hash_data_sha2_512_224_chunks,
    hash_data_sha2_512_256, hash_data_sha2_512_256_chunks, hash_data_sha2_512_chunks,
    hash_data_sha384, hash_data_sha384_chunks,
};
pub use sha3::{
    hash_data_sha3_256, hash_data_sha3_256_chunks, hash_data_sha3_512, hash_data_sha3_512_chunks,
};
pub use shake128::{
    hash_data_shake128, hash_data_shake128_chunks, hash_data_shake128_custom,
    hash_data_shake128_custom_chunks,
};
pub use shake256::{
    hash_data_shake256, hash_data_shake256_chunks, hash_data_shake256_custom,
    hash_data_shake256_custom_chunks,
};

/// Hash algorithm options (including quantum-resistant)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HashAlgorithm {
    /// SHA2-224 algorithm
    Sha2_224,
    /// SHA2-256 algorithm (widely compatible classical hash)
    Sha2_256,
    /// SHA2-384 algorithm (quantum-resistant, 192-bit security)
    Sha2_384,
    /// SHA2-512 algorithm
    Sha2_512,
    /// SHA2-512/224 algorithm
    Sha2_512_224,
    /// SHA2-512/256 algorithm
    Sha2_512_256,
    /// SHA3-256 algorithm (default, quantum-resistant)
    #[default]
    Sha3_256,
    /// SHA3-512 algorithm (higher security, quantum-resistant)
    Sha3_512,
    /// Keccak-256 (Ethereum hash, quantum-resistant)
    Keccak256,
    /// Keccak-384
    Keccak384,
    /// Keccak-512
    Keccak512,
    /// BLAKE2b algorithm (512-bit, quantum-resistant)
    Blake2b,
    /// BLAKE2s algorithm (256-bit, quantum-resistant)
    Blake2s,
    /// BLAKE3 algorithm (faster, equally secure)
    Blake3,
    /// RIPEMD-160 (160-bit)
    Ripemd160,
    /// SHAKE128 (extendable output, quantum-resistant)
    Shake128,
    /// SHAKE256 (extendable output, quantum-resistant)
    Shake256,
}

/// Cryptographic hash using SHA3-256 (default)
#[must_use]
pub fn hash_data(data: &[u8]) -> Vec<u8> {
    hash_data_sha3_256_chunks(&[data])
}

/// Cryptographic hash using the specified algorithm
#[must_use]
pub fn hash_data_with_algorithm(data: &[u8], algorithm: HashAlgorithm) -> Vec<u8> {
    hash_data_with_algorithm_chunks(&[data], algorithm)
}

/// Cryptographic hash over multiple input chunks using the specified algorithm.
#[must_use]
pub fn hash_data_with_algorithm_chunks(chunks: &[&[u8]], algorithm: HashAlgorithm) -> Vec<u8> {
    match algorithm {
        HashAlgorithm::Sha2_224 => hash_data_sha2_224_chunks(chunks),
        HashAlgorithm::Sha2_256 => hash_data_sha2_256_chunks(chunks),
        HashAlgorithm::Sha2_384 => hash_data_sha384_chunks(chunks),
        HashAlgorithm::Sha2_512 => hash_data_sha2_512_chunks(chunks),
        HashAlgorithm::Sha2_512_224 => hash_data_sha2_512_224_chunks(chunks),
        HashAlgorithm::Sha2_512_256 => hash_data_sha2_512_256_chunks(chunks),
        HashAlgorithm::Sha3_256 => hash_data_sha3_256_chunks(chunks),
        HashAlgorithm::Sha3_512 => hash_data_sha3_512_chunks(chunks),
        HashAlgorithm::Blake2b => hash_data_blake2b_chunks(chunks),
        HashAlgorithm::Blake2s => hash_data_blake2s_chunks(chunks),
        HashAlgorithm::Blake3 => hash_data_blake3_chunks(chunks),
        HashAlgorithm::Keccak256 => hash_data_keccak256_chunks(chunks),
        HashAlgorithm::Keccak384 => hash_data_keccak384_chunks(chunks),
        HashAlgorithm::Keccak512 => hash_data_keccak512_chunks(chunks),
        HashAlgorithm::Ripemd160 => hash_data_ripemd160_chunks(chunks),
        HashAlgorithm::Shake128 => hash_data_shake128_chunks(chunks),
        HashAlgorithm::Shake256 => hash_data_shake256_chunks(chunks),
    }
}
