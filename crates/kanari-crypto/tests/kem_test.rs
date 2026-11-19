// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#[test]
fn kem_roundtrip() {
    let kp = kanari_crypto::kem::generate_keypair().unwrap();
    let enc = kanari_crypto::kem::encapsulate(&kp.public_key).unwrap();
    let dec = kanari_crypto::kem::decapsulate(&kp.private_key, &enc.ciphertext).unwrap();
    assert_eq!(enc.shared_secret, dec);
    assert_eq!(enc.shared_secret.len(), 32);
}

#[test]
fn kem_different_keys_differ() {
    let kp1 = kanari_crypto::kem::generate_keypair().unwrap();
    let kp2 = kanari_crypto::kem::generate_keypair().unwrap();
    let enc = kanari_crypto::kem::encapsulate(&kp1.public_key).unwrap();
    let dec = kanari_crypto::kem::decapsulate(&kp2.private_key, &enc.ciphertext).unwrap();
    assert_ne!(enc.shared_secret, dec);
}
