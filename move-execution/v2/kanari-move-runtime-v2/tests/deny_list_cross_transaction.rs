//! End-to-end coverage for the genesis-created shared deny-list singleton.
//!
//! `kanari_system::deny_list` reads/writes a single shared `DenyList` object at
//! `deny_list_object_id()` (`@0x403`). The runtime must materialize that object
//! during genesis; otherwise every regulated transfer path silently sees an
//! empty list. This test boots the runtime (which runs genesis), publishes a
//! thin wrapper module, then drives add / remove / pause across several fresh
//! runtime instances backed by one store to prove the mutations land on the
//! `@0x403` object and survive.
//!
//! Note: `execute_view_function` decodes a 1-byte `bool` return as a JSON
//! number (0/1), so the assertions compare against `0u64`/`1u64`.

use anyhow::{Context, Result};
use kanari_move_runtime_v2::changeset::ChangeSet;
use kanari_move_runtime_v2::move_runtime::EntryFunctionObjectContext;
use kanari_move_runtime_v2::move_runtime::MoveRuntime;
use kanari_move_runtime_v2::state::StateManager;
use kanari_move_runtime_v2::storage::persistent_store::PersistentStore;
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

#[path = "support/move_manifest.rs"]
mod move_manifest;

const TESTER_ADDR: &str = "0x43";
const MODULE_NAME: &str = "deny_list_e2e";
const DENY_LIST_ID: &str = "0x403";
const PER_TYPE_INDEX: u64 = 0;
const MOVE_SOURCE: &str = include_str!("move_packages/deny_list_e2e/sources/deny_list_e2e.move");

/// Persona denied then re-allowed; must be outside the reserved `@0x0..=0xF`.
const SUBJECT: &str = "0xB0B";

fn type_key() -> Vec<u8> {
    b"0x2::coin::COIN".to_vec()
}

#[test]
fn deny_list_singleton_persists_across_runtime_instances() -> Result<()> {
    let package_dir = create_test_package()?;
    let (module_id, module_bytes) = compile_test_module(&package_dir)?;
    let store = Arc::new(PersistentStore::open_in_memory()?);

    let mut state = StateManager::new(store.clone());
    let runtime = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;

    // Genesis is run by the runtime constructor; the deny-list singleton must
    // exist at the well-known id and be typed as the framework `DenyList`.
    let deny_obj = state
        .get_object(DENY_LIST_ID)?
        .context("deny-list singleton @0x403 missing after genesis")?;
    assert!(
        deny_obj.type_.ends_with("::deny_list::DenyList"),
        "unexpected deny-list type: {}",
        deny_obj.type_
    );
    assert!(matches!(deny_obj.owner_kind, ObjectOwnerKind::Shared));

    let _ = runtime
        .publish_module(module_bytes, *module_id.address(), None, None)
        .context("publish deny_list_e2e module")?;

    let subject = AccountAddress::from_hex_literal(SUBJECT)?;

    // --- add ------------------------------------------------------------
    let add_changes = call_entry(
        &runtime,
        &module_id,
        "add",
        vec![
            object_arg(DENY_LIST_ID)?,
            bcs::to_bytes(&PER_TYPE_INDEX)?,
            bcs::to_bytes(&type_key())?,
            bcs::to_bytes(&subject)?,
        ],
    )
    .context("deny add")?;
    state.apply_changeset(&add_changes)?;
    state.commit()?;

    // --- contains (fresh runtime) --------------------------------------
    let rt_read = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    assert_eq!(
        read_contains(&rt_read, &subject)?,
        JsonValue::from(1u64),
        "denied address should read back as contained after a fresh boot"
    );

    // --- remove ---------------------------------------------------------
    let remove_changes = call_entry(
        &rt_read,
        &module_id,
        "remove",
        vec![
            object_arg(DENY_LIST_ID)?,
            bcs::to_bytes(&PER_TYPE_INDEX)?,
            bcs::to_bytes(&type_key())?,
            bcs::to_bytes(&subject)?,
        ],
    )
    .context("deny remove")?;
    state.apply_changeset(&remove_changes)?;
    state.commit()?;

    let rt_after_remove = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    assert_eq!(
        read_contains(&rt_after_remove, &subject)?,
        JsonValue::from(0u64),
        "removed address should no longer be contained"
    );

    // --- pause ----------------------------------------------------------
    let pause_changes = call_entry(
        &rt_after_remove,
        &module_id,
        "pause",
        vec![
            object_arg(DENY_LIST_ID)?,
            bcs::to_bytes(&PER_TYPE_INDEX)?,
            bcs::to_bytes(&type_key())?,
        ],
    )
    .context("deny pause")?;
    state.apply_changeset(&pause_changes)?;
    state.commit()?;

    let rt_paused = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    assert_eq!(
        read_paused(&rt_paused)?,
        JsonValue::from(1u64),
        "paused type should read back as paused after a fresh boot"
    );

    // --- unpause --------------------------------------------------------
    let unpause_changes = call_entry(
        &rt_paused,
        &module_id,
        "unpause",
        vec![
            object_arg(DENY_LIST_ID)?,
            bcs::to_bytes(&PER_TYPE_INDEX)?,
            bcs::to_bytes(&type_key())?,
        ],
    )
    .context("deny unpause")?;
    state.apply_changeset(&unpause_changes)?;
    state.commit()?;

    let rt_unpaused = MoveRuntime::new_with_kanari_natives_and_store(store)?;
    assert_eq!(
        read_paused(&rt_unpaused)?,
        JsonValue::from(0u64),
        "unpaused type should read back as not paused"
    );

    Ok(())
}

