use anyhow::{Result, anyhow};
use kanari_types::address::Address as KanariAddress;
use kanari_types::env::{env_exact, env_flag};
use kanari_types::hash::blake3_hex;
use log::warn;
use move_binary_format::compatibility::Compatibility;
use move_binary_format::file_format::CompiledModule;
use move_binary_format::normalized;
use move_bytecode_verifier::dependencies;
use move_core_types::account_address::AccountAddress;

use std::path::PathBuf;

use super::DiscoveredModule;
use super::mv_filename;
use super::*;

pub(crate) fn module_bytecode_segments(set: &ModuleSetSpec) -> [&'static str; 7] {
    [
        "crates",
        "kanari-frameworks",
        "packages",
        set.dir_name,
        "build",
        set.package_name,
        "bytecode_modules",
    ]
}

pub(crate) fn compute_framework_manifest_and_hash(
    modules: &[DiscoveredModule],
) -> (Vec<(String, String)>, String) {
    let mut manifest: Vec<(String, String)> = modules
        .iter()
        .map(|m| {
            let id = format!("{}::{}", m.module_id.address().to_hex_literal(), m.module_id.name());
            let h = blake3_hex(&m.bytes);
            (id, h)
        })
        .collect();
    manifest.sort_by(|a, b| a.0.cmp(&b.0));

    let mut blob = Vec::new();
    for (id, h) in &manifest {
        blob.extend_from_slice(id.as_bytes());
        blob.push(0);
        blob.extend_from_slice(h.as_bytes());
        blob.push(0);
    }
    let root_hash = blake3_hex(&blob);
    (manifest, root_hash)
}

pub(crate) fn verify_framework_hash(
    state: &crate::storage::move_vm_state::MoveVMState,
    framework_id: &str,
    modules: &[DiscoveredModule],
    expected_env_var: &str,
    changed_warning: &str,
) -> Result<()> {
    let (manifest, hash_hex) = compute_framework_manifest_and_hash(modules);
    log::info!("{} framework hash (disk): {}", framework_id, hash_hex);
    if let Some(prev) = state.try_get_framework_hash(framework_id)?
        && prev != hash_hex
    {
        warn!("{}", changed_warning.replace("{prev}", &prev).replace("{hash}", &hash_hex));
    }
    state.save_framework_manifest(framework_id, &manifest, &hash_hex)?;
    if let Ok(expected) = std::env::var(expected_env_var)
        && expected != hash_hex
    {
        return Err(anyhow!(
            "{} framework hash mismatch: expected {}, got {}",
            framework_id,
            expected,
            hash_hex
        ));
    }
    Ok(())
}

pub(crate) fn prune_framework_modules(
    runtime: &super::super::MoveRuntime,
    modules: &[DiscoveredModule],
    framework_addr: AccountAddress,
) -> Result<()> {
    #[cfg(feature = "framework-pruning")]
    {
        let keep: BTreeSet<ModuleId> = modules.iter().map(|m| m.module_id.clone()).collect();
        for id in runtime.state.get_all_module_ids()? {
            if *id.address() == framework_addr && !keep.contains(&id) {
                if let Err(e) = runtime.state.delete_module(&id) {
                    warn!("Warning: Failed to prune module {}: {}", id, e);
                } else if let Ok(mut mods) = runtime.published_modules.write() {
                    mods.remove(&id);
                }
            }
        }
    }

    #[cfg(not(feature = "framework-pruning"))]
    let _ = (runtime, modules, framework_addr);

    Ok(())
}

