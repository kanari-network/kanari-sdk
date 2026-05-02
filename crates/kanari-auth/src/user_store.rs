// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! User storage and management
//!
//! This module handles persistent storage of user credentials and wallet information,
//! including secure password hashing and user record management.

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{AuthError, AuthResult, email_validator};

/// Number of failed login attempts before account lockout
const MAX_FAILED_ATTEMPTS: u32 = 5;

/// Account lockout duration in seconds (15 minutes)
const LOCKOUT_DURATION: i64 = 15 * 60;

/// Represents a user record with authentication and wallet data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    /// User's email address (normalized, lowercase)
    pub email: String,

    /// Hashed password (Argon2id format)
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub password_hash: String,

    /// Wallet address associated with this user
    pub wallet_address: String,

    /// Curve type associated with the stored wallet
    pub curve_type: String,

    /// Encrypted private key (stored securely)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_private_key: Option<String>,

    /// Pending or active TOTP secret (base32 encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub totp_secret: Option<String>,

    /// Whether 2FA has been fully enabled
    pub totp_enabled: bool,

    /// Remaining backup codes for account recovery
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub backup_codes: Vec<String>,

    /// Account creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last login timestamp
    pub last_login: Option<DateTime<Utc>>,

    /// Number of consecutive failed login attempts
    pub failed_attempts: u32,

    /// Account lockout timestamp (if locked)
    pub locked_until: Option<DateTime<Utc>>,

    /// Whether the account is active
    pub is_active: bool,
}

impl UserRecord {
    /// Create a new user record
    ///
    /// # Arguments
    /// * `email` - User's email address
    /// * `password` - Plain text password (will be hashed)
    /// * `wallet_address` - Associated wallet address
    ///
    /// # Returns
    /// New UserRecord with hashed password
    pub fn new(
        email: String,
        password: &str,
        wallet_address: String,
        curve_type: String,
    ) -> AuthResult<Self> {
        // Validate email
        email_validator::validate_email(&email)?;

        // Validate password strength
        Self::validate_password(password)?;

        // Hash password with Argon2
        let password_hash = Self::hash_password(password)?;

        Ok(Self {
            email: email_validator::normalize_email(&email),
            password_hash,
            wallet_address,
            curve_type,
            encrypted_private_key: None,
            totp_secret: None,
            totp_enabled: false,
            backup_codes: Vec::new(),
            created_at: Utc::now(),
            last_login: None,
            failed_attempts: 0,
            locked_until: None,
            is_active: true,
        })
    }

