// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#[test]
fn quantum_hash_roundtrip() {
    use kanari_crypto::hashs::sha2::hash_data_sha384;
    use kanari_crypto::hashs::shake128::*;
    let data = b"kanari quantum hash test";
    let h1 = hash_data_shake128(data);
    let h2 = hash_data_shake128(data);
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 32);

    let h3 = hash_data_sha384(data);
    assert_eq!(h3.len(), 48);

    // Different algorithms produce different outputs
    assert_ne!(h1, h3[..32]);
}

#[test]
fn quantum_hash_chunks() {
    use kanari_crypto::hashs::sha2::hash_data_sha384;
    use kanari_crypto::hashs::sha2::hash_data_sha384_chunks;
    use kanari_crypto::hashs::shake128::*;
    let a = hash_data_shake128(b"hello");
    let b = hash_data_shake128_chunks(&[b"hel", b"lo"]);
    assert_eq!(a, b);

    let c = hash_data_sha384(b"hello");
    let d = hash_data_sha384_chunks(&[b"hel", b"lo"]);
    assert_eq!(c, d);
}
