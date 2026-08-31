// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Backup and restore functionality for keystore
//!
//! This module provides secure backup and restore capabilities for the keystore,
//! including encryption and verification.

use std::fs::{self};
use std::io::{self};
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::Keystore;
use crate::encryption::{decrypt_data, encrypt_data};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{KeyInit, Mac, SimpleHmac};
use rand::{TryRng, rngs::SysRng};
use sha3::{Digest, Sha3_256};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

mod types;
pub use types::{BackupInfo, BackupMetadata, EncryptedBackup};

type HmacSha3_256 = SimpleHmac<Sha3_256>;

/// Errors related to backup/restore operations
#[derive(Error, Debug)]
pub enum BackupError {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("Invalid backup format")]
    InvalidFormat,

    #[error("Backup verification failed: {0}")]
    VerificationFailed(String),

    #[error("Keystore error: {0}")]
    KeystoreError(String),

    #[error("Backup file not found: {0}")]
    NotFound(String),
}

/// Backup manager
pub struct BackupManager {
    backup_dir: PathBuf,
}

impl Default for BackupManager {
    fn default() -> Self {
        let mut backup_dir = kanari_common::get_kanari_dir();
        backup_dir.push("backups");
        Self { backup_dir }
    }
}

impl BackupManager {
    /// Create new backup manager
    pub fn new(backup_dir: PathBuf) -> Self {
        Self { backup_dir }
    }

    /// Ensure backup directory exists
    fn ensure_backup_dir(&self) -> Result<(), BackupError> {
        if !self.backup_dir.exists() {
            fs::create_dir_all(&self.backup_dir)?;
        }
        Ok(())
    }

    /// Create backup of keystore
    pub fn create_backup(
        &self,
        password: &str,
        description: Option<String>,
    ) -> Result<PathBuf, BackupError> {
        self.ensure_backup_dir()?;

        // Load current keystore
        let keystore = Keystore::load().map_err(|e| BackupError::KeystoreError(e.to_string()))?;

        // Serialize keystore
        let keystore_json = serde_json::to_vec(&keystore)
            .map_err(|e| BackupError::SerializationError(e.to_string()))?;

        // Calculate HMAC for integrity - v2 uses Argon2id with random salt (slow, per-backup)
        let mut hmac_salt_bytes = [0u8; 16];
        SysRng
            .try_fill_bytes(&mut hmac_salt_bytes)
            .map_err(|e| BackupError::EncryptionError(format!("RNG failure: {}", e)))?;
        let hmac_salt_b64 = STANDARD.encode(hmac_salt_bytes);
        let derived_key_zero =
            derive_backup_hmac_key_argon2(password.as_bytes(), &hmac_salt_bytes)?;

        let mut mac = HmacSha3_256::new_from_slice(&derived_key_zero)
            .map_err(|e| BackupError::EncryptionError(format!("HMAC error: {}", e)))?;
        mac.update(&keystore_json);
        let hmac_result = mac.finalize();
        let checksum = hex::encode(hmac_result.into_bytes());

        // Create metadata with Argon2 salt (v2)
        let metadata = BackupMetadata::new_with_hmac_salt(
            keystore.keys.len(),
            keystore.has_mnemonic(),
            checksum,
            hmac_salt_b64,
        );

        let metadata = if let Some(desc) = description {
            metadata.with_description(desc)
        } else {
            metadata
        };

        // Encrypt keystore data
        let encrypted_data = encrypt_data(&keystore_json, password)
            .map_err(|e| BackupError::EncryptionError(e.to_string()))?;

        // Create backup structure
        let backup = EncryptedBackup {
            metadata: metadata.clone(),
            encrypted_data,
        };

        // Generate backup filename with timestamp from metadata (ensures consistency)
        // Sanitize to prevent path traversal attacks
        let filename = format!("keystore_backup_{}.kbak", metadata.created_at);

        // Validate filename doesn't contain dangerous characters
        // Check for path separators, null bytes, and control characters
        if filename.contains(std::path::MAIN_SEPARATOR)
            || filename.contains('/')
            || filename.contains('\\')
            || filename.contains('\0')
            || filename.chars().any(|c| c.is_control())
            || filename.contains("..")
        {
            return Err(BackupError::SerializationError(
                "Invalid backup filename".to_string(),
            ));
        }

        let backup_path = self.backup_dir.join(&filename);

        // Ensure the resolved path is still within backup_dir (prevent traversal)
        if !backup_path.starts_with(&self.backup_dir) {
            return Err(BackupError::SerializationError(
                "Path traversal detected".to_string(),
            ));
        }

        // Write backup to file
        let backup_json = serde_json::to_string_pretty(&backup)
            .map_err(|e| BackupError::SerializationError(e.to_string()))?;
        fs::write(&backup_path, backup_json)?;

        // Set secure file permissions (owner read/write only) - Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&backup_path)?.permissions();
            perms.set_mode(0o600); // rw------- (owner only)
            fs::set_permissions(&backup_path, perms)?;
        }

