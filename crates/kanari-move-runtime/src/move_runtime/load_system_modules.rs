// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use kanari_crypto::hash_data_blake3;
use kanari_types::address::Address as KanariAddress;
use log::warn;
use move_binary_format::compatibility::Compatibility;
use move_binary_format::file_format::CompiledModule;
use move_binary_format::normalized;
use move_bytecode_verifier::dependencies;
use move_bytecode_verifier::verifier::verify_module_unmetered;
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::ModuleId;

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

fn mv_filename(name: &str) -> String {
    if name.ends_with(".mv") {
        name.to_owned()
    } else {
        format!("{name}.mv")
    }
}

struct DiscoveredModule {
    module_id: ModuleId,
    module_name: String,
    file_name: String,
    bytes: Vec<u8>,
    compiled: CompiledModule,
    deps: Vec<ModuleId>,
}

fn discover_modules_in_dir(
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
        if *module_id.address() != expected_addr {
            continue;
        }

        if let Err(e) = verify_module_unmetered(&compiled) {
            warn!("Warning: Bytecode verify failed for {}: {:?}", file_name, e);
            continue;
        }

        let deps = compiled.immediate_dependencies();
        let module_name = module_id.name().to_string();
        modules.push(DiscoveredModule {
            module_id,
            module_name,
            file_name,
            bytes,
            compiled,
            deps,
        });
    }

    modules.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    modules
}

fn topo_sort_modules(modules: Vec<DiscoveredModule>) -> Vec<DiscoveredModule> {
    if modules.len() <= 1 {
        return modules;
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
        // Cycles or missing intra-dir deps. Keep deterministic behavior and try best-effort.
        warn!(
            "Warning: Could not fully resolve module dependency order ({} of {}). Falling back to name order for remaining modules.",
            ordered_idxs.len(),
            modules.len()
        );
        let mut remaining: Vec<(String, usize)> = (0..modules.len())
            .filter(|i| !emitted[*i])
            .map(|i| (modules[i].module_name.clone(), i))
            .collect();
        remaining.sort();
        ordered_idxs.extend(remaining.into_iter().map(|(_, i)| i));
    }

    let mut slots: Vec<Option<DiscoveredModule>> = modules.into_iter().map(Some).collect();
    let mut out = Vec::with_capacity(slots.len());
    for idx in ordered_idxs {
        if let Some(m) = slots.get_mut(idx).and_then(|s| s.take()) {
            out.push(m);
        }
    }
    out
}

