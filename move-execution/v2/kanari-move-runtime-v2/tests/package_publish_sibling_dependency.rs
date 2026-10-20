//! Regression: a package whose modules depend on each other must publish.
//!
//! The publish path verified every module's dependencies against on-chain
//! storage only. A module that depends on a sibling published by the *same*
//! transaction was therefore rejected with
//! `Missing dependency: <address>::<sibling>`, and no package with
//! intra-package dependencies could ever be published — `kanari_pet` failed
//! this way as soon as `pet` started using the sibling module `nft`.
//!
//! The fix resolves sibling dependencies inside the package, and publishes the
//! package in dependency order, because the Move VM links a module against the
//! modules already published in the same session and the compiler guarantees
//! no module order.

use anyhow::{Context, Result};
use kanari_move_runtime_v2::move_runtime::MoveRuntime;
use kanari_types::error::KanariUnwrapExt;
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::ModuleId;
use move_package::BuildConfig;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

const TESTER_ADDR: &str = "0x4c";
const HELPER_MODULE: &str = "helper";
const DEPENDENT_MODULE: &str = "dependent";

const HELPER_SOURCE: &str = r#"
module tester::helper {
    public fun value(): u64 { 7 }
}
"#;

const DEPENDENT_SOURCE: &str = r#"
module tester::dependent {
    use tester::helper;

    public fun value(): u64 { helper::value() + 1 }
}
"#;

#[test]
fn package_publish_accepts_sibling_dependency_in_any_order() -> Result<()> {
    let package_dir = create_test_package()?;
    let modules = compile_test_modules(&package_dir)?;

    for order in [
        vec![DEPENDENT_MODULE, HELPER_MODULE],
        vec![HELPER_MODULE, DEPENDENT_MODULE],
    ] {
        let runtime = MoveRuntime::new_with_kanari_natives_in_memory().invariant("init runtime");
        let sender = AccountAddress::from_hex_literal(TESTER_ADDR)?;

        let ordered: Vec<(String, Vec<u8>)> = order
            .iter()
            .map(|name| {
                let (module_id, bytes) = modules
                    .iter()
                    .find(|(module_id, _)| module_id.name().as_str() == *name)
                    .with_context(|| format!("compiled module {name}"))?;
                Ok((module_id.name().to_string(), bytes.clone()))
            })
            .collect::<Result<_>>()?;

        let _ = runtime
            .publish_package_with_context_and_persistence(ordered, sender, None, None, None, true)
            .with_context(|| format!("publish package in order {order:?}"))?;

        for (module_id, bytes) in &modules {
            let stored = runtime
                .get_module_bytes(module_id)
                .invariant("package module must be stored after publish");
            assert_eq!(stored, *bytes);
        }
    }

    Ok(())
}

#[test]
fn package_publish_rejects_dependency_outside_the_package() -> Result<()> {
    let package_dir = create_test_package()?;
    let modules = compile_test_modules(&package_dir)?;
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory().invariant("init runtime");
    let sender = AccountAddress::from_hex_literal(TESTER_ADDR)?;

    // Publish only the dependent module: its sibling is neither in the
    // package nor on chain, so the dependency cannot be satisfied.
    let (dependent_id, dependent_bytes) = modules
        .iter()
        .find(|(module_id, _)| module_id.name().as_str() == DEPENDENT_MODULE)
        .with_context(|| format!("compiled module {DEPENDENT_MODULE}"))?;

    let err = runtime
        .publish_package_with_context_and_persistence(
            vec![(dependent_id.name().to_string(), dependent_bytes.clone())],
            sender,
            None,
            None,
            None,
            true,
        )
        .expect_err("a dependency outside the package must be rejected");
    let message = format!("{err:#}");
    assert!(
        message.contains("Missing dependency") && message.contains(HELPER_MODULE),
        "unexpected error: {message}"
    );
    assert!(
        runtime.get_module_bytes(dependent_id).is_none(),
        "a rejected package must not store any module"
    );

    Ok(())
}

/// Create a Move package with two modules, one depending on the other.
fn create_test_package() -> Result<PathBuf> {
    let dir = tempdir()?;
    let package_dir = dir.keep();
    fs::create_dir_all(package_dir.join("sources"))?;

    let manifest = format!(
        "[package]\nname = \"SiblingDependency\"\n\n[addresses]\ntester = \"{}\"\n",
        TESTER_ADDR,
    );
    fs::write(package_dir.join("Move.toml"), manifest)?;
    fs::write(
        package_dir.join("sources").join("helper.move"),
        HELPER_SOURCE,
    )?;
    fs::write(
        package_dir.join("sources").join("dependent.move"),
        DEPENDENT_SOURCE,
    )?;

    Ok(package_dir)
}

/// Compile the fixture package and return its root modules.
fn compile_test_modules(package_dir: &Path) -> Result<Vec<(ModuleId, Vec<u8>)>> {
    let package = BuildConfig::default().compile_package(package_dir, &mut Vec::new())?;
    let mut modules = Vec::new();
    for unit in package.root_modules() {
        let module = &unit.unit.module;
        let module_id = module.self_id();
        let mut bytes = Vec::new();
        module.serialize(&mut bytes)?;
        modules.push((module_id, bytes));
    }
    anyhow::ensure!(modules.len() == 2, "fixture must compile two modules");
    Ok(modules)
}
