// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Secure data encryption and decryption
//!
//! This module provides functions for encrypting and decrypting data using
//!
//! **Classical**: AES-256-GCM with Argon2 key derivation

use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use argon2::{Algorithm, Argon2, Version};
use base64::{Engine as _, engine::general_purpose};
use rand::TryRng;
use rand::rngs::SysRng;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::{self, ErrorKind};
use std::string::ToString;
use thiserror::Error;
use zeroize::Zeroize;

pub mod stream;
pub use stream::{
    DEFAULT_STREAM_CHUNK_SIZE, MAX_STREAM_CHUNK_SIZE, StreamDecryptingReader,
    StreamEncryptingWriter, StreamEncryptionHeader, decrypt_stream, encrypt_stream,
    stream_encrypting_writer,
};

/// Encryption scheme selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EncryptionScheme {
    /// AES-256-GCM (Classical, fast)
    #[default]
    Aes256Gcm,
}

impl EncryptionScheme {
    /// Returns true if this scheme is quantum-resistant
    pub fn is_quantum_resistant(&self) -> bool {
        false
    }

    /// Get security level (1-5)
    pub fn security_level(&self) -> u8 {
        match self {
            EncryptionScheme::Aes256Gcm => 4,
        }
    }

    /// Check if this scheme is available (compile-time feature check)
    pub fn is_available(&self) -> bool {
        match self {
            EncryptionScheme::Aes256Gcm => true,
        }
    }
}

/// Error types for encryption operations
#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Encryption error: {0}")]
    AeadError(String),

    #[error("Key derivation error: {0}")]
    KeyDerivationError(String),

    #[error("Invalid format error: {0}")]
    InvalidFormat(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Decryption error")]
    DecryptionError,

    #[error("Post-quantum encryption error: {0}")]
    PqcError(String),

    #[error("Feature not available: {0} requires 'pqc' feature to be enabled")]
    FeatureNotAvailable(String),
}

/// Structure representing encrypted data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    ciphertext_array: Vec<u8>,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    ciphertext: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    nonce_array: Vec<u8>,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    nonce: String,

    salt: String,

    #[serde(default = "default_encryption_version")]
    #[serde(skip_serializing_if = "is_default_encryption_version")]
    pub version: u32,
}

fn default_encryption_version() -> u32 {
    1
}
fn is_default_encryption_version(v: &u32) -> bool {
    *v == 1
}

pub(super) const STREAM_ENCRYPTION_FORMAT_VERSION: u32 = 2;
pub(super) const STREAM_ENCRYPTION_FORMAT_VERSION_V1: u32 = 1;
const ENCRYPTION_FORMAT_VERSION: u32 = 2;
pub(super) const STREAM_ENCRYPTION_ALGORITHM: &str = "AES-256-GCM-FRAMED-ARGON2ID";
pub(super) const STREAM_NONCE_PREFIX_LEN: usize = 4;
pub(super) const AEAD_TAG_LEN: usize = 16;
pub(super) const STREAM_FRAME_AAD_LEN: usize =
    STREAM_ENCRYPTION_ALGORITHM.len() + STREAM_NONCE_PREFIX_LEN + 8 + 1 + 4;

impl EncryptedData {
    /// Get the ciphertext bytes (STANDARD base64 only, no legacy fallback)
    pub fn get_ciphertext(&self) -> Result<Vec<u8>, EncryptionError> {
        if self.ciphertext.is_empty() {
            return Err(EncryptionError::InvalidFormat(
                "Empty ciphertext".to_string(),
            ));
        }
        general_purpose::STANDARD
            .decode(&self.ciphertext)
            .map_err(|e| {
                EncryptionError::InvalidFormat(format!("Invalid ciphertext base64: {}", e))
            })
    }

    /// Get the nonce bytes (STANDARD base64 only, no legacy fallback)
    pub fn get_nonce(&self) -> Result<Vec<u8>, EncryptionError> {
        if self.nonce.is_empty() {
            return Err(EncryptionError::InvalidFormat("Empty nonce".to_string()));
        }
        general_purpose::STANDARD
            .decode(&self.nonce)
            .map_err(|e| EncryptionError::InvalidFormat(format!("Invalid nonce base64: {}", e)))
    }
}

impl fmt::Display for EncryptedData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EncryptedData {{ ciphertext: [{}], nonce: [{}], salt: {} }}",
            self.ciphertext.len(),
            self.nonce.len(),
            self.salt
        )
    }
}

