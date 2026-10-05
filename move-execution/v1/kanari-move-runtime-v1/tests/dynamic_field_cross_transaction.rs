//! `dynamic_field` values must survive across runtime instances backed by one
//! store. The flow itself lives in `support::dynamic_field_e2e`; this file only
//! supplies the Move module under test.

use anyhow::Result;

#[path = "support/dynamic_field_e2e.rs"]
mod dynamic_field_e2e;

use dynamic_field_e2e::{DynamicFieldScenario, run};

const TESTER_ADDR: &str = "0x42";
const MODULE_NAME: &str = "dynamic_field_e2e";

#[test]
fn dynamic_field_persists_across_runtime_instances() -> Result<()> {
    run(&DynamicFieldScenario {
        package_name: "DynamicFieldE2E",
        tester_addr: TESTER_ADDR,
        module_name: MODULE_NAME,
        move_source: MOVE_SOURCE,
        add_fn: "add_value",
        write_fn: "write_value",
        remove_fn: "remove_value",
        read_fn: "read_value",
        has_fn: "has_value",
        key: 7,
        initial_value: 41,
        updated_value: 99,
    })
}

const MOVE_SOURCE: &str = r#"
module tester::{module_name} {
    use kanari_system::dynamic_field;
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    struct Host has key, store {
        id: UID,
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
"#;
