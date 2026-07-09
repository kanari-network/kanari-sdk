// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Kanari Auth - Email & Password Authentication
//!
//! This crate provides email and password-based authentication for the Kanari SDK,
//! enabling users to sign transactions using familiar credentials.
//!
//! ## Quick Start
//!
//! ```ignore
//! use kanari_auth::AuthManager;
//! use kanari_types::transaction::{ObjectRef, Transaction};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut auth = AuthManager::new();
//!     
//!     // Register a new user
//!     auth.register("user@example.com", "SecurePassword123!", None)?;
//!     
//!     // Login
//!     let session = auth.login("user@example.com", "SecurePassword123!", None)?;
//!     
//!     // Create and sign a transaction
//!     let tx = Transaction::new_transfer_with_object_ref(
//!         session.wallet_address.clone(),
//!         ObjectRef::new("0x2", Some(1), Some("0xdigest".to_string())),
//!         "0x3".to_string(),
//!         1_000,
//!         0,
//!     );
//!     let signed_tx = auth.sign_transaction(&session, tx)?;
//!     
//!     println!("Transaction signed!");
//!     Ok(())
//! }
//! ```
//!
//! # Features
//! - Email-based user registration and authentication
//! - Secure password hashing with Argon2
//! - Wallet creation linked to email accounts
//! - Transaction signing with email/password credentials
//! - Session management with automatic timeout
//! - Multi-factor authentication support (optional)
//!
//! # Example
//! ```ignore
//! use kanari_auth::AuthManager;
//! use kanari_types::transaction::Transaction;
//!
//! // Create auth manager
//! let mut auth = AuthManager::new();
//!
//! // Register a new user
//! auth.register_user("user@example.com", "SecurePassword123!")?;
//!
//! // Login and get session token
//! let session = auth.login("user@example.com", "SecurePassword123!")?;
//!
//! // Sign a transaction
//! let signed_tx = auth.sign_transaction(&session, transaction)?;
//! ```

pub mod auth_manager;
pub mod email_validator;
pub mod private_key_crypto;
pub mod session;
pub mod user_store;

pub use auth_manager::AuthManager;
pub use auth_manager::TwoFactorStatus;
pub use session::Session;
pub use user_store::{UserRecord, UserStore};

use thiserror::Error;

/// Errors that can occur during authentication operations
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid email format: {0}")]
    InvalidEmail(String),

    #[error("Invalid password: {0}")]
    InvalidPassword(String),

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("User already exists: {0}")]
    UserAlreadyExists(String),

    #[error("Authentication failed: invalid credentials")]
    AuthenticationFailed,

    #[error("Session expired")]
    SessionExpired,

    #[error("Session not found or invalid")]
    InvalidSession,

    #[error("Wallet error: {0}")]
    WalletError(#[from] kanari_crypto::wallet::WalletError),

    #[error("Signing error: {0}")]
    SigningError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Email validation error: {0}")]
    ValidationError(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Account locked due to too many failed attempts")]
    AccountLocked,

    #[error("Cryptographic error: {0}")]
    CryptoError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Result type for authentication operations
pub type AuthResult<T> = Result<T, AuthError>;