/// Encrypt data with a password
pub fn encrypt_data(data: &[u8], password: &str) -> Result<EncryptedData, EncryptionError> {
    validate_password(password)?;

    // Generate a random salt for key derivation (16 bytes, AB64-encoded) via CSPRNG
    let mut salt_bytes = [0u8; 16];
    SysRng
        .try_fill_bytes(&mut salt_bytes)
        .map_err(|e| EncryptionError::AeadError(format!("RNG failure: {}", e)))?;
    let salt_b64 = general_purpose::STANDARD.encode(salt_bytes);

    let version = ENCRYPTION_FORMAT_VERSION;
    let key_bytes_vec = derive_key_for_version(password, &salt_bytes, version)?;
    let cipher = cipher_from_derived(key_bytes_vec.as_slice())?;

    // Generate a random nonce for AES-GCM via CSPRNG
    let mut nonce_arr = [0u8; 12];
    SysRng
        .try_fill_bytes(&mut nonce_arr)
        .map_err(|e| EncryptionError::AeadError(format!("RNG failure: {}", e)))?;
    let nonce = Nonce::try_from(&nonce_arr[..])
        .map_err(|_| EncryptionError::InvalidInput("Invalid nonce length".to_string()))?;

    // Encrypt with AAD binding salt+version for v2 (fail-closed, legacy v1 has no AAD)
    let ciphertext = if version >= 2 {
        let aad = encryption_aad(&salt_b64, version);
        cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: data,
                    aad: &aad,
                },
            )
            .map_err(|e| EncryptionError::AeadError(e.to_string()))?
    } else {
        cipher
            .encrypt(&nonce, data)
            .map_err(|e| EncryptionError::AeadError(e.to_string()))?
    };

    // Store values in a more compact base64 representation
    let ciphertext_b64 = general_purpose::STANDARD.encode(&ciphertext);
    let nonce_b64 = general_purpose::STANDARD.encode(nonce_arr);

    // Zeroize intermediate derived key material as soon as possible (cipher holds expanded key until drop)
    drop(key_bytes_vec);

    Ok(EncryptedData {
        ciphertext_array: Vec::new(),
        ciphertext: ciphertext_b64,
        nonce_array: Vec::new(),
        nonce: nonce_b64,
        salt: salt_b64,
        version,
    })
}

/// Decrypt data with a password
pub fn decrypt_data(encrypted: &EncryptedData, password: &str) -> Result<Vec<u8>, EncryptionError> {
    validate_password(password)?;

    // Validate ciphertext size to prevent memory exhaustion attacks
    const MAX_CIPHERTEXT_SIZE: usize = 100 * 1024 * 1024; // 100MB
    // Check encoded size before allocation to avoid OOM via large base64
    const MAX_ENCODED_SIZE: usize = 140 * 1024 * 1024; // 100MiB decoded ~133MiB encoded
    if encrypted.ciphertext.len() > MAX_ENCODED_SIZE {
        return Err(EncryptionError::InvalidFormat(
            "Ciphertext size exceeds maximum allowed".to_string(),
        ));
    }
    let ciphertext = encrypted.get_ciphertext()?;
    if ciphertext.len() > MAX_CIPHERTEXT_SIZE {
        return Err(EncryptionError::InvalidFormat(
            "Ciphertext size exceeds maximum allowed".to_string(),
        ));
    }

    let salt_bytes = decode_salt(&encrypted.salt)?;

    let version = if encrypted.version == 0 {
        1
    } else {
        encrypted.version
    };
    let key_bytes_vec = derive_key_for_version(password, &salt_bytes, version)?;
    let cipher = cipher_from_derived(key_bytes_vec.as_slice())?;

    // We already decoded ciphertext above; get the nonce bytes now
    let nonce_bytes = encrypted.get_nonce()?;

    // Create nonce for decryption - need to convert Vec<u8> to Nonce
    if nonce_bytes.len() != 12 {
        return Err(EncryptionError::InvalidFormat(
            "Invalid nonce length".to_string(),
        ));
    }
    let nonce = aes_gcm::Nonce::try_from(nonce_bytes.as_slice())
        .map_err(|_| EncryptionError::InvalidFormat("Invalid nonce length".to_string()))?;

    // Zeroize intermediate derived key material before decryption (cipher holds expanded key)
    drop(key_bytes_vec);

    // Decrypt with AAD for v2, legacy v1 without AAD (strict per version)
    if version >= 2 {
        let aad = encryption_aad(&encrypted.salt, version);
        cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: &ciphertext,
                    aad: &aad,
                },
            )
            .map_err(|_| EncryptionError::DecryptionError)
    } else {
        cipher
            .decrypt(&nonce, ciphertext.as_ref())
            .map_err(|_| EncryptionError::DecryptionError)
    }
}

fn validate_password(password: &str) -> Result<(), EncryptionError> {
    if password.is_empty() {
        return Err(EncryptionError::InvalidInput(
            "Password cannot be empty".to_string(),
        ));
    }
    if password.len() > crate::MAX_PASSWORD_LEN {
        return Err(EncryptionError::InvalidInput(format!(
            "Password exceeds maximum length of {} bytes",
            crate::MAX_PASSWORD_LEN
        )));
    }
    Ok(())
}

const MIN_SALT_LEN: usize = 16;
const MAX_SALT_LEN: usize = 64;

