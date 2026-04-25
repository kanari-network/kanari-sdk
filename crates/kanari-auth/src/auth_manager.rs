// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Main authentication manager
//!
//! This module provides the primary interface for email-based authentication
//! and transaction signing operations.

use std::str::FromStr;
use std::time::Duration;

use kanari_crypto::{keys::CurveType, wallet};
use kanari_types::transaction::{SignedTransaction, Transaction};
use move_core_types::account_address::AccountAddress;

use crate::{
    AuthError, AuthResult, Session, UserStore, email_validator, session::SessionManager,
    user_store::UserRecord,
};

/// Main authentication manager that coordinates user registration,
/// login, and transaction signing operations.
pub struct AuthManager {
    /// User storage
    user_store: UserStore,

    /// Session manager
    session_manager: SessionManager,
}

impl AuthManager {
    /// Create a new AuthManager with in-memory storage
    pub fn new() -> Self {
        Self {
            user_store: UserStore::new(),
            session_manager: SessionManager::new(),
        }
    }

    /// Create an AuthManager with persistent storage
    ///
    /// # Arguments
    /// * `storage_path` - Path to JSON file for user data persistence
    ///
    /// # Returns
    /// Configured AuthManager
    pub fn with_persistence(storage_path: std::path::PathBuf) -> AuthResult<Self> {
        Ok(Self {
            user_store: UserStore::with_persistence(storage_path)?,
            session_manager: SessionManager::new(),
        })
    }

    /// Register a new user with email and password
    ///
    /// This creates a new wallet and associates it with the user's email.
    ///
    /// # Arguments
    /// * `email` - User's email address
    /// * `password` - User's password (must meet strength requirements)
    /// * `curve_type` - Optional cryptographic curve type (defaults to Ed25519)
    ///
    /// # Returns
    /// The created wallet information
    ///
    /// # Examples
    /// ```ignore
    /// use kanari_auth::AuthManager;
    ///
    /// let mut auth = AuthManager::new();
    /// let address = auth.register_user("user@example.com", "SecurePassword123!", None)?;
    /// println!("Wallet created: {}", address);
    /// ```
    pub fn register_user(
        &mut self,
        email: &str,
        password: &str,
        curve_type: Option<CurveType>,
    ) -> AuthResult<wallet::Wallet> {
        // Validate email format
        email_validator::validate_email(email)?;

        let normalized_email = email_validator::normalize_email(email);

        // Check if user already exists
        if self.user_store.user_exists(&normalized_email) {
            return Err(AuthError::UserAlreadyExists(normalized_email));
        }

        // Generate new wallet
        let curve = curve_type.unwrap_or(CurveType::Ed25519);
        let keypair = kanari_crypto::keys::generate_keypair(curve)
            .map_err(|e| AuthError::CryptoError(format!("Failed to generate keypair: {}", e)))?;

        let address = AccountAddress::from_str(&keypair.address)
            .map_err(|e| AuthError::CryptoError(format!("Invalid address format: {}", e)))?;

        // Create wallet - need to convert Zeroizing<String> to String
        let wallet = wallet::Wallet::new(
            address,
            keypair.private_key.to_string(),
            String::new(), // Seed phrase not available from keypair
            None,
            curve,
        );

        let wallet_address = wallet.address.to_hex_literal();

        // Create user record with hashed password
        let mut user_record = UserRecord::new(normalized_email.clone(), password, wallet_address)?;

        // Store encrypted private key (in production, use proper encryption)
        user_record.set_encrypted_private_key(keypair.private_key.to_string());

        // Save user to store
        self.user_store.add_user(user_record)?;

        log::info!("User registered successfully: {}", normalized_email);

        Ok(wallet)
    }

    /// Authenticate a user with email and password
    ///
    /// # Arguments
    /// * `email` - User's email address
    /// * `password` - User's password
    /// * `session_timeout` - Optional custom session timeout duration
    ///
    /// # Returns
    /// A valid session if authentication succeeds
    ///
    /// # Examples
    /// ```ignore
    /// use kanari_auth::AuthManager;
    ///
    /// let mut auth = AuthManager::new();
    /// let session = auth.login("user@example.com", "SecurePassword123!", None)?;
    /// println!("Session ID: {}", session.session_id);
    /// ```
    pub fn login(
        &mut self,
        email: &str,
        password: &str,
        session_timeout: Option<Duration>,
    ) -> AuthResult<Session> {
        let normalized_email = email_validator::normalize_email(email);

        // Get user record and check account status
        let wallet_address = {
            let user = self
                .user_store
                .get_user_mut(&normalized_email)
                .ok_or(AuthError::UserNotFound(normalized_email.clone()))?;

            // Check if account is locked
            if user.is_locked() {
                return Err(AuthError::AccountLocked);
            }

            // Verify password
            if !user.verify_password(password)? {
                user.record_failed_attempt();
                log::warn!("Failed login attempt for: {}", normalized_email);
                return Err(AuthError::AuthenticationFailed);
            }

            // Record successful login
            user.record_successful_login();

            user.wallet_address.clone()
        };

        // Create session (outside the borrow scope)
        let session = self.session_manager.create_session(
            normalized_email.clone(),
            wallet_address,
            session_timeout,
        );

        log::info!("User logged in successfully: {}", normalized_email);

        Ok(session)
    }

