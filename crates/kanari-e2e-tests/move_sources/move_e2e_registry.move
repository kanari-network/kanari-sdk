module kanari_e2e_tests::move_e2e_registry {
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};
    use std::string::{Self, String};
    use std::vector;

    struct Registry has key {
        id: UID,
        owner: address,
        names: vector<String>,
        addresses: vector<address>,
    }

    struct Entry has key, store {
        id: UID,
        name: String,
        value: vector<u8>,
        owner: address,
    }

    public fun create_registry(ctx: &mut TxContext): address {
        let reg = Registry {
            id: object::new(ctx),
            owner: tx_context::sender(ctx),
            names: vector::empty<String>(),
            addresses: vector::empty<address>(),
        };
        let addr = object::uid_to_address(&reg.id);
        transfer::share_object(reg);
        addr
    }

    public fun register_name(reg: &mut Registry, name: vector<u8>, addr: address, ctx: &mut TxContext) {
        assert!(reg.owner == tx_context::sender(ctx), 700);
        let name_str = string::utf8(name);
        let mut i = 0;
        let len = vector::length(&reg.names);
        while (i < len) {
            if (*vector::borrow(&reg.names, i) == name_str) {
                *vector::borrow_mut(&mut reg.addresses, i) = addr;
                return
            };
            i = i + 1;
        };
        vector::push_back(&mut reg.names, name_str);
        vector::push_back(&mut reg.addresses, addr);
    }

    public fun lookup(reg: &Registry, name: vector<u8>): option::Option<address> {
        let name_str = string::utf8(name);
        let mut i = 0;
        let len = vector::length(&reg.names);
        while (i < len) {
            if (*vector::borrow(&reg.names, i) == name_str) {
                return option::some(*vector::borrow(&reg.addresses, i))
            };
            i = i + 1;
        };
        option::none()
    }

    public fun create_entry(name: vector<u8>, value: vector<u8>, ctx: &mut TxContext): address {
        let entry = Entry {
            id: object::new(ctx),
            name: string::utf8(name),
            value,
            owner: tx_context::sender(ctx),
        };
        let addr = object::uid_to_address(&entry.id);
        transfer::public_share_object(entry);
        addr
    }

    public fun update_entry(entry: &mut Entry, value: vector<u8>, ctx: &mut TxContext) {
        assert!(entry.owner == tx_context::sender(ctx), 701);
        entry.value = value;
    }

    public fun get_entry_value(entry: &Entry): &vector<u8> {
        &entry.value
    }
}
