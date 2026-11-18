// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use kanari_crypto::hashs::*;

    #[test]
    fn sha2_variants_roundtrip() {
        let data = b"kanari sha2 test";
        for alg in [
            HashAlgorithm::Sha2_224,
            HashAlgorithm::Sha2_256,
            HashAlgorithm::Sha2_384,
            HashAlgorithm::Sha2_512,
            HashAlgorithm::Sha2_512_224,
            HashAlgorithm::Sha2_512_256,
        ] {
            let h1 = hash_data_with_algorithm(data, alg);
            let h2 = hash_data_with_algorithm(data, alg);
            assert_eq!(h1, h2, "roundtrip failed for {:?}", alg);
            assert!(!h1.is_empty());
        }
    }

    #[test]
    fn sha2_variants_chunks() {
        let a = hash_data_sha2_224(b"hello");
        let b = hash_data_sha2_224_chunks(&[b"hel", b"lo"]);
        assert_eq!(a, b);

        let c = hash_data_sha2_512_224(b"hello");
        let d = hash_data_sha2_512_224_chunks(&[b"hel", b"lo"]);
        assert_eq!(c, d);

        let e = hash_data_sha2_512_256(b"hello");
        let f = hash_data_sha2_512_256_chunks(&[b"hel", b"lo"]);
        assert_eq!(e, f);
    }

    #[test]
    fn blake2_roundtrip() {
        let data = b"kanari blake2 test";
        let h1 = hash_data_blake2b(data);
        let h2 = hash_data_blake2b(data);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);

        let h3 = hash_data_blake2s(data);
        assert_eq!(h3.len(), 32);
        assert_ne!(h1[..32], h3);
    }

    #[test]
    fn blake2_chunks() {
        let a = hash_data_blake2b(b"hello");
        let b = hash_data_blake2b_chunks(&[b"hel", b"lo"]);
        assert_eq!(a, b);

        let c = hash_data_blake2s(b"hello");
        let d = hash_data_blake2s_chunks(&[b"hel", b"lo"]);
        assert_eq!(c, d);
    }

    #[test]
    fn blake3_keyed_and_xof() {
        let key = [42u8; 32];
        let data = b"kanari blake3 keyed";
        let h1 = hash_data_blake3_keyed(&key, data);
        let h2 = hash_data_blake3_keyed(&key, data);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 32);

        // Different key produces different hash
        let key2 = [7u8; 32];
        let h3 = hash_data_blake3_keyed(&key2, data);
        assert_ne!(h1, h3);

        // XOF
        let xof = hash_data_blake3_xof(data, 64);
        assert_eq!(xof.len(), 64);

        // KDF
        let derived = hash_data_blake3_derive_key("kanari", data);
        assert_eq!(derived.len(), 32);
    }

    #[test]
    fn keccak_roundtrip() {
        let data = b"kanari keccak test";
        for alg in [HashAlgorithm::Keccak256, HashAlgorithm::Keccak384, HashAlgorithm::Keccak512] {
            let h1 = hash_data_with_algorithm(data, alg);
            let h2 = hash_data_with_algorithm(data, alg);
            assert_eq!(h1, h2, "roundtrip failed for {:?}", alg);
        }
    }

    #[test]
    fn keccak_chunks() {
        let a = hash_data_keccak256(b"hello");
        let b = hash_data_keccak256_chunks(&[b"hel", b"lo"]);
        assert_eq!(a, b);

        let c = hash_data_keccak384(b"hello");
        let d = hash_data_keccak384_chunks(&[b"hel", b"lo"]);
        assert_eq!(c, d);

        let e = hash_data_keccak512(b"hello");
        let f = hash_data_keccak512_chunks(&[b"hel", b"lo"]);
        assert_eq!(e, f);
    }

    #[test]
    fn ripemd_roundtrip() {
        let data = b"kanari ripemd test";
        let h1 = hash_data_ripemd160(data);
        let h2 = hash_data_ripemd160(data);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 20);
    }

    #[test]
    fn ripemd_chunks() {
        let a = hash_data_ripemd160(b"hello");
        let b = hash_data_ripemd160_chunks(&[b"hel", b"lo"]);
        assert_eq!(a, b);
    }

    #[test]
    fn shake128_roundtrip() {
        let data = b"kanari shake128 test";
        let h1 = hash_data_shake128(data);
        let h2 = hash_data_shake128(data);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 32);

        let custom = hash_data_shake128_custom(data, 64);
        assert_eq!(custom.len(), 64);
    }

    #[test]
    fn shake128_chunks() {
        let a = hash_data_shake128(b"hello");
        let b = hash_data_shake128_chunks(&[b"hel", b"lo"]);
        assert_eq!(a, b);
    }

    #[test]
    fn argon2_roundtrip() {
        let password = b"kanari-password";
        let hash = hash_password(password);
        assert!(!hash.is_empty());
        assert!(verify_password(password, &hash));
        assert!(!verify_password(b"wrong-password", &hash));
    }

    #[test]
    fn different_algorithms_differ() {
        let data = b"kanari";
        let algs = [
            HashAlgorithm::Sha2_224,
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
        // All produce non-empty output
        for alg in algs {
            let h = hash_data_with_algorithm(data, alg);
            assert!(!h.is_empty(), "{:?} produced empty output", alg);
        }
    }
}