    /// Logout and invalidate a session
    ///
    /// # Arguments
    /// * `session_id` - The session identifier to invalidate
    pub fn logout(&mut self, session_id: &str) -> AuthResult<()> {
        self.session_manager.invalidate_session(session_id)?;
        log::info!("Session invalidated: {}", session_id);
        Ok(())
    }

    /// Logout all sessions for a user
    ///
    /// # Arguments
    /// * `email` - User's email address
    pub fn logout_all(&mut self, email: &str) -> AuthResult<()> {
        let normalized_email = email_validator::normalize_email(email);
        self.session_manager
            .invalidate_all_user_sessions(&normalized_email);
        log::info!("All sessions invalidated for: {}", normalized_email);
        Ok(())
    }

    /// Validate a session token
    ///
    /// # Arguments
    /// * `session_id` - The session identifier
    ///
    /// # Returns
    /// Reference to the session if valid
    pub fn validate_session(&mut self, session_id: &str) -> AuthResult<&Session> {
        self.session_manager.validate_session(session_id)
    }

    /// Sign a transaction using session credentials
    ///
    /// This loads the user's wallet and signs the transaction.
    ///
    /// # Arguments
    /// * `session` - Valid session reference
    /// * `transaction` - Transaction to sign
    ///
    /// # Returns
    /// Signed transaction ready for submission
    ///
    /// # Examples
    /// ```ignore
    /// use kanari_auth::AuthManager;
    /// use kanari_types::transaction::Transaction;
    ///
    /// let mut auth = AuthManager::new();
    /// let session = auth.login("user@example.com", "SecurePassword123!", None)?;
    /// 
    /// // Create your transaction here
    /// let tx = Transaction::Transfer { /* ... */ };
    /// let signed_tx = auth.sign_transaction(&session, tx)?;
    /// ```
    pub fn sign_transaction(
        &self,
        session: &Session,
        transaction: Transaction,
    ) -> AuthResult<SignedTransaction> {
        // Validate session
        if session.is_expired() {
            return Err(AuthError::SessionExpired);
        }

        // Get user to retrieve wallet credentials
        let user = self
            .user_store
            .get_user(&session.email)
            .ok_or(AuthError::UserNotFound(session.email.clone()))?;

        // In a real implementation, decrypt the private key here
        // For now, we'll use the stored key directly (not secure!)
        let private_key = user.encrypted_private_key.as_ref().ok_or_else(|| {
            AuthError::WalletError(kanari_crypto::wallet::WalletError::NotFound(
                "Private key not found".to_string(),
            ))
        })?;

        // Parse wallet address
        let address = AccountAddress::from_str(&user.wallet_address)
            .map_err(|e| AuthError::CryptoError(format!("Invalid wallet address: {}", e)))?;

        // Recreate wallet from stored credentials
        // Note: In production, you should properly decrypt and reconstruct the wallet
        // For this example, we're using Ed25519 as default
        let wallet = wallet::Wallet::new(
            address,
            private_key.clone(),
            String::new(), // Seed phrase would be needed for full reconstruction
            None,
            CurveType::Ed25519, // Would need to be stored with user
        );

        // Sign the transaction
        let mut signed_tx = SignedTransaction::new(transaction);
        signed_tx
            .sign(&wallet.private_key, wallet.curve_type)
            .map_err(|e| AuthError::SigningError(format!("Transaction signing failed: {}", e)))?;

        log::info!(
            "Transaction signed for user: {}, hash: {:?}",
            session.email,
            signed_tx.transaction.hash()
        );

        Ok(signed_tx)
    }

    /// Sign a transfer transaction (convenience method)
    ///
    /// # Arguments
    /// * `session` - Valid session reference
    /// * `to` - Recipient address
    /// * `amount` - Amount in Mist
    /// * `gas_limit` - Gas limit (optional, defaults to 100,000)
    /// * `gas_price` - Gas price in Mist (optional, defaults to 1,000)
    ///
    /// # Returns
    /// Signed transfer transaction
    pub fn sign_transfer(
        &self,
        session: &Session,
        to: &str,
        amount: u64,
        gas_limit: Option<u64>,
        gas_price: Option<u64>,
    ) -> AuthResult<SignedTransaction> {
        let from = session.wallet_address.clone();

        let transaction = Transaction::Transfer {
            from,
            to: to.to_string(),
            amount,
            gas_limit: gas_limit.unwrap_or(100_000),
            gas_price: gas_price.unwrap_or(1_000),
            sequence_number: 0, // Would need to fetch from chain
        };

        self.sign_transaction(session, transaction)
    }

