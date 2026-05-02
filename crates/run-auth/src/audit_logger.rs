// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Audit logging for authentication events
//!
//! This module provides comprehensive audit trail logging for all security-related
//! operations including login attempts, password changes, and account modifications.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

/// Types of audit events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    // Authentication Events
    LoginSuccess,
    LoginFailure,
    Logout,
    LogoutAll,
    SessionExpired,

    // Account Management Events
    Registration,
    PasswordChange,
    AccountDeletion,
    AccountLocked,
    AccountUnlocked,

    // Security Events
    EncryptedKeyAccess,
    TransactionSigning,
    RateLimitExceeded,
    SuspiciousActivity,
    TwoFactorSetup,
    TwoFactorEnabled,
    TwoFactorDisabled,
    TwoFactorVerification,
}

impl std::fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditEventType::LoginSuccess => write!(f, "LOGIN_SUCCESS"),
            AuditEventType::LoginFailure => write!(f, "LOGIN_FAILURE"),
            AuditEventType::Logout => write!(f, "LOGOUT"),
            AuditEventType::LogoutAll => write!(f, "LOGOUT_ALL"),
            AuditEventType::SessionExpired => write!(f, "SESSION_EXPIRED"),
            AuditEventType::Registration => write!(f, "REGISTRATION"),
            AuditEventType::PasswordChange => write!(f, "PASSWORD_CHANGE"),
            AuditEventType::AccountDeletion => write!(f, "ACCOUNT_DELETION"),
            AuditEventType::AccountLocked => write!(f, "ACCOUNT_LOCKED"),
            AuditEventType::AccountUnlocked => write!(f, "ACCOUNT_UNLOCKED"),
            AuditEventType::EncryptedKeyAccess => write!(f, "ENCRYPTED_KEY_ACCESS"),
            AuditEventType::TransactionSigning => write!(f, "TRANSACTION_SIGNING"),
            AuditEventType::RateLimitExceeded => write!(f, "RATE_LIMIT_EXCEEDED"),
            AuditEventType::SuspiciousActivity => write!(f, "SUSPICIOUS_ACTIVITY"),
            AuditEventType::TwoFactorSetup => write!(f, "TWO_FACTOR_SETUP"),
            AuditEventType::TwoFactorEnabled => write!(f, "TWO_FACTOR_ENABLED"),
            AuditEventType::TwoFactorDisabled => write!(f, "TWO_FACTOR_DISABLED"),
            AuditEventType::TwoFactorVerification => write!(f, "TWO_FACTOR_VERIFICATION"),
        }
    }
}

/// Severity levels for audit events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for AuditSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditSeverity::Info => write!(f, "INFO"),
            AuditSeverity::Warning => write!(f, "WARNING"),
            AuditSeverity::Error => write!(f, "ERROR"),
            AuditSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Unique event ID
    pub event_id: String,

    /// Timestamp of the event
    pub timestamp: DateTime<Utc>,

    /// Type of event
    pub event_type: AuditEventType,

    /// Severity level
    pub severity: AuditSeverity,

    /// Email address involved (if applicable)
    pub email: Option<String>,

    /// IP address of the client
    pub ip_address: Option<String>,

    /// User agent string
    pub user_agent: Option<String>,

    /// Additional context/data
    pub metadata: serde_json::Value,

    /// Success or failure
    pub success: bool,

    /// Error message if failed
    pub error_message: Option<String>,
}

impl AuditLogEntry {
    /// Create a new audit log entry
    pub fn new(
        event_type: AuditEventType,
        severity: AuditSeverity,
        email: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
        metadata: serde_json::Value,
        success: bool,
        error_message: Option<String>,
    ) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type,
            severity,
            email,
            ip_address,
            user_agent,
            metadata,
            success,
            error_message,
        }
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Convert to human-readable format
    pub fn to_readable(&self) -> String {
        format!(
            "[{}] {} | {} | Email: {} | IP: {} | Success: {}{}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            self.severity,
            self.event_type,
            self.email.as_deref().unwrap_or("N/A"),
            self.ip_address.as_deref().unwrap_or("N/A"),
            self.success,
            if let Some(err) = &self.error_message {
                format!(" | Error: {}", err)
            } else {
                String::new()
            }
        )
    }
}

/// Audit logger that writes to file and console
#[derive(Clone)]
pub struct AuditLogger {
    /// Path to audit log file
    log_path: PathBuf,

    /// Mutex for thread-safe file writing
    file_handle: Arc<Mutex<Option<std::fs::File>>>,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(log_dir: Option<PathBuf>) -> Self {
        let log_dir = log_dir.unwrap_or_else(|| PathBuf::from("logs"));

        // Ensure log directory exists
        if let Err(e) = std::fs::create_dir_all(&log_dir) {
            error!("Failed to create audit log directory: {:?}", e);
        }

        let log_path = log_dir.join(format!("audit_{}.log", Utc::now().format("%Y%m%d")));

        // Initialize file handle
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .ok();

        Self {
            log_path,
            file_handle: Arc::new(Mutex::new(file)),
        }
    }

    /// Log an audit event
    pub async fn log(&self, entry: AuditLogEntry) {
        let readable = entry.to_readable();
        let json = entry.to_json();

        // Log to console with appropriate level
        match entry.severity {
            AuditSeverity::Info => info!("AUDIT: {}", readable),
            AuditSeverity::Warning => {
                tracing::warn!("AUDIT: {}", readable);
            }
            AuditSeverity::Error => {
                tracing::error!("AUDIT: {}", readable);
            }
            AuditSeverity::Critical => {
                tracing::error!("AUDIT CRITICAL: {}", readable);
            }
        }

        // Write to file
        let mut file_guard = self.file_handle.lock().await;
        if let Some(ref mut file) = *file_guard {
            let log_line = format!("{}\n", json);
            if let Err(e) = file.write_all(log_line.as_bytes()) {
                error!("Failed to write audit log: {:?}", e);
            } else if let Err(e) = file.flush() {
                error!("Failed to flush audit log: {:?}", e);
            }
        }
    }

    /// Convenience method for successful events
    pub async fn log_success(
        &self,
        event_type: AuditEventType,
        email: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
        metadata: serde_json::Value,
    ) {
        let entry = AuditLogEntry::new(
            event_type,
            AuditSeverity::Info,
            email,
            ip_address,
            user_agent,
            metadata,
            true,
            None,
        );
        self.log(entry).await;
    }

    /// Convenience method for failed events
    pub async fn log_failure(
        &self,
        event_type: AuditEventType,
        severity: AuditSeverity,
        email: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
        metadata: serde_json::Value,
        error_message: String,
    ) {
        let entry = AuditLogEntry::new(
            event_type,
            severity,
            email,
            ip_address,
            user_agent,
            metadata,
            false,
            Some(error_message),
        );
        self.log(entry).await;
    }

    /// Get the current log file path
    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }
}
