//! End-to-end publish of the `kanari_pet` example package.
//!
//! `kanari_pet` is the package from the bug report: `pet` uses its sibling
//! module `nft` (`use kanari_pet::nft as pet_nft;`), and the publish was
//! rejected with `Missing dependency: <address>::nft` because the dependency
//! check only looked at on-chain storage, where the sibling does not exist
//! until the very transaction that publishes it.
//!
//! The package is published exactly as the CLI publishes it: every root
//! module of the compiled package, in one package transaction.

use anyhow::{Context, Result};
use kanari_move_runtime_v2::move_runtime::MoveRuntime;
use kanari_types::error::KanariUnwrapExt;
use move_core_types::language_storage::ModuleId;
use move_package::BuildConfig;
use std::path::PathBuf;

#[test]
fn example_pet_package_publishes_with_sibling_module_dependency() -> Result<()> {
    let package_dir = example_package_path();
    anyhow::ensure!(
        package_dir.exists(),
        "example package must exist at {}",
        package_dir.display()
    );

    let package = BuildConfig::default()
        .compile_package(&package_dir, &mut Vec::new())
        .context("compile the kanari_pet example package")?;

    let mut modules: Vec<(String, Vec<u8>)> = Vec::new();
    let mut module_ids: Vec<ModuleId> = Vec::new();
    for unit in package.root_modules() {
        let module = &unit.unit.module;
        let module_id = module.self_id();
        let mut bytes = Vec::new();
        module.serialize(&mut bytes)?;
        modules.push((module_id.name().to_string(), bytes));
        module_ids.push(module_id);
    }

    // `pet` depends on `nft`, so the package must carry both.
    anyhow::ensure!(
        module_ids.iter().any(|id| id.name().as_str() == "pet")
            && module_ids.iter().any(|id| id.name().as_str() == "nft"),
        "the example package must compile the pet and nft modules"
    );

    let sender = *module_ids[0].address();
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory().invariant("init runtime");

    let _ = runtime
        .publish_package_with_context_and_persistence(modules, sender, None, None, None, true)
        .context("publish the kanari_pet example package")?;

    for module_id in &module_ids {
        assert!(
            runtime.get_module_bytes(module_id).is_some(),
            "module {module_id} must be stored after publish"
        );
    }

    Ok(())
}

fn example_package_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../example_move/kanari_pet")
}
