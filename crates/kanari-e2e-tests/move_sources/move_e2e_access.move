module kanari_e2e_tests::move_e2e_access {
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    struct AccessControl has key {
        id: UID,
        owner: address,
        admin_list: vector<address>,
        is_public: bool,
    }

    struct Resource has key, store {
        id: UID,
        value: u64,
        metadata: vector<u8>,
    }

    public fun create_access_control(is_public: bool, ctx: &mut TxContext): address {
        let control = AccessControl {
            id: object::new(ctx),
            owner: tx_context::sender(ctx),
            admin_list: vector::empty<address>(),
            is_public,
        };
        let addr = object::uid_to_address(&control.id);
        transfer::share_object(control);
        addr
    }

    public fun add_admin(control: &mut AccessControl, admin: address, ctx: &mut TxContext) {
        assert!(control.owner == tx_context::sender(ctx), 200);
        vector::push_back(&mut control.admin_list, admin);
    }

    public fun remove_admin(control: &mut AccessControl, admin: address, ctx: &mut TxContext) {
        assert!(control.owner == tx_context::sender(ctx), 201);
        let mut i = 0;
        let len = vector::length(&control.admin_list);
        while (i < len) {
            if (*vector::borrow(&control.admin_list, i) == admin) {
                vector::remove(&mut control.admin_list, i);
                return
            };
            i = i + 1;
        };
    }

    public fun is_admin(control: &AccessControl, addr: address): bool {
        let mut i = 0;
        let len = vector::length(&control.admin_list);
        while (i < len) {
            if (*vector::borrow(&control.admin_list, i) == addr) {
                return true
            };
            i = i + 1;
        };
        false
    }

    public fun create_resource(value: u64, metadata: vector<u8>, ctx: &mut TxContext): address {
        let res = Resource {
            id: object::new(ctx),
            value,
            metadata,
        };
        let addr = object::uid_to_address(&res.id);
        transfer::public_share_object(res);
        addr
    }

    public fun update_resource(res: &mut Resource, new_value: u64, new_metadata: vector<u8>) {
        res.value = new_value;
        res.metadata = new_metadata;
    }

    public fun get_resource_value(res: &Resource): u64 {
        res.value
    }

    public fun get_resource_metadata(res: &Resource): &vector<u8> {
        &res.metadata
    }
}
