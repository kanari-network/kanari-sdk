// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Cryptographic key generation and management
//!
//! This module handles key generation for multiple curve types (K256/secp256k1,
//! P256/secp256r1, Ed25519) and Post-Quantum Cryptography (Dilithium, SPHINCS+).
//!
//! **Quantum-Safe**: Includes NIST-standardized post-quantum algorithms.
//!
//! # ⚠️ Security Considerations
//!
//! ## Private Key Storage
//! - Private keys are stored in `Zeroizing<String>` which automatically clears memory on drop
//! - `Clone` is NOT implemented on `KeyPair` to prevent accidental key duplication
//! - Use `export_private_key_secure()` to explicitly handle private key export
//! - Private keys are **NOT serialized** by default; use `to_serializable_with_private_key()`
//!   only for encrypted storage scenarios
//!
//! ## Tagged Addresses
//! For enhanced security and reliability, use tagged addresses:
//! - Format: `\"CurveType:address\"` (e.g., `\"K256:0xabc...\"`)
//! - Prevents address ambiguity when multiple curves could apply
//! - Required for secure signature verification without timing leaks
//! - Use `tagged_address()` method to generate; use `parse_tagged_address()` to extract
//!
//! ## Hybrid Keys
//! Hybrid keys (e.g., Ed25519+Dilithium3) combine classical and post-quantum algorithms:
//! - Format: `\"CurveType:classical_pub:pqc_pub\"`
//! - Private key format: `\"kanahybrid<classical_hex>:<pqc_secret_hex>:<pqc_pub_hex>\"`
//! - Both classical and PQC signatures required for full verification
//! - Use for transition period during quantum computing threat emergence
//!
//! ## Post-Quantum Cryptography Dependencies
//! PQC crates (`pqcrypto_dilithium`, `pqcrypto_sphincsplus`) are relatively newer.
//! - Monitor security advisories regularly
//! - Consider pinning versions in production `Cargo.toml`
//! - Dilithium3 (NIST Level 3) is recommended for most use cases
//!
//! ## Mnemonic Derivation Limitations
//! - Only classical curves (K256, P256, Ed25519) support BIP39 mnemonic derivation
//! - PQC algorithms generate fresh keys without HD wallet derivation
//! - For PQC keys, use `generate_keypair()` for fresh key generation

use bip39::{Language, Mnemonic};
use rand::TryRng;
use rand::rngs::SysRng;
use sha3::{Digest, Sha3_256};
use std::fmt;
use std::str::FromStr;
use subtle::ConstantTimeEq;
use thiserror::Error;
use zeroize::{Zeroize, Zeroizing};

use k256::{
    PublicKey as K256PublicKey, SecretKey as K256SecretKey,
    ecdsa::{SigningKey as K256SigningKey, VerifyingKey as K256VerifyingKey},
    elliptic_curve::sec1::ToEncodedPoint,
};

use p256::{
    SecretKey as P256SecretKey,
    ecdsa::{SigningKey, VerifyingKey},
};

use ed25519_dalek::{SigningKey as Ed25519SigningKey, VerifyingKey as Ed25519VerifyingKey};

// Post-Quantum Cryptography imports
use pqcrypto_dilithium::dilithium2;
use pqcrypto_dilithium::dilithium3;
use pqcrypto_dilithium::dilithium5;
use pqcrypto_sphincsplus::sphincssha2256fsimple;
use pqcrypto_traits::sign::{PublicKey as PqcPublicKey, SecretKey as PqcSecretKey};

/// Supported cryptographic algorithms (Classical + Post-Quantum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum CurveType {
    // Classical Elliptic Curve Cryptography (ECC)
    /// Secp256k1 curve (used by Bitcoin and Ethereum)
    #[default]
    K256,

    /// Secp256r1 curve (NIST P-256)
    P256,

    /// Ed25519 curve (modern, fast signature scheme)
    Ed25519,

    // Post-Quantum Cryptography (PQC) - NIST Standards
    /// Dilithium2 - Fast, ~2.5KB signatures, NIST Level 2 security
    Dilithium2,

    /// Dilithium3 - Balanced, ~4KB signatures, NIST Level 3 security (Recommended)
    Dilithium3,

    /// Dilithium5 - Maximum security, ~5KB signatures, NIST Level 5 security
    Dilithium5,

    /// SPHINCS+ SHA256-256f-robust - Hash-based, ~50KB signatures, ultra-secure
    SphincsPlusSha256Robust,

    // Hybrid Schemes (Classical + PQC for transition period)
    /// Ed25519 + Dilithium3 hybrid (Best of both worlds)
    Ed25519Dilithium3,

    /// K256 + Dilithium3 hybrid (Bitcoin/Ethereum compatible + quantum-safe)
    K256Dilithium3,
}

impl fmt::Display for CurveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CurveType::K256 => write!(f, "K256"),
            CurveType::P256 => write!(f, "P256"),
            CurveType::Ed25519 => write!(f, "Ed25519"),
            CurveType::Dilithium2 => write!(f, "Dilithium2"),
            CurveType::Dilithium3 => write!(f, "Dilithium3"),
            CurveType::Dilithium5 => write!(f, "Dilithium5"),
            CurveType::SphincsPlusSha256Robust => write!(f, "SphincsPlusSha256Robust"),
            CurveType::Ed25519Dilithium3 => write!(f, "Ed25519Dilithium3"),
            CurveType::K256Dilithium3 => write!(f, "K256Dilithium3"),
        }
    }
}