    /// Hash a password using Argon2id
    ///
    /// # Arguments
    /// * `password` - Plain text password
    ///
    /// # Returns
    /// Hashed password string
    pub fn hash_password(password: &str) -> AuthResult<String> {
        use argon2::{Argon2, PasswordHasher};
        use rand::TryRng;
        use rand::rngs::SysRng;

        // Generate a random salt manually (16 bytes)
        let mut rng = SysRng;
        let mut salt_bytes = [0u8; 16];
        rng.try_fill_bytes(&mut salt_bytes).map_err(|e| {
            AuthError::CryptoError(format!("Failed to generate random bytes: {}", e))
        })?;

        // Create SaltString from raw bytes using B64 encoding without padding
        // SaltString expects exactly the right format
        let salt_string = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD_NO_PAD,
            salt_bytes,
        );
        let salt = argon2::password_hash::SaltString::from_b64(&salt_string)
            .map_err(|e| AuthError::CryptoError(format!("Invalid salt: {}", e)))?;

        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AuthError::CryptoError(format!("Password hashing failed: {}", e)))?;

        Ok(password_hash.to_string())
    }

    /// Verify a password against the stored hash
    ///
    /// # Arguments
    /// * `password` - Plain text password to verify
    ///
    /// # Returns
    /// `true` if password matches
    pub fn verify_password(&self, password: &str) -> AuthResult<bool> {
        let argon2 = Argon2::default();
        let parsed_hash = PasswordHash::new(&self.password_hash)
            .map_err(|e| AuthError::CryptoError(format!("Invalid password hash: {}", e)))?;

        let result = argon2.verify_password(password.as_bytes(), &parsed_hash);

        Ok(result.is_ok())
    }

    /// Validate password strength requirements
    ///
    /// # Arguments
    /// * `password` - Password to validate
    ///
    /// # Returns
    /// `Ok(())` if password meets requirements
    pub fn validate_password(password: &str) -> AuthResult<()> {
        if password.len() < 8 {
            return Err(AuthError::InvalidPassword(
                "Password must be at least 8 characters long".to_string(),
            ));
        }

        if password.len() > 128 {
            return Err(AuthError::InvalidPassword(
                "Password too long (max 128 characters)".to_string(),
            ));
        }

        // Check for at least one uppercase letter
        if !password.chars().any(|c| c.is_ascii_uppercase()) {
            return Err(AuthError::InvalidPassword(
                "Password must contain at least one uppercase letter".to_string(),
            ));
        }

        // Check for at least one lowercase letter
        if !password.chars().any(|c| c.is_ascii_lowercase()) {
            return Err(AuthError::InvalidPassword(
                "Password must contain at least one lowercase letter".to_string(),
            ));
        }

        // Check for at least one digit
        if !password.chars().any(|c| c.is_ascii_digit()) {
            return Err(AuthError::InvalidPassword(
                "Password must contain at least one digit".to_string(),
            ));
        }

        // Check for at least one special character
        if !password
            .chars()
            .any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c))
        {
            return Err(AuthError::InvalidPassword(
                "Password must contain at least one special character".to_string(),
            ));
        }

        Ok(())
    }

    /// Record a failed login attempt
    pub fn record_failed_attempt(&mut self) {
        self.failed_attempts += 1;

        if self.failed_attempts >= MAX_FAILED_ATTEMPTS {
            self.locked_until = Some(Utc::now() + chrono::Duration::seconds(LOCKOUT_DURATION));
        }
    }

    /// Record a successful login
    pub fn record_successful_login(&mut self) {
        self.failed_attempts = 0;
        self.locked_until = None;
        self.last_login = Some(Utc::now());
    }

    /// Check if the account is currently locked
    pub fn is_locked(&mut self) -> bool {
        if let Some(lockout_time) = self.locked_until {
            if Utc::now() < lockout_time {
                return true;
            } else {
                // Lockout expired, reset
                self.locked_until = None;
                self.failed_attempts = 0;
            }
        }
        false
    }

    /// Store encrypted private key
    pub fn set_encrypted_private_key(&mut self, encrypted_key: String) {
        self.encrypted_private_key = Some(encrypted_key);
    }

    /// Save a pending or refreshed 2FA enrollment for the user.
    pub fn set_two_factor_setup(&mut self, secret: String, backup_codes: Vec<String>) {
        self.totp_secret = Some(secret);
        self.totp_enabled = false;
        self.backup_codes = backup_codes;
    }

    /// Mark the currently stored 2FA setup as enabled.
    pub fn enable_two_factor(&mut self) {
        self.totp_enabled = self.totp_secret.is_some();
    }

    /// Disable 2FA and remove all stored recovery material.
    pub fn disable_two_factor(&mut self) {
        self.totp_secret = None;
        self.totp_enabled = false;
        self.backup_codes.clear();
    }

    /// Consume a one-time backup code if present.
    pub fn consume_backup_code(&mut self, backup_code: &str) -> bool {
        if let Some(index) = self
            .backup_codes
            .iter()
            .position(|stored| stored == backup_code)
        {
            self.backup_codes.remove(index);
            true
        } else {
            false
        }
    }
}

/// Thread-safe user store backed by SQLite database
#[derive(Debug)]
pub struct UserStore {
    /// SQLite database connection
    conn: Connection,

    /// Database file path (kept for debugging/logging purposes)
    #[allow(dead_code)]
    db_path: PathBuf,
}

/// Helper struct to hold raw user data from database query
#[derive(Debug)]
struct UserRowData {
    email: String,
    password_hash: String,
    wallet_address: String,
    curve_type: String,
    encrypted_private_key: Option<String>,
    totp_secret: Option<String>,
    totp_enabled: bool,
    backup_codes: Option<String>,
    created_at: String,
    last_login: Option<String>,
    failed_attempts: i64,
    locked_until: Option<String>,
    is_active: bool,
}

