//! `dynamic_object_field` values must survive across runtime instances backed by
//! one store. The flow itself lives in `support::dynamic_field_e2e`; this file
//! only supplies the Move module under test.

use anyhow::Result;

#[path = "support/dynamic_field_e2e.rs"]
mod dynamic_field_e2e;

use dynamic_field_e2e::{DynamicFieldScenario, run};

const TESTER_ADDR: &str = "0x43";
const MODULE_NAME: &str = "dynamic_object_field_e2e";

#[test]
fn dynamic_object_field_persists_across_runtime_instances() -> Result<()> {
    run(&DynamicFieldScenario {
        package_name: "DynamicObjectFieldE2E",
        tester_addr: TESTER_ADDR,
        module_name: MODULE_NAME,
        move_source: MOVE_SOURCE,
        add_fn: "add_child",
        write_fn: "write_child_value",
        remove_fn: "remove_child",
        read_fn: "read_child_value",
        has_fn: "has_child",
        key: 3,
        initial_value: 500,
        updated_value: 777,
    })
}

const MOVE_SOURCE: &str =
    include_str!("move_packages/dynamic_object_field_e2e/sources/dynamic_object_field_e2e.move");