fn compute_framework_manifest_and_hash(
    modules: &[DiscoveredModule],
) -> (Vec<(String, String)>, String) {
    let mut manifest: Vec<(String, String)> = modules
        .iter()
        .map(|m| {
            let id = format!(
                "{}::{}",
                m.module_id.address().to_hex_literal(),
                m.module_id.name()
            );
            let h = hex::encode(hash_data_blake3(&m.bytes));
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
    let root_hash = hex::encode(hash_data_blake3(&blob));
    (manifest, root_hash)
}

fn find_modules_dir(env_var: &str, segments: &[&str]) -> PathBuf {
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

/// Load move-stdlib and kanari-system modules as methods on `MoveRuntime`
impl super::MoveRuntime {
    /// Load move-stdlib modules (0x1::*)
    pub fn load_move_stdlib(&self) -> Result<()> {
        let segments = [
            "crates",
            "kanari-frameworks",
            "packages",
            "move-stdlib",
            "build",
            "MoveStdlib",
            "bytecode_modules",
        ];
        let modules_dir = find_modules_dir("MOVE_STDLIB_PATH", &segments);

        eprintln!("✓ Looking for Move stdlib modules at: {:?}", modules_dir);

        if !modules_dir.exists() {
            warn!(
                "Warning: Move stdlib modules not found at {:?}",
                modules_dir
            );
            warn!("Standard library will not be pre-loaded.");
            return Ok(());
        }

        // Load stdlib modules in dependency order
        let std_addr = AccountAddress::from_hex_literal(KanariAddress::STD_ADDRESS)?;

        let mut count = 0;
        let modules = topo_sort_modules(discover_modules_in_dir(&modules_dir, std_addr));
        if modules.is_empty() {
            return Err(anyhow::anyhow!(
                "No Move stdlib modules (*.mv) found at {:?}",
                modules_dir
            ));
        }

        for m in &modules {
            dependencies::verify_module(&m.compiled, modules.iter().map(|d| &d.compiled)).map_err(
                |e| anyhow::anyhow!("Stdlib deps verify failed for {}: {:?}", m.file_name, e),
            )?;
        }

        let (manifest, hash_hex) = compute_framework_manifest_and_hash(&modules);
        eprintln!("âœ“ Move stdlib framework hash (disk): {}", hash_hex);
        if let Some(prev) = self.state.get_framework_hash("0x1") {
            if prev != hash_hex {
                warn!(
                    "Warning: stdlib framework hash changed (db: {}, disk: {}). Ensure all validators upgrade together.",
                    prev, hash_hex
                );
            }
        }
        self.state
            .save_framework_manifest("0x1", &manifest, &hash_hex)?;
        if let Ok(expected) = std::env::var("KANARI_FRAMEWORK_EXPECTED_HASH_0X1") {
            if expected != hash_hex {
                return Err(anyhow::anyhow!(
                    "Stdlib framework hash mismatch: expected {}, got {}",
                    expected,
                    hash_hex
                ));
            }
        }

        if std::env::var("KANARI_FRAMEWORK_PRUNE").ok().as_deref() == Some("1") {
            let keep: BTreeSet<ModuleId> = modules.iter().map(|m| m.module_id.clone()).collect();
            for id in self.state.get_all_module_ids().unwrap_or_default() {
                if *id.address() == std_addr && !keep.contains(&id) {
                    if let Err(e) = self.state.delete_module(&id) {
                        warn!("Warning: Failed to prune module {}: {}", id, e);
                    } else if let Ok(mut mods) = self.published_modules.write() {
                        mods.remove(&id);
                    }
                }
            }
        }

        for m in modules {
            let module_file = mv_filename(&m.file_name);
            if let Some(old_bytes) = self.state.get_module(&m.module_id) {
                if old_bytes != m.bytes {
                    let old_compiled = CompiledModule::deserialize_with_defaults(&old_bytes)?;
                    let old_norm = normalized::Module::new(&old_compiled);
                    let new_norm = normalized::Module::new(&m.compiled);
                    if let Err(e) = Compatibility::full_check().check(&old_norm, &new_norm) {
                        if std::env::var("KANARI_FRAMEWORK_ALLOW_INCOMPATIBLE")
                            .ok()
                            .as_deref()
                            != Some("1")
                        {
                            return Err(anyhow::anyhow!(
                                "Incompatible stdlib upgrade for {} (set KANARI_FRAMEWORK_ALLOW_INCOMPATIBLE=1 to override): {:?}",
                                m.module_id,
                                e
                            ));
                        }
                        warn!(
                            "Warning: Allowing incompatible stdlib upgrade for {}: {:?}",
                            m.module_id, e
                        );
                    }
                }
            }
            if let Err(e) = self.state.save_module(&m.module_id, &m.bytes) {
                eprintln!("Warning: Failed to save {}: {}", module_file, e);
                continue;
            }
            if let Ok(mut mods) = self.published_modules.write() {
                mods.insert(m.module_id);
            }
            count += 1;
        }

        eprintln!("✓ Loaded {} move-stdlib modules (0x1::*)", count);
        Ok(())
    }

    /// Load Kanari system modules (0x2::*)
    pub fn load_kanari_system(&self) -> Result<()> {
        let segments = [
            "crates",
            "kanari-frameworks",
            "packages",
            "kanari-system",
            "build",
            "KanariSystem",
            "bytecode_modules",
        ];
        let modules_dir = find_modules_dir("KANARI_FRAMEWORK_PATH", &segments);

        eprintln!("✓ Looking for Kanari system modules at: {:?}", modules_dir);

        if !modules_dir.exists() {
            warn!(
                "Warning: Kanari system modules not found at {:?}",
                modules_dir
            );
            warn!("System modules will not be pre-loaded. You may need to publish them manually.");
            warn!("To fix this:");
            warn!("  cd crates/kanari-frameworks/packages/move-stdlib");
            warn!("  cd crates/kanari-frameworks/packages/kanari-system");
            warn!("  kanari move build");
            return Ok(());
        }

        let system_addr = AccountAddress::from_hex_literal(KanariAddress::KANARI_SYSTEM_ADDRESS)?;
        let mut count = 0;

        let modules = topo_sort_modules(discover_modules_in_dir(&modules_dir, system_addr));
        if modules.is_empty() {
            return Err(anyhow::anyhow!(
                "No kanari-system modules (*.mv) found at {:?}",
                modules_dir
            ));
        }
        let (manifest, hash_hex) = compute_framework_manifest_and_hash(&modules);
        eprintln!("âœ“ kanari-system framework hash (disk): {}", hash_hex);
        if let Some(prev) = self.state.get_framework_hash("0x2") {
            if prev != hash_hex {
                warn!(
                    "Warning: system framework hash changed (db: {}, disk: {}). Ensure all validators upgrade together.",
                    prev, hash_hex
                );
            }
        }
        self.state
            .save_framework_manifest("0x2", &manifest, &hash_hex)?;
        if let Ok(expected) = std::env::var("KANARI_FRAMEWORK_EXPECTED_HASH_0X2") {
            if expected != hash_hex {
                return Err(anyhow::anyhow!(
                    "System framework hash mismatch: expected {}, got {}",
                    expected,
                    hash_hex
                ));
            }
        }

        let stdlib_segments = [
            "crates",
            "kanari-frameworks",
            "packages",
            "move-stdlib",
            "build",
            "MoveStdlib",
            "bytecode_modules",
        ];
        let stdlib_dir = find_modules_dir("MOVE_STDLIB_PATH", &stdlib_segments);
        let std_addr = AccountAddress::from_hex_literal(KanariAddress::STD_ADDRESS)?;
        let stdlib_modules = topo_sort_modules(discover_modules_in_dir(&stdlib_dir, std_addr));
        let all_deps: Vec<&CompiledModule> = stdlib_modules
            .iter()
            .map(|m| &m.compiled)
            .chain(modules.iter().map(|m| &m.compiled))
            .collect();
        for m in &modules {
            dependencies::verify_module(&m.compiled, all_deps.iter().copied()).map_err(|e| {
                anyhow::anyhow!("System deps verify failed for {}: {:?}", m.file_name, e)
            })?;
        }

        if std::env::var("KANARI_FRAMEWORK_PRUNE").ok().as_deref() == Some("1") {
            let keep: BTreeSet<ModuleId> = modules.iter().map(|m| m.module_id.clone()).collect();
            for id in self.state.get_all_module_ids().unwrap_or_default() {
                if *id.address() == system_addr && !keep.contains(&id) {
                    if let Err(e) = self.state.delete_module(&id) {
                        warn!("Warning: Failed to prune module {}: {}", id, e);
                    } else if let Ok(mut mods) = self.published_modules.write() {
                        mods.remove(&id);
                    }
                }
            }
        }

        for m in modules {
            let module_file = mv_filename(&m.file_name);
            if let Some(old_bytes) = self.state.get_module(&m.module_id) {
                if old_bytes != m.bytes {
                    let old_compiled = CompiledModule::deserialize_with_defaults(&old_bytes)?;
                    let old_norm = normalized::Module::new(&old_compiled);
                    let new_norm = normalized::Module::new(&m.compiled);
                    if let Err(e) = Compatibility::full_check().check(&old_norm, &new_norm) {
                        if std::env::var("KANARI_FRAMEWORK_ALLOW_INCOMPATIBLE")
                            .ok()
                            .as_deref()
                            != Some("1")
                        {
                            return Err(anyhow::anyhow!(
                                "Incompatible system upgrade for {} (set KANARI_FRAMEWORK_ALLOW_INCOMPATIBLE=1 to override): {:?}",
                                m.module_id,
                                e
                            ));
                        }
                        warn!(
                            "Warning: Allowing incompatible system upgrade for {}: {:?}",
                            m.module_id, e
                        );
                    }
                }
            }
            if let Err(e) = self.state.save_module(&m.module_id, &m.bytes) {
                eprintln!("Warning: Failed to save {}: {}", module_file, e);
                continue;
            }
            if let Ok(mut mods) = self.published_modules.write() {
                mods.insert(m.module_id);
            }
            count += 1;
        }

        eprintln!("✓ Loaded {} kanari-system modules (0x2::*)", count);
        Ok(())
    }
}
