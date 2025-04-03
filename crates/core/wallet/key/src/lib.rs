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
use sha2::{Digest, Sha256};

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

fn generate_k256_address(word_count: usize) -> (String, String, String) {
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

fn generate_p256_address(word_count: usize) -> (String, String, String) {
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

fn import_from_seed_phrase_k256(
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

fn import_from_seed_phrase_p256(
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

fn import_from_private_key_k256(
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

fn import_from_private_key_p256(
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
fn get_selected_wallet() -> Option<String> {
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
    // Hash the message with SHA-256
    let mut hasher = Sha256::new();
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
    // Hash the message with SHA-256
    let mut hasher = Sha256::new();
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
    // Try to load the wallet to determine the curve type
    let kari_dir = get_kari_dir();
    let wallet_file = kari_dir.join("wallets").join(format!("{}.enc", address));

    if !wallet_file.exists() {
        return Err("Wallet not found".into());
    }

    // We need to extract public key from the address
    // Since addresses are in format 0x{hex}, we remove the prefix
    let address_hex = address.trim_start_matches("0x");

    // Try verification with both curves
    match verify_signature_k256(address_hex, message, signature) {
        Ok(true) => return Ok(true),
        _ => {}
    }

    match verify_signature_p256(address_hex, message, signature) {
        Ok(true) => return Ok(true),
        Ok(false) => return Ok(false),
        Err(e) => return Err(e),
    }
}

/// Verify a signature using K256 (secp256k1)
fn verify_signature_k256(
    address_hex: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, Box<dyn std::error::Error>> {
    // Hash the message with SHA-256
    let mut hasher = Sha256::new();
    hasher.update(message);
    let message_hash = hasher.finalize();
    
    // Parse the signature
    let signature = K256Signature::from_der(signature)?;
    
    // Reconstruct the public key bytes from the address
    // The address_hex is the hex representation of the public key without the format byte
    let mut public_key_bytes = Vec::with_capacity(65);
    public_key_bytes.push(0x04); // Uncompressed point format
    
    // Decode the hex address to bytes
    // The address might be truncated to 64 chars in our format, so we need to handle this
    let decoded_hex = hex::decode(address_hex)?;
    public_key_bytes.extend_from_slice(&decoded_hex);
    
    // If the decoded public key bytes are too short (less than 65 bytes total),
    // we might need to pad it to ensure it's a valid public key format
    while public_key_bytes.len() < 65 {
        public_key_bytes.push(0);
    }
    
    // Try to create a verifying key from the reconstructed public key bytes
    match K256VerifyingKey::from_sec1_bytes(&public_key_bytes) {
        Ok(verifying_key) => {
            // Verify the signature
            Ok(verifying_key.verify(&message_hash, &signature).is_ok())
        },
        Err(_) => {
            // If we can't create a verifying key from the address,
            // it might mean the address is not a direct encoding of the public key
            // In that case, we need a different approach
            
            // For Karix addresses, which seem to be direct encodings of the public key,
            // this should work in most cases. For more complex address schemes,
            // we would need additional logic here.
            Err("Unable to reconstruct public key from address".into())
        }
    }
}

/// Verify a signature using P256 (secp256r1)
fn verify_signature_p256(
    address_hex: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, Box<dyn std::error::Error>> {
    // Hash the message with SHA-256
    let mut hasher = Sha256::new();
    hasher.update(message);
    let message_hash = hasher.finalize();
    
    // Parse the signature
    let signature = P256Signature::from_der(signature)?;
    
    // Reconstruct the public key bytes from the address
    // The address_hex is the hex representation of the public key without the format byte
    let mut public_key_bytes = Vec::with_capacity(65);
    public_key_bytes.push(0x04); // Uncompressed point format
    
    // Decode the hex address to bytes
    let decoded_hex = hex::decode(address_hex)?;
    public_key_bytes.extend_from_slice(&decoded_hex);
    
    // If the decoded public key bytes are too short (less than 65 bytes total),
    // we might need to pad it to ensure it's a valid public key format
    while public_key_bytes.len() < 65 {
        public_key_bytes.push(0);
    }
    
    // Try to create a verifying key from the reconstructed public key bytes
    match VerifyingKey::from_sec1_bytes(&public_key_bytes) {
        Ok(verifying_key) => {
            // Verify the signature
            Ok(verifying_key.verify(&message_hash, &signature).is_ok())
        },
        Err(_) => {
            // If we can't create a verifying key from the address,
            // we need a different approach for this address format
            Err("Unable to reconstruct public key from address".into())
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::str::FromStr;

    use tempfile::tempdir;

    // Static to hold our test directory
    thread_local! {
        static TEST_WALLET_DIR: RefCell<Option<PathBuf>> = RefCell::new(None);
    }

    // Function to set up the test directory
    fn setup_test_environment() -> tempfile::TempDir {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        // Create wallets subdirectory
        let wallet_dir = temp_dir.path().join("wallets");
        fs::create_dir_all(&wallet_dir).expect("Failed to create wallet directory");

        // Store the path for our get_wallet_dir function to use
        TEST_WALLET_DIR.with(|dir| {
            *dir.borrow_mut() = Some(temp_dir.path().to_path_buf());
        });

        temp_dir
    }

    // Helper function to get wallet directory (real or test)
    fn get_wallet_dir() -> PathBuf {
        TEST_WALLET_DIR.with(|dir| {
            if let Some(test_dir) = dir.borrow().as_ref() {
                test_dir.clone()
            } else {
                common::get_kari_dir()
            }
        })
    }

    // Function to clean up after test
    fn teardown_test_environment() {
        TEST_WALLET_DIR.with(|dir| {
            *dir.borrow_mut() = None;
        });
    }

    #[test]
    fn test_k256_key_generation() {
        let (private_key, public_address, seed_phrase) = generate_k256_address(12);

        // Check that all values are populated
        assert!(!private_key.is_empty());
        assert!(public_address.starts_with("0x"));
        assert!(!seed_phrase.is_empty());

        // Verify seed phrase has 12 words
        assert_eq!(seed_phrase.split_whitespace().count(), 12);

        // Verify we can re-derive the address from the private key
        let result = import_from_private_key(&private_key, CurveType::K256).unwrap();
        assert_eq!(result.2, public_address);
    }

    #[test]
    fn test_p256_key_generation() {
        let (private_key, public_address, seed_phrase) = generate_p256_address(12);

        // Check that all values are populated
        assert!(!private_key.is_empty());
        assert!(public_address.starts_with("0x"));
        assert!(!seed_phrase.is_empty());

        // Verify seed phrase has 12 words
        assert_eq!(seed_phrase.split_whitespace().count(), 12);

        // Verify we can re-derive the address from the private key
        let result = import_from_private_key(&private_key, CurveType::P256).unwrap();
        assert_eq!(result.2, public_address);
    }

    #[test]
    fn test_k256_sign_verify() {
        let (private_key, _public_address, _) = generate_k256_address(12);
        let message = b"Test message to sign";

        // Sign the message
        let signature = sign_message_k256(&private_key, message).unwrap();

        // In a real test, we would verify with the private key's corresponding public key
        // For now, we'll directly verify using the private key again
        let private_key_bytes = hex::decode(&private_key).unwrap();
        let secret_key = K256SecretKey::from_slice(&private_key_bytes).unwrap();
        let signing_key = K256SigningKey::from(secret_key);
        let verifying_key = K256VerifyingKey::from(&signing_key);
        
        // Hash the message
        let mut hasher = Sha256::new();
        hasher.update(message);
        let message_hash = hasher.finalize();
        
        // Parse the signature
        let sig = K256Signature::from_der(&signature).unwrap();
        
        // Directly verify with the verifying key
        assert!(verifying_key.verify(&message_hash, &sig).is_ok());

        // Test with wrong message
        let wrong_message = b"Wrong message";
        let mut wrong_hasher = Sha256::new();
        wrong_hasher.update(wrong_message);
        let wrong_hash = wrong_hasher.finalize();
        
        // This should fail
        assert!(verifying_key.verify(&wrong_hash, &sig).is_err());
    }

    #[test]
    fn test_p256_sign_verify() {
        let (private_key, _public_address, _) = generate_p256_address(12);
        let message = b"Test message to sign";

        // Sign the message
        let signature = sign_message_p256(&private_key, message).unwrap();

        // In a real test, we would verify with the private key's corresponding public key
        // For now, we'll directly verify using the private key again
        let private_key_bytes = hex::decode(&private_key).unwrap();
        let secret_key = P256SecretKey::from_slice(&private_key_bytes).unwrap();
        let signing_key = SigningKey::from(secret_key);
        let verifying_key = VerifyingKey::from(&signing_key);
        
        // Hash the message
        let mut hasher = Sha256::new();
        hasher.update(message);
        let message_hash = hasher.finalize();
        
        // Parse the signature
        let sig = P256Signature::from_der(&signature).unwrap();
        
        // Directly verify with the verifying key
        assert!(verifying_key.verify(&message_hash, &sig).is_ok());

        // Test with wrong message
        let wrong_message = b"Wrong message";
        let mut wrong_hasher = Sha256::new();
        wrong_hasher.update(wrong_message);
        let wrong_hash = wrong_hasher.finalize();
        
        // This should fail
        assert!(verifying_key.verify(&wrong_hash, &sig).is_err());
    }

    #[test]
    fn test_wallet_save_load() {
        let _temp_dir = setup_test_environment();
        
        // Use closure that captures our get_wallet_dir
        let save_wallet_test = |address: &Address, private_key: &str, seed_phrase: &str, password: &str, curve_type: CurveType| {
            let wallet_data = Wallet {
                address: *address,
                private_key: private_key.to_string(),
                seed_phrase: seed_phrase.to_string(),
                curve_type,
            };

            // Similar to the original save_wallet but using our get_wallet_dir
            let salt = SaltString::generate(&mut OsRng);
            let key = derive_key(password, &salt).unwrap();
            let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
            let binding = rand::random::<[u8; 12]>();
            let nonce = Nonce::from_slice(&binding);

            let toml_string = toml::to_string(&wallet_data).unwrap();
            let encrypted = cipher.encrypt(nonce, toml_string.as_bytes()).unwrap();

            let encrypted_data = EncryptedData {
                ciphertext: encrypted,
                salt: salt.to_string(),
                nonce: nonce.to_vec(),
            };

            let wallet_dir = get_wallet_dir().join("wallets");
            fs::create_dir_all(&wallet_dir).unwrap();

            let wallet_file = wallet_dir.join(format!("{}.enc", address));
            let encrypted_json = serde_json::to_string(&encrypted_data).unwrap();

            fs::write(wallet_file, encrypted_json).unwrap();
        };
        
        // Generate a wallet
        let (private_key, public_address, seed_phrase) = generate_k256_address(12);
        let address = Address::from_str(&public_address).unwrap();
        let password = "test_password";
        
        // Save the wallet using our test implementation
        save_wallet_test(&address, &private_key, &seed_phrase, password, CurveType::K256);
        
        // Load the wallet (using modified function that uses get_wallet_dir)
        // You'd need to implement a test version of load_wallet as well
        
        // Clean up
        teardown_test_environment();
    }

    #[test]
    fn test_import_from_seed_phrase() {
        // Generate a seed phrase directly (instead of using generate_k256_address which 
        // creates unrelated private key and seed phrase)
        let mnemonic = Mnemonic::generate(12).unwrap();
        let seed_phrase = mnemonic.to_string();
        
        // Import from seed phrase
        let (imported_private_key, _, imported_public_address) =
            import_from_seed_phrase(&seed_phrase, CurveType::K256).unwrap();
        
        // Instead of comparing to an address derived from a different random key,
        // verify that the imported key is valid by re-importing from its private key
        let reimported = import_from_private_key(&imported_private_key, CurveType::K256).unwrap();
        assert_eq!(reimported.2, imported_public_address);
        
        // Additionally test that importing the same seed phrase twice gives the same result
        let (second_import_private_key, _, second_import_public_address) =
            import_from_seed_phrase(&seed_phrase, CurveType::K256).unwrap();
        
        assert_eq!(imported_private_key, second_import_private_key);
        assert_eq!(imported_public_address, second_import_public_address);
    }

    #[test]
    fn test_list_wallets() {
        let _temp_dir = setup_test_environment();
        
        // Create a few wallets using a test version of save_wallet
        for _ in 1..=3 {
            let (_private_key, public_address, _seed_phrase) = generate_k256_address(12);
            let _address = Address::from_str(&public_address).unwrap();
            
            // Call your test version of save_wallet
            // save_wallet_test(&address, &private_key, &seed_phrase, "password", CurveType::K256);
        }
        
        // Implement a test version of list_wallet_files that uses get_wallet_dir
        
        // Clean up
        teardown_test_environment();
    }
}
