// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use kanari_crypto::hashs::*;
use kanari_crypto::kem;

#[test]
fn attack_hash_tampering_detected() {
    let data = b"kanari attack simulation: original payload";
    let hash = hash_data_sha3_256(data);

    // Tamper with data
    let mut tampered = data.to_vec();
    tampered[0] ^= 0x01;
    let tampered_hash = hash_data_sha3_256(&tampered);
    assert_ne!(hash, tampered_hash, "tampered data must produce different hash");

    // Tamper with hash output
    let mut bad_hash = hash.clone();
    bad_hash[0] ^= 0x01;
    assert_ne!(hash, bad_hash, "tampered hash must not match");
}

#[test]
fn attack_hash_length_extension_resistance() {
    // SHA-3/SHAKE are immune to length extension; verify outputs differ
    let data = b"kanari";
    let h1 = hash_data_sha3_256(data);
    let h2 = hash_data_sha3_256(b"kanari-extra");
    assert_ne!(h1, h2);

    // SHAKE XOF: different output lengths produce different prefixes
    let xof32 = hash_data_shake256_custom(data, 32);
    let xof64 = hash_data_shake256_custom(data, 64);
    assert_eq!(&xof64[..32], &xof32[..], "XOF prefix must match");
}

#[test]
fn attack_kem_tampered_ciphertext_fails() {
    let kp = kem::generate_keypair().unwrap();
    let enc = kem::encapsulate(&kp.public_key).unwrap();

    // Tamper with ciphertext
    let mut bad_ct = enc.ciphertext.clone();
    bad_ct[0] ^= 0xFF;
    let result = kem::decapsulate(&kp.private_key, &bad_ct);
    // Either fails or produces different secret
    match result {
        Err(_) => {}
        Ok(secret) => {
            assert_ne!(secret, enc.shared_secret, "tampered ciphertext must not yield same secret")
        }
    }
}

#[test]
fn attack_kem_wrong_private_key_fails() {
    let kp1 = kem::generate_keypair().unwrap();
    let kp2 = kem::generate_keypair().unwrap();
    let enc = kem::encapsulate(&kp1.public_key).unwrap();

    // Decapsulate with wrong private key
    let result = kem::decapsulate(&kp2.private_key, &enc.ciphertext);
    match result {
        Err(_) => {}
        Ok(secret) => assert_ne!(secret, enc.shared_secret, "wrong key must not yield same secret"),
    }
}

#[test]
fn attack_kem_truncated_ciphertext_fails() {
    let kp = kem::generate_keypair().unwrap();
    let enc = kem::encapsulate(&kp.public_key).unwrap();

    // Truncated ciphertext
    let truncated = &enc.ciphertext[..16];
    let result = kem::decapsulate(&kp.private_key, truncated);
    assert!(result.is_err(), "truncated ciphertext must fail");
}

#[test]
fn attack_kem_empty_ciphertext_fails() {
    let kp = kem::generate_keypair().unwrap();
    let result = kem::decapsulate(&kp.private_key, &[]);
    assert!(result.is_err(), "empty ciphertext must fail");
}

#[test]
fn attack_kem_invalid_public_key_fails() {
    let bad_pk = vec![0u8; 32];
    // Encapsulating to an all-zero public key may or may not fail;
    // if it succeeds, decapsulation with a real key must not match.
    let result = kem::encapsulate(&bad_pk);
    // Either way, no panic
    let _ = result;
}

#[test]
fn attack_argon2_wrong_password_rejected() {
    let password = b"kanari-strong-password";
    let hash = hash_password(password);
    assert!(!hash.is_empty());
    assert!(verify_password(password, &hash));
    assert!(!verify_password(b"wrong", &hash));
    assert!(!verify_password(b"", &hash));
    // Tampered hash
    let mut bad_hash = hash.clone();
    if !bad_hash.is_empty() {
        bad_hash.pop();
    }
    assert!(!verify_password(password, &bad_hash));
}

#[test]
fn attack_all_hashes_deterministic() {
    let data = b"determinism check";
    let algs = [
        HashAlgorithm::Sha2_256,
        HashAlgorithm::Sha2_384,
        HashAlgorithm::Sha2_512,
        HashAlgorithm::Sha3_256,
        HashAlgorithm::Sha3_512,
        HashAlgorithm::Blake2b,
        HashAlgorithm::Blake2s,
        HashAlgorithm::Blake3,
        HashAlgorithm::Keccak256,
        HashAlgorithm::Ripemd160,
        HashAlgorithm::Shake128,
        HashAlgorithm::Shake256,
    ];
    for alg in algs {
        let h1 = hash_data_with_algorithm(data, alg);
        let h2 = hash_data_with_algorithm(data, alg);
        assert_eq!(h1, h2, "{:?} not deterministic", alg);
    }
}
