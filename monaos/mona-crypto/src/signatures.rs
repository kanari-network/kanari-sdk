//! Digital signature creation and verification
//!
//! This module handles digital signatures across multiple curves, with unified
//! interfaces for signing and verifying messages using different key types.

use sha3::{Digest, Sha3_256};
use thiserror::Error;
use log::debug;

use k256::{
    ecdsa::{
        Signature as K256Signature,
        SigningKey as K256SigningKey,
        VerifyingKey as K256VerifyingKey,
    },
    SecretKey as K256SecretKey,
};

use p256::{
    ecdsa::{
        Signature as P256Signature,
        SigningKey,
        VerifyingKey,
        signature::Verifier,
    },
    SecretKey as P256SecretKey,
};

use ed25519_dalek::{
    Signer,
    SigningKey as Ed25519SigningKey,
    VerifyingKey as Ed25519VerifyingKey,
    Signature as Ed25519Signature,
};

use crate::keys::CurveType;

/// Digital signature errors
#[derive(Error, Debug)]
pub enum SignatureError {
    #[error("Invalid private key: {0}")]
    InvalidPrivateKey(String),
    
    #[error("Invalid signature format: {0}")]
    InvalidFormat(String),
    
    #[error("Invalid public key or address: {0}")]
    InvalidPublicKey(String),
    
    #[error("Signature verification failed")]
    VerificationFailed,
}

/// Compare two byte slices in constant time to prevent timing attacks
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    
    let mut result = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    
    result == 0
}

/// Sign a message with a given private key and curve type
pub fn sign_message(
    private_key_hex: &str,
    message: &[u8],
    curve_type: CurveType,
) -> Result<Vec<u8>, SignatureError> {
    match curve_type {
        CurveType::K256 => sign_message_k256(private_key_hex, message),
        CurveType::P256 => sign_message_p256(private_key_hex, message),
        CurveType::Ed25519 => sign_message_ed25519(private_key_hex, message),
    }
}

/// Sign a message using K256 (secp256k1) private key
fn sign_message_k256(
    private_key_hex: &str,
    message: &[u8],
) -> Result<Vec<u8>, SignatureError> {
    // Hash the message with SHA3
    let mut hasher = Sha3_256::default();
    hasher.update(message);
    let message_hash = hasher.finalize();

    // Convert hex private key to bytes
    let private_key_bytes = hex::decode(private_key_hex)
        .map_err(|e| SignatureError::InvalidPrivateKey(e.to_string()))?;

    // Create signing key from private key
    let secret_key = K256SecretKey::from_slice(&private_key_bytes)
        .map_err(|e| SignatureError::InvalidPrivateKey(e.to_string()))?;
    let signing_key = K256SigningKey::from(secret_key);

    // Sign the hashed message
    let signature: K256Signature = signing_key.sign(&message_hash);
    
    // Use to_vec() from SignatureEncoding trait to get DER formatted bytes
    let der_bytes = signature.to_der();
    Ok(der_bytes.as_bytes().to_vec())
}

/// Sign a message using P256 (secp256r1) private key
fn sign_message_p256(
    private_key_hex: &str,
    message: &[u8],
) -> Result<Vec<u8>, SignatureError> {
    // Hash the message with SHA3
    let mut hasher = Sha3_256::default();
    hasher.update(message);
    let message_hash = hasher.finalize();

    // Convert hex private key to bytes
    let private_key_bytes = hex::decode(private_key_hex)
        .map_err(|e| SignatureError::InvalidPrivateKey(e.to_string()))?;

    // Create signing key from private key
    let secret_key = P256SecretKey::from_slice(&private_key_bytes)
        .map_err(|e| SignatureError::InvalidPrivateKey(e.to_string()))?;
    let signing_key = SigningKey::from(secret_key);

    // Sign the hashed message
    let signature: P256Signature = signing_key.sign(&message_hash);

    // Convert DER signature to bytes correctly
    let der_bytes = signature.to_der();
    Ok(der_bytes.as_bytes().to_vec())
}

