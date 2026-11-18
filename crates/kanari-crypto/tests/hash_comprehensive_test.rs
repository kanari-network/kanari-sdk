// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use kanari_crypto::hashs::*;

    #[test]
    fn all_algorithms_empty_input() {
        let algs = [
            HashAlgorithm::Sha2_224,
            HashAlgorithm::Sha2_256,
            HashAlgorithm::Sha2_384,
            HashAlgorithm::Sha2_512,
            HashAlgorithm::Sha2_512_224,
            HashAlgorithm::Sha2_512_256,
            HashAlgorithm::Sha3_256,
            HashAlgorithm::Sha3_512,
            HashAlgorithm::Blake2b,
            HashAlgorithm::Blake2s,
            HashAlgorithm::Blake3,
            HashAlgorithm::Keccak256,
            HashAlgorithm::Keccak384,
            HashAlgorithm::Keccak512,
            HashAlgorithm::Ripemd160,
            HashAlgorithm::Shake128,
            HashAlgorithm::Shake256,
        ];
        for alg in algs {
            let h = hash_data_with_algorithm(b"", alg);
            assert!(!h.is_empty(), "{:?} produced empty output for empty input", alg);
            // Deterministic
            assert_eq!(h, hash_data_with_algorithm(b"", alg));
        }
    }

    #[test]
    fn chunks_match_single_for_all() {
        let data = b"kanari chunk test data";
        let parts = [b"kanari ".as_slice(), b"chunk ".as_slice(), b"test data".as_slice()];
        let all_algs = [
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
            HashAlgorithm::Keccak384,
            HashAlgorithm::Keccak512,
            HashAlgorithm::Ripemd160,
            HashAlgorithm::Shake128,
            HashAlgorithm::Shake256,
        ];
        for alg in all_algs {
            let single = hash_data_with_algorithm(data, alg);
            let chunked = hash_data_with_algorithm_chunks(&parts, alg);
            assert_eq!(single, chunked, "chunks mismatch for {:?}", alg);
        }
    }

    #[test]
    fn output_lengths_contractual() {
        let data = b"length check";
        assert_eq!(hash_data_sha2_224(data).len(), 28);
        assert_eq!(hash_data_sha2_256(data).len(), 32);
        assert_eq!(hash_data_sha384(data).len(), 48);
        assert_eq!(hash_data_sha2_512(data).len(), 64);
        assert_eq!(hash_data_sha2_512_224(data).len(), 28);
        assert_eq!(hash_data_sha2_512_256(data).len(), 32);
        assert_eq!(hash_data_sha3_256_chunks(&[data]).len(), 32);
        assert_eq!(hash_data_sha3_512(data).len(), 64);
        assert_eq!(hash_data_blake2b(data).len(), 64);
        assert_eq!(hash_data_blake2s(data).len(), 32);
        assert_eq!(hash_data_blake3(data).len(), 32);
        assert_eq!(hash_data_keccak256(data).len(), 32);
        assert_eq!(hash_data_keccak384(data).len(), 48);
        assert_eq!(hash_data_keccak512(data).len(), 64);
        assert_eq!(hash_data_ripemd160(data).len(), 20);
        assert_eq!(hash_data_shake128(data).len(), 32);
        assert_eq!(hash_data_shake256(data).len(), 32);
    }

    #[test]
    fn blake3_xof_custom_length() {
        let data = b"xof test";
        let xof = hash_data_blake3_xof(data, 128);
        assert_eq!(xof.len(), 128);
        // Prefix consistency
        let xof64 = hash_data_blake3_xof(data, 64);
        assert_eq!(&xof[..64], &xof64[..]);
    }

    #[test]
    fn shake_custom_length() {
        let data = b"shake custom";
        let s128 = hash_data_shake128_custom(data, 48);
        assert_eq!(s128.len(), 48);
        let s256 = hash_data_shake256_custom(data, 100);
        assert_eq!(s256.len(), 100);
    }

    #[test]
    fn argon2_verify_wrong() {
        let hash = hash_password(b"correct");
        assert!(verify_password(b"correct", &hash));
        assert!(!verify_password(b"incorrect", &hash));
        assert!(!verify_password(b"", &hash));
    }

    #[test]
    fn default_hash_is_sha3_256() {
        let data = b"default";
        assert_eq!(hash_data(data), hash_data_sha3_256_chunks(&[data]));
    }
}
