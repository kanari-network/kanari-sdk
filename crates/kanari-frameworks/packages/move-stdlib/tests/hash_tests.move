#[test_only]
module std::hash_tests {
    use std::hash;
    use std::vector;

    #[test]
    fun sha2_256_expected_hash() {
        let input = x"616263";
        let expected_output = x"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(hash::sha2_256(input) == expected_output, 0);
    }

    #[test]
    fun sha3_256_expected_hash() {
        let input = x"616263";
        let expected_output = x"3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532";
        assert!(hash::sha3_256(input) == expected_output, 0);
    }

    #[test]
    fun test_keccak256_hash() {
        let msg = b"hello world!";
        let hashed_msg_bytes = x"57caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd6";
        let hashed_msg = hash::keccak256(&msg);
        assert!(hashed_msg == hashed_msg_bytes, 0);

        let empty_msg = b"";
        let _ = hash::keccak256(&empty_msg);
        let long_msg = b"57caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd6";
        let _ = hash::keccak256(&long_msg);
    }

    #[test]
    fun test_blake2b256_hash() {
        let msg = b"hello world!";
        let hashed_msg_bytes = x"4fccfb4d98d069558aa93e9565f997d81c33b080364efd586e77a433ddffc5e2";
        let hashed_msg = hash::blake2b256(&msg);
        assert!(hashed_msg == hashed_msg_bytes, 0);

        let empty_msg = b"";
        let _ = hash::blake2b256(&empty_msg);
        let long_msg = b"57caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd6";
        let _ = hash::blake2b256(&long_msg);
    }

    #[test]
    fun test_blake3_256_hash() {
        let msg = b"hello world!";
        let hashed_msg = hash::blake3_256(&msg);
        assert!(vector::length(&hashed_msg) == 32, 0);

        let empty_msg = b"";
        let _ = hash::blake3_256(&empty_msg);
        let long_msg = b"57caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd6";
        let _ = hash::blake3_256(&long_msg);
    }

    #[test]
    fun test_ripemd160_hash() {
        let msg = b"Hello, World!";
        let hashed_msg_bytes = x"527a6a4b9a6da75607546842e0e00105350b1aaf";
        let hashed_msg = hash::ripemd160(&msg);
        assert!(hashed_msg == hashed_msg_bytes, 0);

        let empty_msg = b"";
        let _ = hash::ripemd160(&empty_msg);
        let long_msg = b"57caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd6";
        let _ = hash::ripemd160(&long_msg);
    }
}
