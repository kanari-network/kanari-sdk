// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

module std::signer {
    // Borrows the address of the signer
    // Conceptually, you can think of the `signer` as being a struct wrapper arround an
    // address
    // ```
    // struct signer has drop { addr: address }
    // ```
    // `borrow_address` borrows this inner field
    native public fun borrow_address(s: &signer): &address;

    // Copies the address of the signer
    public fun address_of(s: &signer): address {
        *borrow_address(s)
    }

    /// Converts an `address` to a `u64`: the value of its low 8 bytes
    /// in big-endian order. High bytes are ignored, so this is a hash-like
    /// projection, not an injection -- do not use it as a unique key.
    native public fun address_to_u64(a: address): u64;

    /// Converts an `address` to its 32-byte big-endian representation
    /// (same encoding as `kanari_system::address::to_u256`).
    native public fun address_to_bytes(a: address): vector<u8>;
}