fn decode_salt(salt_b64: &str) -> Result<Vec<u8>, EncryptionError> {
    // STRICT แต่ต้องรองรับ legacy SaltString (B64Unpadded 22 อักขระ) ไม่งั้น wallet เก่าถอดไม่ได้ -> Invalid password
    let bytes = general_purpose::STANDARD
        .decode(salt_b64)
        .or_else(|_| {
            let mut s = salt_b64.to_string();
            let rem = s.len() % 4;
            if rem != 0 {
                s.extend(std::iter::repeat('=').take(4 - rem));
            }
            general_purpose::STANDARD.decode(&s)
        })
        .map_err(|_| EncryptionError::InvalidFormat("Invalid salt format".to_string()))?;
    if bytes.len() < MIN_SALT_LEN {
        return Err(EncryptionError::InvalidFormat(format!(
            "Salt too short: {} < {}",
            bytes.len(),
            MIN_SALT_LEN
        )));
    }
    if bytes.len() > MAX_SALT_LEN {
        return Err(EncryptionError::InvalidFormat(format!(
            "Salt too long: {} > {}",
            bytes.len(),
            MAX_SALT_LEN
        )));
    }
    Ok(bytes)
}

fn derive_key(password: &str, salt: &[u8]) -> Result<zeroize::Zeroizing<Vec<u8>>, EncryptionError> {
    derive_key_for_version(password, salt, 1)
}

pub(super) fn derive_key_for_version(
    password: &str,
    salt: &[u8],
    version: u32,
) -> Result<zeroize::Zeroizing<Vec<u8>>, EncryptionError> {
    let password_zero = zeroize::Zeroizing::new(password.as_bytes().to_vec());
    let params = argon2_params_for_version(version)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut output = vec![0u8; 32];
    argon2
        .hash_password_into(&password_zero, salt, &mut output)
        .map_err(|e| EncryptionError::KeyDerivationError(e.to_string()))?;
    Ok(zeroize::Zeroizing::new(output))
}

fn encryption_aad(salt_b64: &str, version: u32) -> Vec<u8> {
    // Domain separation for v2: binds ciphertext to salt+version, prevents cross-version transplant
    let mut aad = Vec::with_capacity(salt_b64.len() + 16);
    aad.extend_from_slice(b"kanari-aead-v");
    aad.extend_from_slice(version.to_string().as_bytes());
    aad.extend_from_slice(b":");
    aad.extend_from_slice(salt_b64.as_bytes());
    aad
}

fn cipher_from_derived(key_bytes: &[u8]) -> Result<Aes256Gcm, EncryptionError> {
    let key = Key::<Aes256Gcm>::try_from(key_bytes).map_err(|_| {
        EncryptionError::KeyDerivationError("Argon2 produced invalid key length".to_string())
    })?;
    Ok(Aes256Gcm::new(&key))
}

fn encryption_error_to_io(error: EncryptionError) -> io::Error {
    let kind = match error {
        EncryptionError::InvalidFormat(_) | EncryptionError::DecryptionError => {
            ErrorKind::InvalidData
        }
        EncryptionError::InvalidInput(_) => ErrorKind::InvalidInput,
        _ => ErrorKind::Other,
    };
    io::Error::new(kind, error)
}

// Helper function to get consistent argon2 parameters (legacy v1)
#[allow(dead_code)]
fn argon2_params() -> Result<argon2::Params, EncryptionError> {
    argon2_params_for_version(1)
}

fn argon2_params_for_version(version: u32) -> Result<argon2::Params, EncryptionError> {
    let (m_cost, t_cost, p_cost) = match version {
        1 => (47104, 3, 1), // legacy: 46MiB, p=1 (kept for decrypt of old wallets)
        _ => (47104, 3, 4), // v2: 46MiB, p=4 - harden GPU/ASIC, still interactive (~0.4s)
    };
    argon2::Params::new(m_cost, t_cost, p_cost, Some(32)).map_err(|e| {
        EncryptionError::KeyDerivationError(format!("Invalid Argon2 parameters: {}", e))
    })
}

/// Upgrade legacy encrypted data to new base64 format (explicit migration only, no implicit fallback in decrypt)
pub fn upgrade_encrypted_data(old_data: EncryptedData) -> EncryptedData {
    if !old_data.ciphertext_array.is_empty() && old_data.ciphertext.is_empty() {
        EncryptedData {
            ciphertext: general_purpose::STANDARD.encode(&old_data.ciphertext_array),
            ciphertext_array: Vec::new(),
            nonce: general_purpose::STANDARD.encode(&old_data.nonce_array),
            nonce_array: Vec::new(),
            salt: old_data.salt,
            version: old_data.version,
        }
    } else {
        old_data
    }
}

/// Encrypt a string with a password
pub fn encrypt_string(data: &str, password: &str) -> Result<EncryptedData, EncryptionError> {
    encrypt_data(data.as_bytes(), password)
}

/// Decrypt a string with a password
pub fn decrypt_string(
    encrypted: &EncryptedData,
    password: &str,
) -> Result<String, EncryptionError> {
    let bytes = decrypt_data(encrypted, password)?;
    String::from_utf8(bytes).map_err(|e| EncryptionError::InvalidFormat(e.to_string()))
}

/// Securely erase sensitive data from memory
/// Uses zeroize crate for secure memory clearing
pub fn secure_erase(data: &mut [u8]) {
    data.zeroize();
}
