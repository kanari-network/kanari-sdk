module tester::dynamic_field_e2e {
    use kanari_system::dynamic_field;
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    struct Host has key, store {
        id: UID
    }

    public entry fun create_host(ctx: &mut TxContext) {
        let host = Host { id: object::new(ctx) };
        transfer::public_transfer(host, tx_context::sender(ctx));
    }

    public entry fun add_value(host: &mut Host, key: u64, value: u64) {
        dynamic_field::add<u64, u64>(&mut host.id, key, value);
        object::save_object(host);
    }

    public entry fun write_value(host: &mut Host, key: u64, value: u64) {
        *dynamic_field::borrow_mut<u64, u64>(&mut host.id, key) = value;
        object::save_object(host);
    }

    public entry fun remove_value(host: &mut Host, key: u64) {
        let _ = dynamic_field::remove<u64, u64>(&mut host.id, key);
        object::save_object(host);
    }

    public fun read_value(host: &Host, key: u64): u64 {
        *dynamic_field::borrow<u64, u64>(&host.id, key)
    }

    public fun has_value(host: &Host, key: u64): bool {
        dynamic_field::exists_<u64>(&host.id, key)
    }
}

