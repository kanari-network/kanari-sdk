// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::reroot_path;
use clap::*;
use std::path::{Path, PathBuf};

/// Format Move source files with the built-in formatter (same rules as the
/// Move on Kanari VS Code extension). Defaults to the package `sources/`
/// and `tests/` directories. Exits non-zero in `--check` mode when any file
/// is unformatted.
#[derive(Parser, Clone, Debug)]
#[clap(name = "fmt")]
pub struct Fmt {
    /// Check formatting without writing files (for CI).
    #[clap(long)]
    pub check: bool,
    /// Files or directories to format (defaults to the package sources and tests).
    pub paths: Vec<PathBuf>,
}

impl Fmt {
    pub fn execute(self) -> anyhow::Result<()> {
        let targets = if self.paths.is_empty() {
            let root = reroot_path(None)?;
            let mut targets = collect_move_files(&root.join("sources"));
            targets.extend(collect_move_files(&root.join("tests")));
            targets
        } else {
            let mut targets = Vec::new();
            for path in &self.paths {
                let absolute = path.canonicalize().map_err(|_| {
                    anyhow::anyhow!("path does not exist: {}", path.display())
                })?;
                if absolute.is_dir() {
                    targets.extend(collect_move_files(&absolute));
                } else if absolute.extension().is_some_and(|ext| ext == "move") {
                    targets.push(absolute);
                } else {
                    anyhow::bail!("not a Move source file: {}", path.display());
                }
            }
            targets.sort();
            targets.dedup();
            targets
        };

        if targets.is_empty() {
            anyhow::bail!("no Move source files found");
        }

        let options = kanari_move_fmt::FormatOptions::default();
        let mut formatted_count = 0usize;
        let mut unformatted = Vec::new();
        for path in &targets {
            let source = std::fs::read_to_string(path)?;
            let formatted = kanari_move_fmt::format_source(&source, &options)?;
            if formatted != source {
                if self.check {
                    unformatted.push(path.clone());
                } else {
                    std::fs::write(path, formatted)?;
                    eprintln!("formatted {}", path.display());
                    formatted_count += 1;
                }
            }
        }

        if self.check {
            if unformatted.is_empty() {
                eprintln!("checked {} file(s): all formatted", targets.len());
                Ok(())
            } else {
                for path in &unformatted {
                    eprintln!("unformatted {}", path.display());
                }
                anyhow::bail!(
                    "checked {} file(s): {} unformatted",
                    targets.len(),
                    unformatted.len()
                );
            }
        } else {
            eprintln!(
                "formatted {formatted_count} of {} file(s)",
                targets.len()
            );
            Ok(())
        }
    }
}

/// Recursively collect `.move` files under `dir` (sorted for determinism).
fn collect_move_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = match std::fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "move") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}