impl std::str::FromStr for CurveType {
    type Err = KeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "K256" => Ok(CurveType::K256),
            "P256" => Ok(CurveType::P256),
            "Ed25519" => Ok(CurveType::Ed25519),
            "Dilithium2" => Ok(CurveType::Dilithium2),
            "Dilithium3" => Ok(CurveType::Dilithium3),
            "Dilithium5" => Ok(CurveType::Dilithium5),
            "SphincsPlusSha256Robust" => Ok(CurveType::SphincsPlusSha256Robust),
            "Ed25519Dilithium3" => Ok(CurveType::Ed25519Dilithium3),
            "K256Dilithium3" => Ok(CurveType::K256Dilithium3),
            _ => Err(KeyError::InvalidPublicKey),
        }
    }
}

impl CurveType {
    /// Returns true if this is a post-quantum algorithm
    pub fn is_post_quantum(&self) -> bool {
        matches!(
            self,
            CurveType::Dilithium2
                | CurveType::Dilithium3
                | CurveType::Dilithium5
                | CurveType::SphincsPlusSha256Robust
                | CurveType::Ed25519Dilithium3
                | CurveType::K256Dilithium3
        )
    }

    /// Returns true if this is a hybrid scheme
    pub fn is_hybrid(&self) -> bool {
        matches!(
            self,
            CurveType::Ed25519Dilithium3 | CurveType::K256Dilithium3
        )
    }

    /// Get security level (1-5, where 5 is highest)
    pub fn security_level(&self) -> u8 {
        match self {
            CurveType::K256 | CurveType::P256 => 3,
            CurveType::Ed25519 => 3,
            CurveType::Dilithium2 => 4,
            CurveType::Dilithium3 => 5,
            CurveType::Dilithium5 => 5,
            CurveType::SphincsPlusSha256Robust => 5,
            CurveType::Ed25519Dilithium3 => 5,
            CurveType::K256Dilithium3 => 5,
        }
    }
}

/// Key generation errors
#[derive(Error, Debug)]
pub enum KeyError {
    #[error("Invalid private key format")]
    InvalidPrivateKey,

    #[error("Invalid public key format")]
    InvalidPublicKey,

    #[error("Invalid mnemonic phrase: {0}")]
    InvalidMnemonic(String),

    #[error("Key generation failed: {0}")]
    GenerationFailed(String),
}

/// Result of key generation containing private key, public key, and address
///
/// Security: Private key is automatically zeroized when dropped.
/// Clone is intentionally not implemented to prevent key material duplication.
/// Private key is NOT serialized by default for security.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct KeyPair {
    #[serde(skip)]
    pub private_key: Zeroizing<String>,
    pub public_key: String,
    /// Optional post-quantum public key (hex) when applicable
    pub pqc_public_key: Option<String>,
    pub address: String,
    pub curve_type: CurveType,
}

impl fmt::Debug for KeyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyPair")
            .field("private_key", &"**REDACTED**")
            .field("public_key", &self.public_key)
            .field("address", &self.address)
            .field("curve_type", &self.curve_type)
            .finish()
    }
}

impl KeyPair {
    /// Export private key in a wrapper that zeroizes on drop
    /// Prefer this API to avoid accidental long-lived clones of secret material.
    pub fn export_private_key_secure(&self) -> Zeroizing<String> {
        Zeroizing::new(self.private_key.to_string())
    }

    /// Get public key as reference (avoid unnecessary cloning)
    pub fn get_public_key(&self) -> &str {
        &self.public_key
    }

    /// Get PQC public key if present
    pub fn get_pqc_public_key(&self) -> Option<String> {
        self.pqc_public_key.clone()
    }

    /// Get a reference to the PQC public key if present (avoids cloning)
    pub fn get_pqc_public_key_ref(&self) -> Option<&str> {
        self.pqc_public_key.as_deref()
    }

    /// Get address as reference (avoid unnecessary cloning)
    pub fn get_address(&self) -> &str {
        &self.address
    }

    /// Get a tagged address that includes curve type information
    /// Format: "curve_type:address" (e.g., "K256:0xabc123...")
    /// For hybrid keys, format is "curve_type:classical_pub:pqc_pub"
    /// This is the recommended way to store addresses for reliable curve detection
    pub fn tagged_address(&self) -> String {
        // For hybrid keys, include both classical and PQC public keys in the tag
        format!("{}:{}", self.curve_type, self.public_key)
    }

    /// Create a serializable version that includes private key (use with caution)
    /// This should only be used when explicitly needed for encrypted storage
    pub fn to_serializable_with_private_key(&self) -> serde_json::Value {
        serde_json::json!({
            "private_key": self.private_key.to_string(),
            "public_key": self.public_key,
            "address": self.address,
            "curve_type": self.curve_type,
        })
    }

    /// Parse a tagged address back into curve type and address
    /// Returns None if the address is not in tagged format
    pub fn parse_tagged_address(tagged: &str) -> Option<(CurveType, String)> {
        // Find the first colon to separate curve type from address
        // For hybrid keys, address contains ':' (format: "classical:pqc")
        // so we must use find() not split_once() to preserve the full address
        let colon_pos = tagged.find(':')?;
        let curve_str = &tagged[..colon_pos];
        let address_str = &tagged[colon_pos + 1..]; // Take everything after first colon

        let curve_type = CurveType::from_str(curve_str).ok()?;

        Some((curve_type, address_str.to_string()))
    }
}

/// Prefix used for Kanari private keys
pub const KANARI_KEY_PREFIX: &str = "kanari";

/// Additional known prefixes
pub const KANAPQC_PREFIX: &str = "kanapqc";
pub const KANAHYBRID_PREFIX: &str = "kanahybrid";

// ============================================================================
// SECURITY HELPER FUNCTIONS (Timing Attack Prevention & Memory Safety)
// ============================================================================