fn call_entry(
    runtime: &MoveRuntime,
    module_id: &ModuleId,
    function: &str,
    args: Vec<Vec<u8>>,
) -> Result<ChangeSet> {
    runtime.execute_entry_function_with_object_context_and_persistence(
        module_id,
        function,
        vec![],
        args,
        EntryFunctionObjectContext {
            object_inputs: vec![shared_object_input(DENY_LIST_ID, true)],
            sender: Some(*module_id.address()),
            gas_info: None,
            timestamp: None,
            tx_hash: None,
            persist_runtime_state: true,
            state_overlay: None,
        },
    )
}

fn read_contains(runtime: &MoveRuntime, subject: &AccountAddress) -> Result<JsonValue> {
    runtime.execute_view_function(
        TESTER_ADDR,
        MODULE_NAME,
        "contains",
        &[],
        &[
            object_arg(DENY_LIST_ID)?,
            bcs::to_bytes(&PER_TYPE_INDEX)?,
            bcs::to_bytes(&type_key())?,
            bcs::to_bytes(subject)?,
        ],
        &[shared_object_input(DENY_LIST_ID, false)],
    )
}

fn read_paused(runtime: &MoveRuntime) -> Result<JsonValue> {
    runtime.execute_view_function(
        TESTER_ADDR,
        MODULE_NAME,
        "paused",
        &[],
        &[
            object_arg(DENY_LIST_ID)?,
            bcs::to_bytes(&PER_TYPE_INDEX)?,
            bcs::to_bytes(&type_key())?,
        ],
        &[shared_object_input(DENY_LIST_ID, false)],
    )
}

fn create_test_package() -> Result<PathBuf> {
    let dir = tempdir()?;
    let package_dir = dir.keep();
    fs::create_dir_all(package_dir.join("sources"))?;

    let dependency_path = move_manifest::kanari_system_package_path()?;
    let manifest = format!(
        "[package]\nname = \"DenyListE2E\"\n\n[dependencies]\nKanariSystem = {{ local = \"{}\" }}\n\n[addresses]\ntester = \"{}\"\n",
        dependency_path, TESTER_ADDR,
    );
    fs::write(package_dir.join("Move.toml"), manifest)?;

    fs::write(
        package_dir
            .join("sources")
            .join(format!("{}.move", MODULE_NAME)),
        MOVE_SOURCE,
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
    // Canonicalize to the full 32-byte address; the well-known id `0x403` is a
    // short literal and would otherwise fail raw hex-decode (odd digit count).
    Ok(AccountAddress::from_hex_literal(object_id)?.to_vec())
}

fn shared_object_input(object_id: &str, mutable: bool) -> ObjectInput {
    ObjectInput {
        object_ref: ObjectRef::new(object_id.to_string(), None, None),
        owner: Some(ObjectOwnerKind::Shared),
        mutable,
    }
}
