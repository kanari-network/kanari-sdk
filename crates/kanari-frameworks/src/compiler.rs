use anyhow::{Context, Result};
use kanari_types::address::Address;
use move_command_line_common::address::NumericalAddress;
use move_compiler::{Compiler, Flags};
use move_symbol_pool::Symbol;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::packages_config::get_package_configs;

/// Kanari Package Data (JSON format)
#[derive(Serialize, Deserialize)]
pub struct KanariPackage {
    pub package: String,
    pub version: String,
    pub modules: Vec<ModuleData>,
    pub timestamp: u64,
}

/// Module data with hex-encoded bytecode
#[derive(Serialize, Deserialize)]
pub struct ModuleData {
    pub name: String,
    pub address: String,
    #[serde(with = "hex_serde")]
    pub bytecode: Vec<u8>,
}

/// Hex serialization for bytecode
mod hex_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        hex::decode(&s).map_err(serde::de::Error::custom)
    }
}

/// Compile Move package and create .rpd file
pub fn compile_package(
    package_dir: &Path,
    output_dir: &Path,
    version: &str,
    address: &str,
) -> Result<PathBuf> {
    println!("📦 Compiling: {:?}", package_dir);

    // Validate address format first
    NumericalAddress::parse_str(address)
        .map_err(|e| anyhow::anyhow!("Invalid address '{}': {}", address, e))?;

    let sources_dir = package_dir.join("sources");
    if !sources_dir.exists() {
        anyhow::bail!("Sources directory not found: {:?}", sources_dir);
    }

    let package_name = get_package_name(package_dir)?;
    let source_files = collect_move_files(&sources_dir)?;
    let dependencies = if is_stdlib(address)? {
        Vec::new()
    } else {
        load_stdlib_dependencies(package_dir)?
    };

    println!(
        "  Package: {} | Sources: {} | Deps: {}",
        package_name,
        source_files.len(),
        dependencies.len()
    );

    // Compile Move sources
    let compiled_modules = compile_move_source(source_files, dependencies, get_named_addresses())?;

    println!("  ✓ Compiled {} modules", compiled_modules.len());

    // Create package data
    let package = KanariPackage {
        package: package_name.clone(),
        version: version.to_string(),
        modules: compiled_modules,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    // Create output directory structure: output_dir/version/address/
    let version_dir = output_dir.join(version);
    let address_dir = version_dir.join(address);
    fs::create_dir_all(&address_dir)?;

    // Write .rpd file as package.rpd
    let output_file = address_dir.join("package.rpd");
    let json_data = serde_json::to_string_pretty(&package)?;
    fs::write(&output_file, json_data)?;

    println!("  ✓ Created: {:?}", output_file);

    Ok(output_file)
}

/// Compile Move source files to bytecode
fn compile_move_source(
    source_files: Vec<PathBuf>,
    dependencies: Vec<PathBuf>,
    named_addresses: BTreeMap<Symbol, NumericalAddress>,
) -> Result<Vec<ModuleData>> {
    let to_symbols = |paths: &[PathBuf]| {
        paths
            .iter()
            .map(|p| Symbol::from(p.to_string_lossy().as_ref()))
            .collect()
    };

    let (_files, compiled_units) = Compiler::from_files(
        None,
        to_symbols(&source_files),
        to_symbols(&dependencies),
        named_addresses,
    )
    .set_flags(Flags::empty())
    .build_and_report()
    .context("Move compilation failed")?;

    compiled_units
        .into_iter()
        .map(|unit| {
            let module = &unit.into_compiled_unit().module;
            let module_id = module.self_id();
            let mut bytecode = Vec::new();
            module
                .serialize(&mut bytecode)
                .context("Failed to serialize module")?;

            Ok(ModuleData {
                name: module_id.name().to_string(),
                address: format!("{}", module_id.address()),
                bytecode,
            })
        })
        .collect()
}

/// Get package name from Move.toml or directory name
fn get_package_name(package_dir: &Path) -> Result<String> {
    let move_toml = package_dir.join("Move.toml");
    if move_toml.exists() {
        let content = fs::read_to_string(&move_toml)?;
        if let Some(name) = parse_package_name(&content) {
            return Ok(name);
        }
    }
    Ok(package_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string())
}

/// Collect all .move files from directory
fn collect_move_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("move") {
            files.push(path);
        }
    }
    if files.is_empty() {
        anyhow::bail!("No Move source files found in {:?}", dir);
    }
    Ok(files)
}

/// Check if address is stdlib (0x1)
fn is_stdlib(address: &str) -> Result<bool> {
    let addr = NumericalAddress::parse_str(address)
        .map_err(|e| anyhow::anyhow!("Failed to parse address '{}': {}", address, e))?;
    let stdlib_addr =
        NumericalAddress::parse_str(Address::STD_ADDRESS).expect("Invalid stdlib address constant");
    Ok(addr == stdlib_addr)
}

/// Load stdlib dependencies
fn load_stdlib_dependencies(package_dir: &Path) -> Result<Vec<PathBuf>> {
    let stdlib_dir = package_dir.join("../move-stdlib/sources");
    if stdlib_dir.exists() {
        collect_move_files(&stdlib_dir)
    } else {
        Ok(Vec::new())
    }
}

/// Get standard named addresses from packages_config
fn get_named_addresses() -> BTreeMap<Symbol, NumericalAddress> {
    get_package_configs()
        .iter()
        .filter_map(|config| {
            NumericalAddress::parse_str(config.address)
                .ok()
                .map(|addr| (Symbol::from(config.address_name), addr))
        })
        .collect()
}

/// Parse package name from Move.toml (handles comments)
fn parse_package_name(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.split('#').next()?.trim();
        if line.starts_with("name") {
            if let Some(name_part) = line.split('=').nth(1) {
                return Some(
                    name_part
                        .trim()
                        .trim_matches(|c| c == '"' || c == '\'')
                        .to_string(),
                );
            }
        }
    }
    None
}
