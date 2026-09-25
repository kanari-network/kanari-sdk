// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! `kanari move prove`: run the Move Prover analysis pipeline over a package.
//!
//! What this does today: builds the prover model for the package and runs the
//! static-analysis pipeline over it (borrow checking, memory instrumentation,
//! loop analysis, number-operation analysis). This catches model-level issues
//! that the bytecode verifier does not look for.
//!
//! What it does NOT do: discharge verification conditions with an SMT solver.
//! Two blockers, both outside this command:
//! 1. This toolchain's Move compiler marks MSL `spec` blocks as deprecated
//!    and drops them (`move-compiler/src/expansion/translate.rs`), so there
//!    are no specifications for a solver to check. The command therefore
//!    REFUSES packages containing `spec` blocks instead of silently ignoring
//!    them (see [`reject_ignored_specs`] — a silent pass would be a lie).
//! 2. The vendored prover tree has no Boogie backend crate, so even restored
//!    specs could not reach Z3/CVC5. Restoring spec support through
//!    expansion into move-model, then vendoring the backend + solvers, is the
//!    follow-up that makes this command a real prover.

use super::reroot_path;
use anyhow::{Context, Result};
use clap::*;
use move_package::BuildConfig;
use std::path::PathBuf;

/// Run Move Prover static analyses over this package.
///
/// Fails if the package contains MSL `spec` blocks: this toolchain drops
/// them with a deprecation warning, so "proving" them would be vacuous.
/// Remove the blocks (or restore spec support in the compiler) and rerun.
#[derive(Parser, Clone)]
#[clap(name = "prove")]
pub struct Prove {
    #[clap(flatten)]
    pub build_config: BuildConfig,
    /// Only translate to stackless bytecode and dump it; skip analysis.
    /// Useful for inspecting what the prover sees, no solvers involved.
    #[clap(long = "generate-only")]
    pub generate_only: bool,
    /// Dump disassembled bytecode next to the output path.
    #[clap(long = "dump-bytecode")]
    pub dump_bytecode: bool,
    /// Path used for prover output files (dumps). Defaults to `output.bpl`.
    #[clap(long = "output", default_value = "output.bpl")]
    pub output: String,
    /// Verbose prover logging.
    #[clap(long = "verbose", short = 'v')]
    pub verbose: bool,
}

impl Prove {
    pub fn execute(self, path: Option<PathBuf>) -> Result<()> {
        let rerooted_path = reroot_path(path)?;
        let mut options = move_prover::cli::Options::default();

        // Resolve the package graph exactly like `kanari move test` does, so
        // named addresses and dependency sources always agree with the build.
        let mut build_config = self.build_config.clone();
        build_config.test_mode = true;
        build_config.dev_mode = true;
        let resolution_graph =
            build_config.resolution_graph_for_package(&rerooted_path, &mut Vec::new())?;

        let root_package = resolution_graph.root_package().to_string();
        let mut target_sources = Vec::new();
        let mut dep_sources = Vec::new();
        for (package_name, package) in resolution_graph.package_table.iter() {
            let mut sources: Vec<String> = package
                .get_sources(&resolution_graph.build_options)
                .map_err(|err| anyhow::anyhow!("{err:?}"))
                .with_context(|| {
                    format!("Failed to collect Move sources for package {package_name}")
                })?
                .iter()
                .map(|source| source.as_str().to_string())
                .collect();
            if package_name.to_string() == root_package {
                target_sources.append(&mut sources);
            } else {
                dep_sources.append(&mut sources);
            }
        }
        if target_sources.is_empty() {
            anyhow::bail!("No Move sources found for package {root_package}");
        }
        reject_ignored_specs(&target_sources)?;

        options.move_sources = target_sources;
        options.move_deps = dep_sources;
        options.move_named_address_values = resolution_graph
            .extract_named_address_mapping()
            .map(|(name, addr)| {
                (
                    name.to_string(),
                    addr.into_bytes()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>(),
                )
            })
            .map(|(name, hex)| format!("{name}=0x{hex}"))
            .collect();
        options.output_path = self.output.clone();
        options.prover.generate_only = self.generate_only;
        options.prover.dump_bytecode = self.dump_bytecode;
        if self.verbose {
            options.verbosity_level = log::LevelFilter::Debug;
        }

        move_prover::run_move_prover_errors_to_stderr(options)?;
        eprintln!(
            "Move Prover pipeline passed for package '{root_package}': model built, \
             borrow/memory/loop analyses clean.\n\
             Note: SMT discharge (Boogie/Z3) is not wired yet — see module docs."
        );
        Ok(())
    }
}

/// Refuse packages containing MSL `spec` blocks.
///
/// The vendored compiler parses them only to emit "deprecated and no longer
/// used" and drops them before move-model ever sees them. Passing such a
/// package to the prover would print success while checking nothing, so fail
/// loudly instead. Matches lines where `spec` opens a block; ordinary
/// identifiers cannot be the bare keyword `spec`.
fn reject_ignored_specs(target_sources: &[String]) -> Result<()> {
    let mut offenders = Vec::new();
    for source in target_sources {
        let contents = std::fs::read_to_string(source)
            .with_context(|| format!("Failed to read Move source {source}"))?;
        for (index, line) in contents.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("spec ") || trimmed.starts_with("spec{") {
                offenders.push(format!("{source}:{}: {}", index + 1, line.trim()));
            }
        }
    }
    if offenders.is_empty() {
        return Ok(());
    }
    anyhow::bail!(
        "Refusing to prove: {} `spec` block(s) found, but this toolchain drops \
         specification blocks as deprecated (they would be silently ignored):\n{}\n\
         Remove them, or restore spec support through expansion into move-model \
         plus a Boogie backend, then rerun.",
        offenders.len(),
        offenders.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::reject_ignored_specs;

    #[test]
    fn rejects_spec_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.move");
        std::fs::write(&file, "module 0x1::a {\n    spec module {\n    }\n}\n").unwrap();
        let err = reject_ignored_specs(&[file.to_string_lossy().to_string()]).unwrap_err();
        assert!(err.to_string().contains("spec"));
    }

    #[test]
    fn allows_identifiers_containing_spec() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("b.move");
        std::fs::write(
            &file,
            "module 0x1::b {\n    fun inspect(x: u64): u64 { x }\n    fun special(): u64 { 1 }\n}\n",
        )
        .unwrap();
        reject_ignored_specs(&[file.to_string_lossy().to_string()]).unwrap();
    }
}
