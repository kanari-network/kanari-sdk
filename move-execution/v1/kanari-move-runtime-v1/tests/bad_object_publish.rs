//! Publishing a module whose `key` struct lacks the required leading
//! `id: UID` must fail object-model verification (Sui parity rule).

use anyhow::{Context, Result};
use kanari_move_runtime_v1::move_runtime::MoveRuntime;
use kanari_move_runtime_v1::storage::persistent_store::PersistentStore;
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
const MOVE_SOURCE: &str = include_str!("move_packages/bad_object/sources/bad_object.move");

#[test]
fn publish_key_struct_without_uid_fails() -> Result<()> {
    let package_dir = create_test_package()?;
    let (module_id, module_bytes) = compile_test_module(&package_dir)?;
    let sender = *module_id.address();
    assert_eq!(sender, AccountAddress::from_hex_literal(TESTER_ADDR)?);

    let store = Arc::new(PersistentStore::open_in_memory()?);
    let runtime = MoveRuntime::new_with_kanari_natives_and_store(store)?;

    let err = runtime
        .publish_module(module_bytes, sender, None, None)
        .expect_err("publishing a UID-less key struct must fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("UID"),
        "rejection must name the missing UID rule, got: {msg}"
    );
    Ok(())
}

fn create_test_package() -> Result<PathBuf> {
    let dir = tempdir()?;
    let package_dir = dir.keep();
    fs::create_dir_all(package_dir.join("sources"))?;

    let dependency_path = move_manifest::kanari_system_package_path()?;
    let manifest = format!(
        "[package]\nname = \"BadObject\"\n\n[dependencies]\nKanariSystem = {{ local = \"{}\" }}\n\n[addresses]\ntester = \"{}\"\n",
        dependency_path, TESTER_ADDR,
    );
    fs::write(package_dir.join("Move.toml"), manifest)?;
    fs::write(
        package_dir.join("sources").join("bad_object.move"),
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
