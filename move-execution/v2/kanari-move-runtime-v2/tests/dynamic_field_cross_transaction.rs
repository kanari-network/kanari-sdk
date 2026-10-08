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

const MOVE_SOURCE: &str =
    include_str!("move_packages/dynamic_field_e2e/sources/dynamic_field_e2e.move");