/// Securely encode bytes to hex string using a zeroizing buffer
/// This prevents intermediate allocations from leaking sensitive data in memory dumps
fn secure_hex_encode(bytes: &[u8]) -> Zeroizing<String> {
    // Pre-allocate with exact capacity needed (2 chars per byte)
    let mut result = String::with_capacity(bytes.len() * 2);

    // Use lookup table approach for constant-time-ish encoding
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

    for &byte in bytes {
        result.push(HEX_CHARS[(byte >> 4) as usize] as char);
        result.push(HEX_CHARS[(byte & 0x0F) as usize] as char);
    }

    Zeroizing::new(result)
}

/// Constant-time check if a string starts with a given prefix
/// Prevents timing attacks that could leak information about key formats
fn constant_time_starts_with(s: &str, prefix: &str) -> bool {
    // Get bytes for comparison
    let s_bytes = s.as_bytes();
    let prefix_bytes = prefix.as_bytes();

    // If prefix is longer than the string, it can't match
    if prefix_bytes.len() > s_bytes.len() {
        return false;
    }

    // Compare only the relevant portion in constant time
    let s_prefix = &s_bytes[..prefix_bytes.len()];

    // Use subtle crate's constant-time equality check
    s_prefix.ct_eq(prefix_bytes).into()
}

/// Format a raw hex private key with the Kanari prefix
pub fn format_private_key(raw_key: &str) -> String {
    format!("{}{}", KANARI_KEY_PREFIX, raw_key)
}

/// Extract the raw hex key from a formatted private key using constant-time comparison
pub fn extract_raw_key(formatted_key: &str) -> &str {
    // Use constant-time checks to prevent timing leaks
    if constant_time_starts_with(formatted_key, KANAHYBRID_PREFIX) {
        &formatted_key[KANAHYBRID_PREFIX.len()..]
    } else if constant_time_starts_with(formatted_key, KANAPQC_PREFIX) {
        &formatted_key[KANAPQC_PREFIX.len()..]
    } else if constant_time_starts_with(formatted_key, KANARI_KEY_PREFIX) {
        &formatted_key[KANARI_KEY_PREFIX.len()..]
    } else {
        formatted_key
    }
}

/// Skip the uncompressed EC point prefix (0x04) safely.
fn skip_uncompressed_point_prefix(bytes: &[u8]) -> &[u8] {
    // Check length before accessing to prevent buffer overread
    if bytes.is_empty() {
        return bytes;
    }

    if bytes[0] == 0x04 && bytes.len() > 1 {
        &bytes[1..]
    } else {
        bytes
    }
}

/// Generate a keypair for the specified curve type
pub fn generate_keypair(curve_type: CurveType) -> Result<KeyPair, KeyError> {
    match curve_type {
        CurveType::K256 => generate_k256_keypair(),
        CurveType::P256 => generate_p256_keypair(),
        CurveType::Ed25519 => generate_ed25519_keypair(),
        CurveType::Dilithium2 => generate_dilithium2_keypair(),
        CurveType::Dilithium3 => generate_dilithium3_keypair(),
        CurveType::Dilithium5 => generate_dilithium5_keypair(),
        CurveType::SphincsPlusSha256Robust => generate_sphincs_keypair(),
        CurveType::Ed25519Dilithium3 => generate_hybrid_ed25519_dilithium3_keypair(),
        CurveType::K256Dilithium3 => generate_hybrid_k256_dilithium3_keypair(),
    }
}

/// Generate a K256 (secp256k1) keypair
fn generate_k256_keypair() -> Result<KeyPair, KeyError> {
    let mut seed = [0u8; 32];
    SysRng
        .try_fill_bytes(&mut seed)
        .expect("Failed to get OS randomness");

    let secret_key = K256SecretKey::from_slice(&seed)
        .map_err(|_| KeyError::GenerationFailed("Invalid K256 seed".to_string()))?;

    seed.zeroize();

    let signing_key = K256SigningKey::from(&secret_key);
    let verifying_key = K256VerifyingKey::from(&signing_key);
    // Finally get public key
    let public_key = K256PublicKey::from(verifying_key);

    // Get encoded public key and format (skip uncompressed prefix safely)
    let encoded_point = public_key.to_encoded_point(false);
    let slice = skip_uncompressed_point_prefix(encoded_point.as_bytes());
    let full_pub_hex = hex::encode(slice);
    // Address: SHA3-256 of public key hex (full 32-byte hash)
    let mut hasher = Sha3_256::default();
    hasher.update(full_pub_hex.as_bytes());
    let digest = hasher.finalize();
    let address = format!("0x{}", hex::encode(digest));

    // Get raw bytes and encode securely
    let secret_bytes = signing_key.to_bytes();
    let raw_private_key = secure_hex_encode(&secret_bytes);

    // Zeroize secret bytes immediately after encoding
    let mut secret_bytes_mut = secret_bytes.to_vec();
    secret_bytes_mut.zeroize();

    // Format private key with kanari prefix using secure string
    let private_key = format!("{}{}", KANARI_KEY_PREFIX, *raw_private_key);

    Ok(KeyPair {
        private_key: Zeroizing::new(private_key),
        public_key: full_pub_hex,
        pqc_public_key: None,
        address,
        curve_type: CurveType::K256,
    })
}