impl UserStore {
    /// Create a new UserStore with SQLite backend
    ///
    /// # Arguments
    /// * `db_path` - Path to SQLite database file (use None for in-memory)
    pub fn new(db_path: Option<PathBuf>) -> AuthResult<Self> {
        let path = db_path.unwrap_or_else(|| PathBuf::from(":memory:"));

        let conn = if path.to_str() == Some(":memory:") {
            Connection::open_in_memory().map_err(|e| AuthError::DatabaseError(e.to_string()))?
        } else {
            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| AuthError::DatabaseError(e.to_string()))?;
            }
            Connection::open(&path).map_err(|e| AuthError::DatabaseError(e.to_string()))?
        };

        let mut store = Self {
            conn,
            db_path: path,
        };

        // Initialize database schema
        store.init_schema()?;

        Ok(store)
    }

    /// Initialize database schema
    fn init_schema(&mut self) -> AuthResult<()> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS users (
                email TEXT PRIMARY KEY,
                password_hash TEXT NOT NULL,
                wallet_address TEXT NOT NULL UNIQUE,
                curve_type TEXT NOT NULL DEFAULT 'Ed25519',
                encrypted_private_key TEXT,
                totp_secret TEXT,
                totp_enabled BOOLEAN NOT NULL DEFAULT 0,
                backup_codes TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                last_login TEXT,
                failed_attempts INTEGER NOT NULL DEFAULT 0,
                locked_until TEXT,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                CONSTRAINT unique_email UNIQUE (email COLLATE NOCASE)
            );",
            )
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        // Add curve_type column if it doesn't exist (for backward compatibility)
        let _ = self.conn.execute(
            "ALTER TABLE users ADD COLUMN curve_type TEXT NOT NULL DEFAULT 'Ed25519';",
            [],
        );
        let _ = self
            .conn
            .execute("ALTER TABLE users ADD COLUMN totp_secret TEXT;", []);
        let _ = self.conn.execute(
            "ALTER TABLE users ADD COLUMN totp_enabled BOOLEAN NOT NULL DEFAULT 0;",
            [],
        );
        let _ = self.conn.execute(
            "ALTER TABLE users ADD COLUMN backup_codes TEXT NOT NULL DEFAULT '[]';",
            [],
        );

        // SECURITY FIX #1: Create unique index on normalized email to prevent race conditions
        self.conn
            .execute(
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email_unique ON users(email COLLATE NOCASE);",
                [],
            )
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        // Create index for faster lookups
        self.conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_users_active ON users(is_active);",
                [],
            )
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Add a new user to the database
    ///
    /// # Arguments
    /// * `user` - UserRecord to add
    ///
    /// # Returns
    /// Error if user already exists
    pub fn add_user(&mut self, user: UserRecord) -> AuthResult<()> {
        // SECURITY FIX #1: Use transaction to prevent race conditions
        let tx = self
            .conn
            .transaction()
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        // Check if user already exists (within transaction)
        let count: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM users WHERE email = ?1",
                [&user.email],
                |row| row.get(0),
            )
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        if count > 0 {
            return Err(AuthError::UserAlreadyExists(user.email.clone()));
        }

        // Insert user with unique constraint as backup protection
        match tx.execute(
            "INSERT INTO users (
                email, password_hash, wallet_address, curve_type, encrypted_private_key,
                totp_secret, totp_enabled, backup_codes,
                created_at, last_login, failed_attempts, locked_until, is_active
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                user.email,
                user.password_hash,
                user.wallet_address,
                user.curve_type,
                user.encrypted_private_key,
                user.totp_secret,
                user.totp_enabled,
                serde_json::to_string(&user.backup_codes)
                    .map_err(|e| AuthError::SerializationError(e.to_string()))?,
                user.created_at.to_rfc3339(),
                user.last_login.map(|dt| dt.to_rfc3339()),
                user.failed_attempts,
                user.locked_until.map(|dt| dt.to_rfc3339()),
                user.is_active,
            ],
        ) {
            Ok(_) => {
                tx.commit()
                    .map_err(|e| AuthError::DatabaseError(e.to_string()))?;
                Ok(())
            }
            Err(rusqlite::Error::SqliteFailure(sqlite_err, _))
                if sqlite_err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                // Database-level constraint caught duplicate (race condition protection)
                Err(AuthError::UserAlreadyExists(user.email.clone()))
            }
            Err(e) => Err(AuthError::DatabaseError(e.to_string())),
        }
    }

    /// Get a user record by email
    ///
    /// # Arguments
    /// * `email` - Normalized email address
    pub fn get_user(&self, email: &str) -> AuthResult<Option<UserRecord>> {
        let normalized = email_validator::normalize_email(email);

        let result: Option<UserRowData> = self
            .conn
            .query_row(
                "SELECT email, password_hash, wallet_address, curve_type, encrypted_private_key,
                            totp_secret, totp_enabled, backup_codes, created_at, last_login,
                            failed_attempts, locked_until, is_active
                     FROM users WHERE email = ?1",
                [&normalized],
                |row| {
                    Ok(UserRowData {
                        email: row.get(0)?,
                        password_hash: row.get(1)?,
                        wallet_address: row.get(2)?,
                        curve_type: row.get(3)?,
                        encrypted_private_key: row.get(4)?,
                        totp_secret: row.get(5)?,
                        totp_enabled: row.get(6)?,
                        backup_codes: row.get(7)?,
                        created_at: row.get(8)?,
                        last_login: row.get(9)?,
                        failed_attempts: row.get(10)?,
                        locked_until: row.get(11)?,
                        is_active: row.get(12)?,
                    })
                },
            )
            .optional()
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        match result {
            Some(data) => {
                let created_at = DateTime::parse_from_rfc3339(&data.created_at)
                    .map_err(|e| AuthError::SerializationError(e.to_string()))?
                    .with_timezone(&Utc);

                let last_login = data
                    .last_login
                    .map(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .map_err(|e| AuthError::SerializationError(e.to_string()))
                            .map(|dt| dt.with_timezone(&Utc))
                    })
                    .transpose()?;

                let locked_until = data
                    .locked_until
                    .map(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .map_err(|e| AuthError::SerializationError(e.to_string()))
                            .map(|dt| dt.with_timezone(&Utc))
                    })
                    .transpose()?;

                let backup_codes = data
                    .backup_codes
                    .as_deref()
                    .map(serde_json::from_str::<Vec<String>>)
                    .transpose()
                    .map_err(|e| AuthError::SerializationError(e.to_string()))?
                    .unwrap_or_default();

                Ok(Some(UserRecord {
                    email: data.email,
                    password_hash: data.password_hash,
                    wallet_address: data.wallet_address,
                    curve_type: data.curve_type,
                    encrypted_private_key: data.encrypted_private_key,
                    totp_secret: data.totp_secret,
                    totp_enabled: data.totp_enabled,
                    backup_codes,
                    created_at,
                    last_login,
                    failed_attempts: data.failed_attempts as u32,
                    locked_until,
                    is_active: data.is_active,
                }))
            }
            None => Ok(None),
        }
    }

    /// Update a user record in the database
    pub fn update_user(&mut self, user: &UserRecord) -> AuthResult<()> {
        self.conn
            .execute(
                "UPDATE users SET 
                password_hash = ?2,
                wallet_address = ?3,
                curve_type = ?4,
                encrypted_private_key = ?5,
                totp_secret = ?6,
                totp_enabled = ?7,
                backup_codes = ?8,
                last_login = ?9,
                failed_attempts = ?10,
                locked_until = ?11,
                is_active = ?12
             WHERE email = ?1",
                rusqlite::params![
                    user.email,
                    user.password_hash,
                    user.wallet_address,
                    user.curve_type,
                    user.encrypted_private_key,
                    user.totp_secret,
                    user.totp_enabled,
                    serde_json::to_string(&user.backup_codes)
                        .map_err(|e| AuthError::SerializationError(e.to_string()))?,
                    user.last_login.map(|dt| dt.to_rfc3339()),
                    user.failed_attempts,
                    user.locked_until.map(|dt| dt.to_rfc3339()),
                    user.is_active,
                ],
            )
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    /// Check if a user exists
    pub fn user_exists(&self, email: &str) -> AuthResult<bool> {
        let normalized = email_validator::normalize_email(email);
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE email = ?1",
                [&normalized],
                |row| row.get(0),
            )
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;
        Ok(count > 0)
    }

    /// Remove a user from the store
    pub fn remove_user(&mut self, email: &str) -> AuthResult<()> {
        let normalized = email_validator::normalize_email(email);

        let changes = self
            .conn
            .execute("DELETE FROM users WHERE email = ?1", [&normalized])
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        if changes == 0 {
            Err(AuthError::UserNotFound(normalized))
        } else {
            Ok(())
        }
    }

    /// List all registered users (emails only)
    pub fn list_users(&self) -> AuthResult<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT email FROM users ORDER BY created_at")
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        let emails = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?
            .filter_map(|r: Result<String, _>| r.ok())
            .collect();

        Ok(emails)
    }

    /// Get total number of users
    pub fn user_count(&self) -> AuthResult<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get::<_, i64>(0))
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;
        Ok(count as usize)
    }
}

