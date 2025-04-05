use bip39::rand::rngs::OsRng;
use bip39::{Language, Mnemonic, rand};
use k256::ecdsa::signature::{SignatureEncoding, SignerMut, Verifier };
use log::error;
use mona_types::address::Address;
use serde::{Deserialize, Serialize};
use std::{fs, io};

use hex;
// Update k256 imports to include ecdsa types
use k256::{
    PublicKey as K256PublicKey, SecretKey as K256SecretKey,
    ecdsa::{
        Signature as K256Signature, SigningKey as K256SigningKey, VerifyingKey as K256VerifyingKey,
    },
    elliptic_curve::sec1::ToEncodedPoint,
};
use p256::{
    SecretKey as P256SecretKey,
    ecdsa::{Signature as P256Signature, SigningKey, VerifyingKey},
};
use thiserror::Error;

// Replace panorama imports with common
use common::{get_kari_dir, load_config, save_config};
use serde_yaml::{Mapping, Value};

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString},
};
// Replace Sha256 with Sha3
use sha3::{Digest, Sha3_256};

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
    pub ciphertext: Vec<u8>,
    pub salt: String,
    pub nonce: Vec<u8>,
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

/// Supported elliptic curve types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurveType {
    K256,
    P256,
}

impl Default for CurveType {
    fn default() -> Self {
        CurveType::K256
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Wallet {
    pub address: Address,
    pub private_key: String,
    pub seed_phrase: String,
    pub curve_type: CurveType,
}

/// Set the selected wallet address in the configuration
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

pub fn save_wallet(
    address: &Address,
    private_key: &str,
    seed_phrase: &str,
    password: &str,
    curve_type: CurveType,
) -> Result<(), WalletError> {
    let wallet_data = Wallet {
        address: *address,
        private_key: private_key.to_string(),
        seed_phrase: seed_phrase.to_string(),
        curve_type,
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

pub fn generate_karix_address(
    word_count: usize,
    curve_type: CurveType,
) -> (String, String, String) {
    match curve_type {
        CurveType::K256 => generate_k256_address(word_count),
        CurveType::P256 => generate_p256_address(word_count),
    }
}

pub fn generate_k256_address(word_count: usize) -> (String, String, String) {
    // Generate secret key using k256
    let secret_key = K256SecretKey::random(&mut OsRng);
    // Convert to signing key first
    let signing_key = K256SigningKey::from(secret_key);
    // Then get verifying key
    let verifying_key = K256VerifyingKey::from(&signing_key);
    // Finally get public key
    let public_key = K256PublicKey::from(verifying_key);

    // Get encoded public key and format similarly to the previous implementation
    let encoded_point = public_key.to_encoded_point(false);
    let mut hex_encoded = hex::encode(&encoded_point.as_bytes()[1..]);
    hex_encoded.truncate(64); // Keep consistent with the existing approach

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
    let seed_phrase = mnemonic.to_string();

    // Return private key as hex string
    let private_key_bytes = signing_key.to_bytes();
    let private_key = hex::encode(private_key_bytes);

    (private_key, karix_public_address, seed_phrase)
}

pub fn generate_p256_address(word_count: usize) -> (String, String, String) {
    // Generate a random P-256 private key
    let signing_key = SigningKey::random(&mut OsRng);
    let secret_key = signing_key.to_bytes();

    // Get the corresponding public key
    let verifying_key = VerifyingKey::from(&signing_key);
    let public_key = verifying_key.to_encoded_point(false);

    // Format the public key, skipping the 0x04 prefix byte
    let mut hex_encoded = hex::encode(&public_key.as_bytes()[1..]);
    hex_encoded.truncate(64); // Keep consistent with secp256k1 method

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
    let seed_phrase = mnemonic.to_string();

    (hex::encode(secret_key), karix_public_address, seed_phrase)
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
        // Removed unnecessary parentheses
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
    curve_type: CurveType,
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    match curve_type {
        CurveType::K256 => import_from_seed_phrase_k256(phrase),
        CurveType::P256 => import_from_seed_phrase_p256(phrase),
    }
}

pub fn import_from_seed_phrase_k256(
    phrase: &str,
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    // Validate and create mnemonic
    let mnemonic = Mnemonic::parse_in(Language::English, phrase)?;

    // Generate seed from mnemonic
    let seed = mnemonic.to_seed("");

    // Create private key from seed using k256
    let bytes = &seed[0..32];
    let secret_key = K256SecretKey::from_slice(bytes)?;
    // Convert to signing key first
    let signing_key = K256SigningKey::from(secret_key);
    // Then get verifying key
    let verifying_key = K256VerifyingKey::from(&signing_key);
    // Finally get public key
    let public_key = K256PublicKey::from(verifying_key);

    // Get encoded public key
    let encoded_point = public_key.to_encoded_point(false);

    // Generate addresses
    let private_key = hex::encode(signing_key.to_bytes());
    let mut hex_encoded = hex::encode(&encoded_point.as_bytes()[1..]);
    hex_encoded.truncate(64);
    let public_address = format!("0x{}", hex_encoded);

    Ok((private_key, hex_encoded, public_address))
}

pub fn import_from_seed_phrase_p256(
    phrase: &str,
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    // Validate and create mnemonic
    let mnemonic = Mnemonic::parse_in(Language::English, phrase)?;

    // Generate seed from mnemonic
    let seed = mnemonic.to_seed("");

    // Create private key from seed
    let bytes = &seed[0..32];
    // Replace from_be_bytes with from_bytes
    let secret_key = P256SecretKey::from_bytes(bytes.into())?;
    let signing_key = SigningKey::from(secret_key);
    let verifying_key = VerifyingKey::from(&signing_key);

    // Generate public key and address
    let public_key = verifying_key.to_encoded_point(false);
    let private_key = hex::encode(&signing_key.to_bytes());

    // Format the public address
    let mut hex_encoded = hex::encode(&public_key.as_bytes()[1..]);
    hex_encoded.truncate(64);
    let public_address = format!("0x{}", hex_encoded);

    Ok((private_key, hex_encoded, public_address))
}

// import_from_private_key
pub fn import_from_private_key(
    private_key: &str,
    curve_type: CurveType,
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    match curve_type {
        CurveType::K256 => import_from_private_key_k256(private_key),
        CurveType::P256 => import_from_private_key_p256(private_key),
    }
}

pub fn import_from_private_key_k256(
    private_key: &str,
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    // Convert hex private key to bytes
    let private_key_bytes = hex::decode(private_key)?;

    // Create k256 private key and get public key
    let secret_key = K256SecretKey::from_slice(&private_key_bytes)?;
    // Convert to signing key first
    let signing_key = K256SigningKey::from(secret_key);
    // Then get verifying key
    let verifying_key = K256VerifyingKey::from(&signing_key);
    // Finally get public key
    let public_key = K256PublicKey::from(verifying_key);

    // Get encoded public key
    let encoded_point = public_key.to_encoded_point(false);

    // Generate addresses
    let mut hex_encoded = hex::encode(&encoded_point.as_bytes()[1..]);
    hex_encoded.truncate(64);
    let public_address = format!("0x{}", hex_encoded);

    Ok((private_key.to_string(), hex_encoded, public_address))
}

pub fn import_from_private_key_p256(
    private_key: &str,
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    // Convert hex private key to bytes
    let private_key_bytes = hex::decode(private_key)?;

    // Use from_slice instead of from_bytes
    let secret_key = P256SecretKey::from_slice(&private_key_bytes)?;
    let signing_key = SigningKey::from(secret_key);
    let verifying_key = VerifyingKey::from(&signing_key);

    // Generate public key and format address
    let public_key = verifying_key.to_encoded_point(false);
    let mut hex_encoded = hex::encode(&public_key.as_bytes()[1..]);
    hex_encoded.truncate(64);
    let public_address = format!("0x{}", hex_encoded);

    Ok((private_key.to_string(), hex_encoded, public_address))
}

/// Read currently selected wallet from config
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

/// Sign a message using the wallet's private key
pub fn sign_message(
    wallet: &Wallet,
    message: &[u8],
    password: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Load the wallet to get the private key
    let loaded_wallet = load_wallet(&wallet.address.to_string(), password)?;

    // Sign based on curve type
    match loaded_wallet.curve_type {
        CurveType::K256 => sign_message_k256(&loaded_wallet.private_key, message),
        CurveType::P256 => sign_message_p256(&loaded_wallet.private_key, message),
    }
}

/// Sign a message using K256 (secp256k1) private key
pub fn sign_message_k256(
    private_key_hex: &str,
    message: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Hash the message with SHA3
    let mut hasher = Sha3_256::default();
    hasher.update(message);
    let message_hash = hasher.finalize();

    // Convert hex private key to bytes
    let private_key_bytes = hex::decode(private_key_hex)?;

    // Create signing key from private key
    let secret_key = K256SecretKey::from_slice(&private_key_bytes)?;
    let mut signing_key = K256SigningKey::from(secret_key);

    // Sign the hashed message
    // The signature is generated deterministically according to RFC 6979
    let signature: K256Signature = signing_key.sign(&message_hash);

    // Convert to bytes (DER format is common for ECDSA signatures)
    Ok(signature.to_der().to_vec())
}

/// Sign a message using P256 (secp256r1) private key
pub fn sign_message_p256(
    private_key_hex: &str,
    message: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Hash the message with SHA3
    let mut hasher = Sha3_256::default();
    hasher.update(message);
    let message_hash = hasher.finalize();

    // Convert hex private key to bytes
    let private_key_bytes = hex::decode(private_key_hex)?;

    // Create signing key from private key
    let secret_key = P256SecretKey::from_slice(&private_key_bytes)?;
    let mut signing_key = SigningKey::from(secret_key);

    // Sign the hashed message
    // The signature is generated deterministically according to RFC 6979
    let signature: P256Signature = signing_key.sign(&message_hash);

    // Convert to bytes (DER format is common for ECDSA signatures)
    Ok(signature.to_der().to_vec())
}

/// Verify a signature against a message using a wallet address
pub fn verify_signature(
    address: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, Box<dyn std::error::Error>> {
    // Since addresses are in format 0x{hex}, we remove the prefix
    let address_hex = address.trim_start_matches("0x");

    // Try verification with both curves and return true if either succeeds
    let k256_result = verify_signature_k256(address_hex, message, signature);
    let p256_result = verify_signature_p256(address_hex, message, signature);
    
    // Log detailed info about verification attempts when in debug mode
    if cfg!(debug_assertions) {
        match &k256_result {
            Ok(true) => log::debug!("K256 verification succeeded"),
            Ok(false) => log::debug!("K256 verification failed: signature invalid"),
            Err(e) => log::debug!("K256 verification error: {}", e),
        }
        
        match &p256_result {
            Ok(true) => log::debug!("P256 verification succeeded"),
            Ok(false) => log::debug!("P256 verification failed: signature invalid"),
            Err(e) => log::debug!("P256 verification error: {}", e),
        }
    }
    
    // If either verification succeeds, return true
    match (k256_result, p256_result) {
        (Ok(true), _) | (_, Ok(true)) => Ok(true),
        
        // If both could construct a key but verification failed, the signature is invalid
        (Ok(false), Ok(false)) => Ok(false),
        
        // If one curve succeeded in making a key but verification failed
        (Ok(false), Err(e_p256)) => {
            // In this case, we were able to construct a K256 key, which suggests
            // the address is likely K256, so return false for invalid signature
            if cfg!(debug_assertions) {
                log::debug!("Assuming K256 address based on key reconstruction success. P256 error: {}", e_p256);
            }
            Ok(false)
        }
        
        (Err(e_k256), Ok(false)) => {
            // In this case, we were able to construct a P256 key, which suggests
            // the address is likely P256, so return false for invalid signature
            if cfg!(debug_assertions) {
                log::debug!("Assuming P256 address based on key reconstruction success. K256 error: {}", e_k256);
            }
            Ok(false)
        }
        
        // If both errored, combine the errors for better diagnostics
        (Err(e_k256), Err(e_p256)) => {
            Err(format!("Verification failed for both curves. K256: {}. P256: {}", e_k256, e_p256).into())
        }
    }
}

/// Verify a signature using K256 (secp256k1)
pub fn verify_signature_k256(
    address_hex: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, Box<dyn std::error::Error>> {
    // Parse the signature first to fail fast if the signature format is invalid
    let signature = match K256Signature::from_der(signature) {
        Ok(sig) => sig,
        Err(e) => return Err(format!("Invalid K256 signature format: {}", e).into())
    };
    
    // Hash the message with SHA3
    let mut hasher = Sha3_256::default();
    hasher.update(message);
    let message_hash = hasher.finalize();
    
    // Decode the address hex
    let decoded_hex = match hex::decode(address_hex) {
        Ok(hex) => hex,
        Err(e) => return Err(format!("Invalid hex in address: {}", e).into())
    };
    
    if decoded_hex.len() != 64 && decoded_hex.len() != 32 {
        return Err(format!("Invalid address length for K256: {}", decoded_hex.len()).into());
    }
    
    // Track if we were able to construct any valid key
    let mut had_valid_key = false;
    
    // Try with uncompressed format if we have full coordinates
    if decoded_hex.len() == 64 {
        let mut public_key_bytes = Vec::with_capacity(65);
        public_key_bytes.push(0x04); // Uncompressed point format prefix
        public_key_bytes.extend_from_slice(&decoded_hex);
        
        match K256VerifyingKey::from_sec1_bytes(&public_key_bytes) {
            Ok(verifying_key) => {
                had_valid_key = true;
                if verifying_key.verify(&message_hash, &signature).is_ok() {
                    return Ok(true);
                }
            },
            Err(_) => {}
        }
    }
    
    // If direct public key reconstruction fails or we only have X coordinate, try compressed formats
    let mut public_key_bytes = vec![0x02]; // Try 'even' Y coordinate
    public_key_bytes.extend_from_slice(&decoded_hex[0..32]);
    
    match K256VerifyingKey::from_sec1_bytes(&public_key_bytes) {
        Ok(verifying_key) => {
            had_valid_key = true;
            if verifying_key.verify(&message_hash, &signature).is_ok() {
                return Ok(true);
            }
        },
        Err(_) => {}
    }
    
    public_key_bytes[0] = 0x03; // Try 'odd' Y coordinate
    match K256VerifyingKey::from_sec1_bytes(&public_key_bytes) {
        Ok(verifying_key) => {
            had_valid_key = true;
            if verifying_key.verify(&message_hash, &signature).is_ok() {
                return Ok(true);
            }
        },
        Err(_) => {}
    }
    
    // If we were able to create at least one valid key but verification failed,
    // return Ok(false) since this is a valid signature verification failure
    if had_valid_key {
        return Ok(false);
    }
    
    // If we couldn't create any valid keys, return an error
    Err("Unable to reconstruct K256 public key from address".into())
}

/// Verify a signature using P256 (secp256r1)
pub fn verify_signature_p256(
    address_hex: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, Box<dyn std::error::Error>> {
    // Parse the signature first to fail fast if the signature format is invalid
    let signature = match P256Signature::from_der(signature) {
        Ok(sig) => sig,
        Err(e) => return Err(format!("Invalid P256 signature format: {}", e).into())
    };
    
    // Hash the message with SHA3
    let mut hasher = Sha3_256::default();
    hasher.update(message);
    let message_hash = hasher.finalize();
    
    // Decode the address hex
    let decoded_hex = match hex::decode(address_hex) {
        Ok(hex) => hex,
        Err(e) => return Err(format!("Invalid hex in address: {}", e).into())
    };
    
    if decoded_hex.len() != 64 && decoded_hex.len() != 32 {
        return Err(format!("Invalid address length for P256: {}", decoded_hex.len()).into());
    }
    
    // Track if we were able to construct any valid key
    let mut had_valid_key = false;
    
    // Try with uncompressed format if we have full coordinates
    if decoded_hex.len() == 64 {
        let mut public_key_bytes = Vec::with_capacity(65);
        public_key_bytes.push(0x04); // Uncompressed point format prefix
        public_key_bytes.extend_from_slice(&decoded_hex);
        
        match VerifyingKey::from_sec1_bytes(&public_key_bytes) {
            Ok(verifying_key) => {
                had_valid_key = true;
                if verifying_key.verify(&message_hash, &signature).is_ok() {
                    return Ok(true);
                }
            },
            Err(_) => {}
        }
    }
    
    // If direct public key reconstruction fails or we only have X coordinate, try compressed formats
    let mut public_key_bytes = vec![0x02]; // Try 'even' Y coordinate
    public_key_bytes.extend_from_slice(&decoded_hex[0..32]);
    
    match VerifyingKey::from_sec1_bytes(&public_key_bytes) {
        Ok(verifying_key) => {
            had_valid_key = true;
            if verifying_key.verify(&message_hash, &signature).is_ok() {
                return Ok(true);
            }
        },
        Err(_) => {}
    }
    
    public_key_bytes[0] = 0x03; // Try 'odd' Y coordinate
    match VerifyingKey::from_sec1_bytes(&public_key_bytes) {
        Ok(verifying_key) => {
            had_valid_key = true;
            if verifying_key.verify(&message_hash, &signature).is_ok() {
                return Ok(true);
            }
        },
        Err(_) => {}
    }
    
    // If we were able to create at least one valid key but verification failed,
    // return Ok(false) since this is a valid signature verification failure
    if had_valid_key {
        return Ok(false);
    }
    
    // If we couldn't create any valid keys, return an error
    Err("Unable to reconstruct P256 public key from address".into())
}

/// Returns the likely curve type for an address based on point validation
/// This is a heuristic and may not always be accurate
pub fn detect_curve_type(address: &str) -> Option<CurveType> {
    let address_hex = address.trim_start_matches("0x");
    let decoded_hex = match hex::decode(address_hex) {
        Ok(hex) => hex,
        Err(_) => return None,
    };
    
    if decoded_hex.len() != 64 && decoded_hex.len() != 32 {
        return None;
    }
    
    // Try to construct a K256 key first
    let k256_key_valid = if decoded_hex.len() == 64 {
        let mut public_key_bytes = Vec::with_capacity(65);
        public_key_bytes.push(0x04); // Uncompressed point format prefix
        public_key_bytes.extend_from_slice(&decoded_hex);
        K256VerifyingKey::from_sec1_bytes(&public_key_bytes).is_ok()
    } else {
        // Try compressed format with even Y
        let mut compressed_bytes = vec![0x02];
        compressed_bytes.extend_from_slice(&decoded_hex[0..32]);
        K256VerifyingKey::from_sec1_bytes(&compressed_bytes).is_ok() || {
            // Try with odd Y
            compressed_bytes[0] = 0x03;
            K256VerifyingKey::from_sec1_bytes(&compressed_bytes).is_ok()
        }
    };
    
    // Try to construct a P256 key
    let p256_key_valid = if decoded_hex.len() == 64 {
        let mut public_key_bytes = Vec::with_capacity(65);
        public_key_bytes.push(0x04); // Uncompressed point format prefix
        public_key_bytes.extend_from_slice(&decoded_hex);
        VerifyingKey::from_sec1_bytes(&public_key_bytes).is_ok()
    } else {
        // Try compressed format with even Y
        let mut compressed_bytes = vec![0x02];
        compressed_bytes.extend_from_slice(&decoded_hex[0..32]);
        VerifyingKey::from_sec1_bytes(&compressed_bytes).is_ok() || {
            // Try with odd Y
            compressed_bytes[0] = 0x03;
            VerifyingKey::from_sec1_bytes(&compressed_bytes).is_ok()
        }
    };
    
    match (k256_key_valid, p256_key_valid) {
        (true, false) => Some(CurveType::K256),
        (false, true) => Some(CurveType::P256),
        (true, true) => {
            // Both curves work! This is rare but possible.
            // Default to K256 as it's more common in blockchain contexts
            Some(CurveType::K256)
        },
        (false, false) => None,
    }
}

/// Convenience functions to add to the Wallet implementation
impl Wallet {
    /// Sign a message using this wallet
    pub fn sign(
        &self,
        message: &[u8],
        password: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        sign_message(self, message, password)
    }

    /// Get wallet's curve type
    pub fn get_curve_type(&self) -> CurveType {
        self.curve_type
    }
}