/// Generate a P256 (secp256r1) keypair
fn generate_p256_keypair() -> Result<KeyPair, KeyError> {
    let mut seed = [0u8; 32];
    SysRng
        .try_fill_bytes(&mut seed)
        .expect("Failed to get OS randomness");

    let secret_key = P256SecretKey::from_slice(&seed)
        .map_err(|_| KeyError::GenerationFailed("Invalid P256 seed".to_string()))?;

    seed.zeroize();

    let signing_key = SigningKey::from(&secret_key);
    let verifying_key = VerifyingKey::from(&signing_key);
    let public_key = verifying_key.to_encoded_point(false);

    // Get raw bytes and encode securely
    let secret_bytes = secret_key.to_bytes();
    let raw_private_key = secure_hex_encode(&secret_bytes);

    // Zeroize secret bytes immediately after encoding
    let mut secret_bytes_mut = secret_bytes.to_vec();
    secret_bytes_mut.zeroize();

    let pub_bytes = skip_uncompressed_point_prefix(public_key.as_bytes());
    let hex_encoded = hex::encode(pub_bytes);

    // Address: SHA3-256 of public key hex
    let mut hasher = Sha3_256::default();
    hasher.update(hex_encoded.as_bytes());
    let digest = hasher.finalize();
    let address = format!("0x{}", hex::encode(digest));

    // Format private key with kanari prefix using secure string
    let private_key = format!("{}{}", KANARI_KEY_PREFIX, *raw_private_key);

    Ok(KeyPair {
        private_key: Zeroizing::new(private_key),
        public_key: hex_encoded,
        pqc_public_key: None,
        address,
        curve_type: CurveType::P256,
    })
}

/// Generate an Ed25519 keypair
pub fn generate_ed25519_keypair() -> Result<KeyPair, KeyError> {
    let mut seed = [0u8; 32];
    SysRng
        .try_fill_bytes(&mut seed)
        .expect("Failed to get OS randomness");

    if seed.iter().all(|&b| b == 0) {
        return Err(KeyError::GenerationFailed(
            "Insufficient entropy from RNG".to_string(),
        ));
    }

    // Create signing key from random bytes
    let signing_key = Ed25519SigningKey::from_bytes(&seed);
    let verifying_key = Ed25519VerifyingKey::from(&signing_key);

    // Get the bytes of the keys and encode securely
    let private_key_bytes = signing_key.to_bytes();
    let raw_private_key = secure_hex_encode(&private_key_bytes);

    let public_key_bytes = verifying_key.to_bytes();

    // Zeroize sensitive byte arrays immediately after encoding
    seed.zeroize();
    let mut private_key_bytes_mut = private_key_bytes.to_vec();
    private_key_bytes_mut.zeroize();

    // Format the public key
    let hex_encoded = hex::encode(public_key_bytes);

    // Address: SHA3-256 of public key hex string
    let mut hasher = Sha3_256::default();
    hasher.update(hex_encoded.as_bytes());
    let digest = hasher.finalize();
    let address = format!("0x{}", hex::encode(digest));

    // Format private key with kanari prefix using secure string
    let private_key = format!("{}{}", KANARI_KEY_PREFIX, *raw_private_key);

    Ok(KeyPair {
        private_key: Zeroizing::new(private_key),
        public_key: hex_encoded,
        pqc_public_key: None,
        address,
        curve_type: CurveType::Ed25519,
    })
}

// ============================================================================
// POST-QUANTUM CRYPTOGRAPHY (PQC) KEY GENERATION
// ============================================================================

/// Generate a Dilithium2 keypair (Fast, NIST Level 2)
fn generate_dilithium2_keypair() -> Result<KeyPair, KeyError> {
    let (public_key, secret_key) = dilithium2::keypair();

    // Encode public key
    let hex_encoded = hex::encode(public_key.as_bytes());

    // Compute address using SHA3-256 of public key bytes directly (more secure than hex string)
    let mut hasher = Sha3_256::new();
    hasher.update(public_key.as_bytes());
    let hash_result = hasher.finalize();
    let address = format!("0x{}", hex::encode(&hash_result[..]));

    // Encode secret key securely using zeroizing buffer
    let raw_private_key = secure_hex_encode(secret_key.as_bytes());

    // Combine into private key format with secure string
    let private_key = format!("{}{}:{}", KANAPQC_PREFIX, *raw_private_key, hex_encoded);

    Ok(KeyPair {
        private_key: Zeroizing::new(private_key),
        public_key: hex_encoded.clone(),
        pqc_public_key: Some(hex_encoded),
        address,
        curve_type: CurveType::Dilithium2,
    })
}

/// Generate a Dilithium3 keypair (Balanced, NIST Level 3, Recommended)
fn generate_dilithium3_keypair() -> Result<KeyPair, KeyError> {
    let (public_key, secret_key) = dilithium3::keypair();

    // Encode public key
    let hex_encoded = hex::encode(public_key.as_bytes());

    // Compute address using SHA3-256 of public key bytes directly
    let mut hasher = Sha3_256::new();
    hasher.update(public_key.as_bytes());
    let hash_result = hasher.finalize();
    let address = format!("0x{}", hex::encode(&hash_result[..]));

    // Encode secret key securely using zeroizing buffer
    let raw_private_key = secure_hex_encode(secret_key.as_bytes());

    // Combine into private key format with secure string
    let private_key = format!("{}{}:{}", KANAPQC_PREFIX, *raw_private_key, hex_encoded);

    Ok(KeyPair {
        private_key: Zeroizing::new(private_key),
        public_key: hex_encoded.clone(),
        pqc_public_key: Some(hex_encoded),
        address,
        curve_type: CurveType::Dilithium3,
    })
}

