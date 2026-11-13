// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::reroot_path;
use clap::Parser;
use kanari_move_audit::{Severity, audit_source};
use std::env;
use std::path::{Path, PathBuf};

const COLOR_RESET: &str = "\x1b[0m";
const COLOR_DIM: &str = "\x1b[2m";

fn severity_color(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "\x1b[31m",
        Severity::High => "\x1b[91m",
        Severity::Medium => "\x1b[33m",
        Severity::Low => "\x1b[32m",
        Severity::Info => "\x1b[34m",
    }
}

/// Audit Move source files for common security issues (reentrancy,
/// missing access control, unsafe borrow_global, hardcoded addresses,
/// undocumented public functions, unused variables).
#[derive(Parser, Clone, Debug)]
#[clap(name = "audit")]
pub struct Audit {
    /// Files or directories to audit (defaults to the package sources and tests).
    pub paths: Vec<PathBuf>,
    /// Only report findings at this severity or higher (info|low|medium|high|critical).
    #[clap(long, default_value = "info")]
    pub min_severity: String,
    /// Emit results as JSON.
    #[clap(long)]
    pub json: bool,
}

impl Audit {
    pub fn execute(self) -> anyhow::Result<()> {
        let min_severity = parse_severity(&self.min_severity)?;

        let targets = if self.paths.is_empty() {
            let root = reroot_path(None)?;
            let mut targets = collect_move_files(&root.join("sources"));
            targets.extend(collect_move_files(&root.join("tests")));
            targets
        } else {
            let mut targets = Vec::new();
            for path in &self.paths {
                let absolute = path
                    .canonicalize()
                    .map_err(|_| anyhow::anyhow!("path does not exist: {}", path.display()))?;
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

        let mut all_findings = Vec::new();
        for path in &targets {
            let source = std::fs::read_to_string(path)?;
            let file = path.to_string_lossy().to_string();
            let findings = audit_source(&source, &file);
            all_findings.extend(findings);
        }

        // Filter by minimum severity and sort by file/line.
        all_findings.retain(|f| f.severity >= min_severity);
        all_findings.sort_by_key(|f| (f.file.clone(), f.line, f.column));

        if self.json {
            let json = serde_json::to_string_pretty(&all_findings)?;
            eprintln!("{json}");
        } else {
            if all_findings.is_empty() {
                eprintln!(
                    "checked {} file(s): no findings (min severity {})",
                    targets.len(),
                    self.min_severity
                );
                return Ok(());
            }
            let max_sev = all_findings.iter().map(|f| f.severity).max().unwrap_or(Severity::Info);
            let color = severity_color(max_sev);
            eprintln!(
                "{color}checked {files} file(s): {count} finding(s){reset} (min severity {sev})",
                color = color,
                files = targets.len(),
                count = all_findings.len(),
                reset = COLOR_RESET,
                sev = self.min_severity
            );

            // Group findings by severity for readability.
            let mut by_severity: Vec<(Severity, Vec<&kanari_move_audit::AuditFinding>)> = vec![
                (Severity::Critical, Vec::new()),
                (Severity::High, Vec::new()),
                (Severity::Medium, Vec::new()),
                (Severity::Low, Vec::new()),
                (Severity::Info, Vec::new()),
            ];
            for f in &all_findings {
                if let Some((_, group)) = by_severity.iter_mut().find(|(s, _)| *s == f.severity) {
                    group.push(f);
                }
            }

            for (sev, group) in by_severity {
                if group.is_empty() {
                    continue;
                }
                eprintln!();
                let scolor = severity_color(sev);
                eprintln!(
                    "{scolor} [ {sev} ({count} finding) ] {reset}",
                    scolor = scolor,
                    sev = sev,
                    count = group.len(),
                    reset = COLOR_RESET
                );
                for f in group {
                    let cwd = env::current_dir().unwrap_or_default();
                    let cwd_str = cwd.to_str().unwrap_or("");
                    let rel = f.file.as_str().strip_prefix(cwd_str).unwrap_or(&f.file);
                    let fcolor = severity_color(f.severity);
                    eprintln!(
                        "  [{fcolor}{sev}{reset}] {rel}:{line}:{col} -- {title} ({rule})",
                        fcolor = fcolor,
                        sev = f.severity,
                        reset = COLOR_RESET,
                        line = f.line,
                        col = f.column,
                        title = f.title,
                        rule = f.rule,
                    );
                    if let Some(snippet) = &f.code_snippet {
                        eprintln!(
                            "      {dim}{snippet}{reset}",
                            dim = COLOR_DIM,
                            reset = COLOR_RESET
                        );
                    }
                    eprintln!(
                        "      {dim}remediation: {reset}{}",
                        f.description,
                        dim = COLOR_DIM,
                        reset = COLOR_RESET
                    );
                }
            }

            anyhow::bail!("audit failed: {} finding(s)", all_findings.len());
        }

        Ok(())
    }
}

fn parse_severity(s: &str) -> anyhow::Result<Severity> {
    match s.to_lowercase().as_str() {
        "info" => Ok(Severity::Info),
        "low" => Ok(Severity::Low),
        "medium" => Ok(Severity::Medium),
        "high" => Ok(Severity::High),
        "critical" => Ok(Severity::Critical),
        other => anyhow::bail!("invalid min-severity '{other}'"),
    }
}

/// Recursively collect `.move` files under `dir` (sorted for determinism).
fn collect_move_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
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
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_severity_works() {
        assert_eq!(parse_severity("info").unwrap(), Severity::Info);
        assert_eq!(parse_severity("critical").unwrap(), Severity::Critical);
        assert!(parse_severity("bogus").is_err());
    }
}
