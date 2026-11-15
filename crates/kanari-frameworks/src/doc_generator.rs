// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Documentation generator for Move packages
// This module handles generating documentation
// for Move packages using move-docgen and move-errmapgen.
use anyhow::{Context, Result};
use log::{LevelFilter, info};
use move_command_line_common::{
    address::NumericalAddress,
    files::{MOVE_EXTENSION, extension_equals, find_filenames},
};
use std::{collections::BTreeMap, path::PathBuf, time::Instant};

/// Configuration for a Move package documentation
#[derive(Debug, Clone)]
pub struct PackageDocConfig {
    /// Package name (e.g. `KanariGovernance`), used for display and
    /// `--package` filtering.
    pub name: String,
    /// Package directory name (e.g. `kanari-governance`), also accepted by
    /// `--package` filtering.
    pub directory: String,
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
            directory: name.to_string(),
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
        self.named_addresses.insert(name.to_string(), numerical_addr);
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
            anyhow::bail!("Sources directory not found: {:?}", path);
        }
        let files = find_filenames(&[path], |p| extension_equals(p, MOVE_EXTENSION))
            .with_context(|| format!("Failed to find Move files in {}", self.sources_dir))?;
        if files.is_empty() {
            anyhow::bail!("No Move source files found in {}", self.sources_dir);
        }
        Ok(files)
    }
}

/// Time a function and print the elapsed time
fn time_it<F, R>(label: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    let now = Instant::now();
    info!("{}...", label);
    let result = f();
    let elapsed = now.elapsed();
    info!("{} (took {:.2}s)", label, elapsed.as_secs_f64());
    result
}

/// Generate documentation for a Move package
pub fn generate_documentation(config: &PackageDocConfig) -> Result<()> {
    info!("\nGenerating documentation for: {}", config.name);
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let source_files = config.get_source_files()?;

    info!("Found {} Move source file(s)", source_files.len());

    time_it(&format!("Generating {} documentation", config.name), || {
        std::fs::remove_dir_all(&config.docs_dir).ok();
        run_prover(make_docgen_options(config, &source_files)?)
    })?;

    time_it(&format!("Generating {} error map", config.name), || {
        std::fs::remove_file(&config.errmap_file).ok();
        run_prover(make_errmap_options(config, &source_files))
    })?;

    info!("Documentation generated successfully!");
    info!("Docs: {}", config.docs_dir);
    info!("Error map: {}", config.errmap_file);

    Ok(())
}

/// Require a configured doc template to exist. A configured-but-missing
/// template previously degraded silently to generic docs instead of failing.
fn require_template(config: &PackageDocConfig, path: &Option<String>, label: &str) -> Result<()> {
    if let Some(p) = path
        && !PathBuf::from(p).exists()
    {
        anyhow::bail!("{} template not found for {}: {:?}", label, config.name, p);
    }
    Ok(())
}

/// Create prover options for documentation generation
fn make_docgen_options(
    config: &PackageDocConfig,
    sources: &[String],
) -> Result<move_prover::cli::Options> {
    require_template(config, &config.overview_template, "Overview")?;
    require_template(config, &config.references_template, "References")?;

    let templates = match &config.overview_template {
        Some(p) => vec![p.clone()],
        None => anyhow::bail!("Missing overview template for {}", config.name),
    };

    let references = match &config.references_template {
        Some(p) => Some(p.clone()),
        None => anyhow::bail!("Missing references template for {}", config.name),
    };

    Ok(move_prover::cli::Options {
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
    })
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

/// Run move prover with options, returning the model building errors instead
/// of panicking: a failing package aborts `docs` with a proper error message
/// and non-zero exit code (previously an `unwrap()` panicked and skipped the
/// remaining packages without a summary).
fn run_prover(options: move_prover::cli::Options) -> Result<()> {
    options.setup_logging_for_test();
    move_prover::run_move_prover_errors_to_stderr(options)
        .with_context(|| "Move model building failed".to_string())
}
