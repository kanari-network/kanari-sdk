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
use common::{get_kari_dir, load_config, save_config};
use serde_yaml::{Mapping, Value};

use crate::encryption;
use crate::encryption::EncryptedData;
use crate::signatures;



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
        _password: &str, // Added underscore to mark as intentionally unused
    ) -> Result<Vec<u8>, WalletError> {
        // Validate message is not empty
        if message.is_empty() {
            return Err(WalletError::SigningError("Cannot sign empty message".to_string()));
        }
        
        signatures::sign_message(
            &self.private_key,
            message,
            self.curve_type,
        ).map_err(|e| WalletError::SigningError(e.to_string()))
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

/// Save a wallet to disk with encryption
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

    // Serialize the encrypted data
    let encrypted_json = serde_json::to_string(&encrypted_data)
        .map_err(|e| WalletError::SerializationError(e.to_string()))?;

    // Ensure wallet directory exists
    let kari_dir = get_kari_dir();
    let wallet_dir = kari_dir.join("wallets");
    fs::create_dir_all(&wallet_dir)?;

    // Write encrypted wallet to file
    let wallet_file = wallet_dir.join(format!("{}.enc", address));
    fs::write(wallet_file, encrypted_json)?;
    
    Ok(())
}

/// Load a wallet from disk and decrypt it
pub fn load_wallet(address: &str, password: &str) -> Result<Wallet, WalletError> {
    // Validate inputs
    if address.is_empty() {
        return Err(WalletError::NotFound("Empty address".to_string()));
    }
    
    if password.is_empty() {
        return Err(WalletError::InvalidPassword);
    }
    
    // Get path to wallet file
    let kari_dir = get_kari_dir();
    let wallet_file = kari_dir.join("wallets").join(format!("{}.enc", address));

    // Read encrypted wallet data
    let encrypted_json = fs::read_to_string(&wallet_file)
        .map_err(|_| WalletError::NotFound(address.to_string()))?;

    // Parse encrypted data
    let encrypted_data: EncryptedData = serde_json::from_str(&encrypted_json)
        .map_err(|e| WalletError::SerializationError(e.to_string()))?;

    // Decrypt wallet data
    let decrypted = encryption::decrypt_data(&encrypted_data, password)
        .map_err(|_| WalletError::InvalidPassword)?;

    // Parse wallet data
    let decrypted_str = String::from_utf8(decrypted)
        .map_err(|e| WalletError::DecryptionError(e.to_string()))?;

    // Parse TOML
    let wallet_data: Wallet = toml::from_str(&decrypted_str)
        .map_err(|e| WalletError::SerializationError(e.to_string()))?;

    Ok(wallet_data)
}

/// Check if any wallets exist on the system
pub fn check_wallet_exists() -> bool {
    match list_wallet_files() {
        Ok(wallets) => !wallets.is_empty(),
        Err(_) => false,
    }
}

/// List all available wallets with selection status
pub fn list_wallet_files() -> Result<Vec<(String, bool)>, io::Error> {
    // Get wallet directory
    let kari_dir = get_kari_dir();
    let wallet_dir = kari_dir.join("wallets");

    // Create wallet directory if it doesn't exist
    if !wallet_dir.exists() {
        fs::create_dir_all(&wallet_dir)?;
    }

    // Get currently selected wallet
    let selected = get_selected_wallet().unwrap_or_default();
    
    let mut wallets = Vec::new();
    
    // Read all wallet files
    for entry in fs::read_dir(wallet_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                // Only include .enc files
                if filename.ends_with(".enc") {
                    // Get wallet address without .enc extension
                    let wallet_name = filename.trim_end_matches(".enc");
                    let is_selected = wallet_name == selected;
                    wallets.push((filename.to_string(), is_selected));
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
    // Load existing config
    let mut config = load_config()?;

    // Format the address (remove .enc if present)
    let formatted_address = wallet_address.trim_end_matches(".enc").to_string();

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

/// Get the currently selected wallet from configuration
pub fn get_selected_wallet() -> Option<String> {
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

/// Get the path to the wallet directory
pub fn get_wallet_dir() -> PathBuf {
    get_kari_dir().join("wallets")
}
