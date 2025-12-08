//! Secure cryptographic primitives for the Kanari blockchain platform
//!
//! This crate provides cryptographic operations including:
//! - Key generation and management
//! - Digital signatures
//! - Encryption and decryption
//! - Wallet operations
//! - Hardware Security Module (HSM) support
//! - Key rotation and lifecycle management
//! - Security audit logging
//! - Backup and restore functionality

pub mod audit;
pub mod backup;
pub mod compression;
pub mod encryption;
pub mod hd_wallet;
pub mod hsm;
pub mod key_rotation;
pub mod keys;
pub mod keystore;
pub mod signatures;
pub mod wallet;

// Re-export signature functionality
pub use signatures::{
    SignatureError, secure_clear, sign_message, verify_signature, verify_signature_with_curve,
};

// Re-export encryption functionality - now using actual functions from the module
pub use encryption::{
    EncryptedData, EncryptionError, decrypt_data, decrypt_string, encrypt_data, encrypt_string,
    secure_erase,
};

// Re-export wallet functionality
pub use wallet::{
    Wallet, WalletError, check_mnemonic_exists, check_wallet_exists, clear_session_keys,
    get_mnemonic_addresses, get_selected_wallet, list_wallet_files, load_mnemonic,
    load_session_key, load_wallet, remove_mnemonic, remove_session_key, save_mnemonic,
    save_session_key, save_wallet, set_selected_wallet,
};

// Re-export keystore functionality
pub use keystore::{Keystore, get_keystore_path, keystore_exists};

// Re-export compression functionality
pub use compression::{compress_data, decompress_data};

// Timestamp utilities
use std::time::{SystemTime, UNIX_EPOCH};

/// Get current Unix timestamp in seconds
/// Returns 0 if system time is before UNIX_EPOCH (should never happen in practice)
#[must_use]
pub fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// Re-export HSM functionality
pub use hsm::{HsmConfig, HsmError, HsmInterface, HsmProvider, SoftwareHsm, create_hsm};

// Re-export key rotation functionality
pub use key_rotation::{
    KeyMetadata, KeyRotationError, KeyRotationManager, KeyRotationPolicy, RotationStatistics,
};

// Re-export audit functionality
pub use audit::{
    AuditEntry, AuditError, AuditLogger, EventSeverity, SecurityEvent, create_default_logger,
    get_default_audit_log_path,
};

// Re-export backup functionality
pub use backup::{BackupError, BackupInfo, BackupManager, BackupMetadata, EncryptedBackup};

/// Hash algorithm options (including quantum-resistant)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HashAlgorithm {
    /// SHA3-256 algorithm (default, quantum-resistant)
    #[default]
    Sha3_256,
    /// SHA3-512 algorithm (higher security, quantum-resistant)
    Sha3_512,
    /// Blake3 algorithm (faster, equally secure)
    Blake3,
    /// SHAKE256 (extendable output, quantum-resistant)
    Shake256,
}

/// Cryptographic hash using SHA3-256 (default)
#[must_use]
pub fn hash_data(data: &[u8]) -> Vec<u8> {
    hash_data_with_algorithm(data, HashAlgorithm::Sha3_256)
}

/// Cryptographic hash using SHA3-512 (quantum-resistant, 512-bit)
#[must_use]
pub fn hash_data_sha3_512(data: &[u8]) -> Vec<u8> {
    hash_data_with_algorithm(data, HashAlgorithm::Sha3_512)
}

/// Cryptographic hash using Blake3 (faster alternative)
#[must_use]
pub fn hash_data_blake3(data: &[u8]) -> Vec<u8> {
    hash_data_with_algorithm(data, HashAlgorithm::Blake3)
}

/// Cryptographic hash using SHAKE256 with 256-bit output (quantum-resistant)
#[must_use]
pub fn hash_data_shake256(data: &[u8]) -> Vec<u8> {
    hash_data_with_algorithm(data, HashAlgorithm::Shake256)
}

