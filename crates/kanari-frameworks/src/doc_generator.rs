// Copyright (c) Kanari Team
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use log::LevelFilter;
use move_command_line_common::{
    address::NumericalAddress,
    files::{MOVE_EXTENSION, extension_equals, find_filenames},
};
use std::{collections::BTreeMap, path::PathBuf, time::Instant};

/// Configuration for a Move package documentation
#[derive(Debug, Clone)]
pub struct PackageDocConfig {
    pub name: String,
    pub sources_dir: String,
    pub docs_dir: String,
    pub errmap_file: String,
    pub overview_template: Option<String>,
    pub references_template: Option<String>,
    pub named_addresses: BTreeMap<String, NumericalAddress>,
    pub dependencies: Vec<String>,
}

impl PackageDocConfig {
    /// Create a new package documentation configuration
    pub fn new(name: &str, base_dir: &str) -> Self {
        Self {
            name: name.to_string(),
            sources_dir: format!("{}/sources", base_dir),
            docs_dir: format!("{}/docs", base_dir),
            errmap_file: format!("{}/error_description.errmap", base_dir),
            overview_template: Some(format!("{}/doc_templates/overview.md", base_dir)),
            references_template: Some(format!("{}/doc_templates/references.md", base_dir)),
            named_addresses: BTreeMap::new(),
            dependencies: vec![],
        }
    }

    /// Add a named address
    pub fn with_address(mut self, name: &str, address: &str) -> Result<Self> {
        let numerical_addr = NumericalAddress::parse_str(address)
            .map_err(|e| anyhow::anyhow!("Failed to parse address '{}': {}", address, e))?;
        self.named_addresses
            .insert(name.to_string(), numerical_addr);
        Ok(self)
    }

    /// Add a dependency path
    pub fn with_dependency(mut self, dep_path: String) -> Self {
        self.dependencies.push(dep_path);
        self
    }

    /// Get all Move source files
    pub fn get_source_files(&self) -> Result<Vec<String>> {
        let path = PathBuf::from(&self.sources_dir);
        if !path.exists() {
            return Ok(vec![]);
        }
        find_filenames(&[path], |p| extension_equals(p, MOVE_EXTENSION))
            .with_context(|| format!("Failed to find Move files in {}", self.sources_dir))
    }
}

/// Time a function and print the elapsed time
fn time_it<F, R>(label: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    let now = Instant::now();
    println!("⏳ {}...", label);
    let result = f();
    let elapsed = now.elapsed();
    println!("✓ {} (took {:.2}s)", label, elapsed.as_secs_f64());
    result
}

/// Generate documentation for a Move package
pub fn generate_documentation(config: &PackageDocConfig) -> Result<()> {
    println!("\n📚 Generating documentation for: {}", config.name);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let source_files = config.get_source_files()?;
    if source_files.is_empty() {
        println!("⚠️  No Move source files found in {}", config.sources_dir);
        return Ok(());
    }

    println!("📝 Found {} Move source file(s)", source_files.len());

    time_it(&format!("Generating {} documentation", config.name), || {
        std::fs::remove_dir_all(&config.docs_dir).ok();
        run_prover(make_docgen_options(config, &source_files))
    });

    time_it(&format!("Generating {} error map", config.name), || {
        std::fs::remove_file(&config.errmap_file).ok();
        run_prover(make_errmap_options(config, &source_files))
    });

    println!("✅ Documentation generated successfully!");
    println!("  📁 Docs: {}", config.docs_dir);
    println!("  📋 Error map: {}", config.errmap_file);

    Ok(())
}

/// Create prover options for documentation generation
fn make_docgen_options(config: &PackageDocConfig, sources: &[String]) -> move_prover::cli::Options {
    let templates = config
        .overview_template
        .as_ref()
        .filter(|p| PathBuf::from(p).exists())
        .map(|p| vec![p.clone()])
        .unwrap_or_default();

    let references = config
        .references_template
        .as_ref()
        .filter(|p| PathBuf::from(p).exists())
        .cloned();

    move_prover::cli::Options {
        move_sources: sources.to_vec(),
        move_deps: config.dependencies.clone(),
        move_named_address_values: move_prover::cli::named_addresses_for_options(
            &config.named_addresses,
        ),
        verbosity_level: LevelFilter::Warn,
        run_docgen: true,
        docgen: move_docgen::DocgenOptions {
            root_doc_templates: templates,
            references_file: references,
            output_directory: config.docs_dir.clone(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Create prover options for error map generation
fn make_errmap_options(config: &PackageDocConfig, sources: &[String]) -> move_prover::cli::Options {
    move_prover::cli::Options {
        move_sources: sources.to_vec(),
        move_deps: config.dependencies.clone(),
        move_named_address_values: move_prover::cli::named_addresses_for_options(
            &config.named_addresses,
        ),
        verbosity_level: LevelFilter::Warn,
        run_errmapgen: true,
        errmapgen: move_errmapgen::ErrmapOptions {
            output_file: config.errmap_file.clone(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Run move prover with options
fn run_prover(options: move_prover::cli::Options) {
    options.setup_logging_for_test();
    move_prover::run_move_prover_errors_to_stderr(options).unwrap();
}