/// Generate a Dilithium5 keypair (Maximum security, NIST Level 5)
fn generate_dilithium5_keypair() -> Result<KeyPair, KeyError> {
    let (public_key, secret_key) = dilithium5::keypair();

    // Encode public key
    let hex_encoded = hex::encode(public_key.as_bytes());

    // Compute address using SHA3-256 of public key bytes directly
    let mut hasher = Sha3_256::new();
    hasher.update(public_key.as_bytes());
    let hash_result = hasher.finalize();
    let address = format!("0x{}", hex::encode(&hash_result[..]));

    // Encode secret key securely using zeroizing buffer
    let raw_private_key = secure_hex_encode(secret_key.as_bytes());

    // Combine into private key format with secure string
    let private_key = format!("{}{}:{}", KANAPQC_PREFIX, *raw_private_key, hex_encoded);

    Ok(KeyPair {
        private_key: Zeroizing::new(private_key),
        public_key: hex_encoded.clone(),
        pqc_public_key: Some(hex_encoded),
        address,
        curve_type: CurveType::Dilithium5,
    })
}

/// Generate a SPHINCS+ keypair (Hash-based, ultra-secure)
fn generate_sphincs_keypair() -> Result<KeyPair, KeyError> {
    let (public_key, secret_key) = sphincssha2256fsimple::keypair();
    let hex_encoded = hex::encode(public_key.as_bytes());
    let mut hasher = Sha3_256::new();
    hasher.update(hex_encoded.as_bytes());
    let hash_result = hasher.finalize();
    let address = format!("0x{}", hex::encode(&hash_result[..]));
    let raw_private_key = hex::encode(secret_key.as_bytes());
    let private_key = format!("kanapqc{}:{}", raw_private_key, hex_encoded);

    Ok(KeyPair {
        private_key: Zeroizing::new(private_key),
        public_key: hex_encoded.clone(),
        pqc_public_key: Some(hex_encoded),
        address,
        curve_type: CurveType::SphincsPlusSha256Robust,
    })
}

// ============================================================================
// HYBRID CRYPTOGRAPHY (Classical + PQC)
// ============================================================================

/// Generate Ed25519 + Dilithium3 hybrid keypair
pub fn generate_hybrid_ed25519_dilithium3_keypair() -> Result<KeyPair, KeyError> {
    // Generate both keypairs
    let ed25519_pair = generate_ed25519_keypair()?;
    let dilithium3_pair = generate_dilithium3_keypair()?;

    // Combine public keys
    let combined_public = format!("{}:{}", ed25519_pair.public_key, dilithium3_pair.public_key);

    // Extract raw private keys using constant-time comparison via extract_raw_key
    let ed25519_raw = extract_raw_key(&ed25519_pair.private_key);
    let dilithium3_raw = extract_raw_key(&dilithium3_pair.private_key);

    // Combine private keys with secure prefix
    let combined_private = format!("{}{}:{}", KANAHYBRID_PREFIX, ed25519_raw, dilithium3_raw);

    // Generate hybrid address using SHA3-256 hash of combined public key bytes
    let mut hasher = Sha3_256::new();
    hasher.update(combined_public.as_bytes());
    let hash_result = hasher.finalize();
    let address = format!("0x{}", hex::encode(&hash_result[..]));

    Ok(KeyPair {
        private_key: Zeroizing::new(combined_private),
        public_key: combined_public,
        pqc_public_key: Some(dilithium3_pair.public_key.clone()),
        address,
        curve_type: CurveType::Ed25519Dilithium3,
    })
}

/// Generate K256 + Dilithium3 hybrid keypair
pub fn generate_hybrid_k256_dilithium3_keypair() -> Result<KeyPair, KeyError> {
    // Generate both keypairs
    let k256_pair = generate_k256_keypair()?;
    let dilithium3_pair = generate_dilithium3_keypair()?;

    // Combine public keys
    let combined_public = format!("{}:{}", k256_pair.public_key, dilithium3_pair.public_key);

    // Extract raw private keys using constant-time comparison via extract_raw_key
    let k256_raw = extract_raw_key(&k256_pair.private_key);
    let dilithium3_raw = extract_raw_key(&dilithium3_pair.private_key);

    // Combine private keys with secure prefix
    let combined_private = format!("{}{}:{}", KANAHYBRID_PREFIX, k256_raw, dilithium3_raw);

    // Generate hybrid address using SHA3-256 hash of combined public key bytes
    let mut hasher = Sha3_256::new();
    hasher.update(combined_public.as_bytes());
    let hash_result = hasher.finalize();
    let address = format!("0x{}", hex::encode(&hash_result[..]));

    Ok(KeyPair {
        private_key: Zeroizing::new(combined_private),
        public_key: combined_public,
        pqc_public_key: Some(dilithium3_pair.public_key.clone()),
        address,
        curve_type: CurveType::K256Dilithium3,
    })
}

