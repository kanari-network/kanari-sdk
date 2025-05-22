//! Keystore management functionality
//!
//! This module handles the kanari.keystore format for secure storage of wallet information.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

use common::get_kanari_config_path;

use crate::encryption::EncryptedData;

/// Errors related to keystore operations
#[derive(Error, Debug)]
pub enum KeystoreError {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    #[error("Invalid keystore format")]
    InvalidFormat,
    
    #[error("Password verification failed")]
    PasswordVerificationFailed,
}

/// Structure representing the keystore file
#[derive(Serialize, Deserialize, Default)]
pub struct Keystore {
    /// Individual wallet keys by address
    pub keys: HashMap<String, EncryptedData>,
    
    /// Mnemonic phrase information
    pub mnemonic: MnemonicStore,
    
    /// Temporary session keys
    pub session_keys: HashMap<String, String>,
    
    /// Hashed master password for verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_hash: Option<String>,
    
    /// Whether the password is empty
    #[serde(default)]
    pub is_password_empty: bool,
}

/// Structure for storing mnemonic phrases
#[derive(Serialize, Deserialize, Default)]
pub struct MnemonicStore {
    /// List of addresses derived from the mnemonic
    pub addresses: Vec<String>,
    
    /// Encrypted mnemonic phrase
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mnemonic_phrase_encryption: Option<EncryptedData>,
}

impl Keystore {
    /// Load keystore from disk
    pub fn load() -> Result<Self, KeystoreError> {
        let keystore_path = get_keystore_path();
        
        if !keystore_path.exists() {
            return Ok(Keystore::default());
        }
        
        // Load the keystore data
        let keystore_data = fs::read_to_string(keystore_path)?;
        let mut keystore: Keystore = serde_json::from_str(&keystore_data)?;
        
        // Upgrade any keys that might be using the old format
        for (_, encrypted_data) in keystore.keys.iter_mut() {
            *encrypted_data = crate::encryption::upgrade_encrypted_data(encrypted_data.clone());
        }
        
        // Save if any changes were made (conversion from array to base64)
        keystore.save()?;
        
        Ok(keystore)
    }
    
    /// Save keystore to disk
    pub fn save(&self) -> Result<(), KeystoreError> {
        let keystore_path = get_keystore_path();
        let keystore_dir = keystore_path.parent().unwrap();
        
        // Create directory if it doesn't exist
        if !keystore_dir.exists() {
            fs::create_dir_all(keystore_dir)?;
        }
        
        let keystore_data = serde_json::to_string_pretty(self)?;
        fs::write(keystore_path, keystore_data)?;
        
        Ok(())
    }
    
    /// Add a wallet to the keystore
    pub fn add_wallet(&mut self, address: &str, encrypted_data: EncryptedData) -> Result<(), KeystoreError> {
        self.keys.insert(address.to_string(), encrypted_data);
        self.save()?;
        Ok(())
    }
    
    /// Get a wallet from the keystore
    pub fn get_wallet(&self, address: &str) -> Option<&EncryptedData> {
        self.keys.get(address)
    }
    
    /// Remove a wallet from the keystore
    pub fn remove_wallet(&mut self, address: &str) -> Result<(), KeystoreError> {
        if self.keys.remove(address).is_none() {
            return Err(KeystoreError::KeyNotFound(address.to_string()));
        }
        
        // Also remove from mnemonic addresses if present
        self.mnemonic.addresses.retain(|addr| addr != address);
        
        self.save()?;
        Ok(())
    }
    
    /// Check if a wallet exists in the keystore
    pub fn wallet_exists(&self, address: &str) -> bool {
        self.keys.contains_key(address)
    }
    
    /// List all wallets in the keystore
    pub fn list_wallets(&self) -> Vec<String> {
        self.keys.keys().cloned().collect()
    }
}

/// Get path to the keystore file
pub fn get_keystore_path() -> PathBuf {
    let mut keystore_dir = get_kanari_config_path();
    // Remove 'kanari.yaml' from the path and add 'kanari.keystore'
    keystore_dir.pop();
    keystore_dir.push("kanari.keystore");
    keystore_dir
}

/// Check if keystore file exists
pub fn keystore_exists() -> bool {
    get_keystore_path().exists()
}
