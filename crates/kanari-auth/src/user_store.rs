// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! User storage and management
//!
//! This module handles persistent storage of user credentials and wallet information,
//! including secure password hashing and user record management.

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

    /// Encrypted private key (stored securely)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_private_key: Option<String>,

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
    pub fn new(email: String, password: &str, wallet_address: String) -> AuthResult<Self> {
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
            encrypted_private_key: None,
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
        use password_hash::{SaltString, rand_core::OsRng};

        // Generate a random salt
        let salt = SaltString::generate(&mut OsRng);

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
}

/// In-memory user store with persistence support
#[derive(Debug)]
pub struct UserStore {
    /// Users indexed by normalized email
    users: HashMap<String, UserRecord>,

    /// Path to persistent storage file (optional)
    storage_path: Option<PathBuf>,
}

impl UserStore {
    /// Create a new empty user store
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            storage_path: None,
        }
    }

    /// Create a user store with persistent storage
    ///
    /// # Arguments
    /// * `storage_path` - Path to JSON file for persistence
    pub fn with_persistence(storage_path: PathBuf) -> AuthResult<Self> {
        let mut store = Self {
            users: HashMap::new(),
            storage_path: Some(storage_path.clone()),
        };

        // Load existing data if file exists
        if storage_path.exists() {
            store.load_from_disk()?;
        }

        Ok(store)
    }

    /// Add a new user to the store
    ///
    /// # Arguments
    /// * `user` - UserRecord to add
    ///
    /// # Returns
    /// Error if user already exists
    pub fn add_user(&mut self, user: UserRecord) -> AuthResult<()> {
        let email = user.email.clone();

        if self.users.contains_key(&email) {
            return Err(AuthError::UserAlreadyExists(email));
        }

        self.users.insert(email, user);
        self.save_to_disk()?;

        Ok(())
    }

    /// Get a user by email
    ///
    /// # Arguments
    /// * `email` - User's email address
    ///
    /// # Returns
    /// Reference to UserRecord if found
    pub fn get_user(&self, email: &str) -> Option<&UserRecord> {
        let normalized = email_validator::normalize_email(email);
        self.users.get(&normalized)
    }

    /// Get mutable reference to a user
    pub fn get_user_mut(&mut self, email: &str) -> Option<&mut UserRecord> {
        let normalized = email_validator::normalize_email(email);
        self.users.get_mut(&normalized)
    }

    /// Check if a user exists
    pub fn user_exists(&self, email: &str) -> bool {
        let normalized = email_validator::normalize_email(email);
        self.users.contains_key(&normalized)
    }

    /// Remove a user from the store
    pub fn remove_user(&mut self, email: &str) -> AuthResult<()> {
        let normalized = email_validator::normalize_email(email);

        if self.users.remove(&normalized).is_some() {
            self.save_to_disk()?;
            Ok(())
        } else {
            Err(AuthError::UserNotFound(normalized))
        }
    }

    /// List all registered users (emails only)
    pub fn list_users(&self) -> Vec<String> {
        self.users.keys().cloned().collect()
    }

    /// Get total number of users
    pub fn user_count(&self) -> usize {
        self.users.len()
    }

    /// Save user store to disk
    fn save_to_disk(&self) -> AuthResult<()> {
        if let Some(ref path) = self.storage_path {
            let json = serde_json::to_string_pretty(&self.users).map_err(|e| {
                AuthError::SerializationError(format!("Failed to serialize users: {}", e))
            })?;

            std::fs::write(path, json).map_err(AuthError::IoError)?;
        }
        Ok(())
    }

    /// Load user store from disk
    fn load_from_disk(&mut self) -> AuthResult<()> {
        if let Some(ref path) = self.storage_path
            && path.exists() {
                let json = std::fs::read_to_string(path).map_err(AuthError::IoError)?;

                let users: HashMap<String, UserRecord> =
                    serde_json::from_str(&json).map_err(|e| {
                        AuthError::SerializationError(format!("Failed to deserialize users: {}", e))
                    })?;

                self.users = users;
            }
        Ok(())
    }
}

impl Default for UserStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = UserRecord::new(
            "user@example.com".to_string(),
            "SecurePass123!",
            "0x123".to_string(),
        )
        .unwrap();

        assert_eq!(user.email, "user@example.com");
        assert!(!user.password_hash.is_empty());
        assert_eq!(user.wallet_address, "0x123");
        assert_eq!(user.failed_attempts, 0);
        assert!(user.is_active);
    }

    #[test]
    fn test_password_verification() {
        let user = UserRecord::new(
            "user@example.com".to_string(),
            "SecurePass123!",
            "0x123".to_string(),
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
        let mut store = UserStore::new();

        let user = UserRecord::new(
            "user@example.com".to_string(),
            "SecurePass123!",
            "0x123".to_string(),
        )
        .unwrap();

        // Add user
        assert!(store.add_user(user).is_ok());

        // Check existence
        assert!(store.user_exists("user@example.com"));
        assert_eq!(store.user_count(), 1);

        // Get user
        assert!(store.get_user("user@example.com").is_some());

        // List users
        let users = store.list_users();
        assert_eq!(users.len(), 1);

        // Duplicate should fail
        let duplicate = UserRecord::new(
            "user@example.com".to_string(),
            "AnotherPass456!",
            "0x456".to_string(),
        )
        .unwrap();
        assert!(store.add_user(duplicate).is_err());
    }
}