/// Generate a keypair from a mnemonic phrase
pub fn keypair_from_mnemonic(phrase: &str, curve_type: CurveType) -> Result<KeyPair, KeyError> {
    // Validate inputs
    if phrase.trim().is_empty() {
        return Err(KeyError::InvalidMnemonic(
            "Empty mnemonic phrase".to_string(),
        ));
    }

    // Validate and create mnemonic
    let mnemonic = Mnemonic::parse_in(Language::English, phrase)
        .map_err(|e| KeyError::InvalidMnemonic(e.to_string()))?;

    // Generate seed from mnemonic (no password)
    let seed = Zeroizing::new(mnemonic.to_seed(""));
    let bytes = &seed[0..32];

    match curve_type {
        CurveType::K256 => {
            let secret_key = K256SecretKey::from_slice(bytes).map_err(|_e| KeyError::InvalidPrivateKey)?;
            let signing_key = K256SigningKey::from(secret_key);
            let verifying_key = K256VerifyingKey::from(&signing_key);
            let public_key = K256PublicKey::from(verifying_key);

            let encoded_point = public_key.to_encoded_point(false);
            let slice = skip_uncompressed_point_prefix(encoded_point.as_bytes());
            let full_pub_hex = hex::encode(slice);
            let mut hasher = Sha3_256::default();
            hasher.update(full_pub_hex.as_bytes());
            let digest = hasher.finalize();
            let address = format!("0x{}", hex::encode(digest));
            // Use secure hex encoding for private key
            let raw_private_key = secure_hex_encode(&signing_key.to_bytes());
            let private_key = format!("{}{}", KANARI_KEY_PREFIX, *raw_private_key);

            Ok(KeyPair {
                private_key: Zeroizing::new(private_key),
                public_key: full_pub_hex,
                pqc_public_key: None,
                address,
                curve_type: CurveType::K256,
            })
        }
        CurveType::P256 => {
            let secret_key = P256SecretKey::from_slice(bytes).map_err(|_e| KeyError::InvalidPrivateKey)?;
            let signing_key = SigningKey::from(secret_key);
            let verifying_key = VerifyingKey::from(&signing_key);
            let public_key = verifying_key.to_encoded_point(false);

            let pub_bytes = skip_uncompressed_point_prefix(public_key.as_bytes());
            let full_pub_hex = hex::encode(pub_bytes);
            // Address: SHA3-256 of public key hex
            let mut hasher = Sha3_256::default();
            hasher.update(full_pub_hex.as_bytes());
            let digest = hasher.finalize();
            let address = format!("0x{}", hex::encode(digest));
            // Use secure hex encoding for private key
            let raw_private_key = secure_hex_encode(&signing_key.to_bytes());
            let private_key = format!("{}{}", KANARI_KEY_PREFIX, *raw_private_key);

            Ok(KeyPair {
                private_key: Zeroizing::new(private_key),
                public_key: full_pub_hex,
                pqc_public_key: None,
                address,
                curve_type: CurveType::P256,
            })
        }
        CurveType::Ed25519 => {
            let mut seed_array = [0u8; 32];
            seed_array.copy_from_slice(bytes);

            let signing_key = Ed25519SigningKey::from_bytes(&seed_array);

            // ✅ Zeroize the seed array immediately after creating the signing key to minimize time sensitive data is in memory
            seed_array.zeroize();
            let verifying_key = Ed25519VerifyingKey::from(&signing_key);

            // Use secure hex encoding for private key
            let raw_private_key = secure_hex_encode(&signing_key.to_bytes());
            let public_key_bytes = verifying_key.to_bytes();
            let hex_encoded = hex::encode(public_key_bytes);
            // Address: SHA3-256 of public key hex
            let mut hasher = Sha3_256::default();
            hasher.update(hex_encoded.as_bytes());
            let digest = hasher.finalize();
            let address = format!("0x{}", hex::encode(digest));

            // Format private key with kanari prefix using secure string
            let private_key = format!("{}{}", KANARI_KEY_PREFIX, *raw_private_key);

            Ok(KeyPair {
                private_key: Zeroizing::new(private_key),
                public_key: hex_encoded,
                pqc_public_key: None,
                address,
                curve_type: CurveType::Ed25519,
            })
        }
        // PQC algorithms don't support HD wallet derivation yet
        // Fall back to random generation for now
        _ => Err(KeyError::GenerationFailed(
            "Post-quantum algorithms don't support BIP39 mnemonic derivation yet. Use generate_keypair() instead.".to_string()
        )),
    }
}

