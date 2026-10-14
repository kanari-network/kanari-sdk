//! Child-object link index end to end: dynamic-object-field writes record
//! parent->child links, `children` reads them back within the transaction
//! (pending visibility) and across transactions (durable persistence).

use anyhow::{Context, Result};
use kanari_move_runtime_v1::changeset::ChangeSet;
use kanari_move_runtime_v1::move_runtime::{EntryFunctionObjectContext, MoveRuntime};
use kanari_move_runtime_v1::state::StateManager;
use kanari_move_runtime_v1::storage::persistent_store::PersistentStore;
use kanari_types::transaction::{ObjectInput, ObjectOwnerKind, ObjectRef};
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::ModuleId;
use move_package::BuildConfig;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::tempdir;

#[path = "support/move_manifest.rs"]
mod move_manifest;

const TESTER_ADDR: &str = "0x42";

const MOVE_SOURCE: &str = include_str!("move_packages/child_index/sources/child_index.move");

#[test]
fn child_links_survive_across_transactions() -> Result<()> {
    let package_dir = create_test_package()?;
    let (module_id, module_bytes) = compile_test_module(&package_dir)?;
    let sender = *module_id.address();
    assert_eq!(sender, AccountAddress::from_hex_literal(TESTER_ADDR)?);

    let store = Arc::new(PersistentStore::open_in_memory()?);
    let mut nonce = Nonce::default();
    let runtime = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    let mut state = StateManager::new(store.clone());

    let _ = runtime
        .publish_module(module_bytes, sender, None, None)
        .context("publish child_index module")?;

    // One transaction adds two children, asserts pending visibility,
    // removes one, and re-checks (all inside Move asserts).
    let cycle = entry(
        &runtime,
        &mut state,
        &module_id,
        "full_cycle",
        vec![],
        vec![],
        &mut nonce,
    )?;
    let host = find_created(&cycle, |t| t.contains("child_index::Host"))?;

    // A later transaction observes exactly the surviving child.
    let check = entry(
        &runtime,
        &mut state,
        &module_id,
        "check_persisted",
        vec![object_arg(&host.id)?],
        vec![],
        &mut nonce,
    )?;
    drop(check);

    // Deleting the parent purges its link list (children themselves are
    // untouched — only the index entry goes).
    let sender = *module_id.address();
    let _ = entry(
        &runtime,
        &mut state,
        &module_id,
        "delete_host",
        vec![object_arg(&host.id)?],
        vec![owned_input(&host.id, sender, true)],
        &mut nonce,
    )?;
    let mut link_key = b"children:".to_vec();
    link_key.extend_from_slice(host.id.as_bytes());
    let leftover: Option<Vec<(Vec<u8>, Vec<u8>)>> = store.load(&link_key)?;
    assert!(
        leftover.is_none(),
        "parent delete must purge its child-index entry"
    );
    Ok(())
}

fn entry(
    runtime: &MoveRuntime,
    state: &mut StateManager,
    module_id: &ModuleId,
    function: &str,
    args: Vec<Vec<u8>>,
    object_inputs: Vec<ObjectInput>,
    nonce: &mut Nonce,
) -> Result<ChangeSet> {
    let cs = runtime.execute_entry_function_with_object_context_and_persistence(
        module_id,
        function,
        vec![],
        args,
        EntryFunctionObjectContext {
            object_inputs,
            sender: Some(*module_id.address()),
            gas_info: None,
            timestamp: None,
            tx_hash: Some(nonce.next_bytes()),
            persist_runtime_state: true,
            state_overlay: None,
        },
    )?;
    state.apply_changeset(&cs)?;
    state.commit()?;
    Ok(cs)
}

struct Created {
    id: String,
}

fn find_created(cs: &ChangeSet, pred: impl Fn(&str) -> bool) -> Result<Created> {
    cs.created_objects
        .iter()
        .find(|(_, created)| pred(&created.type_))
        .map(|(id, _)| Created { id: id.clone() })
        .context("expected created object not found")
}

fn object_arg(object_id: &str) -> Result<Vec<u8>> {
    let clean = object_id.strip_prefix("0x").unwrap_or(object_id);
    hex::decode(clean).context("decode object id argument")
}

fn owned_input(id: &str, owner: AccountAddress, mutable: bool) -> ObjectInput {
    ObjectInput {
        object_ref: ObjectRef::new(id.to_string(), None, None),
        owner: Some(ObjectOwnerKind::AddressOwner(owner.to_hex_literal())),
        mutable,
    }
}

#[derive(Default)]
struct Nonce(u64);

impl Nonce {
    fn next_bytes(&mut self) -> Vec<u8> {
        self.0 += 1;
        self.0.to_le_bytes().to_vec()
    }
}

fn create_test_package() -> Result<PathBuf> {
    let dir = tempdir()?;
    let package_dir = dir.keep();
    fs::create_dir_all(package_dir.join("sources"))?;

    let dependency_path = move_manifest::kanari_system_package_path()?;
    let manifest = format!(
        "[package]\nname = \"ChildIndex\"\n\n[dependencies]\nKanariSystem = {{ local = \"{}\" }}\n\n[addresses]\ntester = \"{}\"\n",
        dependency_path, TESTER_ADDR,
    );
    fs::write(package_dir.join("Move.toml"), manifest)?;
    fs::write(
        package_dir.join("sources").join("child_index.move"),
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
