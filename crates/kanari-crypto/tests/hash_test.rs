// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use kanari_crypto::{
    HashAlgorithm, hash_data, hash_data_blake3, hash_data_sha3_512, hash_data_shake256,
    hash_data_shake256_custom, hash_data_with_algorithm,
};

#[test]
fn default_hash_is_sha3_256_and_has_stable_length() {
    let data = b"kanari hash api regression test";

    let default_hash = hash_data(data);
    let explicit_hash = hash_data_with_algorithm(data, HashAlgorithm::Sha3_256);

    assert_eq!(default_hash.len(), 32);
    assert_eq!(default_hash, explicit_hash);
}

#[test]
fn named_hash_wrappers_match_explicit_algorithm_dispatch() {
    let data = b"kanari flutter bridge uses these public wrappers";

    assert_eq!(
        hash_data_sha3_512(data),
        hash_data_with_algorithm(data, HashAlgorithm::Sha3_512)
    );
    assert_eq!(
        hash_data_blake3(data),
        hash_data_with_algorithm(data, HashAlgorithm::Blake3)
    );
    assert_eq!(
        hash_data_shake256(data),
        hash_data_with_algorithm(data, HashAlgorithm::Shake256)
    );
}

#[test]
fn hash_output_lengths_are_contractual_for_callers() {
    let data = b"length compatibility";

    assert_eq!(hash_data(data).len(), 32);
    assert_eq!(hash_data_blake3(data).len(), 32);
    assert_eq!(hash_data_shake256(data).len(), 32);
    assert_eq!(hash_data_sha3_512(data).len(), 64);
    assert_eq!(hash_data_shake256_custom(data, 64).len(), 64);
}

#[test]
fn shake256_custom_output_extends_default_prefix() {
    let data = b"shake xof deterministic prefix";

    let default_output = hash_data_shake256(data);
    let extended_output = hash_data_shake256_custom(data, 64);

    assert_eq!(default_output, extended_output[..32]);
}

#[test]
fn supported_hashes_are_deterministic_and_domain_distinct() {
    let data = b"same input, different algorithms";

    let sha3_256_a = hash_data(data);
    let sha3_256_b = hash_data(data);
    let blake3 = hash_data_blake3(data);
    let shake256 = hash_data_shake256(data);

    assert_eq!(sha3_256_a, sha3_256_b);
    assert_ne!(sha3_256_a, blake3);
    assert_ne!(sha3_256_a, shake256);
    assert_ne!(blake3, shake256);
}