impl Default for UserStore {
    fn default() -> Self {
        Self::new(None).expect("Failed to create default UserStore")
    }
}

#[cfg(test)]
mod tests {
    use kanari_crypto::CurveType;

    use super::*;

    #[test]
    fn test_user_creation() {
        let user = UserRecord::new(
            "user@example.com".to_string(),
            "SecurePass123!",
            "0x123".to_string(),
            CurveType::Ed25519.to_string(),
        )
        .unwrap();

        assert_eq!(user.email, "user@example.com");
        assert!(!user.password_hash.is_empty());
        assert_eq!(user.wallet_address, "0x123");
        assert_eq!(user.failed_attempts, 0);
        assert!(user.is_active);
        assert!(!user.totp_enabled);
        assert!(user.backup_codes.is_empty());
    }

    #[test]
    fn test_password_verification() {
        let user = UserRecord::new(
            "user@example.com".to_string(),
            "SecurePass123!",
            "0x123".to_string(),
            CurveType::Ed25519.to_string(),
        )
        .unwrap();

        assert!(user.verify_password("SecurePass123!").unwrap());
        assert!(!user.verify_password("WrongPassword").unwrap());
    }

    #[test]
    fn test_password_validation() {
        // Too short
        assert!(UserRecord::validate_password("Short1!").is_err());

        // No uppercase
        assert!(UserRecord::validate_password("nouppercase123!").is_err());

        // No lowercase
        assert!(UserRecord::validate_password("NOLOWERCASE123!").is_err());

        // No digit
        assert!(UserRecord::validate_password("NoDigit!").is_err());

        // No special char
        assert!(UserRecord::validate_password("NoSpecial123").is_err());

        // Valid password
        assert!(UserRecord::validate_password("ValidPass123!").is_ok());
    }