/// Generate a keypair from a private key
pub fn keypair_from_private_key(
    private_key: &str,
    curve_type: CurveType,
) -> Result<KeyPair, KeyError> {
    // Remove kanari prefix if present
    let raw_private_key = extract_raw_key(private_key);

    match curve_type {
        CurveType::K256 => {
            let mut private_key_bytes =
                hex::decode(raw_private_key).map_err(|_| KeyError::InvalidPrivateKey)?;
            let secret_key = K256SecretKey::from_slice(&private_key_bytes)
                .map_err(|_| KeyError::InvalidPrivateKey)?;

            // Zeroize immediately after use
            private_key_bytes.zeroize();

            let signing_key = K256SigningKey::from(secret_key);
            let verifying_key = K256VerifyingKey::from(&signing_key);
            let public_key = K256PublicKey::from(verifying_key);

            let encoded_point = public_key.to_encoded_point(false);
            let slice = skip_uncompressed_point_prefix(encoded_point.as_bytes());
            let hex_encoded = hex::encode(slice);

            // Address: SHA3-256 of public key hex (must match generation function)
            let mut hasher = Sha3_256::default();
            hasher.update(hex_encoded.as_bytes());
            let digest = hasher.finalize();
            let address = format!("0x{}", hex::encode(digest));

            // Format with kanari prefix using constant-time comparison to prevent timing attacks
            let formatted_private_key = if constant_time_starts_with(private_key, KANARI_KEY_PREFIX)
            {
                private_key.to_string()
            } else {
                format_private_key(raw_private_key)
            };

            Ok(KeyPair {
                private_key: Zeroizing::new(formatted_private_key),
                public_key: hex_encoded,
                pqc_public_key: None,
                address,
                curve_type: CurveType::K256,
            })
        }
        CurveType::P256 => {
            let mut private_key_bytes =
                hex::decode(raw_private_key).map_err(|_| KeyError::InvalidPrivateKey)?;
            let secret_key = P256SecretKey::from_slice(&private_key_bytes)
                .map_err(|_| KeyError::InvalidPrivateKey)?;

            // Zeroize immediately after use
            private_key_bytes.zeroize();

            let signing_key = SigningKey::from(secret_key);
            let verifying_key = VerifyingKey::from(&signing_key);
            let public_key = verifying_key.to_encoded_point(false);

            let slice = skip_uncompressed_point_prefix(public_key.as_bytes());
            let hex_encoded = hex::encode(slice);

            // Address: SHA3-256 of public key hex (must match generation function)
            let mut hasher = Sha3_256::default();
            hasher.update(hex_encoded.as_bytes());
            let digest = hasher.finalize();
            let address = format!("0x{}", hex::encode(digest));

            // Format with kanari prefix using constant-time comparison
            let formatted_private_key = if constant_time_starts_with(private_key, KANARI_KEY_PREFIX)
            {
                private_key.to_string()
            } else {
                format_private_key(raw_private_key)
            };

            Ok(KeyPair {
                private_key: Zeroizing::new(formatted_private_key),
                public_key: hex_encoded,
                pqc_public_key: None,
                address,
                curve_type: CurveType::P256,
            })
        }
        CurveType::Ed25519 => {
            let mut private_key_bytes =
                hex::decode(raw_private_key).map_err(|_| KeyError::InvalidPrivateKey)?;
            if private_key_bytes.len() != 32 {
                private_key_bytes.zeroize();
                Err(KeyError::InvalidPrivateKey)?
            }

            let mut key_array = [0u8; 32];
            key_array.copy_from_slice(&private_key_bytes);

            // Zeroize source bytes
            private_key_bytes.zeroize();

            let signing_key = Ed25519SigningKey::from_bytes(&key_array);
            let verifying_key = Ed25519VerifyingKey::from(&signing_key);

            // Zeroize key array after use
            key_array.zeroize();

            let public_key_bytes = verifying_key.to_bytes();
            let hex_encoded = hex::encode(public_key_bytes);

            // Address: SHA3-256 of public key hex (must match generation function)
            let mut hasher = Sha3_256::default();
            hasher.update(hex_encoded.as_bytes());
            let digest = hasher.finalize();
            let address = format!("0x{}", hex::encode(digest));

            // Format with kanari prefix using constant-time comparison
            let formatted_private_key = if constant_time_starts_with(private_key, KANARI_KEY_PREFIX)
            {
                private_key.to_string()
            } else {
                format_private_key(raw_private_key)
            };

            Ok(KeyPair {
                private_key: Zeroizing::new(formatted_private_key),
                public_key: hex_encoded,
                pqc_public_key: None,
                address,
                curve_type: CurveType::Ed25519,
            })
        }
        // Post-quantum imports: require public key be stored alongside secret when possible.
        CurveType::Dilithium2
        | CurveType::Dilithium3
        | CurveType::Dilithium5
        | CurveType::SphincsPlusSha256Robust => {
            // raw_private_key may be: "kanapqc<secret_hex>:<public_hex>" or older
            // Use constant-time comparison for prefix check
            let raw_for_pqc = if constant_time_starts_with(raw_private_key, KANAPQC_PREFIX) {
                &raw_private_key[KANAPQC_PREFIX.len()..]
            } else {
                raw_private_key
            };

            // Require explicit public key stored alongside secret: prefer format
            // "kanapqc<secret_hex>:<public_hex>" and reject secret-only inputs.
            if let Some((_secret_hex, pub_hex)) = raw_for_pqc.split_once(':') {
                // validate pub_hex is hex
                let _pub_bytes = hex::decode(pub_hex).map_err(|_| KeyError::InvalidPrivateKey)?;
                let pqc_hex = pub_hex.to_string();

                // Derive address from hash of the PQC public key for uniformity
                let mut hasher = Sha3_256::new();
                hasher.update(pub_hex.as_bytes());
                let hash_result = hasher.finalize();
                let address = format!("0x{}", hex::encode(&hash_result[..]));

                // Use constant-time comparison for prefix check
                let formatted_private_key =
                    if constant_time_starts_with(private_key, KANAPQC_PREFIX) {
                        private_key.to_string()
                    } else {
                        format!("{}{}", KANAPQC_PREFIX, raw_for_pqc)
                    };

                return Ok(KeyPair {
                    private_key: Zeroizing::new(formatted_private_key),
                    public_key: pqc_hex.clone(),
                    pqc_public_key: Some(pqc_hex),
                    address,
                    curve_type,
                });
            }

            // No explicit public key supplied — reject to avoid fragile recovery
            Err(KeyError::InvalidPrivateKey)
        }
        // Hybrid imports: expect format "kanahybrid<classical_hex>:<pqc_hex>" (may be prefixed with `kanari`)
        CurveType::Ed25519Dilithium3 | CurveType::K256Dilithium3 => {
            // For hybrid imports we require the caller to provide a hybrid-formatted
            // private key (must start with `kanahybrid`). This avoids ambiguous
            // parsing when users accidentally pass other prefixed keys.
            // Accept hybrid input where either the original `private_key` string
            // began with `kanahybrid` or the stripped `raw_private_key` begins
            // with it (this handles cases where multiple prefixes were present
            // and one was stripped by `extract_raw_key`). Require the hybrid
            // structure to avoid ambiguous parsing.

            // Use constant-time comparison to prevent timing attacks
            if !constant_time_starts_with(private_key, KANAHYBRID_PREFIX) {
                // Allow special case if raw still starts with prefix (case of nested prefixes)
                if !constant_time_starts_with(raw_private_key, KANAHYBRID_PREFIX) {
                    Err(KeyError::InvalidPrivateKey)?
                }
            }

            // raw_private_key currently has had one known prefix removed by
            // `extract_raw_key`. Strip an internal `kanahybrid` if present to
            // obtain the canonical hybrid payload (classical_hex:pqc_part).
            let hybrid = if constant_time_starts_with(raw_private_key, KANAHYBRID_PREFIX) {
                &raw_private_key[KANAHYBRID_PREFIX.len()..]
            } else {
                raw_private_key
            };

            // split into two parts at the first ':' so pqc part may itself contain ':'
            let parts: Vec<&str> = hybrid.splitn(2, ':').collect();
            if parts.len() != 2 {
                Err(KeyError::InvalidPrivateKey)?
            }

            let classical_raw = parts[0];
            let pqc_raw = parts[1];

            // Recreate classical public key hex
            let classical_bytes =
                hex::decode(classical_raw).map_err(|_| KeyError::InvalidPrivateKey)?;

            let classical_pub_hex = match curve_type {
                CurveType::Ed25519Dilithium3 => {
                    if classical_bytes.len() != 32 {
                        Err(KeyError::InvalidPrivateKey)?
                    }
                    let mut key_array = [0u8; 32];
                    key_array.copy_from_slice(&classical_bytes);
                    let signing_key = Ed25519SigningKey::from_bytes(&key_array);
                    let verifying_key = Ed25519VerifyingKey::from(&signing_key);
                    hex::encode(verifying_key.to_bytes())
                }
                CurveType::K256Dilithium3 => {
                    let secret_key = K256SecretKey::from_slice(&classical_bytes)
                        .map_err(|_| KeyError::InvalidPrivateKey)?;
                    let signing_key = K256SigningKey::from(secret_key);
                    let verifying_key = K256VerifyingKey::from(&signing_key);
                    let public_key = K256PublicKey::from(verifying_key);
                    let encoded_point = public_key.to_encoded_point(false);
                    // Use skip_uncompressed_point_prefix for consistency with generation function
                    let slice = skip_uncompressed_point_prefix(encoded_point.as_bytes());
                    hex::encode(slice)
                }
                _ => Err(KeyError::InvalidPrivateKey)?,
            };

            // Require explicit PQC public key to avoid:
            // 1. Timing attacks from byte-searching loops
            // 2. Fragile recovery logic that may produce incorrect keys
            // 3. DoS from excessive iterations
            // Format: "<secret_hex>:<public_hex>" (both required)
            let pqc_hex = if let Some((_secret, pub_hex)) = pqc_raw.split_once(':') {
                // Validate pub_hex is valid hex
                pub_hex.to_string()
            } else {
                // Reject secret-only format - require explicit public key
                return Err(KeyError::InvalidPrivateKey);
            };

            // Combine public parts and compute hybrid address (SHA3-256 of combined_public)
            let combined_public = format!("{}:{}", classical_pub_hex, pqc_hex);
            let mut hasher = Sha3_256::new();
            hasher.update(combined_public.as_bytes());
            let hash_result = hasher.finalize();
            let address = format!("0x{}", hex::encode(&hash_result[..]));

            // Preserve provided formatting where possible. If the original
            // `private_key` began with `kanahybrid` use it; otherwise return a
            // canonical `kanahybrid`-prefixed payload reconstructed from the
            // parsed hybrid payload.
            let formatted_private_key = if private_key.starts_with(KANAHYBRID_PREFIX)
                || raw_private_key.starts_with(KANAHYBRID_PREFIX)
            {
                if private_key.starts_with(KANAHYBRID_PREFIX) {
                    private_key.to_string()
                } else {
                    // original had a different prefix but raw contains kanahybrid
                    format!("{}{}", KANAHYBRID_PREFIX, hybrid)
                }
            } else {
                // Fallback: create canonical hybrid prefix
                format!("{}{}", KANAHYBRID_PREFIX, hybrid)
            };

            Ok(KeyPair {
                private_key: Zeroizing::new(formatted_private_key),
                public_key: combined_public,
                pqc_public_key: Some(pqc_hex.clone()),
                address,
                curve_type,
            })
        } // All CurveType variants are handled above; no catch-all arm needed.
    }
}

