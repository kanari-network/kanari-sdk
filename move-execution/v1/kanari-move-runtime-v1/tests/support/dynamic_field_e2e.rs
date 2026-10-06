//! Shared harness for the dynamic-field / dynamic-object-field persistence tests.
//!
//! Both variants assert the same thing — that a field written by one runtime
//! instance is still readable from a freshly constructed one, survives a
//! mutation, and disappears after removal. Only the Move module under test, its
//! entry/view function names, and the sample values differ, so those are the
//! only knobs [`DynamicFieldScenario`] takes.

use anyhow::{Context, Result};
use kanari_move_runtime_v1::move_runtime::EntryFunctionObjectContext;
use kanari_move_runtime_v1::move_runtime::MoveRuntime;
use kanari_move_runtime_v1::state::StateManager;
use kanari_move_runtime_v1::storage::persistent_store::PersistentStore;
use kanari_types::transaction::{ObjectInput, ObjectOwnerKind, ObjectRef};
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::ModuleId;
use move_package::BuildConfig;
use serde_json::Value as JsonValue;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::tempdir;

#[path = "move_manifest.rs"]
mod move_manifest;

/// Entry point shared by both Move modules under test.
const CREATE_HOST_FN: &str = "create_host";

/// A dynamic-field persistence scenario: one Move module plus the values the
/// harness drives through it.
pub struct DynamicFieldScenario {
    /// Move package name, used only in the generated `Move.toml`.
    pub package_name: &'static str,
    /// `tester` address the generated package publishes under.
    pub tester_addr: &'static str,
    /// Move module name; also the `Host` struct's type suffix.
    pub module_name: &'static str,
    /// Module source. `{module_name}` is substituted before writing.
    pub move_source: &'static str,
    /// Entry function adding a field.
    pub add_fn: &'static str,
    /// Entry function mutating an existing field.
    pub write_fn: &'static str,
    /// Entry function removing a field.
    pub remove_fn: &'static str,
    /// View function reading a field's value.
    pub read_fn: &'static str,
    /// View function reporting whether a field exists.
    pub has_fn: &'static str,
    /// Field key driven through every call.
    pub key: u64,
    /// Value written by `add_fn` and asserted by the first read.
    pub initial_value: u64,
    /// Value written by `write_fn` and asserted by the second read.
    pub updated_value: u64,
}

/// Compile `scenario`, publish it, then add / read / mutate / read / remove /
/// exists across four separate runtime instances to prove the field is
/// persisted in the shared store rather than an in-process cache.
pub fn run(scenario: &DynamicFieldScenario) -> Result<()> {
    let package_dir = create_test_package(scenario)?;
    let (module_id, module_bytes) = compile_test_module(&package_dir)?;
    let store = Arc::new(PersistentStore::open_in_memory()?);

    let mut state = StateManager::new(store.clone());
    let runtime = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    let _ = runtime
        .publish_module(module_bytes, *module_id.address(), None, None)
        .with_context(|| format!("publish {} module", scenario.module_name))?;

    let create_host = runtime
        .execute_entry_function(
            &module_id,
            CREATE_HOST_FN,
            vec![],
            vec![],
            Some(*module_id.address()),
            None,
            None,
        )
        .context("create host object")?;

    let host_type_suffix = format!("::{}::Host", scenario.module_name);
    let host_id = create_host
        .created_objects
        .iter()
        .find(|(_, created)| created.type_.ends_with(&host_type_suffix))
        .map(|(id, _)| id.clone())
        .context("host object should be created")?;

    let add_changes = call_with_host(
        &runtime,
        &module_id,
        scenario.add_fn,
        &host_id,
        vec![
            object_arg(&host_id)?,
            bcs::to_bytes(&scenario.key)?,
            bcs::to_bytes(&scenario.initial_value)?,
        ],
    )
    .context("add field")?;
    assert_eq!(add_changes.added_dynamic_fields.len(), 1);
    assert!(add_changes.removed_dynamic_fields.is_empty());
    state.apply_changeset(&add_changes)?;
    state.commit()?;

    let second_runtime = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    let read_before_update = read_field(&second_runtime, scenario, &module_id, &host_id)
        .context("read persisted field value")?;
    assert_eq!(read_before_update, JsonValue::from(scenario.initial_value));

    let update_changes = call_with_host(
        &second_runtime,
        &module_id,
        scenario.write_fn,
        &host_id,
        vec![
            object_arg(&host_id)?,
            bcs::to_bytes(&scenario.key)?,
            bcs::to_bytes(&scenario.updated_value)?,
        ],
    )
    .context("mutate persisted field value")?;
    assert_eq!(update_changes.added_dynamic_fields.len(), 1);
    assert!(update_changes.removed_dynamic_fields.is_empty());
    state.apply_changeset(&update_changes)?;
    state.commit()?;

    let third_runtime = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    let read_after_update = read_field(&third_runtime, scenario, &module_id, &host_id)
        .context("read updated field value")?;
    assert_eq!(read_after_update, JsonValue::from(scenario.updated_value));

    let remove_changes = call_with_host(
        &third_runtime,
        &module_id,
        scenario.remove_fn,
        &host_id,
        vec![object_arg(&host_id)?, bcs::to_bytes(&scenario.key)?],
    )
    .context("remove persisted field")?;
    assert!(remove_changes.added_dynamic_fields.is_empty());
    assert_eq!(remove_changes.removed_dynamic_fields.len(), 1);
    state.apply_changeset(&remove_changes)?;
    state.commit()?;

    let fourth_runtime = MoveRuntime::new_with_kanari_natives_and_store(store)?;
    let exists_after_remove = view(
        &fourth_runtime,
        scenario,
        &module_id,
        &host_id,
        scenario.has_fn,
    )
    .context("check field absence after remove")?;
    assert_eq!(exists_after_remove, JsonValue::from(0u64));

    Ok(())
}

