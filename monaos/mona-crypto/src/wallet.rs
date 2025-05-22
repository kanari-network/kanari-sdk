//! Wallet management functionality
//!
//! This module handles wallet operations including creation, encryption, 
//! storage, and loading of cryptocurrency wallets.

use std::fs;
use std::io;
use std::path::PathBuf;
use key::keys::CurveType;
use thiserror::Error;
use serde::{Serialize, Deserialize};

use mona_types::address::Address;
use common::{get_kari_dir, load_config, save_config, load_kanari_config, save_kanari_config};
use serde_yaml::{Mapping, Value};

use crate::encryption;
use crate::signatures;
use crate::EncryptedData;
use crate::Keystore;

/// Errors that can occur during wallet operations
#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    
    #[error("Decryption error: {0}")]
    DecryptionError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Wallet not found: {0}")]
    NotFound(String),
    
    #[error("Invalid password")]
    InvalidPassword,
    
    #[error("Signing error: {0}")]
    SigningError(String),
    
    #[error("Keystore error: {0}")]
    KeystoreError(String),
}

/// Structure representing a wallet with private key and address
#[derive(Serialize, Deserialize, Clone)]
pub struct Wallet {
    pub address: Address,
    pub private_key: String,
    pub seed_phrase: String,
    pub curve_type: CurveType,
}

impl Wallet {
    /// Create a new wallet instance
    pub fn new(
        address: Address,
        private_key: String,
        seed_phrase: String,
        curve_type: CurveType,
    ) -> Self {
        Self {
            address,
            private_key,
            seed_phrase,
            curve_type,
        }
    }
    
    /// Sign a message using this wallet's private key
    pub fn sign(
        &self,
        message: &[u8],
        password: &str,
    ) -> Result<Vec<u8>, WalletError> {
        // Validate message is not empty
        if message.is_empty() {
            return Err(WalletError::SigningError("Cannot sign empty message".to_string()));
        }
        
        // Validate password is not empty - this makes the parameter used and required
        if password.is_empty() {
            return Err(WalletError::InvalidPassword);
        }
        
        // Create a temporary copy of the private key for signing
        let private_key_copy = self.private_key.clone();
        
        // Sign the message
        let result = signatures::sign_message(
            &private_key_copy,
            message,
            self.curve_type,
        ).map_err(|e| WalletError::SigningError(e.to_string()));
        
        // Securely clear the private key copy from memory
        let mut private_key_bytes = private_key_copy.into_bytes();
        signatures::secure_clear(&mut private_key_bytes);
        
        result
    }
    
    /// Verify a signature made with this wallet against a message
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool, WalletError> {
        // Validate inputs
        if message.is_empty() {
            return Err(WalletError::SigningError("Cannot verify empty message".to_string()));
        }
        
        if signature.is_empty() {
            return Err(WalletError::SigningError("Cannot verify empty signature".to_string()));
        }
        
        signatures::verify_signature_with_curve(
            &self.address.to_string(),
            message,
            signature,
            self.curve_type,
        ).map_err(|e| WalletError::SigningError(e.to_string()))
    }
}

/// Save a wallet to the keystore
pub fn save_wallet(
    address: &Address,
    private_key: &str,
    seed_phrase: &str,
    password: &str,
    curve_type: CurveType,
) -> Result<(), WalletError> {
    // Validate inputs
    if password.is_empty() {
        return Err(WalletError::EncryptionError("Empty password not allowed".to_string()));
    }
    
    if private_key.is_empty() {
        return Err(WalletError::EncryptionError("Empty private key not allowed".to_string()));
    }
    
    // Create wallet object
    let wallet_data = Wallet {
        address: *address,
        private_key: private_key.to_string(),
        seed_phrase: seed_phrase.to_string(),
        curve_type,
    };

    // Serialize wallet to TOML (more readable than JSON)
    let toml_string = toml::to_string(&wallet_data)
        .map_err(|e| WalletError::SerializationError(e.to_string()))?;

    // Encrypt the wallet data
    let encrypted_data = encryption::encrypt_data(
        toml_string.as_bytes(),
        password
    ).map_err(|e| WalletError::EncryptionError(e.to_string()))?;

    // Load or create the keystore
    let mut keystore = Keystore::load()
        .map_err(|e| WalletError::KeystoreError(e.to_string()))?;
    
    // Add the wallet to the keystore with the address as the key
    keystore.add_wallet(&address.to_string(), encrypted_data)
        .map_err(|e| WalletError::KeystoreError(e.to_string()))?;
    
    // Also update the active_address in kanari.yaml
    update_active_address(&address.to_string())?;
    
    Ok(())
}

