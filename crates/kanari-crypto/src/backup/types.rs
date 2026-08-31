// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Backup data structures and formatting helpers.

use crate::encryption::EncryptedData;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Backup metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    /// Backup creation timestamp (Unix timestamp)
    pub created_at: u64,
    /// Version of the backup format
    pub version: String,
    /// Number of keys in the backup
    pub key_count: usize,
    /// Whether mnemonic is included
    pub has_mnemonic: bool,
    /// Checksum for verification (HMAC-SHA3-256)
    pub checksum: String,
    /// Optional description
    pub description: Option<String>,
    /// HMAC salt (base64) - v2 uses Argon2-derived key with random salt, None = legacy SHA3
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hmac_salt: Option<String>,
}

impl BackupMetadata {
    /// Create new backup metadata (legacy SHA3 path, hmac_salt None)
    pub fn new(key_count: usize, has_mnemonic: bool, checksum: String) -> Self {
        let timestamp = crate::get_current_timestamp();

        Self {
            created_at: timestamp,
            version: env!("CARGO_PKG_VERSION").to_string(),
            key_count,
            has_mnemonic,
            checksum,
            description: None,
            hmac_salt: None,
        }
    }

    /// Create new backup metadata with Argon2 HMAC salt (v2)
    pub fn new_with_hmac_salt(
        key_count: usize,
        has_mnemonic: bool,
        checksum: String,
        hmac_salt: String,
    ) -> Self {
        let timestamp = crate::get_current_timestamp();
        Self {
            created_at: timestamp,
            version: env!("CARGO_PKG_VERSION").to_string(),
            key_count,
            has_mnemonic,
            checksum,
            description: None,
            hmac_salt: Some(hmac_salt),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Encrypted backup structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedBackup {
    /// Metadata about the backup
    pub metadata: BackupMetadata,
    /// Encrypted keystore data
    pub encrypted_data: EncryptedData,
}

/// Backup information
#[derive(Debug, Clone)]
pub struct BackupInfo {
    /// Path to backup file
    pub path: PathBuf,
    /// Backup metadata
    pub metadata: BackupMetadata,
    /// File size in bytes
    pub file_size: u64,
}

impl BackupInfo {
    /// Get formatted creation time
    pub fn created_at_formatted(&self) -> String {
        chrono::DateTime::from_timestamp(self.metadata.created_at as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| format!("timestamp:{}", self.metadata.created_at))
    }

    /// Get human-readable file size
    pub fn file_size_formatted(&self) -> String {
        let size = self.file_size as f64;
        if size < 1024.0 {
            format!("{:.0} B", size)
        } else if size < 1024.0 * 1024.0 {
            format!("{:.2} KB", size / 1024.0)
        } else {
            format!("{:.2} MB", size / (1024.0 * 1024.0))
        }
    }
}
