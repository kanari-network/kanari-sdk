mod compiler;
mod doc_generator;
mod packages_config;

use anyhow::Result;
use clap::{Parser, Subcommand};
use doc_generator::{PackageDocConfig, generate_documentation};
use kanari_types::address::Address;
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

/// Get the packages directory from current working directory
fn get_packages_dir() -> Result<PathBuf> {
    let current_dir = env::current_dir()?;

    // Try common candidate locations in order of likelihood:
    // 1. ./packages (when running from repo root)
    // 2. ./packages (when running from crates/kanari-frameworks)
    // 3. ./crates/kanari-frameworks/packages (when running from repo root)
    let candidate1 = current_dir.join("packages");
    if candidate1.exists() && candidate1.is_dir() {
        return Ok(candidate1);
    }

    // If running from crates/kanari-frameworks, packages are in ./packages
    if current_dir.ends_with("crates/kanari-frameworks")
        || current_dir.ends_with("kanari-frameworks")
    {
        let cand = current_dir.join("packages");
        if cand.exists() && cand.is_dir() {
            return Ok(cand);
        }
    }

    let candidate2 = current_dir.join("crates/kanari-frameworks/packages");
    if candidate2.exists() && candidate2.is_dir() {
        return Ok(candidate2);
    }

    // Fallback: return the conventional path (may not exist) so caller can report clear errors
    Ok(candidate2)
}

/// Print summary of operations
fn print_summary(operation: &str, success: usize, failed: usize) {
    println!("\n✨ {} Summary:", operation);
    println!("   ✅ Successful: {}", success);
    if failed > 0 {
        println!("   ❌ Failed: {}", failed);
    }
}

fn build_packages(packages_dir: &Path, version: String) -> Result<()> {
    println!("🚀 Kanari Package Compiler");
    println!("==========================\n");
    println!("📌 Version: {}\n", version);

    // Place released artifacts in the `kanari-frameworks` crate root (not inside `packages/`).
    // Find nearest ancestor folder named `kanari-frameworks` starting from `packages_dir`.
    // `packages_dir` is already a `&Path`, use it directly.
    let mut ancestor: &Path = packages_dir;
    let mut framework_dir: Option<PathBuf> = None;
    loop {
        if let Some(name) = ancestor.file_name() {
            if name == "kanari-frameworks" {
                framework_dir = Some(ancestor.to_path_buf());
                break;
            }
        }
        if let Some(p) = ancestor.parent() {
            ancestor = p;
        } else {
            break;
        }
    }

    let framework_dir = framework_dir.unwrap_or_else(|| {
        // Fallback to packages_dir parent (best-effort)
        packages_dir
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| packages_dir.to_path_buf())
    });

    let output_dir = framework_dir.join("released");
    println!("📁 Packages: {:?}", packages_dir);
    println!("📁 Output: {:?}\n", output_dir);

    let (success, failed) = process_packages(|config| {
        let package_dir = packages_dir.join(config.directory);
        if !package_dir.exists() {
            eprintln!("⚠️  Not found: {:?}\n", package_dir);
            return Err(anyhow::anyhow!("Directory not found"));
        }

        println!("Compiling {} ({})...", config.name, config.address);
        compiler::compile_package(&package_dir, &output_dir, &version, config.address).map(|file| {
            println!("✅ {}", config.name);
            println!("   {:?}\n", file);
        })
    });

    print_summary("Compilation", success, failed);

    Ok(())
}

fn generate_docs(packages_dir: &Path, specific_package: Option<String>) -> Result<()> {
    println!("📚 Kanari Documentation Generator");
    println!("==================================\n");

    let mut doc_configs = get_doc_configs(packages_dir)?;

    if let Some(pkg_name) = &specific_package {
        doc_configs.retain(|cfg| cfg.name == *pkg_name);
        if doc_configs.is_empty() {
            eprintln!("❌ Package not found: {}", pkg_name);
            return Ok(());
        }
    }

    if doc_configs.is_empty() {
        eprintln!("❌ No packages configured");
        return Ok(());
    }

    println!("📦 Generating docs for {} package(s)\n", doc_configs.len());

    let (success, failed) = process_doc_configs(doc_configs);
    print_summary("Documentation", success, failed);

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
                eprintln!("❌ {}: {}\n", config.name, e);
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
                eprintln!("❌ {}: {}", config.name, e);
                failed += 1;
            }
        }
    }

    (success, failed)
}

fn get_doc_configs(packages_dir: &Path) -> Result<Vec<PackageDocConfig>> {
    let package_configs = get_package_configs();
    let mut doc_configs = Vec::new();

    for config in package_configs {
        let package_path = packages_dir.join(config.directory);
        if !package_path.exists() {
            continue;
        }

        let mut doc_config =
            PackageDocConfig::new(config.directory, package_path.to_str().unwrap());

        // Add address mapping using config method
        doc_config = doc_config.with_address(config.address_name, config.address)?;

        // Add stdlib dependency for non-stdlib packages
        if !config.is_stdlib() {
            doc_config = doc_config.with_address("std", Address::STD_ADDRESS)?;

            // Add dependency paths
            for dep in config.get_dependencies() {
                let dep_path = packages_dir.join(format!("{}/sources", dep));
                if dep_path.exists() {
                    doc_config = doc_config.with_dependency(dep_path.to_string_lossy().to_string());
                }
            }
        }

        doc_configs.push(doc_config);
    }

    Ok(doc_configs)
}