/// Load a wallet from the keystore
pub fn load_wallet(address: &str, password: &str) -> Result<Wallet, WalletError> {
    // Validate inputs
    if address.is_empty() {
        return Err(WalletError::NotFound("Empty address".to_string()));
    }
    
    if password.is_empty() {
        return Err(WalletError::InvalidPassword);
    }
    
    // Load the keystore
    let keystore = Keystore::load()
        .map_err(|e| WalletError::KeystoreError(e.to_string()))?;
    
    // Get the encrypted data for this wallet
    let encrypted_data = keystore.get_wallet(address)
        .ok_or_else(|| WalletError::NotFound(address.to_string()))?;
    
    // Decrypt wallet data
    let decrypted = encryption::decrypt_data(encrypted_data, password)
        .map_err(|_| WalletError::InvalidPassword)?;

    // Parse wallet data
    let decrypted_str = String::from_utf8(decrypted)
        .map_err(|e| WalletError::DecryptionError(e.to_string()))?;

    // Parse TOML
    let wallet_data: Wallet = toml::from_str(&decrypted_str)
        .map_err(|e| WalletError::SerializationError(e.to_string()))?;

    Ok(wallet_data)
}

/// Check if any wallets exist
pub fn check_wallet_exists() -> bool {
    if let Ok(keystore) = Keystore::load() {
        !keystore.list_wallets().is_empty()
    } else {
        false
    }
}

