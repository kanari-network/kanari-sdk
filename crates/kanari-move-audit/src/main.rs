// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Standalone `kanari-move-audit` binary.
//!
//! Usage: kanari-move-audit <path-to-move-file-or-directory>

use kanari_move_audit::{audit_directory_with_summary, audit_source_with_summary};
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: kanari-move-audit <path-to-move-file-or-directory>");
        std::process::exit(2);
    }
    let path = Path::new(&args[1]);

    let (_findings, summary) = if path.is_dir() {
        audit_directory_with_summary(path)
    } else {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("failed to read {}: {e}", path.display());
                std::process::exit(1);
            }
        };
        let (f, s) = audit_source_with_summary(&source, &path.to_string_lossy());
        (f, s)
    };

    if summary.has_findings() {
        eprintln!(
            "Audit of {} file(s): {} finding(s)  (critical={} high={} medium={} low={} info={})",
            summary.files_scanned,
            summary.findings.len(),
            summary.critical_count(),
            summary.high_count(),
            summary.medium_count(),
            summary.low_count(),
            summary.info_count(),
        );
        for f in &summary.findings {
            eprintln!("  [{}] {}:{} -- {} ({})", f.severity, f.file, f.line, f.title, f.rule);
            if let Some(snippet) = &f.code_snippet {
                eprintln!("      {snippet}");
            }
        }
        std::process::exit(1);
    } else {
        eprintln!("Audit clean: {} file(s), no findings.", summary.files_scanned);
    }
}
