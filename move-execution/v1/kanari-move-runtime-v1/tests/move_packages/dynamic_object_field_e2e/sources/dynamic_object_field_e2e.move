module tester::dynamic_object_field_e2e {
    use kanari_system::dynamic_object_field;
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    struct Host has key, store {
        id: UID
    }

    struct Child has key, store {
        id: UID,
        value: u64
    }

    fun new_child(ctx: &mut TxContext, value: u64): Child {
        Child { id: object::new(ctx), value }
    }

    public entry fun create_host(ctx: &mut TxContext) {
        let host = Host { id: object::new(ctx) };
        transfer::public_transfer(host, tx_context::sender(ctx));
    }

    public entry fun add_child(
        host: &mut Host, key: u64, value: u64, ctx: &mut TxContext
    ) {
        let child = new_child(ctx, value);
        dynamic_object_field::add<u64, Child>(&mut host.id, key, child);
        object::save_object(host);
    }

    public entry fun write_child_value(host: &mut Host, key: u64, value: u64) {
        dynamic_object_field::borrow_mut<u64, Child>(&mut host.id, key).value = value;
        object::save_object(host);
    }

    public entry fun remove_child(host: &mut Host, key: u64) {
        let child = dynamic_object_field::remove<u64, Child>(&mut host.id, key);
        let Child { id, value: _ } = child;
        object::delete(id);
        object::save_object(host);
    }

    public fun read_child_value(host: &Host, key: u64): u64 {
        dynamic_object_field::borrow<u64, Child>(&host.id, key).value
    }

    public fun has_child(host: &Host, key: u64): bool {
        dynamic_object_field::exists_<u64>(&host.id, key)
    }
}