/// Cryptographic hash using SHAKE256 with custom output length
#[must_use]
pub fn hash_data_shake256_custom(data: &[u8], output_len: usize) -> Vec<u8> {
    use sha3::{
        Shake256,
        digest::{ExtendableOutput, Update, XofReader},
    };
    let mut hasher = Shake256::default();
    hasher.update(data);
    let mut reader = hasher.finalize_xof();
    let mut output = vec![0u8; output_len];
    reader.read(&mut output);
    output
}

/// Cryptographic hash using the specified algorithm
#[must_use]
pub fn hash_data_with_algorithm(data: &[u8], algorithm: HashAlgorithm) -> Vec<u8> {
    match algorithm {
        HashAlgorithm::Sha3_256 => {
            use sha3::{Digest, Sha3_256};
            let mut hasher = Sha3_256::new();
            hasher.update(data);
            hasher.finalize().to_vec()
        }
        HashAlgorithm::Sha3_512 => {
            use sha3::{Digest, Sha3_512};
            let mut hasher = Sha3_512::new();
            hasher.update(data);
            hasher.finalize().to_vec()
        }
        HashAlgorithm::Blake3 => {
            let mut hasher = blake3::Hasher::new();
            hasher.update(data);
            hasher.finalize().as_bytes().to_vec()
        }
        HashAlgorithm::Shake256 => {
            use sha3::{
                Shake256,
                digest::{ExtendableOutput, Update, XofReader},
            };
            let mut hasher = Shake256::default();
            hasher.update(data);
            let mut reader = hasher.finalize_xof();
            let mut output = vec![0u8; 32]; // 256-bit default
            reader.read(&mut output);
            output
        }
    }
}

// Add constant for recommended password length
pub const MIN_RECOMMENDED_PASSWORD_LENGTH: usize = 16; // Increased for quantum era

/// Security level used by this library
pub const SECURITY_LEVEL: &str = "Maximum - Post-Quantum Ready with Hybrid Cryptography";

/// Version information for the crypto library
#[must_use]
pub const fn version() -> &'static str {
    "2.0.0-pqc"
}

/// Returns security information about the library
#[must_use]
pub const fn security_info() -> &'static str {
    "🔒 Kanari Crypto v2.0 - Post-Quantum Ready
    
    Classical Algorithms:
    - AES-256-GCM encryption
    - Ed25519, K256, P256 signatures
    - Argon2id password hashing
    - SHA3-256/512, Blake3, SHAKE256 hashing
    
    Post-Quantum Algorithms (NIST Standard):
    - Dilithium2/3/5 signatures (ML-DSA)
    - SPHINCS+ hash-based signatures
    - Kyber768/1024 key encapsulation (ML-KEM)
    
    Hybrid Schemes:
    - Ed25519+Dilithium3 signatures
    - K256+Dilithium3 signatures
    - AES+Kyber encryption
    
    Security: Resistant to quantum computer attacks (Shor's and Grover's algorithms)
    Always use post-quantum or hybrid schemes for long-term security!"
}

/// Checks if a password meets minimum security requirements
#[must_use]
pub fn is_password_strong(password: &str) -> bool {
    if password.len() < MIN_RECOMMENDED_PASSWORD_LENGTH {
        return false;
    }

    // Use a single pass to check all conditions (more efficient and resistant to timing attacks)
    let mut has_uppercase = false;
    let mut has_lowercase = false;
    let mut has_digit = false;
    let mut has_special = false;

    for c in password.chars() {
        if c.is_uppercase() {
            has_uppercase = true;
        } else if c.is_lowercase() {
            has_lowercase = true;
        } else if c.is_ascii_digit() {
            has_digit = true;
        } else if !c.is_alphanumeric() {
            has_special = true;
        }
    }

    has_uppercase && has_lowercase && has_digit && has_special
}