    #[test]
    fn test_account_lockout() {
        let mut user = UserRecord::new(
            "user@example.com".to_string(),
            "SecurePass123!",
            "0x123".to_string(),
            CurveType::Ed25519.to_string(),
        )
        .unwrap();

        // Record failed attempts
        for _ in 0..MAX_FAILED_ATTEMPTS {
            user.record_failed_attempt();
        }

        assert!(user.is_locked());
    }

    #[test]
    fn test_user_store() {
        let mut store = UserStore::new(None).unwrap();

        let user = UserRecord::new(
            "user@example.com".to_string(),
            "SecurePass123!",
            "0x123".to_string(),
            CurveType::Ed25519.to_string(),
        )
        .unwrap();

        // Add user
        assert!(store.add_user(user).is_ok());

        // Check existence
        assert!(store.user_exists("user@example.com").unwrap());
        assert_eq!(store.user_count().unwrap(), 1);

        // Get user
        assert!(store.get_user("user@example.com").unwrap().is_some());

        // List users
        let users = store.list_users().unwrap();
        assert_eq!(users.len(), 1);

        // Duplicate should fail
        let duplicate = UserRecord::new(
            "user@example.com".to_string(),
            "AnotherPass456!",
            "0x456".to_string(),
            CurveType::Ed25519.to_string(),
        )
        .unwrap();
        assert!(store.add_user(duplicate).is_err());
    }

    #[test]
    fn test_two_factor_state_persists() {
        let mut store = UserStore::new(None).unwrap();
        let mut user = UserRecord::new(
            "user@example.com".to_string(),
            "SecurePass123!",
            "0x123".to_string(),
            CurveType::Ed25519.to_string(),
        )
        .unwrap();
        user.set_two_factor_setup(
            "BASE32SECRET".to_string(),
            vec!["CODE1234".to_string(), "CODE5678".to_string()],
        );
        user.enable_two_factor();

        store.add_user(user).unwrap();
        let loaded = store.get_user("user@example.com").unwrap().unwrap();

        assert_eq!(loaded.totp_secret.as_deref(), Some("BASE32SECRET"));
        assert!(loaded.totp_enabled);
        assert_eq!(loaded.backup_codes.len(), 2);
    }
}