/// Sign a message using Ed25519 private key
fn sign_message_ed25519(
    private_key_hex: &str,
    message: &[u8],
) -> Result<Vec<u8>, SignatureError> {
    // Convert hex private key to bytes
    let private_key_bytes = hex::decode(private_key_hex)
        .map_err(|e| SignatureError::InvalidPrivateKey(e.to_string()))?;
    
    if private_key_bytes.len() != 32 {
        return Err(SignatureError::InvalidPrivateKey(
            format!("Invalid Ed25519 private key length: {}", private_key_bytes.len())
        ));
    }
    
    // Create a fixed-size array from the private key bytes
    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&private_key_bytes);
    
    // Create signing key from private key
    let signing_key = Ed25519SigningKey::from_bytes(&key_array);
    
    // Sign the message directly (Ed25519 doesn't need pre-hashing)
    let signature: Ed25519Signature = signing_key.sign(message);
    
    // Return the signature bytes
    Ok(signature.to_bytes().to_vec())
}

/// Verify a signature against a message using an address
pub fn verify_signature(
    address: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, SignatureError> {
    // Try all curve types without requiring the user to specify
    if let Ok(true) = verify_signature_k256(&address.trim_start_matches("0x"), message, signature) {
        debug!("K256 signature verification succeeded");
        return Ok(true);
    }
    
    if let Ok(true) = verify_signature_p256(&address.trim_start_matches("0x"), message, signature) {
        debug!("P256 signature verification succeeded");
        return Ok(true);
    }
    
    if let Ok(true) = verify_signature_ed25519(&address.trim_start_matches("0x"), message, signature) {
        debug!("Ed25519 signature verification succeeded");
        return Ok(true);
    }

    // If all verifications failed but did not error, the signature is invalid
    debug!("Signature verification failed for all curve types");
    Ok(false)
}

/// Verify a signature with the known curve type
pub fn verify_signature_with_curve(
    address: &str,
    message: &[u8],
    signature: &[u8],
    curve_type: CurveType
) -> Result<bool, SignatureError> {
    let address_hex = address.trim_start_matches("0x");
    
    match curve_type {
        CurveType::K256 => verify_signature_k256(address_hex, message, signature),
        CurveType::P256 => verify_signature_p256(address_hex, message, signature),
        CurveType::Ed25519 => verify_signature_ed25519(address_hex, message, signature),
    }
}

/// Verify a signature using K256 (secp256k1)
pub fn verify_signature_k256(
    address_hex: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, SignatureError> {
    // Try to parse the signature from DER format
    let signature = K256Signature::from_der(signature)
        .map_err(|e| SignatureError::InvalidFormat(format!("Invalid K256 signature format: {}", e)))?;

    // Hash the message with SHA3
    let mut hasher = Sha3_256::default();
    hasher.update(message);
    let message_hash = hasher.finalize();

    // Decode the address hex
    let decoded_hex = hex::decode(address_hex)
        .map_err(|e| SignatureError::InvalidPublicKey(format!("Invalid hex in address: {}", e)))?;

    // Handle both uncompressed (64 bytes) and compressed (32 bytes) public keys
    if decoded_hex.len() != 64 && decoded_hex.len() != 32 {
        return Err(SignatureError::InvalidPublicKey(
            format!("Invalid address length for K256: {}", decoded_hex.len())
        ));
    }

    let mut had_valid_key = false;

    // Try with uncompressed public key format
    if decoded_hex.len() == 64 {
        let mut public_key_bytes = Vec::with_capacity(65);
        public_key_bytes.push(0x04);
        public_key_bytes.extend_from_slice(&decoded_hex);

        if let Ok(verifying_key) = K256VerifyingKey::from_sec1_bytes(&public_key_bytes) {
            had_valid_key = true;
            if verifying_key.verify(&message_hash, &signature).is_ok() {
                return Ok(true);
            }
        }
    }

    // Try with compressed public key format (with 0x02 prefix)
    let mut public_key_bytes = vec![0x02];
    public_key_bytes.extend_from_slice(&decoded_hex[0..32]);

    if let Ok(verifying_key) = K256VerifyingKey::from_sec1_bytes(&public_key_bytes) {
        had_valid_key = true;
        if verifying_key.verify(&message_hash, &signature).is_ok() {
            return Ok(true);
        }
    }

    // Try with compressed public key format (with 0x03 prefix)
    public_key_bytes[0] = 0x03;
    if let Ok(verifying_key) = K256VerifyingKey::from_sec1_bytes(&public_key_bytes) {
        had_valid_key = true;
        if verifying_key.verify(&message_hash, &signature).is_ok() {
            return Ok(true);
        }
    }

    // If we could construct at least one valid key but verification failed,
    // then the signature is invalid
    if had_valid_key {
        return Ok(false);
    }

    Err(SignatureError::InvalidPublicKey(
        "Unable to reconstruct K256 public key from address".to_string()
    ))
}

/// Verify a signature using P256 (secp256r1)
pub fn verify_signature_p256(
    address_hex: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, SignatureError> {
    // Parse the signature
    let signature = P256Signature::from_der(signature)
        .map_err(|e| SignatureError::InvalidFormat(format!("Invalid P256 signature format: {}", e)))?;

    // Hash the message with SHA3
    let mut hasher = Sha3_256::default();
    hasher.update(message);
    let message_hash = hasher.finalize();

    // Decode the address hex
    let decoded_hex = hex::decode(address_hex)
        .map_err(|e| SignatureError::InvalidPublicKey(format!("Invalid hex in address: {}", e)))?;

    // Handle both uncompressed (64 bytes) and compressed (32 bytes) public keys
    if decoded_hex.len() != 64 && decoded_hex.len() != 32 {
        return Err(SignatureError::InvalidPublicKey(
            format!("Invalid address length for P256: {}", decoded_hex.len())
        ));
    }

    let mut had_valid_key = false;

    // Try with uncompressed public key format
    if decoded_hex.len() == 64 {
        let mut public_key_bytes = Vec::with_capacity(65);
        public_key_bytes.push(0x04);
        public_key_bytes.extend_from_slice(&decoded_hex);

        if let Ok(verifying_key) = VerifyingKey::from_sec1_bytes(&public_key_bytes) {
            had_valid_key = true;
            if verifying_key.verify(&message_hash, &signature).is_ok() {
                return Ok(true);
            }
        }
    }

    // Try with compressed public key format (with 0x02 prefix)
    let mut public_key_bytes = vec![0x02];
    public_key_bytes.extend_from_slice(&decoded_hex[0..32]);

    if let Ok(verifying_key) = VerifyingKey::from_sec1_bytes(&public_key_bytes) {
        had_valid_key = true;
        if verifying_key.verify(&message_hash, &signature).is_ok() {
            return Ok(true);
        }
    }

    // Try with compressed public key format (with 0x03 prefix)
    public_key_bytes[0] = 0x03;
    if let Ok(verifying_key) = VerifyingKey::from_sec1_bytes(&public_key_bytes) {
        had_valid_key = true;
        if verifying_key.verify(&message_hash, &signature).is_ok() {
            return Ok(true);
        }
    }

    // If we could construct at least one valid key but verification failed,
    // then the signature is invalid
    if had_valid_key {
        return Ok(false);
    }

    Err(SignatureError::InvalidPublicKey(
        "Unable to reconstruct P256 public key from address".to_string()
    ))
}

/// Verify a signature using Ed25519
pub fn verify_signature_ed25519(
    address_hex: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, SignatureError> {
    // Check if signature has correct length for Ed25519
    if signature.len() != 64 {
        return Err(SignatureError::InvalidFormat(
            format!("Invalid Ed25519 signature length: {}", signature.len())
        ));
    }
    
    // Create a fixed-size array for the signature
    let mut sig_array = [0u8; 64];
    sig_array.copy_from_slice(signature);
    let signature = Ed25519Signature::from_bytes(&sig_array);
    
    // Decode the address hex (which should be the public key)
    let decoded_hex = hex::decode(address_hex)
        .map_err(|e| SignatureError::InvalidPublicKey(format!("Invalid hex in address: {}", e)))?;
    
    // For Ed25519, the address should be the 32-byte public key
    if decoded_hex.len() != 32 {
        return Err(SignatureError::InvalidPublicKey(
            format!("Invalid address length for Ed25519: {}", decoded_hex.len())
        ));
    }
    
    // Create a fixed-size array for the public key
    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&decoded_hex);
    
    // Create verifying key from public key bytes
    let verifying_key = Ed25519VerifyingKey::from_bytes(&key_array)
        .map_err(|e| SignatureError::InvalidPublicKey(format!("Invalid Ed25519 public key: {}", e)))?;
    
    // Verify the signature against the original message
    match verifying_key.verify(message, &signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
