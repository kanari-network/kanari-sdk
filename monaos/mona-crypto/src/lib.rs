//! Secure cryptographic primitives for the Mona blockchain platform
//! 
//! This crate provides cryptographic operations including:
//! - Key generation and management
//! - Digital signatures
//! - Encryption and decryption
//! - Wallet operations

pub mod keys;
pub mod signatures;
pub mod encryption;
pub mod wallet;

// Re-export key functionality
pub use keys::{
    CurveType, 
    generate_keypair,
    derive_address_from_pubkey,
};

// Re-export signature functionality
pub use signatures::{
    sign_message,
    verify_signature,
    SignatureError,
};

// Re-export encryption functionality
pub use encryption::{
    encrypt_data,
    decrypt_data,
    EncryptionError,
};

// Re-export wallet functionality
pub use wallet::{
    Wallet,
    WalletError,
    save_wallet,
    load_wallet,
    list_wallets,
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

/// Version information for the crypto library
pub fn version() -> &'static str {
    "1.0.0"
}
