// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

mod compiler;
mod doc_generator;
mod packages_config;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use doc_generator::{PackageDocConfig, generate_documentation};
use log::{info, warn};
use packages_config::get_package_configs;
use std::{
    env,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(name = "kanari-frameworks")]
#[command(about = "Kanari Package Manager - Compiler & Documentation Generator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile Move kanari-frameworks
    Build {
        /// Package version to compile (default: latest)
        #[arg(long, default_value = "latest")]
        version: String,
    },
    /// Generate documentation for Move kanari-frameworks
    Docs {
        /// Specific package to generate docs for (optional, generates for all if not specified)
        #[arg(long)]
        package: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let packages_dir = get_packages_dir()?;

    match cli.command {
        Commands::Build { version } => build_packages(&packages_dir, version),
        Commands::Docs { package } => generate_docs(&packages_dir, package),
    }
}

/// A directory only counts as the Move packages dir if it actually
/// contains the configured Move packages. A bare existence check is not
/// enough: the repo root also has an unrelated SDK `packages/` directory
/// (`kanari-kotlin`, `kanari_flutter`), which used to shadow the real one
/// and made every build fail with `Directory not found`.
fn looks_like_move_packages_dir(dir: &Path) -> bool {
    dir.is_dir() && dir.join("move-stdlib").is_dir() && dir.join("kanari-system").is_dir()
}

/// Get the packages directory from current working directory.
///
/// Fail if none of the candidate locations is a real Move packages directory
/// instead of returning a non-existent path that would only surface as
/// confusing "Directory not found" errors later.
fn get_packages_dir() -> Result<PathBuf> {
    let current_dir = env::current_dir()?;

    // Candidate locations in order of likelihood:
    // 1. ./packages (when running from crates/kanari-frameworks)
    // 2. ./crates/kanari-frameworks/packages (when running from repo root)
    let candidates =
        [current_dir.join("packages"), current_dir.join("crates/kanari-frameworks/packages")];
    for candidate in &candidates {
        if looks_like_move_packages_dir(candidate) {
            return Ok(candidate.clone());
        }
    }

    anyhow::bail!(
        "Move packages directory not found (looked in: {})",
        candidates.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")
    )
}

/// Print summary of operations
fn print_summary(operation: &str, success: usize, failed: usize) {
    info!("\n{} Summary:", operation);
    info!("   Successful: {}", success);
    if failed > 0 {
        info!("   Failed: {}", failed);
    }
}

fn build_packages(packages_dir: &Path, version: String) -> Result<()> {
    info!("Kanari Package Compiler");
    info!("==========================\n");
    info!("Version: {}\n", version);

    // Place released artifacts in the `kanari-frameworks` crate root (not inside `packages/`).
    // Find nearest ancestor folder named `kanari-frameworks` starting from `packages_dir`.
    // `packages_dir` is already a `&Path`, use it directly.
    let mut ancestor: &Path = packages_dir;
    let mut framework_dir: Option<PathBuf> = None;
    loop {
        if let Some(name) = ancestor.file_name()
            && name == "kanari-frameworks"
        {
            framework_dir = Some(ancestor.to_path_buf());
            break;
        }
        if let Some(p) = ancestor.parent() {
            ancestor = p;
        } else {
            break;
        }
    }

    let framework_dir = framework_dir.with_context(|| {
        format!(
            "Cannot locate a 'kanari-frameworks' ancestor directory above {packages_dir:?}; \
             released artifacts would have no output directory"
        )
    })?;

    let output_dir = framework_dir.join("released");
    info!("Packages: {:?}", packages_dir);
    info!("Output: {:?}\n", output_dir);

    let (success, failed) = process_packages(|config| {
        let package_dir = packages_dir.join(config.directory);
        if !package_dir.exists() {
            warn!("Not found: {:?}\n", package_dir);
            return Err(anyhow::anyhow!("Directory not found"));
        }

        info!("Compiling {} ({})...", config.name, config.address);
        compiler::compile_package(&package_dir, &output_dir, &version, config.address).map(|file| {
            info!("{}", config.name);
            info!("   {:?}\n", file);
        })
    });

    print_summary("Compilation", success, failed);

    // Surface failures via the exit code. Previously this always returned
    // `Ok`, so a failed package build still exited 0 and CI kept going.
    if failed > 0 {
        anyhow::bail!("Compilation failed for {failed} package(s)");
    }
    Ok(())
}

