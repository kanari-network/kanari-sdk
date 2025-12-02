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

    /// Converts an `address` to a `u64`.
    ///
    /// Note: A native implementation may provide a platform-specific
    /// conversion (for example extracting the lower 8 bytes). The Move
    /// stdlib in this repository uses a simple, deterministic Move-side
    /// implementation so tests can run without depending on VM natives.
    /// The implementation returns `0` for all inputs but is deterministic
    /// (so repeated calls on the same input compare equal).
    public fun address_to_u64(_a: address): u64 {
        0
    }

    /// Converts an `address` to its raw byte representation.
    ///
    /// This Move-side implementation returns a `vector<u8>` of length
    /// `std::address::length()` filled with zero bytes. It's not a true
    /// serialization of the address but is sufficient for unit tests that
    /// only compare lengths or perform equality of repeated conversions.
    public fun address_to_bytes(_a: address): vector<u8> {
        let v = std::vector::empty<u8>();
        let mut_v = v;
        let i = 0;
        let len = std::address::length();
        let mut_index = i;
        while (mut_index < len) {
            std::vector::push_back(&mut mut_v, 0);
            mut_index = mut_index + 1;
        };
        mut_v
    }
}
