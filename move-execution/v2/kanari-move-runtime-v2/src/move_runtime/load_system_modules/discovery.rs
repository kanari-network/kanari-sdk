use anyhow::{Result, anyhow};
use log::warn;
use move_binary_format::file_format::CompiledModule;
use move_bytecode_verifier::verifier::verify_module_unmetered;
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::ModuleId;

use move_package::BuildConfig;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

pub(crate) fn mv_filename(name: &str) -> String {
    if name.ends_with(".mv") { name.to_owned() } else { format!("{name}.mv") }
}

pub(crate) fn is_test_module_artifact(file_name: &str, module_name: &str) -> bool {
    let lower_file = file_name.to_ascii_lowercase();
    let lower_module = module_name.to_ascii_lowercase();
    lower_file.ends_with("_tests.mv")
        || lower_file.ends_with("_test.mv")
        || lower_module.ends_with("_tests")
        || lower_module.ends_with("_test")
}

pub(crate) fn build_info_path(modules_dir: &Path) -> Option<PathBuf> {
    Some(modules_dir.parent()?.join("BuildInfo.yaml"))
}

pub(crate) fn package_root_from_modules_dir(modules_dir: &Path) -> Option<PathBuf> {
    Some(modules_dir.parent()?.parent()?.parent()?.to_path_buf())
}

pub(crate) fn ensure_production_build_artifacts(modules_dir: &Path) -> Result<()> {
    let Some(build_info) = build_info_path(modules_dir) else {
        return Ok(());
    };
    let Ok(contents) = std::fs::read_to_string(&build_info) else {
        return Ok(());
    };

    if !contents.contains("test_mode: true") {
        return Ok(());
    }

    let package_root = package_root_from_modules_dir(modules_dir).ok_or_else(|| {
        anyhow!(
            "Could not infer package root from contaminated build dir: {}",
            modules_dir.display()
        )
    })?;

    tracing::warn!(
        "Detected test-mode Move build artifacts at {}; rebuilding package in production mode",
        modules_dir.display()
    );

    let mut sink = std::io::sink();
    BuildConfig::default().compile_package(&package_root, &mut sink).map_err(|e| {
        anyhow!("Failed to rebuild production Move package at {}: {}", package_root.display(), e)
    })?;

    let refreshed = std::fs::read_to_string(&build_info).map_err(|e| {
        anyhow!("Failed to re-read BuildInfo after rebuilding {}: {}", build_info.display(), e)
    })?;
    if refreshed.contains("test_mode: true") {
        return Err(anyhow!(
            "Move package at {} still reports test_mode: true after rebuild",
            package_root.display()
        ));
    }

    Ok(())
}

#[derive(Clone)]
pub(crate) struct DiscoveredModule {
    pub module_id: ModuleId,
    pub module_name: String,
    pub file_name: String,
    pub bytes: Vec<u8>,
    pub compiled: CompiledModule,
    pub deps: Vec<ModuleId>,
}

