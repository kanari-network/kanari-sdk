// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Module which defines SHA hashes for byte vectors.
///
/// The functions in this module are natively declared both in the Move runtime
/// as in the Move prover's prelude.
///
/// Unit tests live in `tests/hash_tests.move`, not here: this module has no
/// `use` imports on purpose so non-test builds stay warning-free.
module std::hash {
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
}