fn generate_docs(packages_dir: &Path, specific_package: Option<String>) -> Result<()> {
    info!("Kanari Documentation Generator");
    info!("==================================\n");

    let mut doc_configs = get_doc_configs(packages_dir)?;

    if let Some(pkg_name) = &specific_package {
        // Match on either the package name (`KanariGovernance`) or the package
        // directory (`kanari-governance`); previously only the directory was
        // stored in `name`, so `--package KanariGovernance` silently matched
        // nothing and exited 0 without generating anything.
        doc_configs
            .retain(|cfg| cfg.name == *pkg_name || cfg.directory.eq_ignore_ascii_case(pkg_name));
        if doc_configs.is_empty() {
            anyhow::bail!("Package not found: {}", pkg_name);
        }
    }

    if doc_configs.is_empty() {
        warn!("No packages configured");
        return Ok(());
    }

    info!("Generating docs for {} package(s)\n", doc_configs.len());

    let (success, failed) = process_doc_configs(doc_configs);
    print_summary("Documentation", success, failed);

    // Surface failures via the exit code, like the build subcommand.
    if failed > 0 {
        anyhow::bail!("Documentation generation failed for {failed} package(s)");
    }
    Ok(())
}

/// Process packages with a given function
fn process_packages<F>(mut process_fn: F) -> (usize, usize)
where
    F: FnMut(&packages_config::PackageConfig) -> Result<()>,
{
    let mut success = 0;
    let mut failed = 0;

    for config in get_package_configs() {
        match process_fn(&config) {
            Ok(_) => success += 1,
            Err(e) => {
                // NOTE: `log` has no logger initialized in this binary, so
                // `error!` would be silently dropped. Print to stderr so
                // failures are always visible (exit code alone hid them).
                eprintln!("{}: {:#}\n", config.name, e);
                failed += 1;
            }
        }
    }

    (success, failed)
}

/// Process documentation configurations
fn process_doc_configs(configs: Vec<PackageDocConfig>) -> (usize, usize) {
    let mut success = 0;
    let mut failed = 0;

    for config in configs {
        match generate_documentation(&config) {
            Ok(_) => success += 1,
            Err(e) => {
                // See note in `process_packages`: keep failures visible.
                eprintln!("{}: {:#}", config.name, e);
                failed += 1;
            }
        }
    }

    (success, failed)
}

fn get_doc_configs(packages_dir: &Path) -> Result<Vec<PackageDocConfig>> {
    let package_configs = get_package_configs();
    let mut doc_configs = Vec::new();

    // Every framework named address, resolved strictly from packages_config.
    // Documenting a package compiles its dependency sources too, so those
    // dependencies' named addresses must be assigned as well; assigning only
    // the package's own address made `kanari_system::*` unbound when
    // documenting `kanari-governance` and failed the whole docs run.
    let mut all_addresses = Vec::new();
    for config in &package_configs {
        all_addresses.push((config.address_name, config.address));
    }

    for config in package_configs {
        let package_path = packages_dir.join(config.directory);
        if !package_path.exists() {
            anyhow::bail!("Package directory not found for {}: {:?}", config.name, package_path);
        }

        let mut doc_config = PackageDocConfig::new(config.name, package_path.to_str().unwrap());

        // Assign every framework address, then record which package owns
        // this config for `--package` filtering.
        for (name, address) in &all_addresses {
            doc_config = doc_config.with_address(name, address)?;
        }
        doc_config.directory = config.directory.to_string();

        // Add dependency source paths for non-stdlib packages. A missing
        // dependency directory is an error, mirroring the build path
        // (`compiler::load_dependency_sources`).
        if !config.is_stdlib() {
            for dep in config.get_dependencies() {
                let dep_path = packages_dir.join(format!("{}/sources", dep));
                if !dep_path.exists() {
                    anyhow::bail!("Dependency sources not found: {:?}", dep_path);
                }
                doc_config = doc_config.with_dependency(dep_path.to_string_lossy().to_string());
            }
        }

        doc_configs.push(doc_config);
    }

    Ok(doc_configs)
}