/// Invoke an entry function against the host object as a mutable declared input.
fn call_with_host(
    runtime: &MoveRuntime,
    module_id: &ModuleId,
    function: &str,
    host_id: &str,
    args: Vec<Vec<u8>>,
) -> Result<kanari_move_runtime_v1::changeset::ChangeSet> {
    runtime.execute_entry_function_with_object_context_and_persistence(
        module_id,
        function,
        vec![],
        args,
        EntryFunctionObjectContext {
            object_inputs: vec![host_object_input(host_id, *module_id.address(), true)],
            sender: Some(*module_id.address()),
            gas_info: None,
            timestamp: None,
            tx_hash: None,
            persist_runtime_state: true,
            state_overlay: None,
        },
    )
}

/// Call a view function on the host object as an immutable declared input.
fn view(
    runtime: &MoveRuntime,
    scenario: &DynamicFieldScenario,
    module_id: &ModuleId,
    host_id: &str,
    function: &str,
) -> Result<JsonValue> {
    runtime.execute_view_function(
        scenario.tester_addr,
        scenario.module_name,
        function,
        &[],
        &[object_arg(host_id)?, bcs::to_bytes(&scenario.key)?],
        &[host_object_input(host_id, *module_id.address(), false)],
    )
}

fn read_field(
    runtime: &MoveRuntime,
    scenario: &DynamicFieldScenario,
    module_id: &ModuleId,
    host_id: &str,
) -> Result<JsonValue> {
    view(runtime, scenario, module_id, host_id, scenario.read_fn)
}

fn create_test_package(scenario: &DynamicFieldScenario) -> Result<PathBuf> {
    let dir = tempdir()?;
    let package_dir = dir.keep();
    fs::create_dir_all(package_dir.join("sources"))?;

    let dependency_path = move_manifest::kanari_system_package_path()?;
    let manifest = format!(
        "[package]\nname = \"{}\"\n\n[dependencies]\nKanariSystem = {{ local = \"{}\" }}\n\n[addresses]\ntester = \"{}\"\n",
        scenario.package_name, dependency_path, scenario.tester_addr,
    );
    fs::write(package_dir.join("Move.toml"), manifest)?;

    let source = scenario
        .move_source
        .replace("{module_name}", scenario.module_name);
    fs::write(
        package_dir
            .join("sources")
            .join(format!("{}.move", scenario.module_name)),
        source,
    )?;

    Ok(package_dir)
}

fn compile_test_module(package_dir: &Path) -> Result<(ModuleId, Vec<u8>)> {
    let package = BuildConfig::default().compile_package(package_dir, &mut Vec::new())?;
    let unit = package
        .root_modules()
        .next()
        .context("compiled package should contain a root module")?;
    let module = &unit.unit;
    let module_id = ModuleId::new(
        AccountAddress::new(module.address.into_bytes()),
        Identifier::new(module.name.to_string())?,
    );
    Ok((module_id, module.serialize(None)))
}

fn object_arg(object_id: &str) -> Result<Vec<u8>> {
    let clean = object_id.strip_prefix("0x").unwrap_or(object_id);
    hex::decode(clean).context("decode object id argument")
}

fn host_object_input(host_id: &str, owner: AccountAddress, mutable: bool) -> ObjectInput {
    ObjectInput {
        object_ref: ObjectRef::new(host_id.to_string(), None, None),
        owner: Some(ObjectOwnerKind::AddressOwner(owner.to_hex_literal())),
        mutable,
    }
}
