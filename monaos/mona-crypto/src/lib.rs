//! Secure cryptographic primitives for the Mona blockchain platform
//! 
//! This crate provides cryptographic operations including:
//! - Key generation and management
//! - Digital signatures
//! - Encryption and decryption
//! - Wallet operations


pub mod signatures;
pub mod encryption;
pub mod wallet;

// Re-export signature functionality
pub use signatures::{
    sign_message,
    verify_signature,
    verify_signature_with_curve,
    SignatureError,
    secure_clear,
};

// Re-export encryption functionality
pub use encryption::{
    encrypt_data,
    decrypt_data,
    encrypt_string,
    decrypt_string,
    secure_erase,
    EncryptionError,
};

// Re-export wallet functionality
pub use wallet::{
    Wallet,
    WalletError,
    save_wallet,
    load_wallet,
    list_wallet_files,
    get_selected_wallet,
    set_selected_wallet,
    check_wallet_exists,
};

/// Cryptographic hash using SHA3-256
pub fn hash_data(data: &[u8]) -> Vec<u8> {
    use sha3::{Digest, Sha3_256};
    
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Security level used by this library
pub const SECURITY_LEVEL: &str = "High - AES-256-GCM with Argon2id key derivation";

/// Version information for the crypto library
pub fn version() -> &'static str {
    "1.0.0"
}

/// Returns security information about the library
pub fn security_info() -> &'static str {
    "This library uses Argon2id for password hashing, AES-256-GCM for encryption, 
    and constant-time comparisons for secure signature verification.
    Always keep your private keys secure and use strong, unique passwords."
}
