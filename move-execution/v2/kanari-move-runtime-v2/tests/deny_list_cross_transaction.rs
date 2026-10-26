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
const DENY_LIST_ID: &str = kanari_types::deny_list::DenyListModule::DENY_LIST_OBJECT_ID;
const MOVE_SOURCE: &str = include_str!("move_packages/deny_list_e2e/sources/deny_list_e2e.move");

/// Persona denied then re-allowed; must be outside the reserved `@0x0..=0xF`.
const SUBJECT: &str = "0xB0B";

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

    // `deny_list`'s mutators are `public(friend)` — only `kanari_system::coin`
    // may call them — so every write below must present a `DenyCap<TEST_COIN>`
    // and goes through the `coin` wrappers. Reads stay `public`.
    let create_cap_changes = call_entry_no_objects(&runtime, &module_id, "create_cap", vec![])
        .context("create legacy deny cap")?;
    let (cap_id, cap_object) = create_cap_changes
        .created_objects
        .iter()
        .find(|(_, object)| {
            object.type_.contains("::deny_list::DenyCap<")
                && object.type_.contains("::deny_list_e2e::TEST_COIN")
        })
        .context("legacy deny cap should be created")?;
    let cap_id = cap_id.clone();
    let cap_ref = cap_object.object_ref(&cap_id);
    let cap_owner = cap_object.owner.to_hex_literal();
    state.apply_changeset(&create_cap_changes)?;
    state.commit()?;

    // The cap is only ever borrowed (`&DenyCap<T>`), so its version/digest stay
    // put across every call below and this input can be reused verbatim.
    let cap_input = || ObjectInput {
        object_ref: cap_ref.clone(),
        owner: Some(ObjectOwnerKind::AddressOwner(cap_owner.clone())),
        mutable: true,
    };

    let subject = AccountAddress::from_hex_literal(SUBJECT)?;

    // --- add ------------------------------------------------------------
    let add_changes = call_entry(
        &runtime,
        &module_id,
        "add",
        vec![
            object_arg(DENY_LIST_ID)?,
            object_arg(&cap_id)?,
            bcs::to_bytes(&subject)?,
        ],
        vec![cap_input()],
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

    // --- singleton auto-borrow (no declared object input) -----------------
    // The runtime preloads `@0x403` for every execution, so framework code
    // borrowing the singleton must observe the same state.
    assert_eq!(
        view_auto(&rt_read, "contains_auto", vec![bcs::to_bytes(&subject)?],)?,
        JsonValue::from(1u64),
        "singleton borrow should see the denied address without an object input"
    );
    call_entry_no_objects(
        &rt_read,
        &module_id,
        "assert_not_denied",
        vec![bcs::to_bytes(&subject)?],
    )
    .err()
    .context("singleton entry guard should abort for a denied address")?;

    // --- remove ---------------------------------------------------------
    let remove_changes = call_entry(
        &rt_read,
        &module_id,
        "remove",
        vec![
            object_arg(DENY_LIST_ID)?,
            object_arg(&cap_id)?,
            bcs::to_bytes(&subject)?,
        ],
        vec![cap_input()],
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
    assert_eq!(
        view_auto(
            &rt_after_remove,
            "contains_auto",
            vec![bcs::to_bytes(&subject)?],
        )?,
        JsonValue::from(0u64),
        "singleton borrow should see the removal without an object input"
    );
    let guard_changes = call_entry_no_objects(
        &rt_after_remove,
        &module_id,
        "assert_not_denied",
        vec![bcs::to_bytes(&subject)?],
    )
    .context("singleton entry guard should pass for a removed address")?;
    state.apply_changeset(&guard_changes)?;
    state.commit()?;

    // --- pause ----------------------------------------------------------
    let pause_changes = call_entry(
        &rt_after_remove,
        &module_id,
        "pause",
        vec![object_arg(DENY_LIST_ID)?, object_arg(&cap_id)?],
        vec![cap_input()],
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
    assert_eq!(
        view_auto(&rt_paused, "paused_auto", vec![])?,
        JsonValue::from(1u64),
        "singleton borrow should see the pause without an object input"
    );

    // --- unpause --------------------------------------------------------
    let unpause_changes = call_entry(
        &rt_paused,
        &module_id,
        "unpause",
        vec![object_arg(DENY_LIST_ID)?, object_arg(&cap_id)?],
        vec![cap_input()],
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
    assert_eq!(
        view_auto(&rt_unpaused, "paused_auto", vec![])?,
        JsonValue::from(0u64),
        "singleton borrow should see the unpause without an object input"
    );

    Ok(())
}

/// Security regression guard: `deny_list`'s mutators are `public(friend)`
/// with `kanari_system::coin` as the sole friend, so an outside package
/// cannot deny-list an arbitrary `(per_type_index, type_bytes)` pair without
/// presenting a `DenyCap<T>`. The `deny_list_friend_violation` package calls
/// `v1_add`/`v1_remove`/`v2_enable_global_pause`/`v2_disable_global_pause`
/// directly, so it must fail to compile.
#[test]
fn deny_list_friend_locked_is_rejected() -> Result<()> {
    assert_fixture_does_not_compile(
        "deny_list_friend_violation",
        "deny_list_friend_locked_is_rejected",
        "outside package compiled calls to deny_list mutators; the \
         `friend kanari_system::coin` declaration on kanari_system::deny_list \
         was lost, so any published package can bypass the DenyCap check",
    )
}

/// Security regression guard for the other half of the gate: every write path
/// takes `&DenyCap<T>`, so `deny_list::new_denycap` must be `public(friend)`.
/// If it were `public`, `deny_list_cap_forgery` could mint its own cap and
/// freeze arbitrary addresses for arbitrary coin types (e.g. KANARI) without
/// ever being the currency issuer.
#[test]
fn deny_list_denycap_minting_is_locked() -> Result<()> {
    assert_fixture_does_not_compile(
        "deny_list_cap_forgery",
        "deny_list_denycap_minting_is_locked",
        "outside package compiled a call to deny_list::new_denycap; cap \
         minting is public again, so any package can forge a DenyCap<T> and \
         deny-list arbitrary addresses for arbitrary coin types",
    )
}

/// Assert a checked-in negative fixture fails to compile.
///
/// `move_compiler::diagnostics::report_diagnostics` reports and then calls
/// `std::process::exit(1)`, so an in-process compile attempt would kill the
/// whole test binary. The compile therefore runs in a child process (this same
/// test binary re-executed with a sentinel argument) and is judged by its exit
/// status.
///
/// The sentinel must not start with `--`: libtest rejects unknown leading
/// options, which would make every child exit non-zero and let a leaked
/// fixture pass unnoticed. `is_child` is therefore also asserted in the
/// parent, so a broken child invocation fails loudly instead of silently
/// reporting success.
fn assert_fixture_does_not_compile(
    package: &str,
    test_name: &str,
    failure_message: &str,
) -> Result<()> {
    let sentinel = format!("{package}-compile-child");
    let package_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("move_packages")
        .join(package);

    if is_child(&sentinel) {
        // Child: exit 0 only if the fixture compiles (which means it leaked).
        let compiled = BuildConfig::default()
            .compile_package(&package_dir, &mut Vec::new())
            .is_ok();
        std::process::exit(i32::from(!compiled));
    }

    let status = std::process::Command::new(std::env::current_exe()?)
        .arg(sentinel)
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .output()?;

    // Guard against a child that failed for an unrelated reason (bad args,
    // harness rejection, panic): anything other than a clean compile-failure
    // must not be reported as "the fixture was rejected".
    let stderr = String::from_utf8_lossy(&status.stderr);
    assert!(
        stderr.contains("restricted visibility") || stderr.contains("E04001"),
        "child for fixture {package} did not report a visibility rejection; \
         the negative test is not exercising what it claims. stderr:\n{stderr}"
    );
    assert!(
        !status.status.success(),
        "{failure_message} (fixture: {package})"
    );
    Ok(())
}

fn is_child(sentinel: &str) -> bool {
    std::env::args().any(|arg| arg == sentinel)
}

/// Call a deny-list mutator entry function. `deny_list`'s raw mutators are
/// friend-only, so the shared list plus the `DenyCap` are both supplied.
fn call_entry(
    runtime: &MoveRuntime,
    module_id: &ModuleId,
    function: &str,
    args: Vec<Vec<u8>>,
    extra_inputs: Vec<ObjectInput>,
) -> Result<ChangeSet> {
    let mut object_inputs = vec![shared_object_input(DENY_LIST_ID, true)];
    object_inputs.extend(extra_inputs);
    runtime.execute_entry_function_with_object_context_and_persistence(
        module_id,
        function,
        vec![],
        args,
        EntryFunctionObjectContext {
            object_inputs,
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
        &[object_arg(DENY_LIST_ID)?, bcs::to_bytes(subject)?],
        &[shared_object_input(DENY_LIST_ID, false)],
    )
}

fn read_paused(runtime: &MoveRuntime) -> Result<JsonValue> {
    runtime.execute_view_function(
        TESTER_ADDR,
        MODULE_NAME,
        "paused",
        &[],
        &[object_arg(DENY_LIST_ID)?],
        &[shared_object_input(DENY_LIST_ID, false)],
    )
}

/// Call a singleton-borrowing entry function with no declared object inputs.
fn call_entry_no_objects(
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
            object_inputs: vec![],
            sender: Some(*module_id.address()),
            gas_info: None,
            timestamp: None,
            tx_hash: None,
            persist_runtime_state: true,
            state_overlay: None,
        },
    )
}

/// Call a singleton-borrowing view function with no declared object inputs.
fn view_auto(runtime: &MoveRuntime, function: &str, args: Vec<Vec<u8>>) -> Result<JsonValue> {
    runtime.execute_view_function(TESTER_ADDR, MODULE_NAME, function, &[], &args, &[])
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
