//! End-to-end coverage for `kanari_system::types` witness claims.
//!
//! `types::is_one_time_witness` is a pure convention check, but
//! `types::claim_witness` has host-side effects: the runtime persists each
//! claim and must abort a second claim for the same type *even across
//! transactions* (this is what gives each coin type a single canonical
//! currency). Only a runtime-level test can observe that persistence, so this
//! test boots the runtime, claims through a thin wrapper module, then
//! re-claims from fresh runtime instances backed by one store.
//!
//! Note: `execute_view_function` decodes a 1-byte `bool` return as a JSON
//! number (0/1), so the convention assertions compare against `0u64`/`1u64`.

use anyhow::{Context, Result};
use kanari_move_runtime_v2::changeset::ChangeSet;
use kanari_move_runtime_v2::move_runtime::EntryFunctionObjectContext;
use kanari_move_runtime_v2::move_runtime::MoveRuntime;
use kanari_move_runtime_v2::state::StateManager;
use kanari_move_runtime_v2::storage::persistent_store::PersistentStore;
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

const TESTER_ADDR: &str = "0x54";
const MODULE_NAME: &str = "types_e2e";
const MOVE_SOURCE: &str = include_str!("move_packages/types_e2e/sources/types_e2e.move");

#[test]
fn witness_claims_persist_across_runtime_instances() -> Result<()> {
    let package_dir = create_test_package()?;
    let (module_id, module_bytes) = compile_test_module(&package_dir)?;
    let store = Arc::new(PersistentStore::open_in_memory()?);

    let mut state = StateManager::new(store.clone());
    let runtime = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    let _ = runtime
        .publish_module(module_bytes, *module_id.address(), None, None)
        .context("publish types_e2e module")?;

    // --- convention checks (stateless, but must work through the runtime) ---
    assert_eq!(
        view(&runtime, "is_otw")?,
        JsonValue::from(1u64),
        "module-named witness should satisfy the OTW convention"
    );
    assert_eq!(
        view(&runtime, "is_not_otw")?,
        JsonValue::from(0u64),
        "misnamed struct should fail the OTW convention"
    );

    // --- same-transaction double claim aborts via the pending set ----------
    let twice_err = call_entry(&runtime, &module_id, "claim_twice")
        .err()
        .context("claiming twice in one transaction should abort")?;
    assert!(
        twice_err.to_string().contains("already claimed"),
        "unexpected abort for same-tx double claim: {twice_err}"
    );

    // --- first claim succeeds ------------------------------------------------
    let claim_changes = call_entry(&runtime, &module_id, "claim_otw").context("claim_otw")?;
    state.apply_changeset(&claim_changes)?;
    state.commit()?;

    // --- second claim from a fresh runtime aborts via durable state ---------
    let rt_claimed = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    let reclaim_err = call_entry(&rt_claimed, &module_id, "claim_otw")
        .err()
        .context("re-claiming after a fresh boot should abort")?;
    assert!(
        reclaim_err.to_string().contains("already claimed"),
        "unexpected abort for cross-tx double claim: {reclaim_err}"
    );

    // --- an unrelated type is still claimable --------------------------------
    let other_changes =
        call_entry(&rt_claimed, &module_id, "claim_other").context("claim_other")?;
    state.apply_changeset(&other_changes)?;
    state.commit()?;

    // --- ... and stays claimed across runtimes too ---------------------------
    let rt_other = MoveRuntime::new_with_kanari_natives_and_store(store)?;
    let reother_err = call_entry(&rt_other, &module_id, "claim_other")
        .err()
        .context("re-claiming OTHER after a fresh boot should abort")?;
    assert!(
        reother_err.to_string().contains("already claimed"),
        "unexpected abort for cross-tx OTHER double claim: {reother_err}"
    );

    Ok(())
}

/// Invoke a no-arg entry function with no object inputs.
fn call_entry(runtime: &MoveRuntime, module_id: &ModuleId, function: &str) -> Result<ChangeSet> {
    runtime.execute_entry_function_with_object_context_and_persistence(
        module_id,
        function,
        vec![],
        vec![],
        EntryFunctionObjectContext {
            object_inputs: vec![],
            sender: Some(*module_id.address()),
            gas_info: None,
            timestamp: None,
            tx_hash: Some({
                use std::sync::atomic::{AtomicU64, Ordering};
                static C: AtomicU64 = AtomicU64::new(1);
                C.fetch_add(1, Ordering::Relaxed).to_le_bytes().to_vec()
            }),
            persist_runtime_state: true,
            state_overlay: None,
        },
    )
}

fn view(runtime: &MoveRuntime, function: &str) -> Result<JsonValue> {
    runtime.execute_view_function(TESTER_ADDR, MODULE_NAME, function, &[], &[], &[])
}

fn create_test_package() -> Result<PathBuf> {
    let dir = tempdir()?;
    let package_dir = dir.keep();
    fs::create_dir_all(package_dir.join("sources"))?;

    let dependency_path = move_manifest::kanari_system_package_path()?;
    let manifest = format!(
        "[package]\nname = \"TypesE2E\"\n\n[dependencies]\nKanariSystem = {{ local = \"{}\" }}\n\n[addresses]\ntester = \"{}\"\n",
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