    /// Get user information by session
    ///
    /// # Arguments
    /// * `session` - Valid session reference
    ///
    /// # Returns
    /// User's email and wallet address
    pub fn get_user_info(&self, session: &Session) -> AuthResult<(String, String)> {
        if session.is_expired() {
            return Err(AuthError::SessionExpired);
        }

        let user = self
            .user_store
            .get_user(&session.email)
            .ok_or(AuthError::UserNotFound(session.email.clone()))?;

        Ok((user.email.clone(), user.wallet_address.clone()))
    }

    /// Change user password
    ///
    /// # Arguments
    /// * `email` - User's email
    /// * `old_password` - Current password
    /// * `new_password` - New password
    pub fn change_password(
        &mut self,
        email: &str,
        old_password: &str,
        new_password: &str,
    ) -> AuthResult<()> {
        let normalized_email = email_validator::normalize_email(email);

        let user = self
            .user_store
            .get_user_mut(&normalized_email)
            .ok_or(AuthError::UserNotFound(normalized_email.clone()))?;

        // Verify old password
        if !user.verify_password(old_password)? {
            return Err(AuthError::AuthenticationFailed);
        }

        // Validate new password
        UserRecord::validate_password(new_password)?;

        // Hash and update password
        let new_hash = UserRecord::hash_password(new_password)?;
        user.password_hash = new_hash;

        // Invalidate all sessions for security
        self.logout_all(&normalized_email)?;

        log::info!("Password changed for user: {}", normalized_email);

        Ok(())
    }

    /// Delete a user account
    ///
    /// # Arguments
    /// * `email` - User's email
    /// * `password` - Password for verification
    pub fn delete_account(&mut self, email: &str, password: &str) -> AuthResult<()> {
        let normalized_email = email_validator::normalize_email(email);

        let user = self
            .user_store
            .get_user(&normalized_email)
            .ok_or(AuthError::UserNotFound(normalized_email.clone()))?;

        // Verify password before deletion
        if !user.verify_password(password)? {
            return Err(AuthError::AuthenticationFailed);
        }

        // Invalidate all sessions
        self.logout_all(&normalized_email)?;

        // Remove user
        self.user_store.remove_user(&normalized_email)?;

        log::info!("Account deleted: {}", normalized_email);

        Ok(())
    }

    /// List all registered users
    pub fn list_users(&self) -> Vec<String> {
        self.user_store.list_users()
    }

    /// Get total number of registered users
    pub fn user_count(&self) -> usize {
        self.user_store.user_count()
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_login() {
        let mut auth = AuthManager::new();

        // Register user
        let wallet = auth
            .register_user(
                "test@example.com",
                "SecurePass123!",
                Some(CurveType::Ed25519),
            )
            .unwrap();

        assert!(!wallet.address.to_string().is_empty());

        // Login
        let session = auth
            .login("test@example.com", "SecurePass123!", None)
            .unwrap();

        assert!(!session.session_id.is_empty());
        assert_eq!(session.email, "test@example.com");
    }

    #[test]
    fn test_duplicate_registration() {
        let mut auth = AuthManager::new();

        auth.register_user(
            "test@example.com",
            "SecurePass123!",
            Some(CurveType::Ed25519),
        )
        .unwrap();

        // Second registration should fail
        let result = auth.register_user(
            "test@example.com",
            "AnotherPass456!",
            Some(CurveType::Ed25519),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_login() {
        let mut auth = AuthManager::new();

        auth.register_user(
            "test@example.com",
            "SecurePass123!",
            Some(CurveType::Ed25519),
        )
        .unwrap();

        // Wrong password
        let result = auth.login("test@example.com", "WrongPassword", None);
        assert!(result.is_err());

        // Non-existent user
        let result = auth.login("nonexistent@example.com", "SecurePass123!", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_logout() {
        let mut auth = AuthManager::new();

        auth.register_user(
            "test@example.com",
            "SecurePass123!",
            Some(CurveType::Ed25519),
        )
        .unwrap();

        let session = auth
            .login("test@example.com", "SecurePass123!", None)
            .unwrap();

        let session_id = session.session_id.clone();

        // Logout
        assert!(auth.logout(&session_id).is_ok());

        // Session should be invalid
        assert!(auth.validate_session(&session_id).is_err());
    }

    #[test]
    fn test_password_change() {
        let mut auth = AuthManager::new();

        auth.register_user(
            "test@example.com",
            "SecurePass123!",
            Some(CurveType::Ed25519),
        )
        .unwrap();

        // Change password
        assert!(
            auth.change_password("test@example.com", "SecurePass123!", "NewPass456!")
                .is_ok()
        );

        // Old password should fail
        assert!(
            auth.login("test@example.com", "SecurePass123!", None)
                .is_err()
        );

        // New password should work
        assert!(auth.login("test@example.com", "NewPass456!", None).is_ok());
    }
}
