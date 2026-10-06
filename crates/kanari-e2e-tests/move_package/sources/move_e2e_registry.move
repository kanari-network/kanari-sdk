module kanari_e2e_tests::move_e2e_registry {
    use std::string::{Self, String};
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    const E_NOT_OWNER: u64 = 700;
    const E_BAD_LOOKUP: u64 = 701;

    /// Shared name -> address registry.
    public struct Registry has key, store {
        id: UID,
        owner: address,
        names: vector<String>,
        addresses: vector<address>,
    }

    /// Shared registry entry with an opaque payload.
    public struct Entry has key, store {
        id: UID,
        name: String,
        value: vector<u8>,
        owner: address,
    }

    /// Create a shared registry and return its address.
    public fun create_registry(ctx: &mut TxContext): address {
        let registry = e2e_new_registry(ctx);
        let addr = object::uid_address(&registry.id);
        transfer::share_object(registry);
        addr
    }

    /// Owner-only name registration; re-registration overwrites the address.
    public entry fun register_name(
        registry: &mut Registry,
        name: vector<u8>,
        addr: address,
        ctx: &mut TxContext,
    ) {
        assert!(registry.owner == tx_context::sender(ctx), E_NOT_OWNER);
        let name_str = string::utf8(name);
        let mut pos = vector::length(&registry.names);
        while (pos > 0) {
            pos = pos - 1;
            if (*vector::borrow(&registry.names, pos) == name_str) {
                *vector::borrow_mut(&mut registry.addresses, pos) = addr;
                return
            };
        };
        vector::push_back(&mut registry.names, name_str);
        vector::push_back(&mut registry.addresses, addr);
    }

    /// Resolve a registered name.
    public fun lookup(registry: &Registry, name: vector<u8>): option::Option<address> {
        let name_str = string::utf8(name);
        let mut pos = vector::length(&registry.names);
        while (pos > 0) {
            pos = pos - 1;
            if (*vector::borrow(&registry.names, pos) == name_str) {
                return option::some(*vector::borrow(&registry.addresses, pos))
            };
        };
        option::none()
    }

    /// Create a shared entry and return its address.
    public fun create_entry(
        name: vector<u8>,
        value: vector<u8>,
        ctx: &mut TxContext,
    ): address {
        let entry = e2e_new_entry(name, value, ctx);
        let addr = object::uid_address(&entry.id);
        transfer::share_object(entry);
        addr
    }

    /// Owner-only payload update.
    public entry fun update_entry(
        entry: &mut Entry, value: vector<u8>, ctx: &mut TxContext
    ) {
        assert!(entry.owner == tx_context::sender(ctx), E_NOT_OWNER);
        entry.value = value;
    }

    public fun entry_value(entry: &Entry): &vector<u8> {
        &entry.value
    }

    public fun entry_name(entry: &Entry): &String {
        &entry.name
    }

    public fun registry_size(registry: &Registry): u64 {
        vector::length(&registry.names)
    }

    /// Builds an unshared registry owned by the sender.
    fun e2e_new_registry(ctx: &mut TxContext): Registry {
        Registry {
            id: object::new(ctx),
            owner: tx_context::sender(ctx),
            names: vector::empty<String>(),
            addresses: vector::empty<address>(),
        }
    }

    /// Builds an unshared entry owned by the sender.
    fun e2e_new_entry(
        name: vector<u8>, value: vector<u8>, ctx: &mut TxContext
    ): Entry {
        Entry {
            id: object::new(ctx),
            name: string::utf8(name),
            value,
            owner: tx_context::sender(ctx),
        }
    }

    /// Entry wrapper for `create_registry`.
    public entry fun e2e_create_registry(ctx: &mut TxContext) {
        create_registry(ctx);
    }

    /// Entry wrapper for `create_entry`.
    public entry fun e2e_create_entry(
        name: vector<u8>, value: vector<u8>, ctx: &mut TxContext
    ) {
        create_entry(name, value, ctx);
    }

    /// Entry wrapper asserting the registry size.
    public entry fun e2e_check_registry_size(registry: &Registry, expected: u64) {
        assert!(registry_size(registry) == expected, E_BAD_LOOKUP);
    }

    /// Entry wrapper asserting that a name resolves to an address.
    public entry fun e2e_check_lookup(
        registry: &Registry, name: vector<u8>, expected: address
    ) {
        let mut found = lookup(registry, name);
        assert!(option::is_some(&found), E_BAD_LOOKUP);
        assert!(option::extract(&mut found) == expected, E_BAD_LOOKUP);
    }

    /// Entry wrapper asserting that a name is unregistered.
    public entry fun e2e_check_lookup_absent(registry: &Registry, name: vector<u8>) {
        assert!(option::is_none(&lookup(registry, name)), E_BAD_LOOKUP);
    }

    /// Entry wrapper asserting the entry payload.
    public entry fun e2e_check_entry_value(entry: &Entry, expected: vector<u8>) {
        assert!(*entry_value(entry) == expected, E_BAD_LOOKUP);
    }

    /// Full registration and update round trip, asserted in-transaction.
    public entry fun e2e_registry_roundtrip(ctx: &mut TxContext) {
        let sender = tx_context::sender(ctx);
        let mut registry = e2e_new_registry(ctx);
        assert!(registry_size(&registry) == 0, E_BAD_LOOKUP);
        assert!(option::is_none(&lookup(&registry, b"missing")), E_BAD_LOOKUP);

        register_name(&mut registry, b"kanari", sender, ctx);
        register_name(&mut registry, b"james", @0x3, ctx);
        assert!(registry_size(&registry) == 2, E_BAD_LOOKUP);

        // Re-registration overwrites the address without growing the registry.
        register_name(&mut registry, b"kanari", @0x4, ctx);
        assert!(registry_size(&registry) == 2, E_BAD_LOOKUP);

        let mut resolved = lookup(&registry, b"kanari");
        assert!(option::is_some(&resolved), E_BAD_LOOKUP);
        assert!(option::extract(&mut resolved) == @0x4, E_BAD_LOOKUP);
        assert!(option::is_none(&lookup(&registry, b"absent")), E_BAD_LOOKUP);

        let mut entry = e2e_new_entry(b"entry", b"value1", ctx);
        assert!(*entry_value(&entry) == b"value1", E_BAD_LOOKUP);
        update_entry(&mut entry, b"value2", ctx);
        assert!(*entry_value(&entry) == b"value2", E_BAD_LOOKUP);
        assert!(*entry_name(&entry) == string::utf8(b"entry"), E_BAD_LOOKUP);

        transfer::share_object(registry);
        transfer::share_object(entry);
    }

    /// Creates a shared registry owned by the sender so foreign writes abort.
    public entry fun e2e_create_owned_registry(ctx: &mut TxContext) {
        let registry = e2e_new_registry(ctx);
        transfer::share_object(registry);
    }
}