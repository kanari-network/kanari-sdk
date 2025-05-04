//! Secure data encryption and decryption
//!
//! This module provides functions for encrypting and decrypting data using
//! modern secure algorithms (AES-256-GCM with Argon2 key derivation).

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString},
};
use bip39::rand::{rngs::OsRng, RngCore};
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// Errors that can occur during encryption/decryption
#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Encryption error: {0}")]
    EncryptionFailed(String),

    #[error("Decryption error: {0}")]
    DecryptionFailed(String),
    
    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Structure for encrypted data with necessary metadata
#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptedData {
    /// The encrypted data (ciphertext)
    pub ciphertext: Vec<u8>,
    
    /// Salt used for key derivation
    pub salt: String,
    
    /// Nonce used for encryption
    pub nonce: Vec<u8>,
    
    /// Optional tag for authentication
    pub tag: Option<Vec<u8>>,
}

/// Encrypt data using a password
pub fn encrypt_data(data: &[u8], password: &str) -> Result<EncryptedData, EncryptionError> {
    // Generate a random salt
    let salt = SaltString::generate(&mut OsRng);
    
    // Derive encryption key from password
    let key = derive_key(password, &salt)?;
    
    // Initialize AES-GCM cipher
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;
    
    // Generate random nonce
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    // Encrypt the data
    let ciphertext = cipher.encrypt(nonce, data)
        .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;
    
    Ok(EncryptedData {
        ciphertext,
        salt: salt.to_string(),
        nonce: nonce.to_vec(),
        tag: None, // AES-GCM includes the tag with the ciphertext
    })
}

/// Decrypt data using a password
pub fn decrypt_data(encrypted_data: &EncryptedData, password: &str) -> Result<Vec<u8>, EncryptionError> {
    // Parse the salt
    let salt = SaltString::from_b64(&encrypted_data.salt)
        .map_err(|e| EncryptionError::DecryptionFailed(format!("Invalid salt: {}", e)))?;
    
    // Derive the key from password and salt
    let key = derive_key(password, &salt)?;
    
    // Initialize AES-GCM cipher
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;
    
    // Create nonce from stored bytes
    let nonce = Nonce::from_slice(&encrypted_data.nonce);
    
    // Decrypt the data
    let plaintext = cipher.decrypt(nonce, encrypted_data.ciphertext.as_slice())
        .map_err(|e| EncryptionError::DecryptionFailed(format!("Decryption failed: {}", e)))?;
    
    Ok(plaintext)
}

/// Derive a cryptographic key from a password using Argon2
fn derive_key(password: &str, salt: &SaltString) -> Result<[u8; 32], EncryptionError> {
    // Configure Argon2 with secure parameters
    let argon2 = Argon2::default();
    
    // Generate password hash
    let password_hash = argon2.hash_password(password.as_bytes(), salt)
        .map_err(|e| EncryptionError::KeyDerivationFailed(e.to_string()))?;
    
    // Extract bytes for the key
    let hash_bytes = password_hash.hash
        .ok_or_else(|| EncryptionError::KeyDerivationFailed("No hash produced".to_string()))?;
    
    // Ensure we have enough bytes for the key
    if hash_bytes.as_bytes().len() < 32 {
        return Err(EncryptionError::KeyDerivationFailed(
            "Hash too short for key derivation".to_string()
        ));
    }
    
    // Copy first 32 bytes into key array
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash_bytes.as_bytes()[0..32]);
    
    Ok(key)
}

/// Encrypt a string with a password and convert to base64
pub fn encrypt_string(text: &str, password: &str) -> Result<String, EncryptionError> {
    // Encrypt the text
    let encrypted = encrypt_data(text.as_bytes(), password)?;
    
    // Serialize to JSON and encode as base64
    let json = serde_json::to_string(&encrypted)
        .map_err(|e| EncryptionError::SerializationError(e.to_string()))?;
    
    Ok(base64::encode(json))
}

/// Decrypt a base64-encoded encrypted string
pub fn decrypt_string(encoded: &str, password: &str) -> Result<String, EncryptionError> {
    // Decode from base64
    let json_bytes = base64::decode(encoded)
        .map_err(|e| EncryptionError::InvalidInput(format!("Invalid base64: {}", e)))?;
    
    // Parse JSON
    let encrypted: EncryptedData = serde_json::from_slice(&json_bytes)
        .map_err(|e| EncryptionError::SerializationError(e.to_string()))?;
    
    // Decrypt the data
    let plaintext = decrypt_data(&encrypted, password)?;
    
    // Convert back to string
    String::from_utf8(plaintext)
        .map_err(|e| EncryptionError::InvalidInput(format!("Invalid UTF-8: {}", e)))
}