pub(crate) fn save_framework_modules(
    runtime: &super::super::MoveRuntime,
    modules: Vec<DiscoveredModule>,
    incompatible_error_prefix: &str,
) -> Result<usize> {
    let mut count = 0;
    for m in modules {
        let module_file = mv_filename(&m.file_name);
        if let Some(old_bytes) = runtime.state.try_get_module(&m.module_id)?
            && old_bytes != m.bytes
        {
            let old_compiled = CompiledModule::deserialize_with_defaults(&old_bytes)?;
            let old_norm = normalized::Module::new(&old_compiled);
            let new_norm = normalized::Module::new(&m.compiled);
            if let Err(e) = Compatibility::full_check().check(&old_norm, &new_norm) {
                if !env_exact("KANARI_FRAMEWORK_ALLOW_INCOMPATIBLE", "1") {
                    return Err(anyhow!(
                        "Incompatible {} upgrade for {} (set KANARI_FRAMEWORK_ALLOW_INCOMPATIBLE=1 to override): {:?}",
                        incompatible_error_prefix,
                        m.module_id,
                        e
                    ));
                }
                warn!(
                    "Warning: Allowing incompatible {} upgrade for {}: {:?}",
                    incompatible_error_prefix, m.module_id, e
                );
            }
        }
        if let Err(e) = runtime.state.save_module(&m.module_id, &m.bytes) {
            tracing::warn!("Failed to save {}: {}", module_file, e);
            continue;
        }
        if let Ok(mut mods) = runtime.published_modules.write() {
            mods.insert(m.module_id);
        }
        count += 1;
    }
    Ok(count)
}

