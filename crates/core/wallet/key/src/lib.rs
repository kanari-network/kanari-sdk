use std::{fs, io::{self, Write}, path::PathBuf, str::FromStr, };
use serde::{Deserialize, Serialize};
use bip39::{Mnemonic, Language};
use log::{debug, error};
use move_core_types::{
    account_address::AccountAddress,
    // identifier::Identifier,
};
use secp256k1::{Secp256k1, SecretKey, PublicKey};
use rand::rngs::OsRng;
use hex;
use thiserror::Error;

// Import Mutex and HashMap from std::sync
use k2::{blockchain::get_kari_dir, config::{load_config, save_config}};
use serde_json::json;

pub fn check_wallet_exists() -> bool {
    match list_wallet_files() {
        Ok(wallets) => !wallets.is_empty(),
        Err(_) => false
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Wallet {
    pub address: String,
    pub private_key: String,
    pub seed_phrase: String,
}

pub fn save_wallet(address: &str, private_key: &str, seed_phrase: &str) {
    let kari_dir = get_kari_dir();
    let wallet_dir = kari_dir.join("wallets");
    fs::create_dir_all(&wallet_dir).expect("Unable to create wallets directory");

    let wallet_file = wallet_dir.join(format!("{}.toml", address));
    let wallet_data = Wallet {
        address: address.to_string(),
        private_key: private_key.to_string(),
        seed_phrase: seed_phrase.to_string(),
    };

    let toml_string = toml::to_string(&wallet_data).expect("Unable to serialize wallet to TOML");
    let mut file = fs::File::create(&wallet_file).expect("Unable to create wallet file");
    file.write_all(toml_string.as_bytes()).expect("Unable to write wallet to file");
    println!("Wallet saved to {:?}", wallet_file);
}

/// Set the selected wallet address in the configuration
pub fn set_selected_wallet(wallet_address: &str) -> io::Result<()> {
    // Load existing config
    let mut config = load_config()?;
    
    // Update miner_address in config
    if let Some(obj) = config.as_object_mut() {
        obj.insert("miner_address".to_string(), json!(wallet_address));
    }

    // Save updated config
    save_config(&config)
}

pub fn load_wallet(address: &str) -> Option<Wallet> {
    // Input validation with logging
    if address.trim().is_empty() {
        debug!("Attempted to load wallet with empty address");
        return None;
    }

    let kari_dir = get_kari_dir();
    let wallet_file: PathBuf = kari_dir.join("wallets").join(format!("{}.toml", address));

    debug!("Attempting to load wallet from: {}", wallet_file.display());

    if wallet_file.exists() {
        // Handle potential IO and parsing errors with logging
        fs::read_to_string(&wallet_file)
            .map_err(|e| {
                error!("Failed to read wallet file: {}", e);
                e
            })
            .ok()
            .and_then(|data| {
                toml::from_str(&data).map_err(|e| {
                    error!("Failed to parse wallet TOML: {}", e);
                    e
                }).ok()
            })
    } else {
        debug!("Wallet file not found: {}", wallet_file.display());
        None
    }
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
        seed_phrase
    )
}


/// Returns list of wallet files with selection status
/// 
/// # Returns
/// * `Result<Vec<(String, bool)>>` - List of (wallet_filename, is_selected) tuples
pub fn list_wallet_files() -> Result<Vec<(String, bool)>, std::io::Error> {
    let kari_dir = get_kari_dir();
    let wallet_dir = kari_dir.join("wallets");
    
    // Get currently selected wallet
    let selected = get_selected_wallet().unwrap_or_default();

    let mut wallets = Vec::new();
    for entry in fs::read_dir(wallet_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                // Check if this wallet is selected
                let is_selected = filename.trim_end_matches(".toml") == selected;
                wallets.push((filename.to_string(), is_selected));
            }
        }
    }
    Ok(wallets)
}

pub fn import_from_seed_phrase(phrase: &str) -> Result<(String, String, String), Box<dyn std::error::Error>> {
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


pub fn import_from_private_key(private_key: &str) -> Result<(String, String, String), Box<dyn std::error::Error>> {
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


#[derive(Error, Debug)]
pub enum KeyError {
    #[error("Invalid seed phrase")]
    InvalidSeedPhrase,
    #[error("Invalid key format")]
    InvalidKeyFormat,
    #[error("Address generation failed")]
    AddressGenerationFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveKeyPair {
    pub private_key: String,
    pub public_key: String,
    pub address: AccountAddress,
    pub seed_phrase: String,
}

impl MoveKeyPair {
    pub fn generate(word_count: usize) -> Result<Self, KeyError> {
        let (secret_key, pub_addr, seed) = generate_karix_address(word_count);
        
        // Convert to Move address format
        let address_bytes = hex::decode(&pub_addr[2..])
            .map_err(|_| KeyError::AddressGenerationFailed)?;
        let move_address = AccountAddress::new(
            address_bytes.try_into()
                .map_err(|_| KeyError::AddressGenerationFailed)?
        );

        Ok(Self {
            private_key: secret_key,
            public_key: pub_addr,
            address: move_address,
            seed_phrase: seed,
        })
    }

    pub fn from_private_key(private_key: &str) -> Result<Self, KeyError> {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_str(private_key)
            .map_err(|_| KeyError::InvalidKeyFormat)?;
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);

        // Generate Move address from public key
        let pub_bytes = public_key.serialize_uncompressed();
        let move_address = AccountAddress::new(
            pub_bytes[1..33].try_into()
                .map_err(|_| KeyError::AddressGenerationFailed)?
        );

        Ok(Self {
            private_key: private_key.to_string(),
            public_key: hex::encode(&pub_bytes[1..]),
            address: move_address,
            seed_phrase: String::new(), // Empty for key-derived wallets
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let result = MoveKeyPair::generate(12);
        assert!(result.is_ok());
        let keypair = result.unwrap();
        println!("Private key: {}", keypair.private_key);
        assert!(!keypair.private_key.is_empty());
        println!("Public key: {}", keypair.public_key);
        assert!(!keypair.seed_phrase.is_empty());
    }
}
