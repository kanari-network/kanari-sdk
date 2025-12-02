/// Module which defines SHA hashes for byte vectors.
///
/// The functions in this module are natively declared both in the Move runtime
/// as in the Move prover's prelude.
module std::hash {
    use std::vector;
    
    native public fun sha2_256(data: vector<u8>): vector<u8>;
    native public fun sha3_256(data: vector<u8>): vector<u8>;

    /// @param data: Arbitrary binary data to hash
    /// Hash the input bytes using Blake2b-256 and returns 32 bytes.
    native public fun blake2b256(data: &vector<u8>): vector<u8>;

    /// @param data: Arbitrary binary data to hash
    /// Hash the input bytes using Blake3-256 and returns 32 bytes. 
    native public fun blake3_256(data: &vector<u8>): vector<u8>;

    /// @param data: Arbitrary binary data to hash
    /// Hash the input bytes using keccak256 and returns 32 bytes.
    native public fun keccak256(data: &vector<u8>): vector<u8>;

    /// @param data: Arbitrary binary data to hash
    /// Hash the input bytes using ripemd160 and returns 20 bytes.
    native public fun ripemd160(data: &vector<u8>): vector<u8>;

    #[test]
    fun test_keccak256_hash() {
        let msg = b"hello world!";
        let hashed_msg_bytes = x"57caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd6";
        let hashed_msg = keccak256(&msg);
        assert!(hashed_msg == hashed_msg_bytes, 0);

        let empty_msg = b"";
        let _ = keccak256(&empty_msg);
        let long_msg = b"57caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd6";
        let _ = keccak256(&long_msg);
    }

    #[test]
    fun test_blake2b256_hash() {
        let msg = b"hello world!";
        let hashed_msg_bytes = x"4fccfb4d98d069558aa93e9565f997d81c33b080364efd586e77a433ddffc5e2";
        let hashed_msg = blake2b256(&msg);
        assert!(hashed_msg == hashed_msg_bytes, 0);

        let empty_msg = b"";
        let _ = blake2b256(&empty_msg);
        let long_msg = b"57caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd6";
        let _ = blake2b256(&long_msg);
    }

    #[test]
    fun test_blake3_256_hash() {
        let msg = b"hello world!";
        let hashed_msg = blake3_256(&msg);
        assert!(vector::length(&hashed_msg) == 32, 0);

        let empty_msg = b"";
        let _ = blake3_256(&empty_msg);
        let long_msg = b"57caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd6";
        let _ = blake3_256(&long_msg);
    }

    #[test]
    fun test_ripemd160_hash() {
        let msg = b"Hello, World!";
        let hashed_msg_bytes = x"527a6a4b9a6da75607546842e0e00105350b1aaf";
        let hashed_msg = ripemd160(&msg);
        assert!(hashed_msg == hashed_msg_bytes, 0);

        let empty_msg = b"";
        let _ = ripemd160(&empty_msg);
        let long_msg = b"57caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd657caa176af1ac0433c5df30e8dabcd2ec1af1e92a26eced5f719b88458777cd6";
        let _ = ripemd160(&long_msg);
    }

}
