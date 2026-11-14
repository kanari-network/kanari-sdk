// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Data types for Move code audit findings.

use serde::Serialize;
use std::fmt;

/// Severity level of an audit finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Severity {
    /// Informational note, not a vulnerability.
    Info,
    /// Low-severity issue (style, minor risk).
    Low,
    /// Medium-severity issue (should be reviewed).
    Medium,
    /// High-severity issue (likely exploitable).
    High,
    /// Critical-severity issue (severe risk, fix immediately).
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Info => write!(f, "INFO"),
            Severity::Low => write!(f, "LOW"),
            Severity::Medium => write!(f, "MEDIUM"),
            Severity::High => write!(f, "HIGH"),
            Severity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Unique identifier for an audit rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum RuleId {
    EntryWithoutAuth,
    ExternalCall,
    StateChangeBeforeExternalCall,
    MissingAccessControl,
    UnsafePublicTransfer,
    BorrowGlobalWithoutCheck,
    HardcodedAddress,
    PublicEntryWithoutSigner,
    UnusedVariable,
    MissingDoc,
    IntegerOverflowRisk,
    UnsafeNativeCall,
    UncheckedExternalCall,
    CapabilityLeak,
    ShareObjectWithoutAuth,
    DivisionByZeroRisk,
    UncheckedVectorBorrow,
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuleId::EntryWithoutAuth => write!(f, "entry-without-auth"),
            RuleId::ExternalCall => write!(f, "external-call"),
            RuleId::StateChangeBeforeExternalCall => write!(f, "state-change-before-external-call"),
            RuleId::MissingAccessControl => write!(f, "missing-access-control"),
            RuleId::UnsafePublicTransfer => write!(f, "unsafe-public-transfer"),
            RuleId::BorrowGlobalWithoutCheck => write!(f, "borrow-global-without-check"),
            RuleId::HardcodedAddress => write!(f, "hardcoded-address"),
            RuleId::PublicEntryWithoutSigner => write!(f, "public-entry-without-signer"),
            RuleId::UnusedVariable => write!(f, "unused-variable"),
            RuleId::MissingDoc => write!(f, "missing-doc"),
            RuleId::IntegerOverflowRisk => write!(f, "integer-overflow-risk"),
            RuleId::UnsafeNativeCall => write!(f, "unsafe-native-call"),
            RuleId::UncheckedExternalCall => write!(f, "unchecked-external-call"),
            RuleId::CapabilityLeak => write!(f, "capability-leak"),
            RuleId::ShareObjectWithoutAuth => write!(f, "share-object-without-auth"),
            RuleId::DivisionByZeroRisk => write!(f, "division-by-zero-risk"),
            RuleId::UncheckedVectorBorrow => write!(f, "unchecked-vector-borrow"),
        }
    }
}

/// A single audit finding produced while scanning Move source.
#[derive(Clone, Debug, Serialize)]
pub struct AuditFinding {
    /// The rule that was triggered.
    pub rule: RuleId,
    /// Severity of the finding.
    pub severity: Severity,
    /// Short human-readable title.
    pub title: String,
    /// Detailed description of the issue and how to remediate.
    pub description: String,
    /// File path where the finding was detected.
    pub file: String,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number.
    pub column: usize,
    /// The offending source line (trimmed).
    pub code_snippet: Option<String>,
}

impl AuditFinding {
    pub fn new(
        rule: RuleId,
        severity: Severity,
        title: impl Into<String>,
        description: impl Into<String>,
        file: impl Into<String>,
        line: usize,
        column: usize,
        code_snippet: Option<String>,
    ) -> Self {
        Self {
            rule,
            severity,
            title: title.into(),
            description: description.into(),
            file: file.into(),
            line,
            column,
            code_snippet,
        }
    }
}

/// Summary of an audit run across files.
#[derive(Clone, Debug, Default, Serialize)]
pub struct AuditSummary {
    pub files_scanned: usize,
    pub findings: Vec<AuditFinding>,
    pub counts_by_severity: [usize; 5],
}

impl AuditSummary {
    pub fn new(files_scanned: usize, findings: Vec<AuditFinding>) -> Self {
        let mut counts = [0usize; 5];
        for f in &findings {
            let idx = match f.severity {
                Severity::Info => 0,
                Severity::Low => 1,
                Severity::Medium => 2,
                Severity::High => 3,
                Severity::Critical => 4,
            };
            counts[idx] += 1;
        }
        Self { files_scanned, findings, counts_by_severity: counts }
    }

    pub fn has_findings(&self) -> bool {
        !self.findings.is_empty()
    }

    pub fn critical_count(&self) -> usize {
        self.counts_by_severity[4]
    }

    pub fn high_count(&self) -> usize {
        self.counts_by_severity[3]
    }

    pub fn medium_count(&self) -> usize {
        self.counts_by_severity[2]
    }

    pub fn low_count(&self) -> usize {
        self.counts_by_severity[1]
    }

    pub fn info_count(&self) -> usize {
        self.counts_by_severity[0]
    }
}
