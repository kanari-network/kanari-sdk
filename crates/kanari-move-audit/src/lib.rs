// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! `kanari-move-audit`: static security auditing for Move source code.
//!
//! Parse Move files with tree-sitter and run a set of security detectors,
//! producing `AuditFinding`s. Used by the `kanari move audit` subcommand and
//! usable as a library.

pub mod detectors;
pub mod finding;

pub use detectors::{audit_directory, audit_source};
pub use finding::{AuditFinding, AuditSummary, RuleId, Severity};

/// Audit a single Move source string, returning findings and a summary.
pub fn audit_source_with_summary(source: &str, file: &str) -> (Vec<AuditFinding>, AuditSummary) {
    let findings = audit_source(source, file);
    let summary = AuditSummary::new(1, findings.clone());
    (findings, summary)
}

/// Audit every `.move` file under `dir`, returning findings and a summary.
pub fn audit_directory_with_summary(dir: &std::path::Path) -> (Vec<AuditFinding>, AuditSummary) {
    let findings = audit_directory(dir);
    let summary = AuditSummary::new(count_move_files(dir), findings.clone());
    (findings, summary)
}

/// Count `.move` files under `dir` (recursively).
fn count_move_files(dir: &std::path::Path) -> usize {
    let mut count = 0usize;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "move") {
                count += 1;
            }
        }
    }
    count
}