pub(crate) fn find_modules_dir(env_var: &str, segments: &[&str]) -> PathBuf {
    if let Ok(path_str) = std::env::var(env_var) {
        return PathBuf::from(path_str);
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut dir = Some(cwd.as_path());
    while let Some(d) = dir {
        let mut p = PathBuf::from(d);
        for seg in segments.iter() {
            p.push(seg);
        }
        if p.exists() {
            return p;
        }
        dir = d.parent();
    }
    let mut p = cwd;
    for seg in segments.iter() {
        p.push(seg);
    }
    p
}

/// Static description of a framework package's module set: where its compiled
/// bytecode lives on disk and the copy embedded in the binary.
#[derive(Clone, Copy)]
pub(crate) struct ModuleSetSpec {
    /// Framework address (e.g. `0x1`).
    address: &'static str,
    /// Human-readable name used in warnings and the final load log.
    display_name: &'static str,
    /// Name used in the "no modules found" error message.
    error_name: &'static str,
    /// Env var that can override the on-disk modules directory.
    modules_dir_env: &'static str,
    /// Package directory name under `packages/` (e.g. `move-stdlib`).
    dir_name: &'static str,
    /// Compiled package name (e.g. `MoveStdlib`).
    package_name: &'static str,
    /// Compiled modules embedded in this binary.
    embedded: &'static [(&'static str, &'static [u8])],
}

/// Static description of a loadable framework and the framework module sets it
/// must be dependency-verified against (in addition to itself).
pub(crate) struct FrameworkSpec {
    /// Key used for the manifest/hash stored in state (same as the address).
    id: &'static str,
    /// Lowercase label used in log/error text ("stdlib", "system", "governance").
    label: &'static str,
    set: ModuleSetSpec,
    /// Env var holding the expected framework hash (consensus check).
    expected_hash_env: &'static str,
    /// Warning template used when the on-disk hash differs from the stored one.
    changed_warning: &'static str,
    /// Dependency module sets to verify against, in addition to `set`.
    deps: Vec<ModuleSetSpec>,
}

/// Build the descriptions of every framework package in dependency order.
///
/// Centralising them here removes the per-framework duplication: adding a new
/// framework (e.g. `0x4`) only requires one more entry in this list.
pub(crate) fn framework_specs() -> Vec<FrameworkSpec> {
    let stdlib_set = ModuleSetSpec {
        address: KanariAddress::STD_ADDRESS,
        display_name: "Move stdlib",
        error_name: "Move stdlib",
        modules_dir_env: "MOVE_STDLIB_PATH",
        dir_name: "move-stdlib",
        package_name: "MoveStdlib",
        embedded: super::EMBEDDED_MOVE_STDLIB_MODULES,
    };
    let system_set = ModuleSetSpec {
        address: KanariAddress::KANARI_SYSTEM_ADDRESS,
        display_name: "Kanari system",
        error_name: "kanari-system",
        modules_dir_env: "KANARI_FRAMEWORK_PATH",
        dir_name: "kanari-system",
        package_name: "KanariSystem",
        embedded: super::EMBEDDED_KANARI_SYSTEM_MODULES,
    };
    let governance_set = ModuleSetSpec {
        address: KanariAddress::KANARI_GOVERNANCE_ADDRESS,
        display_name: "Kanari governance",
        error_name: "kanari-governance",
        modules_dir_env: "KANARI_GOVERNANCE_PATH",
        dir_name: "kanari-governance",
        package_name: "KanariGovernance",
        embedded: super::EMBEDDED_KANARI_GOVERNANCE_MODULES,
    };

    vec![
        FrameworkSpec {
            id: KanariAddress::STD_ADDRESS,
            label: "stdlib",
            set: stdlib_set,
            expected_hash_env: "KANARI_FRAMEWORK_EXPECTED_HASH_0X1",
            changed_warning: "Warning: stdlib framework hash changed (db: {prev}, disk: {hash}). Ensure all validators upgrade together.",
            deps: Vec::new(),
        },
        FrameworkSpec {
            id: KanariAddress::KANARI_SYSTEM_ADDRESS,
            label: "system",
            set: system_set,
            expected_hash_env: "KANARI_FRAMEWORK_EXPECTED_HASH_0X2",
            changed_warning: "Warning: system framework hash changed (db: {prev}, disk: {hash}). Ensure all validators upgrade together.",
            deps: vec![stdlib_set],
        },
        FrameworkSpec {
            id: KanariAddress::KANARI_GOVERNANCE_ADDRESS,
            label: "governance",
            set: governance_set,
            expected_hash_env: "KANARI_FRAMEWORK_EXPECTED_HASH_0X3",
            changed_warning: "Warning: governance framework hash changed (db: {prev}, disk: {hash}). Ensure all validators upgrade together.",
            deps: vec![stdlib_set, system_set],
        },
    ]
}

pub(crate) fn verbose_startup_enabled() -> bool {
    env_flag("KANARI_VERBOSE_STARTUP")
}

pub(crate) fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Discover and topologically sort the modules of a module set, preferring
/// on-disk build artifacts and using the embedded bytecode otherwise.
///
/// `ensure_production` rebuilds test-mode artifacts; only the framework's own
/// directory is checked this way, matching the previous per-framework loaders.
pub(crate) fn discover_module_set(
    set: &ModuleSetSpec,
    use_embedded: bool,
    ensure_production: bool,
) -> Result<Vec<DiscoveredModule>> {
    let segments = module_bytecode_segments(set);
    let modules_dir = find_modules_dir(set.modules_dir_env, &segments);
    let use_disk = !use_embedded && modules_dir.exists();
    if use_disk && ensure_production {
        ensure_production_build_artifacts(&modules_dir)?;
    }

    let expected_addr = AccountAddress::from_hex_literal(set.address)?;
    let modules = if use_disk {
        topo_sort_modules(discover_modules_in_dir(&modules_dir, expected_addr))?
    } else {
        warn!(
            "{} artifacts not found on disk; using bytecode embedded in this binary",
            set.display_name
        );
        topo_sort_modules(discover_embedded_modules(set.embedded, expected_addr))?
    };
    if modules.is_empty() {
        anyhow::bail!("No {} modules (*.mv) found at {:?}", set.error_name, modules_dir);
    }
    Ok(modules)
}

/// Load one framework into the runtime state: discover, hash-check,
/// dependency-verify, prune, and save its modules.
pub(crate) fn load_framework(
    runtime: &super::super::MoveRuntime,
    spec: &FrameworkSpec,
    use_embedded: bool,
) -> Result<()> {
    let segments = module_bytecode_segments(&spec.set);
    let modules_dir = find_modules_dir(spec.set.modules_dir_env, &segments);
    if verbose_startup_enabled() {
        tracing::info!("Looking for {} modules at {:?}", spec.set.display_name, modules_dir);
    }

    let modules = discover_module_set(&spec.set, use_embedded, true)?;

    verify_framework_hash(
        &runtime.state,
        spec.id,
        &modules,
        spec.expected_hash_env,
        spec.changed_warning,
    )?;

    // Dependency verification against the framework's own modules plus every
    // framework it depends on (stdlib for system; stdlib + system for governance).
    // The dependency module sets are kept alive here because `all_deps`
    // borrows from them.
    let mut dep_module_sets: Vec<Vec<DiscoveredModule>> = Vec::new();
    for dep in &spec.deps {
        dep_module_sets.push(discover_module_set(dep, use_embedded, false)?);
    }
    let mut all_deps: Vec<&CompiledModule> = Vec::new();
    for dep_modules in &dep_module_sets {
        all_deps.extend(dep_modules.iter().map(|m| &m.compiled));
    }
    all_deps.extend(modules.iter().map(|m| &m.compiled));

    let verify_label = capitalize_first(spec.label);
    for m in &modules {
        dependencies::verify_module(&m.compiled, all_deps.iter().copied()).map_err(|e| {
            anyhow::anyhow!("{} deps verify failed for {}: {:?}", verify_label, m.file_name, e)
        })?;
    }

    let framework_addr = AccountAddress::from_hex_literal(spec.set.address)?;
    prune_framework_modules(runtime, &modules, framework_addr)?;
    let count = save_framework_modules(runtime, modules, spec.label)?;

    tracing::info!(
        modules = count,
        address = spec.set.address,
        "{} modules loaded",
        spec.set.display_name
    );
    Ok(())
}

/// Load framework modules as methods on `MoveRuntime`
impl super::super::MoveRuntime {
    /// Load move-stdlib modules (0x1::*)
    pub(crate) fn load_move_stdlib(&self, use_embedded: bool) -> Result<()> {
        let spec = framework_specs()
            .into_iter()
            .find(|s| s.label == "stdlib")
            .expect("stdlib framework spec is always defined");
        load_framework(self, &spec, use_embedded)
    }

    /// Load Kanari system modules (0x2::*)
    pub(crate) fn load_kanari_system(&self, use_embedded: bool) -> Result<()> {
        let spec = framework_specs()
            .into_iter()
            .find(|s| s.label == "system")
            .expect("system framework spec is always defined");
        load_framework(self, &spec, use_embedded)
    }

    /// Load Kanari governance modules (0x3::*)
    pub(crate) fn load_kanari_governance(&self, use_embedded: bool) -> Result<()> {
        let spec = framework_specs()
            .into_iter()
            .find(|s| s.label == "governance")
            .expect("governance framework spec is always defined");
        load_framework(self, &spec, use_embedded)
    }
}

pub(crate) fn load_embedded_kanari_governance_modules() -> Result<Vec<DiscoveredModule>> {
    let spec = framework_specs()
        .into_iter()
        .find(|s| s.label == "governance")
        .expect("governance framework spec is always defined");
    let set = spec.set;
    let governance_addr = AccountAddress::from_hex_literal(set.address)?;
    topo_sort_modules(discover_embedded_modules(set.embedded, governance_addr))
}

pub(crate) fn load_embedded_kanari_system_modules() -> Result<Vec<DiscoveredModule>> {
    let spec = framework_specs()
        .into_iter()
        .find(|s| s.label == "system")
        .expect("system framework spec is always defined");
    let set = spec.set;
    let system_addr = AccountAddress::from_hex_literal(set.address)?;
    topo_sort_modules(discover_embedded_modules(set.embedded, system_addr))
}

#[cfg(test)]
mod embedded_framework_tests {
    use super::*;

    #[test]
    fn embedded_kanari_framework_is_nonempty_and_address_bound() {
        let modules = load_embedded_kanari_system_modules().unwrap();
        assert!(!modules.is_empty());
        let expected =
            AccountAddress::from_hex_literal(KanariAddress::KANARI_SYSTEM_ADDRESS).unwrap();
        assert!(modules.iter().all(|module| *module.module_id.address() == expected));
    }

    #[test]
    fn embedded_kanari_governance_is_nonempty_and_address_bound() {
        let modules = load_embedded_kanari_governance_modules().unwrap();
        assert!(!modules.is_empty());
        let expected =
            AccountAddress::from_hex_literal(KanariAddress::KANARI_GOVERNANCE_ADDRESS).unwrap();
        assert!(modules.iter().all(|module| *module.module_id.address() == expected));
    }
}