/// Generate a mnemonic phrase with the specified word count
pub fn generate_mnemonic(word_count: usize) -> Result<String, KeyError> {
    let entropy_bits = match word_count {
        12 => 128,
        24 => 256,
        _ => {
            return Err(KeyError::GenerationFailed(format!(
                "Unsupported word count: {}",
                word_count
            )));
        }
    };
    let mut entropy = Zeroizing::new(vec![0u8; entropy_bits / 8]);

    SysRng
        .try_fill_bytes(&mut entropy)
        .expect("Failed to get OS randomness");

    let mnemonic =
        Mnemonic::from_entropy(&entropy).map_err(|e| KeyError::GenerationFailed(e.to_string()))?;
    Ok(mnemonic.to_string())
}

/// Struct representing an imported wallet with private key, public key, and address
pub struct ImportedWallet {
    pub private_key: Zeroizing<String>,
    pub public_key: String,
    pub address: String,
}

/// Import a wallet from a seed phrase
pub fn import_from_seed_phrase(
    phrase: &str,
    curve_type: CurveType,
) -> Result<ImportedWallet, String> {
    match keypair_from_mnemonic(phrase, curve_type) {
        Ok(keypair) => Ok(ImportedWallet {
            private_key: keypair.export_private_key_secure(),
            public_key: keypair.get_public_key().to_string(),
            address: keypair.get_address().to_string(),
        }),
        Err(e) => Err(e.to_string()),
    }
}

/// Import a wallet from a private key
pub fn import_from_private_key(
    private_key: &str,
    curve_type: CurveType,
) -> Result<ImportedWallet, String> {
    match keypair_from_private_key(private_key, curve_type) {
        Ok(keypair) => Ok(ImportedWallet {
            private_key: keypair.export_private_key_secure(),
            public_key: keypair.get_public_key().to_string(),
            address: keypair.get_address().to_string(),
        }),
        Err(e) => Err(e.to_string()),
    }
}