/// List all available wallets with selection status
pub fn list_wallet_files() -> Result<Vec<(String, bool)>, io::Error> {
    // Get currently selected wallet
    let selected = get_selected_wallet().unwrap_or_default();
    let mut wallets = Vec::new();
    
    // Try to load the keystore
    match Keystore::load() {
        Ok(keystore) => {
            // Return addresses from the keystore
            for address in keystore.list_wallets() {
                let is_selected = address == selected;
                wallets.push((address, is_selected));
            }
        },
        Err(_) => {
            // Fall back to checking legacy wallet files (one-time migration path)
            let kari_dir = get_kari_dir();
            let wallet_dir = kari_dir.join("wallets");
            
            if wallet_dir.exists() {
                for entry in fs::read_dir(wallet_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    
                    if path.is_file() {
                        if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                            if filename.ends_with(".enc") {
                                let wallet_address = filename.trim_end_matches(".enc").to_string();
                                let is_selected = wallet_address == selected;
                                wallets.push((wallet_address, is_selected));
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Sort wallets alphabetically
    wallets.sort_by(|a, b| a.0.cmp(&b.0));
    
    Ok(wallets)
}

/// Set the currently selected wallet address in configuration
pub fn set_selected_wallet(wallet_address: &str) -> io::Result<()> {
    // Strip any extension to get the clean address
    let formatted_address = wallet_address.trim_end_matches(".enc").to_string();
    
    // Update active_address in kanari.yaml
    update_active_address(&formatted_address)?;
    
    // Keep the old config.yaml updated for backward compatibility
    // Load existing config
    let mut config = load_config()?;
    
    // Update address in config using the keys expected by the system
    if let Some(mapping) = config.as_mapping_mut() {
        // Set both keys for maximum compatibility
        mapping.insert(
            Value::String("address".to_string()),
            Value::String(formatted_address.clone()),
        );

        mapping.insert(
            Value::String("selected_wallet".to_string()),
            Value::String(formatted_address),
        );
    } else {
        // Create new mapping if none exists
        let mut mapping = Mapping::new();
        mapping.insert(
            Value::String("address".to_string()),
            Value::String(formatted_address.clone()),
        );
        mapping.insert(
            Value::String("selected_wallet".to_string()),
            Value::String(formatted_address),
        );
        config = Value::Mapping(mapping);
    }

    // Save updated config
    save_config(&config)
}

/// Helper function to update active_address in kanari.yaml
fn update_active_address(address: &str) -> io::Result<()> {
    // Try to load kanari config
    match load_kanari_config() {
        Ok(mut kanari_config) => {
            if let Some(mapping) = kanari_config.as_mapping_mut() {
                mapping.insert(
                    Value::String("active_address".to_string()),
                    Value::String(address.to_string()),
                );
                save_kanari_config(&kanari_config)?;
            }
        },
        Err(_) => {
            // If kanari config doesn't exist or load, create it
            let mut mapping = Mapping::new();
            mapping.insert(
                Value::String("active_address".to_string()),
                Value::String(address.to_string()),
            );
            save_kanari_config(&Value::Mapping(mapping))?;
        }
    }
    Ok(())
}

/// Get the currently selected wallet from configuration
pub fn get_selected_wallet() -> Option<String> {
    // First try to get from kanari config
    if let Ok(kanari_config) = load_kanari_config() {
        if let Some(active_address) = kanari_config.get("active_address").and_then(|v| v.as_str()) {
            return Some(active_address.to_string());
        }
    }
    
    // Fall back to legacy config
    match load_config() {
        Ok(config) => {
            if let Some(mapping) = config.as_mapping() {
                // Try each possible key for wallet selection
                if let Some(wallet) = mapping.get("selected_wallet").and_then(|v| v.as_str()) {
                    return Some(wallet.trim_end_matches(".enc").to_string());
                }

                if let Some(wallet) = mapping.get("address").and_then(|v| v.as_str()) {
                    return Some(wallet.trim_end_matches(".enc").to_string());
                }
            }
            None
        }
        Err(_) => None,
    }
}

/// Get the path to the wallet directory (legacy)
pub fn get_wallet_dir() -> PathBuf {
    get_kari_dir().join("wallets")
}

/// Migrate legacy .enc wallet files to the keystore format
pub fn migrate_legacy_wallets(password: &str) -> Result<usize, WalletError> {
    let _ = password;
    let wallet_dir = get_wallet_dir();
    if !wallet_dir.exists() {
        return Ok(0);
    }
    
    let mut migrated_count = 0;
    let mut keystore = Keystore::load()
        .map_err(|e| WalletError::KeystoreError(e.to_string()))?;
    
    // Read each .enc file
    match fs::read_dir(wallet_dir) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                            if filename.ends_with(".enc") {
                                // Extract address from filename
                                let address = filename.trim_end_matches(".enc");
                                
                                // Skip if already in keystore
                                if keystore.wallet_exists(address) {
                                    continue;
                                }
                                
                                // Read encrypted wallet data
                                if let Ok(encrypted_json) = fs::read_to_string(&path) {
                                    if let Ok(encrypted_data) = serde_json::from_str::<EncryptedData>(&encrypted_json) {
                                        // Add to keystore
                                        if keystore.add_wallet(address, encrypted_data).is_ok() {
                                            migrated_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        Err(e) => {
            return Err(WalletError::IoError(e));
        }
    }
    
    // Save the keystore after migration
    keystore.save().map_err(|e| WalletError::KeystoreError(e.to_string()))?;
    
    Ok(migrated_count)
}
