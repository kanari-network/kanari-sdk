module kanari_system::object {
    use kanari_system::tx_context;
    use kanari_system::tx_context::TxContext;
    use std::signer;
    use std::vector;

    /// Simple UID wrapper used for resource IDs in this package.
    /// The UID contains an object-style address generated from the
    /// transaction context, ensuring it is unique per creation.
    struct UID has store, drop {
        addr: address,
    }

    // --- Public Creator ---

    /// Create a new UID by deriving a fresh object address from the
    /// transaction context. This ensures the address is unique and based on the
    /// current transaction input (e.g., transaction hash, counter).
    /// Used by resources that need a guaranteed unique ID when they are created.
    public fun new(ctx: &mut TxContext): UID {
        UID { addr: tx_context::fresh_object_address(ctx) }
    }

    // --- Public Getters ---

    /// Return the underlying address for a UID.
    /// This is the canonical representation of the object's ID.
    public fun uid_address(u: &UID): address {
        u.addr
    }

    /// Return the object's address as a `u64` value.
    /// This is useful for numerical operations or compatibility with systems
    /// that require integer IDs, provided the address fits within u64.
    /// Note: This relies on Move's standard casting behavior for `address` to `u64`.
    public fun id_address_as_u64(u: &UID): u64 {
        signer::address_to_u64(u.addr)
    }

    /// Return the object's address as a `vector<u8>` (32 bytes).
    /// This is useful for serialization, hashing, and interoperability across modules.
    public fun id_bytes(u: &UID): vector<u8> {
        signer::address_to_bytes(u.addr)
    }

    // --- Tests ---
    // Note: Since `tx_context` is external, we cannot fully test `new` here,
    // but we can test the getters on a hardcoded address.

    #[test]
    fun test_uid_getters() {
        let test_addr = @0x1234;
        let test_u64 = signer::address_to_u64(test_addr);
        let test_bytes = signer::address_to_bytes(test_addr);

        let uid = UID { addr: test_addr };
        
        // 1. Check address
        assert!(uid_address(&uid) == test_addr, 0);

        // 2. Check u64 conversion
        assert!(id_address_as_u64(&uid) == test_u64, 1);

        // 3. Check bytes conversion
        assert!(vector::length(&id_bytes(&uid)) == vector::length(&test_bytes), 2);
        
        let UID { addr: _ } = uid; // Consume resource
    }

}