pub(crate) fn discover_modules_in_dir(
    modules_dir: &Path,
    expected_addr: AccountAddress,
) -> Vec<DiscoveredModule> {
    let Ok(dir) = std::fs::read_dir(modules_dir) else {
        return vec![];
    };

    let mut modules = Vec::new();
    for entry in dir.flatten() {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if !ft.is_file() {
            continue;
        }

        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("mv") {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        let Ok(bytes) = std::fs::read(&path) else {
            warn!("Warning: Failed to read module file: {:?}", path);
            continue;
        };

        let compiled = match CompiledModule::deserialize_with_defaults(&bytes) {
            Ok(m) => m,
            Err(e) => {
                warn!("Warning: Failed to decode {}: {}", file_name, e);
                continue;
            }
        };

        let module_id = compiled.self_id();
        let module_name = module_id.name().to_string();

        if is_test_module_artifact(&file_name, &module_name) {
            continue;
        }

        if *module_id.address() != expected_addr {
            continue;
        }

        if let Err(e) = verify_module_unmetered(&compiled) {
            warn!("Warning: Bytecode verify failed for {}: {:?}", file_name, e);
            continue;
        }

        let deps = compiled.immediate_dependencies();
        modules.push(DiscoveredModule { module_id, module_name, file_name, bytes, compiled, deps });
    }

    modules.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    modules
}

pub(crate) fn discover_embedded_modules(
    entries: &[(&str, &[u8])],
    expected_addr: AccountAddress,
) -> Vec<DiscoveredModule> {
    entries
        .iter()
        .filter_map(|(file_name, bytes)| {
            let compiled = CompiledModule::deserialize_with_defaults(bytes).ok()?;
            let module_id = compiled.self_id();
            let module_name = module_id.name().to_string();
            if is_test_module_artifact(file_name, &module_name)
                || *module_id.address() != expected_addr
                || verify_module_unmetered(&compiled).is_err()
            {
                return None;
            }
            Some(DiscoveredModule {
                module_id,
                module_name,
                file_name: (*file_name).to_string(),
                bytes: (*bytes).to_vec(),
                deps: compiled.immediate_dependencies(),
                compiled,
            })
        })
        .collect()
}

pub(crate) fn topo_sort_modules(
    modules: Vec<DiscoveredModule>,
) -> Result<Vec<DiscoveredModule>, anyhow::Error> {
    if modules.len() <= 1 {
        return Ok(modules);
    }

    let mut index_by_id: HashMap<ModuleId, usize> = HashMap::with_capacity(modules.len());
    for (idx, m) in modules.iter().enumerate() {
        index_by_id.insert(m.module_id.clone(), idx);
    }

    let mut indegree = vec![0usize; modules.len()];
    let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); modules.len()];

    for (idx, m) in modules.iter().enumerate() {
        for dep in &m.deps {
            if let Some(&dep_idx) = index_by_id.get(dep) {
                outgoing[dep_idx].push(idx);
                indegree[idx] += 1;
            }
        }
    }

    let mut ready: BTreeSet<(String, usize)> = BTreeSet::new();
    for idx in 0..modules.len() {
        if indegree[idx] == 0 {
            ready.insert((modules[idx].module_name.clone(), idx));
        }
    }

    let mut ordered_idxs = Vec::with_capacity(modules.len());
    let mut emitted = vec![false; modules.len()];

    while let Some((module_name, idx)) = ready.iter().next().cloned() {
        ready.remove(&(module_name, idx));
        ordered_idxs.push(idx);
        emitted[idx] = true;

        for &next in &outgoing[idx] {
            indegree[next] = indegree[next].saturating_sub(1);
            if indegree[next] == 0 {
                ready.insert((modules[next].module_name.clone(), next));
            }
        }
    }

    if ordered_idxs.len() != modules.len() {
        // Cycles or missing intra-dir deps. In Move, circular dependencies are prohibited,
        // and modules must be published in a strict dependency order. An error should be returned
        // to alert the operator of an invalid framework state.
        let remaining: Vec<(String, usize)> = (0..modules.len())
            .filter(|i| !emitted[*i])
            .map(|i| (modules[i].module_name.clone(), i))
            .collect();

        return Err(anyhow!(
            "Could not fully resolve module dependency order ({} of {}). \
             Found {} modules with cyclic or missing dependencies: {:?}",
            ordered_idxs.len(),
            modules.len(),
            remaining.len(),
            remaining.iter().map(|(name, _)| name).collect::<Vec<_>>()
        ));
    }

    let mut slots: Vec<Option<DiscoveredModule>> = modules.into_iter().map(Some).collect();
    let mut out = Vec::with_capacity(slots.len());
    for idx in ordered_idxs {
        if let Some(m) = slots.get_mut(idx).and_then(|s| s.take()) {
            out.push(m);
        }
    }
    Ok(out)
}
