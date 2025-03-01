use bip39::rand::rngs::OsRng;
use bip39::{rand, Language, Mnemonic};
use log::error;
use mona_types::address::Address;
use serde::{Deserialize, Serialize};
use std::{fs, io};

use hex;
use secp256k1::{Secp256k1, SecretKey};
use thiserror::Error;

// Import Mutex and HashMap from std::sync
use k2::{
    blockchain::get_kari_dir,
    config::{load_config, save_config},
};
use serde_yaml::{Mapping, Value};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    #[error("Decryption error: {0}")]
    DecryptionError(String),
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptedData {
    ciphertext: Vec<u8>,
    salt: String,
    nonce: Vec<u8>,
}

fn derive_key(password: &str, salt: &SaltString) -> Result<[u8; 32], WalletError> {
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), salt)
        .map_err(|e| WalletError::EncryptionError(e.to_string()))?;
    let mut key = [0u8; 32];
    key.copy_from_slice(&password_hash.hash.unwrap().as_bytes()[0..32]);
    Ok(key)
}

pub fn check_wallet_exists() -> bool {
    match list_wallet_files() {
        Ok(wallets) => !wallets.is_empty(),
        Err(_) => false,
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Wallet {
    pub address: Address,
    pub private_key: String,
    pub seed_phrase: String,
}

/// Set the selected wallet address in the configuration
pub fn set_selected_wallet(wallet_address: &str) -> io::Result<()> {
    // Load existing config
    let mut config = load_config()?;

    // Update address in config
    if let Some(mapping) = config.as_mapping_mut() {
        mapping.insert(
            Value::String("address".to_string()),
            Value::String(wallet_address.to_string()),
        );
    } else {
        // Create new mapping if none exists
        let mut mapping = Mapping::new();
        mapping.insert(
            Value::String("address".to_string()),
            Value::String(wallet_address.to_string()),
        );
        config = Value::Mapping(mapping);
    }

    // Save updated config
    save_config(&config)
}

pub fn save_wallet(
    address: &Address,
    private_key: &str,
    seed_phrase: &str,
    password: &str,
) -> Result<(), WalletError> {
    let wallet_data = Wallet {
        address: *address,
        private_key: private_key.to_string(),
        seed_phrase: seed_phrase.to_string(),
    };

    let salt = SaltString::generate(&mut OsRng);
    let key = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
    let binding = rand::random::<[u8; 12]>();
    let nonce = Nonce::from_slice(&binding);

    let toml_string =
        toml::to_string(&wallet_data).map_err(|e| WalletError::EncryptionError(e.to_string()))?;

    let encrypted = cipher
        .encrypt(nonce, toml_string.as_bytes())
        .map_err(|e| WalletError::EncryptionError(e.to_string()))?;

    let encrypted_data = EncryptedData {
        ciphertext: encrypted,
        salt: salt.to_string(),
        nonce: nonce.to_vec(),
    };

    let kari_dir = get_kari_dir();
    let wallet_dir = kari_dir.join("wallets");
    fs::create_dir_all(&wallet_dir)?;

    let wallet_file = wallet_dir.join(format!("{}.enc", address));
    let encrypted_json = serde_json::to_string(&encrypted_data)
        .map_err(|e| WalletError::EncryptionError(e.to_string()))?;

    fs::write(wallet_file, encrypted_json)?;
    Ok(())
}

pub fn load_wallet(address: &str, password: &str) -> Result<Wallet, WalletError> {
    let kari_dir = get_kari_dir();
    let wallet_file = kari_dir.join("wallets").join(format!("{}.enc", address));

    let encrypted_json = fs::read_to_string(wallet_file)?;
    let encrypted_data: EncryptedData = serde_json::from_str(&encrypted_json)
        .map_err(|e| WalletError::DecryptionError(e.to_string()))?;

    let salt = SaltString::from_b64(&encrypted_data.salt)
        .map_err(|e| WalletError::DecryptionError(e.to_string()))?;
    let key = derive_key(password, &salt)?;

    let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
    let nonce = Nonce::from_slice(&encrypted_data.nonce);

    let decrypted = cipher
        .decrypt(nonce, encrypted_data.ciphertext.as_slice())
        .map_err(|e| WalletError::DecryptionError(e.to_string()))?;

    let decrypted_str =
        String::from_utf8(decrypted).map_err(|e| WalletError::DecryptionError(e.to_string()))?;

    let wallet_data: Wallet =
        toml::from_str(&decrypted_str).map_err(|e| WalletError::DecryptionError(e.to_string()))?;

    Ok(wallet_data)
}

pub fn generate_karix_address(word_count: usize) -> (String, String, String) {
    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);

    // Serialize and encode the public key
    let mut hex_encoded = hex::encode(&public_key.serialize_uncompressed()[1..]);
    hex_encoded.truncate(64); // Adjust as needed

    let karix_public_address = format!("0x{}", hex_encoded);

    // Generate mnemonic with specified word count
    let mnemonic_result = match word_count {
        12 => Mnemonic::generate(12),
        24 => Mnemonic::generate(24),
        _ => panic!("Unsupported word count: {}", word_count),
    };

    let mnemonic = match mnemonic_result {
        Ok(m) => m,
        Err(e) => panic!("Failed to generate mnemonic: {:?}", e),
    };
    let seed_phrase = mnemonic.to_string(); // Directly convert Mnemonic to String

    (
        secret_key.display_secret().to_string(),
        karix_public_address,
        seed_phrase,
    )
}

/// Returns list of wallet files with selection status
///
/// # Returns
/// * `Result<Vec<(String, bool)>>` - List of (wallet_filename, is_selected) tuples
pub fn list_wallet_files() -> Result<Vec<(String, bool)>, std::io::Error> {
    let kari_dir = get_kari_dir();
    let wallet_dir = kari_dir.join("wallets");

    // Create wallet directory if it doesn't exist
    if !wallet_dir.exists() {
        fs::create_dir_all(&wallet_dir)?;
    }

    // Get currently selected wallet
    let selected = get_selected_wallet().unwrap_or_default();

    let mut wallets = Vec::new();
    for entry in fs::read_dir(wallet_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                // Only include .enc files
                if filename.ends_with(".enc") {
                    // Check if this wallet is selected
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

// import_from_seed_phrase
pub fn import_from_seed_phrase(
    phrase: &str,
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    // Validate and create mnemonic
    let mnemonic = Mnemonic::parse_in(Language::English, phrase)?;

    // Generate seed from mnemonic
    let seed = mnemonic.to_seed("");

    // Create private key from seed
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&seed[0..32])?;
    let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);

    // Generate addresses
    let private_key = hex::encode(secret_key.as_ref());
    let mut hex_encoded = hex::encode(&public_key.serialize_uncompressed()[1..]);
    hex_encoded.truncate(64);
    let public_address = format!("0x{}", hex_encoded);

    Ok((private_key, hex_encoded, public_address))
}

// import_from_private_key
pub fn import_from_private_key(
    private_key: &str,
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    // Convert hex private key to bytes
    let private_key_bytes = hex::decode(private_key)?;

    // Create secret key and generate public key
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&private_key_bytes)?;
    let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);

    // Generate addresses
    let mut hex_encoded = hex::encode(&public_key.serialize_uncompressed()[1..]);
    hex_encoded.truncate(64);
    let public_address = format!("0x{}", hex_encoded);

    Ok((private_key.to_string(), hex_encoded, public_address))
}

/// Read currently selected wallet from config
fn get_selected_wallet() -> Option<String> {
    let config_path = get_kari_dir().join("config.toml");
    fs::read_to_string(config_path)
        .ok()
        .and_then(|data| toml::from_str::<toml::Value>(&data).ok())
        .and_then(|toml| toml.get("selected_wallet")?.as_str().map(String::from))
}