        Ok(backup_path)
    }

    /// Validate backup file before loading (size and basic checks)
    fn validate_backup_file(&self, path: &Path) -> Result<(), BackupError> {
        const MAX_BACKUP_SIZE: u64 = 50 * 1024 * 1024; // 50MB

        let metadata = fs::metadata(path)?;
        if metadata.len() > MAX_BACKUP_SIZE {
            return Err(BackupError::VerificationFailed(
                "Backup file size exceeds maximum allowed".to_string(),
            ));
        }

        Ok(())
    }

    /// Restore keystore from backup
    pub fn restore_backup(
        &self,
        backup_path: &Path,
        password: &str,
        verify: bool,
    ) -> Result<(), BackupError> {
        // Validate file size first (before checking existence to avoid TOCTOU)
        self.validate_backup_file(backup_path)?;

        // Read backup file atomically
        let backup_data = fs::read_to_string(backup_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BackupError::NotFound(backup_path.display().to_string())
            } else {
                BackupError::IoError(e)
            }
        })?;

        // Deserialize backup
        let backup: EncryptedBackup = serde_json::from_str(&backup_data)
            .map_err(|e| BackupError::SerializationError(e.to_string()))?;

        // Decrypt keystore data
        let decrypted_data = decrypt_data(&backup.encrypted_data, password)
            .map_err(|e| BackupError::DecryptionError(e.to_string()))?;

        // Verify HMAC if requested
        if verify {
            let derived_key_zero = if let Some(ref salt_b64) = backup.metadata.hmac_salt {
                let salt_bytes = STANDARD.decode(salt_b64).map_err(|e| {
                    BackupError::VerificationFailed(format!("Invalid HMAC salt: {}", e))
                })?;
                derive_backup_hmac_key_argon2(password.as_bytes(), &salt_bytes)
                    .map_err(|e| BackupError::VerificationFailed(e.to_string()))?
            } else {
                // Legacy fallback: SHA3(static_salt || password) for backups created before v2
                let password_zero = Zeroizing::new(password.as_bytes().to_vec());
                let hkdf_salt = b"kanari-backup-hmac-v1";
                let mut derived_key = vec![0u8; 32];
                let mut hasher = Sha3_256::new();
                hasher.update(hkdf_salt);
                hasher.update(&password_zero[..]);
                derived_key.copy_from_slice(&hasher.finalize()[..]);
                Zeroizing::new(derived_key)
            };

            let mut mac = HmacSha3_256::new_from_slice(&derived_key_zero)
                .map_err(|e| BackupError::VerificationFailed(format!("HMAC error: {}", e)))?;
            mac.update(&decrypted_data);
            let hmac_result = mac.finalize();
            let calculated_hmac = hex::encode(hmac_result.into_bytes());

            if calculated_hmac
                .as_bytes()
                .ct_eq(backup.metadata.checksum.as_bytes())
                .unwrap_u8()
                == 0
            {
                return Err(BackupError::VerificationFailed(
                    "HMAC verification failed".to_string(),
                ));
            }
        }

        // Deserialize keystore
        let mut keystore: Keystore = serde_json::from_slice(&decrypted_data)
            .map_err(|e| BackupError::SerializationError(e.to_string()))?;

        // Verify restored keystore structure
        if verify {
            if keystore.keys.len() != backup.metadata.key_count {
                return Err(BackupError::VerificationFailed(format!(
                    "Key count mismatch: expected {}, got {}",
                    backup.metadata.key_count,
                    keystore.keys.len()
                )));
            }

            if keystore.has_mnemonic() != backup.metadata.has_mnemonic {
                return Err(BackupError::VerificationFailed(
                    "Mnemonic presence mismatch".to_string(),
                ));
            }
        }

        // Save restored keystore
        keystore
            .save()
            .map_err(|e| BackupError::KeystoreError(e.to_string()))?;

        Ok(())
    }

    /// List all available backups
    ///
    /// Note: This function reads backup metadata from disk. Large backup directories
    /// may consume significant memory. Files are processed sequentially to limit
    /// memory usage, and oversized files (>50MB) are automatically skipped.
    pub fn list_backups(&self) -> Result<Vec<BackupInfo>, BackupError> {
        self.ensure_backup_dir()?;

        let mut backups = Vec::new();
        const MAX_BACKUP_READ_SIZE: u64 = 50 * 1024 * 1024; // 50MB

        // Get canonical backup directory path for security validation
        let canonical_backup_dir = self
            .backup_dir
            .canonicalize()
            .map_err(BackupError::IoError)?;

        for entry in fs::read_dir(&self.backup_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Security: Block symlinks to prevent directory traversal
            let symlink_meta = path.symlink_metadata()?;
            if symlink_meta.is_symlink() {
                continue; // Skip symlinks
            }
            // Use regular metadata for file size and other checks
            let metadata = entry.metadata()?;

            // Security: Validate path is within backup directory
            if let Ok(canonical_path) = path.canonicalize() {
                if !canonical_path.starts_with(&canonical_backup_dir) {
                    continue; // Skip files outside backup directory
                }
            } else {
                continue; // Skip if can't canonicalize
            }

            if path.extension().and_then(|s| s.to_str()) == Some("kbak") {
                // Check file size before reading
                if metadata.len() > MAX_BACKUP_READ_SIZE {
                    continue; // Skip oversized files
                }

                if let Ok(data) = fs::read_to_string(&path)
                    && let Ok(backup) = serde_json::from_str::<EncryptedBackup>(&data)
                {
                    backups.push(BackupInfo {
                        path: path.clone(),
                        metadata: backup.metadata,
                        file_size: metadata.len(),
                    });
                }
            }
        }

        // Sort by creation time (newest first)
        backups.sort_by_key(|backup| std::cmp::Reverse(backup.metadata.created_at));

        Ok(backups)
    }

    /// Delete a backup file
    pub fn delete_backup(&self, backup_path: &Path) -> Result<(), BackupError> {
        if !backup_path.exists() {
            return Err(BackupError::NotFound(backup_path.display().to_string()));
        }

        fs::remove_file(backup_path)?;
        Ok(())
    }

    /// Get backup directory path
    pub fn get_backup_dir(&self) -> &Path {
        &self.backup_dir
    }

    /// Clean old backups (keep only N most recent)
    pub fn clean_old_backups(&self, keep_count: usize) -> Result<usize, BackupError> {
        let mut backups = self.list_backups()?;

        if backups.len() <= keep_count {
            return Ok(0);
        }

        // Keep only the most recent backups
        let to_delete = backups.split_off(keep_count);
        let deleted_count = to_delete.len();

        for backup in to_delete {
            self.delete_backup(&backup.path)?;
        }

        Ok(deleted_count)
    }
}

fn derive_backup_hmac_key_argon2(
    password: &[u8],
    salt: &[u8],
) -> Result<Zeroizing<Vec<u8>>, BackupError> {
    let pwd_zero = Zeroizing::new(password.to_vec());
    let params = Params::new(47104, 3, 4, Some(32))
        .map_err(|e| BackupError::EncryptionError(format!("Invalid Argon2 params: {}", e)))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = vec![0u8; 32];
    argon2
        .hash_password_into(&pwd_zero, salt, &mut out)
        .map_err(|e| BackupError::EncryptionError(format!("Argon2 HMAC KDF failed: {}", e)))?;
    Ok(Zeroizing::new(out))
}

#[cfg(test)]
#[path = "../tests/unit/backup_test.rs"]
mod tests;
