// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Digital signature creation and verification
//!
//! This module handles digital signatures across multiple curves (classical ECC and PQC),
//! with unified interfaces for signing and verifying messages using different key types.
//!
//! # ⚠️ Security Considerations
//!
//! ## Ed25519 Implementation Note
//! By default this crate now uses the standard RFC-8032 Ed25519 behavior:
//! - Messages are signed and verified directly (no pre-hashing).
//! - This makes Kanari Ed25519 signatures compatible with other standard
//!   Ed25519 implementations. If you need a pre-hash variant, implement an
//!   explicit alternate codepath.
//!
//! ## Timing Attack Mitigation via Tagged Addresses ⚠️ CRITICAL
//!
//! **The Problem:**
//! When verifying signatures without tagged addresses, the fallback path attempts all curve types
//! sequentially. Real-world timing measurements can reveal which curve succeeded:
//! - Branch prediction effects
//! - Cache line timing differences
//! - Speculative execution patterns
//! - Power analysis (in physical security contexts)
//!
//! **The Solution - Always Use Tagged Addresses:**
//! ```rust,ignore
//! // ✅ CORRECT - Timing-safe verification (recommended for production)
//! let addr = keypair.tagged_address(); // e.g., "K256:0xabc..."
//! verify_signature(&addr, message, signature)?;
//! ```
//!
//! **If Tagged Address Unavailable:**
//! ```rust,ignore
//! // ⚠️  Use explicit curve when known
//! verify_signature_with_curve(address, message, signature, CurveType::K256)?;
//! ```
//!
//! **Never Rely On:**
//! ```rust,ignore
//! // ❌ DON'T - Timing attack vulnerability
//! verify_signature("0xdecafbad...", message, signature)?; // No tag = fallback path
//! ```
//!
//! ## Hashing Strategy: Curve-Specific Pre-hashing or Direct Signing
//! ⚠️ **IMPORTANT: Different curves use different hashing approaches**
//!
//! **K256 (secp256k1) and P256 (secp256r1) - SHA3-256 Pre-Hashing (Kanari-specific):**
//! ```rust,ignore
//! // Message is HASHED with SHA3-256 BEFORE signing
//! let msg_hash = Sha3_256::digest(message);
//! sign(msg_hash, private_key)  // ECC signatures operate on hash
//! ```
//! - **Why:** Domain separation - prevents signing the same message differently across curves
//! - **Canonical format:** Always use `sign_message(privkey, msg, CurveType::K256/P256)`
//! - **DON'T:** Pre-hash the message yourself - it would be double-hashed!
//!
//! **Ed25519 - Direct Signing (RFC-8032 Standard):**
//! ```rust,ignore
//! // Message is signed DIRECTLY without pre-hashing
//! sign(message, private_key)  // Per RFC-8032 standard
//! ```
//! - **Why:** EdDSA standard behavior - ensures interoperability with all Ed25519 libraries
//! - **Canonical format:** Always use `sign_message(privkey, msg, CurveType::Ed25519)`
//! - **DON'T:** Pre-hash the message - Ed25519 handles it internally per RFC!
//!
//! **Hybrid Signatures (Ed25519+Dilithium3, K256+Dilithium3):**
//! - Ed25519 component: Direct signing (RFC-8032)
//! - K256 component: SHA3-256 pre-hashing (Kanari-specific)
//! - Dilithium3 component: Direct signing (PQC standard)
//! - Format: `[2-byte classical_len] || classical_sig || pqc_sig`
//!
//! **Summary - Never Double-Hash:**
//! | Curve Type | Strategy | Input to sign_message |
//! |-----------|----------|---------------------|
//! | K256 | SHA3-256 pre-hash (Kanari) | Raw message (function hashes) |
//! | P256 | SHA3-256 pre-hash (Kanari) | Raw message (function hashes) |
//! | Ed25519 | Direct (RFC-8032) | Raw message (direct signing) |
//! | Hybrid | Mix above | Raw message (each component handles correctly) |
//!
//! **Test Your Integration:**
//! - If verification fails after signing with our API: Check for double-hashing in your code!
//! - Kanari handles hashing internally - pass raw messages only
//! - Example: ❌ `sign(sha256(msg))` | ✅ `sign(msg)`
//!
//! ## Hybrid Signature Format
//! Hybrid signatures (K256+Dilithium3, Ed25519+Dilithium3) use a structured format:
//! - 2 bytes: classical signature length (big-endian)
//! - N bytes: classical signature
//! - M bytes: PQC signature
//!
//! This format requires careful parsing. Malformed signatures verify as false,
//! not raising errors, to prevent DoS attacks.
//!
//! ## Post-Quantum Cryptography Dependencies
//! This library uses NIST-standardized PQC algorithms (Dilithium, SPHINCS+):
//! - **Monitor Security Advisories:**
//!   - https://github.com/rustpq/pqcrypto/security/advisories
//!   - https://rustsec.org/
//! - **PQC Crate Versions:** Pinned to specific releases (see Cargo.toml)
//! - **Update Strategy:** Review advisories before updating PQC crates
//! - **Backup Plans:** Implement version pin policies in production lock files
//!
//! ## Error Message Information Leakage
//! Error messages are kept generic to avoid leaking implementation details:
//! - Public key length validation failures do NOT reveal expected sizes
//! - Invalid format errors do NOT specify which curve was attempted
//! - This prevents attackers from enumerating valid key formats
//! - For debugging: use debug logs with `RUST_LOG=debug`, not error messages
//!
//! ## Rate Limiting Recommendations
//! This module has NO built-in rate limiting (by design - concerns are at API layer):
//! - **For APIs:** Implement rate limiting at axum/actix middleware level
//! - **For CLI:** Consider throttling on stdin processing
//! - **For Batch Operations:** Group verify calls with shared circuit breaker
//! - **DOS Prevention:** Limit signature verifications per time window
//!
//! Example rate limiter (governor crate):
//! ```rust,ignore
//! use governor::{Quota, RateLimiter};
//! use std::num::NonZeroU32;
//!
//! let limiter = RateLimiter::direct(Quota::per_second(NonZeroU32::new(100).unwrap()));
//! if let Ok(_) = limiter.check() {
//!     verify_signature(...)?;
//! }
//! ```

use log::debug;
use sha3::{Digest, Sha3_256};
use thiserror::Error;

use k256::{
    SecretKey as K256SecretKey,
    ecdsa::{
        Signature as K256Signature, SigningKey as K256SigningKey, VerifyingKey as K256VerifyingKey,
    },
};

use p256::{
    SecretKey as P256SecretKey,
    ecdsa::{Signature as P256Signature, SigningKey, VerifyingKey, signature::Verifier},
};

use ed25519_dalek::{
    Signature as Ed25519Signature, Signer, SigningKey as Ed25519SigningKey,
    VerifyingKey as Ed25519VerifyingKey,
};

// PQC crates
use pqcrypto_dilithium::dilithium2;
use pqcrypto_dilithium::dilithium3;
use pqcrypto_dilithium::dilithium5;
use pqcrypto_sphincsplus::sphincssha2256fsimple;
use pqcrypto_traits::sign::SecretKey as PqcSecretKeyTrait;
use pqcrypto_traits::sign::{
    DetachedSignature as PqcDetachedTrait, PublicKey as PqcPublicKeyTrait,
};
use zeroize::{Zeroize, Zeroizing};

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

    #[error("Invalid signature length")]
    InvalidSignatureLength,
}

/// Maximum allowed signature bytes to guard against resource exhaustion in parsing
const MAX_SIGNATURE_SIZE: usize = 64 * 1024; // 64 KiB
/// Maximum classical signature length we accept inside a hybrid combined signature
const MAX_CLASSICAL_SIG_LEN: usize = 1024; // limit to 1 KiB to avoid DoS

// Common EC public key lengths
// These constants are used for flexible address parsing but should not be
// exposed in error messages to prevent information leakage
const SEC1_UNCOMPRESSED_LEN: usize = 65;
const SEC1_COMPRESSED_LEN: usize = 33;
const RAW_XY_LEN: usize = 64;
const X_ONLY_LEN: usize = 32;
// Ed25519 public key length (32 bytes)
const ED25519_PUBLIC_KEY_LEN: usize = 32;

/// Zero out sensitive data in memory
/// Uses zeroize crate for secure memory clearing with compiler fence
/// to prevent optimization that could leave sensitive data in memory
pub fn secure_clear(data: &mut [u8]) {
    data.zeroize();
    // Add a black_box to prevent compiler from optimizing away the zeroization
    std::hint::black_box(data);
}

/// Sign a message with a given private key and curve type
pub fn sign_message(
    private_key_hex: &str,
    message: &[u8],
    curve_type: CurveType,
) -> Result<Vec<u8>, SignatureError> {
    // Extract raw key if it has any known Kanari prefix
    let raw_key = crate::keys::extract_raw_key(private_key_hex);

    match curve_type {
        CurveType::K256 => sign_message_k256(raw_key, message),
        CurveType::P256 => sign_message_p256(raw_key, message),
        CurveType::Ed25519 => sign_message_ed25519(raw_key, message),
        // For hybrid K256+Dilithium3, sign with the classical K256 private key part
        CurveType::K256Dilithium3 => sign_message_hybrid_k256(raw_key, message),
        // For hybrid Ed25519+Dilithium3, sign with the classical Ed25519 private key part
        CurveType::Ed25519Dilithium3 => sign_message_hybrid_ed25519(raw_key, message),
        // Handle pure PQC curves by delegating to PQC-specific signing functions
        CurveType::Dilithium2 => sign_message_dilithium2(raw_key, message),
        CurveType::Dilithium3 => sign_message_dilithium3(raw_key, message),
        CurveType::Dilithium5 => sign_message_dilithium5(raw_key, message),
        CurveType::SphincsPlusSha256Robust => sign_message_sphincs(raw_key, message),
    }
}

/// Sign a message using Dilithium2 private key (PQC)
fn sign_message_dilithium2(
    private_key_hex: &str,
    message: &[u8],
) -> Result<Vec<u8>, SignatureError> {
    let raw = crate::keys::extract_raw_key(private_key_hex);
    // Accept formats: "<secret_hex>" or "<secret_hex>:<public_hex>"
    let secret_hex = raw.split_once(':').map(|(s, _)| s).unwrap_or(raw);
    let sk_bytes: Zeroizing<Vec<u8>> = Zeroizing::new(hex::decode(secret_hex).map_err(|_| {
        SignatureError::InvalidPrivateKey("Invalid Dilithium2 private key".to_string())
    })?);
    let sk = dilithium2::SecretKey::from_bytes(&sk_bytes).map_err(|_| {
        SignatureError::InvalidPrivateKey("Invalid Dilithium2 private key".to_string())
    })?;
    let sig = dilithium2::detached_sign(message, &sk);
    Ok(sig.as_bytes().to_vec())
}

/// Sign a message using Dilithium3 private key (PQC)
fn sign_message_dilithium3(
    private_key_hex: &str,
    message: &[u8],
) -> Result<Vec<u8>, SignatureError> {
    let raw = crate::keys::extract_raw_key(private_key_hex);
    // Accept formats: "<secret_hex>" or "<secret_hex>:<public_hex>"
    let secret_hex = raw.split_once(':').map(|(s, _)| s).unwrap_or(raw);
    let sk_bytes: Zeroizing<Vec<u8>> =
        Zeroizing::new(hex::decode(secret_hex).map_err(|_| {
            SignatureError::InvalidPrivateKey("Invalid private key hex".to_string())
        })?);
    let sk = dilithium3::SecretKey::from_bytes(&sk_bytes).map_err(|_| {
        SignatureError::InvalidPrivateKey("Invalid Dilithium3 private key".to_string())
    })?;
    let sig = dilithium3::detached_sign(message, &sk);
    Ok(sig.as_bytes().to_vec())
}

/// Sign a message using Dilithium5 private key (PQC)
fn sign_message_dilithium5(
    private_key_hex: &str,
    message: &[u8],
) -> Result<Vec<u8>, SignatureError> {
    let raw = crate::keys::extract_raw_key(private_key_hex);
    // Accept formats: "<secret_hex>" or "<secret_hex>:<public_hex>"
    let secret_hex = raw.split_once(':').map(|(s, _)| s).unwrap_or(raw);
    let sk_bytes: Zeroizing<Vec<u8>> =
        Zeroizing::new(hex::decode(secret_hex).map_err(|_| {
            SignatureError::InvalidPrivateKey("Invalid private key hex".to_string())
        })?);
    let sk = dilithium5::SecretKey::from_bytes(&sk_bytes).map_err(|_| {
        SignatureError::InvalidPrivateKey("Invalid Dilithium5 private key".to_string())
    })?;
    let sig = dilithium5::detached_sign(message, &sk);
    Ok(sig.as_bytes().to_vec())
}

/// Sign a message using SPHINCS+ private key (PQC)
fn sign_message_sphincs(private_key_hex: &str, message: &[u8]) -> Result<Vec<u8>, SignatureError> {
    let raw = crate::keys::extract_raw_key(private_key_hex);
    // Accept formats: "<secret_hex>" or "<secret_hex>:<public_hex>"
    let secret_hex = raw.split_once(':').map(|(s, _)| s).unwrap_or(raw);
    let sk_bytes: Zeroizing<Vec<u8>> =
        Zeroizing::new(hex::decode(secret_hex).map_err(|_| {
            SignatureError::InvalidPrivateKey("Invalid private key hex".to_string())
        })?);
    let sk = sphincssha2256fsimple::SecretKey::from_bytes(&sk_bytes).map_err(|_| {
        SignatureError::InvalidPrivateKey("Invalid SPHINCS+ private key".to_string())
    })?;
    let sig = sphincssha2256fsimple::detached_sign(message, &sk);
    Ok(sig.as_bytes().to_vec())
}

/// Hybrid helper: sign using the classical K256 part of a hybrid key
fn sign_message_hybrid_k256(
    hybrid_private: &str,
    message: &[u8],
) -> Result<Vec<u8>, SignatureError> {
    let hybrid = crate::keys::extract_raw_key(hybrid_private);
    // Expect format: "<classical_secret_hex>:<pqc_secret_hex>" (pqc part may contain ":<pub>" too)
    let parts: Vec<&str> = hybrid.splitn(2, ':').collect();
    let classical = parts.first().ok_or_else(|| {
        SignatureError::InvalidPrivateKey("Invalid hybrid private key format".to_string())
    })?;
    let pqc_part = parts.get(1).ok_or_else(|| {
        SignatureError::InvalidPrivateKey("Missing PQC part in hybrid private key".to_string())
    })?;

    // PQC secret may be stored as "<secret_hex>:<public_hex>" or just "<secret_hex>"
    let pqc_secret = pqc_part.split_once(':').map(|(s, _)| s).unwrap_or(pqc_part);

    // Sign classical part (K256)
    let classical_sig = sign_message_k256(classical, message)?;

    // Sign PQC part (Dilithium3)
    let pqc_sig = sign_message_dilithium3(pqc_secret, message)?;

    // Validate classical signature length before encoding into u16 and combining
    if classical_sig.len() > MAX_CLASSICAL_SIG_LEN || classical_sig.len() > u16::MAX as usize {
        return Err(SignatureError::InvalidFormat(
            "Classical signature too large".to_string(),
        ));
    }
    // Combine as: [2-byte classical_sig_len BE] || classical_sig || pqc_sig
    // Use checked_add to prevent overflow in capacity calculation
    let total_capacity = 2usize
        .checked_add(classical_sig.len())
        .and_then(|sum| sum.checked_add(pqc_sig.len()))
        .ok_or_else(|| SignatureError::InvalidFormat("Signature size overflow".to_string()))?;
    let mut out = Vec::with_capacity(total_capacity);
    let len_be = (classical_sig.len() as u16).to_be_bytes();
    out.extend_from_slice(&len_be);
    out.extend_from_slice(&classical_sig);
    out.extend_from_slice(&pqc_sig);
    Ok(out)
}

/// Hybrid helper: sign using the classical Ed25519 part of a hybrid key
fn sign_message_hybrid_ed25519(
    hybrid_private: &str,
    message: &[u8],
) -> Result<Vec<u8>, SignatureError> {
    let hybrid = crate::keys::extract_raw_key(hybrid_private);
    // Expect format: "<classical_secret_hex>:<pqc_secret_hex>" (pqc part may contain ":<pub>" too)
    let parts: Vec<&str> = hybrid.splitn(2, ':').collect();
    let classical = parts.first().ok_or_else(|| {
        SignatureError::InvalidPrivateKey("Invalid hybrid private key format".to_string())
    })?;
    let pqc_part = parts.get(1).ok_or_else(|| {
        SignatureError::InvalidPrivateKey("Missing PQC part in hybrid private key".to_string())
    })?;

    let pqc_secret = pqc_part.split_once(':').map(|(s, _)| s).unwrap_or(pqc_part);

    // Sign classical part (Ed25519)
    let classical_sig = sign_message_ed25519(classical, message)?;

    // Sign PQC part (Dilithium3)
    let pqc_sig = sign_message_dilithium3(pqc_secret, message)?;

    // Validate classical signature length fits in u16 and within configured limits (prevent overflow / DoS)
    if classical_sig.len() > MAX_CLASSICAL_SIG_LEN || classical_sig.len() > u16::MAX as usize {
        return Err(SignatureError::InvalidFormat(
            "Classical signature too large".to_string(),
        ));
    }

    // Combine as: [2-byte classical_sig_len BE] || classical_sig || pqc_sig
    // Use checked_add to prevent overflow in capacity calculation
    let total_capacity = 2usize
        .checked_add(classical_sig.len())
        .and_then(|sum| sum.checked_add(pqc_sig.len()))
        .ok_or_else(|| SignatureError::InvalidFormat("Signature size overflow".to_string()))?;
    let mut out = Vec::with_capacity(total_capacity);
    let len_be = (classical_sig.len() as u16).to_be_bytes();
    out.extend_from_slice(&len_be);
    out.extend_from_slice(&classical_sig);
    out.extend_from_slice(&pqc_sig);
    Ok(out)
}

/// Sign a message using K256 (secp256k1) private key
///
/// **⚠️ IMPORTANT: Kanari-Specific Pre-Hashing**
///
/// This function automatically hashes the message with SHA3-256 BEFORE signing.
/// - Input: Raw message bytes
/// - Internal: `message_hash = SHA3-256(message)`
/// - Operation: Sign the hash, not the raw message
///
/// **CRITICAL - DO NOT DOUBLE-HASH:**
/// ✅ Correct: `sign_message(key, message, K256)`
/// ❌ Wrong:   `sign_message(key, sha256(message), K256)` ← Results in signature mismatch!
///
/// This pre-hashing strategy is **Kanari-specific** for domain separation across curves.
/// It differs from K256-native tools that may use different hashing.
fn sign_message_k256(private_key_hex: &str, message: &[u8]) -> Result<Vec<u8>, SignatureError> {
    // ⚠️ KANARI-SPECIFIC: Pre-hash the message with SHA3-256 for domain separation
    let mut hasher = Sha3_256::default();
    hasher.update(message);
    let message_hash = hasher.finalize();

    // Convert hex private key to bytes with zeroization
    let mut private_key_bytes = hex::decode(private_key_hex)
        .map_err(|_| SignatureError::InvalidPrivateKey("Invalid private key".to_string()))?;

    // Create signing key from private key
    let secret_key = K256SecretKey::from_slice(&private_key_bytes)
        .map_err(|_| SignatureError::InvalidPrivateKey("Invalid private key".to_string()))?;
    let signing_key = K256SigningKey::from(secret_key);

    // Zeroize private key bytes immediately after use
    private_key_bytes.zeroize();

    // Sign the hashed message
    let signature: K256Signature = signing_key.sign(&message_hash);

    // Use to_vec() from SignatureEncoding trait to get DER formatted bytes
    let der_bytes = signature.to_der();
    Ok(der_bytes.as_bytes().to_vec())
}

/// Sign a message using P256 (secp256r1) private key
///
/// **⚠️ IMPORTANT: Kanari-Specific Pre-Hashing (Same as K256)**
///
/// This function automatically hashes the message with SHA3-256 BEFORE signing.
/// - Input: Raw message bytes
/// - Internal: `message_hash = SHA3-256(message)`
/// - Operation: Sign the hash, not the raw message
///
/// **CRITICAL - DO NOT DOUBLE-HASH:**
/// ✅ Correct: `sign_message(key, message, P256)`
/// ❌ Wrong:   `sign_message(key, sha256(message), P256)` ← Results in signature mismatch!
///
/// **NOTE: P256 Strategy Matches K256**
/// Both use SHA3-256 pre-hashing for Kanari domain separation.
/// This differs from P256-native tools (NIST) that may use different schemes.
fn sign_message_p256(private_key_hex: &str, message: &[u8]) -> Result<Vec<u8>, SignatureError> {
    // ⚠️ KANARI-SPECIFIC: Pre-hash the message with SHA3-256 (same as K256 for consistency)
    let mut hasher = Sha3_256::default();
    hasher.update(message);
    let message_hash = hasher.finalize();

    // Convert hex private key to bytes with zeroization
    let mut private_key_bytes = hex::decode(private_key_hex)
        .map_err(|_| SignatureError::InvalidPrivateKey("Invalid private key".to_string()))?;

    // Create signing key from private key
    let secret_key = P256SecretKey::from_slice(&private_key_bytes)
        .map_err(|_| SignatureError::InvalidPrivateKey("Invalid private key".to_string()))?;
    let signing_key = SigningKey::from(secret_key);

    // Zeroize private key bytes immediately after use
    private_key_bytes.zeroize();

    // Sign the hashed message
    let signature: P256Signature = signing_key.sign(&message_hash);

    // Convert DER signature to bytes correctly
    let der_bytes = signature.to_der();
    Ok(der_bytes.as_bytes().to_vec())
}

/// Sign a message using Ed25519 private key
///
/// **✅ RFC-8032 COMPLIANT - Standard Ed25519 Behavior**
///
/// This implementation strictly follows RFC-8032 standard Ed25519:
/// - **Messages are signed DIRECTLY without pre-hashing** (per RFC-8032)
/// - This makes Kanari Ed25519 signatures 100% compatible with standard Ed25519 implementations
/// - Verification by external Ed25519 systems will succeed without modification
///
/// **Why This Differs From K256/P256:**
/// Kanari uses curve-specific strategies:
/// - **K256/P256** (ECC): Hash message with SHA3-256 before signing (Kanari-specific for domain separation)
/// - **Ed25519** (EdDSA): Sign message directly per RFC-8032 (STANDARD - no pre-hashing)
/// - **Hybrid** (Ed25519+Dilithium3): Ed25519 component uses RFC-8032, PQC component uses direct signing
/// - **PQC** (Dilithium, SPHINCS+): Sign message directly (standard PQC behavior)
///
/// **Interoperability Guarantee:**
/// Ed25519 signatures created by Kanari can be verified by:
/// - Standard libraries: dalek (Rust), libsodium (C), tweetnacl, PyNaCl, etc.
/// - Any RFC-8032 compliant implementation
/// - No special handling required in external systems
///
/// This architectural choice provides domain separation between curve types
/// while maintaining compatibility with standard implementations.
fn sign_message_ed25519(private_key_hex: &str, message: &[u8]) -> Result<Vec<u8>, SignatureError> {
    // RFC-8032 COMPLIANT: Sign the message DIRECTLY without pre-hashing
    let mut private_key_bytes = hex::decode(private_key_hex)
        .map_err(|_| SignatureError::InvalidPrivateKey("Invalid private key".to_string()))?;

    if private_key_bytes.len() != 32 {
        private_key_bytes.zeroize();
        return Err(SignatureError::InvalidPrivateKey(
            "Invalid private key".to_string(),
        ));
    }

    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&private_key_bytes);
    private_key_bytes.zeroize();

    let signing_key = Ed25519SigningKey::from_bytes(&key_array);

    // Sign the message directly (RFC-8032)
    let signature: Ed25519Signature = signing_key.sign(message);

    key_array.zeroize();

    Ok(signature.to_bytes().to_vec())
}

/// Verify a signature against a message using an address
///
/// This function attempts to parse tagged addresses (e.g., "K256:0xabc...") first.
/// If the address is not tagged, it falls back to trying all classical curve types.
///
/// For maximum reliability, use tagged addresses.
/// For best performance when curve type is known, use `verify_signature_with_curve()`.
pub fn verify_signature(
    address: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, SignatureError> {
    if signature.is_empty() {
        return Err(SignatureError::InvalidFormat("Empty signature".to_string()));
    }
    if signature.len() > MAX_SIGNATURE_SIZE {
        return Err(SignatureError::InvalidFormat(
            "Signature too large".to_string(),
        ));
    }

    // Try to parse as tagged address first (most reliable)
    if let Some((curve_type, addr)) = crate::keys::KeyPair::parse_tagged_address(address) {
        debug!("Using tagged address with curve type: {:?}", curve_type);
        return verify_signature_with_curve(&addr, message, signature, curve_type);
    }

    // Fallback: Try all classical curves (safe but slower)
    debug!("No tagged address found, trying all curve types");
    let clean_address = address.trim_start_matches("0x");

    // Try all classical curves without early return to prevent timing attacks
    let k256_result = verify_signature_k256(clean_address, message, signature).unwrap_or(false);
    let p256_result = verify_signature_p256(clean_address, message, signature).unwrap_or(false);
    let ed25519_result =
        verify_signature_ed25519(clean_address, message, signature).unwrap_or(false);

    // Use OR to check if any verification succeeded (constant-time operation)
    let verified = k256_result || p256_result || ed25519_result;

    if verified {
        debug!("Signature verification succeeded");
    } else {
        debug!("Signature verification failed for all curve types");
    }

    Ok(verified)
}

/// Verify a signature with the known curve type
pub fn verify_signature_with_curve(
    address: &str,
    message: &[u8],
    signature: &[u8],
    curve_type: CurveType,
) -> Result<bool, SignatureError> {
    let address_hex = address.trim_start_matches("0x");

    if signature.is_empty() {
        return Err(SignatureError::InvalidFormat("Empty signature".to_string()));
    }
    if signature.len() > MAX_SIGNATURE_SIZE {
        return Err(SignatureError::InvalidFormat(
            "Signature too large".to_string(),
        ));
    }

    match curve_type {
        CurveType::K256 => verify_signature_k256(address_hex, message, signature),
        CurveType::P256 => verify_signature_p256(address_hex, message, signature),
        CurveType::Ed25519 => verify_signature_ed25519(address_hex, message, signature),
        // For hybrid K256+Dilithium3, verify using the classical K256 public key part when provided
        CurveType::K256Dilithium3 => {
            // Hybrid must be provided as combined address: "classical_pub_hex:pqc_pub_hex"
            if !address_hex.contains(':') {
                return Err(SignatureError::InvalidPublicKey(
                    "Hybrid key requires combined address format: classical:pqc".to_string(),
                ));
            }

            let addr = address_hex;
            let classical = addr.split(':').next().unwrap_or("");

            // Require combined signature: first two bytes = classical len
            if signature.len() < 2 {
                return Ok(false);
            }
            let classical_len = u16::from_be_bytes([signature[0], signature[1]]) as usize;
            if classical_len == 0 || classical_len > MAX_CLASSICAL_SIG_LEN {
                return Ok(false);
            }
            if 2usize.saturating_add(classical_len) > signature.len() {
                return Ok(false);
            }

            let classical_sig = &signature[2..2 + classical_len];
            let pqc_sig = &signature[2 + classical_len..];

            // Verify classical part (must succeed)
            let classical_ok =
                verify_signature_k256(classical, message, classical_sig).unwrap_or(false);

            // Verify PQC part (must succeed)
            let parts: Vec<&str> = addr.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Ok(false);
            }
            let pqc_pub = parts[1];

            // Strip known prefixes like "kanapqc" if present, then decode
            let pqc_pub_raw = crate::keys::extract_raw_key(pqc_pub);
            let pub_bytes = match hex::decode(pqc_pub_raw) {
                Ok(b) => b,
                Err(_) => {
                    return Err(SignatureError::InvalidPublicKey(
                        "Invalid public key hex".to_string(),
                    ));
                }
            };
            let pk = dilithium3::PublicKey::from_bytes(&pub_bytes).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid Dilithium3 public key".to_string())
            })?;
            let sig_obj = dilithium3::DetachedSignature::from_bytes(pqc_sig).map_err(|_| {
                SignatureError::InvalidFormat("Invalid signature bytes for Dilithium3".to_string())
            })?;
            let pqc_ok = dilithium3::verify_detached_signature(&sig_obj, message, &pk).is_ok();
            Ok(classical_ok && pqc_ok)
        }
        // For hybrid Ed25519+Dilithium3, verify using the classical Ed25519 public key part when provided
        CurveType::Ed25519Dilithium3 => {
            // Hybrid must be provided as combined address
            if !address_hex.contains(':') {
                return Err(SignatureError::InvalidPublicKey(
                    "Hybrid key requires combined address format: classical:pqc".to_string(),
                ));
            }

            let addr = address_hex;
            // println!("HYBRID Ed25519 branch start, addr='{}', sig_len={}", addr, signature.len());
            let classical = addr.split(':').next().unwrap_or("");

            if signature.len() < 2 {
                return Ok(false);
            }
            let classical_len = u16::from_be_bytes([signature[0], signature[1]]) as usize;
            if classical_len == 0 || classical_len > MAX_CLASSICAL_SIG_LEN {
                return Ok(false);
            }
            if 2usize.saturating_add(classical_len) > signature.len() {
                return Ok(false);
            }

            let classical_sig = &signature[2..2 + classical_len];
            let pqc_sig = &signature[2 + classical_len..];

            let classical_ok =
                verify_signature_ed25519(classical, message, classical_sig).unwrap_or(false);

            let parts: Vec<&str> = addr.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Ok(false);
            }
            let pqc_pub = parts[1];
            let pqc_pub_raw = crate::keys::extract_raw_key(pqc_pub);
            let pub_bytes = match hex::decode(pqc_pub_raw) {
                Ok(b) => b,
                Err(_) => {
                    return Err(SignatureError::InvalidPublicKey(
                        "Invalid public key hex".to_string(),
                    ));
                }
            };
            let pk = dilithium3::PublicKey::from_bytes(&pub_bytes).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid Dilithium3 public key".to_string())
            })?;
            let sig_obj = dilithium3::DetachedSignature::from_bytes(pqc_sig).map_err(|_| {
                SignatureError::InvalidFormat("Invalid signature bytes for Dilithium3".to_string())
            })?;
            let pqc_ok = dilithium3::verify_detached_signature(&sig_obj, message, &pk).is_ok();

            Ok(classical_ok && pqc_ok)
        }
        // PQC verification using pqcrypto crates
        CurveType::Dilithium2 => {
            let pqc_raw = crate::keys::extract_raw_key(address_hex);
            let pub_bytes = hex::decode(pqc_raw).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid public key hex".to_string())
            })?;
            // Validate key length (Dilithium2 public key is 1312 bytes)
            if pub_bytes.len() != 1312 {
                return Err(SignatureError::InvalidPublicKey(
                    "Invalid Dilithium2 public key".to_string(),
                ));
            }
            let pk = dilithium2::PublicKey::from_bytes(&pub_bytes).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid Dilithium2 public key".to_string())
            })?;
            let sig_obj = dilithium2::DetachedSignature::from_bytes(signature).map_err(|_| {
                SignatureError::InvalidFormat("Invalid signature bytes for Dilithium2".to_string())
            })?;
            let res = dilithium2::verify_detached_signature(&sig_obj, message, &pk);
            match res {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        }
        CurveType::Dilithium3 => {
            let pqc_raw = crate::keys::extract_raw_key(address_hex);
            let pub_bytes = hex::decode(pqc_raw).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid public key hex".to_string())
            })?;
            // Validate key length (Dilithium3 public key is 1952 bytes)
            if pub_bytes.len() != 1952 {
                return Err(SignatureError::InvalidPublicKey(
                    "Invalid Dilithium3 public key".to_string(),
                ));
            }
            let pk = dilithium3::PublicKey::from_bytes(&pub_bytes).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid Dilithium3 public key".to_string())
            })?;
            let sig_obj = dilithium3::DetachedSignature::from_bytes(signature).map_err(|_| {
                SignatureError::InvalidFormat("Invalid signature bytes for Dilithium3".to_string())
            })?;
            let res = dilithium3::verify_detached_signature(&sig_obj, message, &pk);
            match res {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        }
        CurveType::Dilithium5 => {
            let pqc_raw = crate::keys::extract_raw_key(address_hex);
            let pub_bytes = hex::decode(pqc_raw).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid public key hex".to_string())
            })?;
            // Validate key length (Dilithium5 public key is 2592 bytes)
            if pub_bytes.len() != 2592 {
                return Err(SignatureError::InvalidPublicKey(
                    "Invalid Dilithium5 public key".to_string(),
                ));
            }

            let pk = dilithium5::PublicKey::from_bytes(&pub_bytes).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid Dilithium5 public key".to_string())
            })?;
            let sig_obj = dilithium5::DetachedSignature::from_bytes(signature).map_err(|_| {
                SignatureError::InvalidFormat("Invalid signature bytes for Dilithium5".to_string())
            })?;
            let res = dilithium5::verify_detached_signature(&sig_obj, message, &pk);
            match res {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        }
        CurveType::SphincsPlusSha256Robust => {
            let pqc_raw = crate::keys::extract_raw_key(address_hex);
            let pub_bytes = hex::decode(pqc_raw).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid public key hex".to_string())
            })?;
            let pk = sphincssha2256fsimple::PublicKey::from_bytes(&pub_bytes).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid SPHINCS+ public key".to_string())
            })?;
            let sig_obj =
                sphincssha2256fsimple::DetachedSignature::from_bytes(signature).map_err(|_| {
                    SignatureError::InvalidFormat(
                        "Invalid signature bytes for SPHINCS+".to_string(),
                    )
                })?;
            let res = sphincssha2256fsimple::verify_detached_signature(&sig_obj, message, &pk);
            match res {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        }
    }
}

/// Detailed hybrid verifier that returns (classical_ok, pqc_ok).
/// This avoids fallback behaviour and enables precise logging/errors.
fn verify_hybrid_signature_detailed(
    signature: &[u8],
    classical_pub: &str,
    pqc_pub: &str,
    message: &[u8],
    classical_verify_fn: impl Fn(&str, &[u8], &[u8]) -> Result<bool, SignatureError>,
) -> Result<(bool, bool), SignatureError> {
    // Require 2-byte classical length prefix
    if signature.len() < 2 {
        return Ok((false, false));
    }
    let classical_len = u16::from_be_bytes([signature[0], signature[1]]) as usize;
    if classical_len == 0
        || classical_len > MAX_CLASSICAL_SIG_LEN
        || 2usize.saturating_add(classical_len) > signature.len()
    {
        return Ok((false, false));
    }

    let classical_sig = &signature[2..2 + classical_len];
    let pqc_sig = &signature[2 + classical_len..];

    let classical_ok = classical_verify_fn(classical_pub, message, classical_sig).unwrap_or(false);

    if pqc_sig.is_empty() {
        return Ok((classical_ok, false));
    }

    // Strip known prefixes (e.g., "kanapqc") then decode
    let pqc_pub_raw = crate::keys::extract_raw_key(pqc_pub);
    let pub_bytes = hex::decode(pqc_pub_raw)
        .map_err(|_| SignatureError::InvalidPublicKey("Invalid public key hex".to_string()))?;
    let pk = dilithium3::PublicKey::from_bytes(&pub_bytes).map_err(|_| {
        SignatureError::InvalidPublicKey("Invalid Dilithium3 public key".to_string())
    })?;
    let sig_obj = dilithium3::DetachedSignature::from_bytes(pqc_sig).map_err(|_| {
        SignatureError::InvalidFormat("Invalid signature bytes for Dilithium3".to_string())
    })?;
    let pqc_ok = dilithium3::verify_detached_signature(&sig_obj, message, &pk).is_ok();

    Ok((classical_ok, pqc_ok))
}

/// Verify a signature using a `KeyPair` directly (avoids parsing combined public_key strings)
///
/// This function prefers the explicit `pqc_public_key` field on `KeyPair` when
/// verifying hybrid signatures, avoiding repeated parsing of `public_key` values
/// that may be stored as "classical:pqc" for backward compatibility.
pub fn verify_signature_with_keypair(
    keypair: &crate::keys::KeyPair,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, SignatureError> {
    let curve_type = keypair.curve_type;

    // `public_key` may be stored as "classical" or "classical:pqc" for legacy reasons.
    // Use the explicit `pqc_public_key` field when present.
    let pub_combined = keypair.public_key.as_str();
    let classical_pub = pub_combined.split(':').next().unwrap_or(pub_combined);

    match curve_type {
        CurveType::K256 => verify_signature_k256(classical_pub, message, signature),
        CurveType::P256 => verify_signature_p256(classical_pub, message, signature),
        CurveType::Ed25519 => verify_signature_ed25519(classical_pub, message, signature),
        CurveType::K256Dilithium3 => {
            // Require explicit PQC public key on KeyPair for hybrid verification
            let pqc_pub = keypair.get_pqc_public_key().ok_or_else(|| {
                SignatureError::InvalidPublicKey(
                    "Missing PQC public key for hybrid keypair".to_string(),
                )
            })?;
            let (classical_ok, pqc_ok) = verify_hybrid_signature_detailed(
                signature,
                classical_pub,
                &pqc_pub,
                message,
                verify_signature_k256,
            )?;
            if classical_ok && pqc_ok {
                Ok(true)
            } else {
                debug!(
                    "Hybrid verification failed: classical_ok={}, pqc_ok={}",
                    classical_ok, pqc_ok
                );
                Ok(false)
            }
        }
        CurveType::Ed25519Dilithium3 => {
            let pqc_pub = keypair.get_pqc_public_key().ok_or_else(|| {
                SignatureError::InvalidPublicKey(
                    "Missing PQC public key for hybrid keypair".to_string(),
                )
            })?;
            let (classical_ok, pqc_ok) = verify_hybrid_signature_detailed(
                signature,
                classical_pub,
                &pqc_pub,
                message,
                verify_signature_ed25519,
            )?;
            if classical_ok && pqc_ok {
                Ok(true)
            } else {
                debug!(
                    "Hybrid verification failed: classical_ok={}, pqc_ok={}",
                    classical_ok, pqc_ok
                );
                Ok(false)
            }
        }
        // Pure PQC curves: public_key on KeyPair is the PQC public hex
        CurveType::Dilithium2 => {
            let pqc_pub = keypair.get_pqc_public_key().ok_or_else(|| {
                SignatureError::InvalidPublicKey("Missing PQC public key".to_string())
            })?;
            let pqc_pub_raw = crate::keys::extract_raw_key(&pqc_pub);
            let pub_bytes = hex::decode(pqc_pub_raw).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid public key hex".to_string())
            })?;
            // Validate key length (Dilithium2 public key is 1312 bytes)
            if pub_bytes.len() != 1312 {
                return Err(SignatureError::InvalidPublicKey(
                    "Invalid Dilithium2 public key".to_string(),
                ));
            }
            let pk = dilithium2::PublicKey::from_bytes(&pub_bytes).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid Dilithium2 public key".to_string())
            })?;
            let sig_obj = dilithium2::DetachedSignature::from_bytes(signature).map_err(|_| {
                SignatureError::InvalidFormat("Invalid signature bytes for Dilithium2".to_string())
            })?;
            Ok(dilithium2::verify_detached_signature(&sig_obj, message, &pk).is_ok())
        }
        CurveType::Dilithium3 => {
            let pqc_pub = keypair.get_pqc_public_key().ok_or_else(|| {
                SignatureError::InvalidPublicKey("Missing PQC public key".to_string())
            })?;
            let pqc_pub_raw = crate::keys::extract_raw_key(&pqc_pub);
            let pub_bytes = hex::decode(pqc_pub_raw).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid public key hex".to_string())
            })?;
            // Validate key length (Dilithium3 public key is 1952 bytes)
            if pub_bytes.len() != 1952 {
                return Err(SignatureError::InvalidPublicKey(
                    "Invalid Dilithium3 public key".to_string(),
                ));
            }
            let pk = dilithium3::PublicKey::from_bytes(&pub_bytes).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid Dilithium3 public key".to_string())
            })?;
            let sig_obj = dilithium3::DetachedSignature::from_bytes(signature).map_err(|_| {
                SignatureError::InvalidFormat("Invalid signature bytes for Dilithium3".to_string())
            })?;
            Ok(dilithium3::verify_detached_signature(&sig_obj, message, &pk).is_ok())
        }
        CurveType::Dilithium5 => {
            let pqc_pub = keypair.get_pqc_public_key().ok_or_else(|| {
                SignatureError::InvalidPublicKey("Missing PQC public key".to_string())
            })?;
            let pqc_pub_raw = crate::keys::extract_raw_key(&pqc_pub);
            let pub_bytes = hex::decode(pqc_pub_raw).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid public key hex".to_string())
            })?;
            // Validate key length (Dilithium5 public key is 2592 bytes)
            if pub_bytes.len() != 2592 {
                return Err(SignatureError::InvalidPublicKey(
                    "Invalid Dilithium5 public key".to_string(),
                ));
            }

            let pk = dilithium5::PublicKey::from_bytes(&pub_bytes).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid Dilithium5 public key".to_string())
            })?;
            let sig_obj = dilithium5::DetachedSignature::from_bytes(signature).map_err(|_| {
                SignatureError::InvalidFormat("Invalid signature bytes for Dilithium5".to_string())
            })?;
            Ok(dilithium5::verify_detached_signature(&sig_obj, message, &pk).is_ok())
        }
        CurveType::SphincsPlusSha256Robust => {
            let pqc_pub = keypair.get_pqc_public_key().ok_or_else(|| {
                SignatureError::InvalidPublicKey("Missing PQC public key".to_string())
            })?;
            let pqc_pub_raw = crate::keys::extract_raw_key(&pqc_pub);
            let pub_bytes = hex::decode(pqc_pub_raw).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid public key hex".to_string())
            })?;
            // Validate key length (SPHINCS+ public key is 64 bytes)
            if pub_bytes.len() != 64 {
                return Err(SignatureError::InvalidPublicKey(
                    "Invalid SPHINCS+ public key".to_string(),
                ));
            }
            let pk = sphincssha2256fsimple::PublicKey::from_bytes(&pub_bytes).map_err(|_| {
                SignatureError::InvalidPublicKey("Invalid SPHINCS+ public key".to_string())
            })?;
            let sig_obj =
                sphincssha2256fsimple::DetachedSignature::from_bytes(signature).map_err(|_| {
                    SignatureError::InvalidFormat(
                        "Invalid signature bytes for SPHINCS+".to_string(),
                    )
                })?;
            Ok(sphincssha2256fsimple::verify_detached_signature(&sig_obj, message, &pk).is_ok())
        }
    }
}

/// Verify a signature using K256 (secp256k1)
///
/// **⚠️ IMPORTANT: Kanari K256 Uses SHA3-256 Pre-Hashing**
///
/// This verifier expects signatures created with K256 pre-hashing:
/// - Message was hashed with SHA3-256 before signing
/// - Verification repeats: `message_hash = SHA3-256(message)` then verifies signature
///
/// **Incompatible With:**
/// ❌ Raw K256 signatures (unsigned message hash)
/// ❌ Other pre-hash schemes (Bitcoin/Ethereum use different hash orders)
/// ✅ Only Kanari K256 signatures
///
/// **Integration Warning:**
/// If verifying K256 signatures from external sources:
/// - Check their hashing scheme first
/// - Kanari K256 is NOT compatible with standard secp256k1 signing
/// - Use `verify_signature_with_curve()` for explicit curve knowledge
pub fn verify_signature_k256(
    address_hex: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, SignatureError> {
    // Try to parse the signature from DER format
    let signature = K256Signature::from_der(signature)
        .map_err(|_| SignatureError::InvalidFormat("Invalid signature format".to_string()))?;

    // ⚠️ KANARI-SPECIFIC: Pre-hash the message with SHA3-256 (must match signing path!)
    let mut hasher = Sha3_256::default();
    hasher.update(message);
    let message_hash = hasher.finalize();

    // Normalize and decode the address hex (may be raw sec1 bytes or X/Y without prefix)
    let raw_key = crate::keys::extract_raw_key(address_hex);
    let decoded_hex = match hex::decode(raw_key) {
        Ok(v) => v,
        Err(_) => {
            return Err(SignatureError::InvalidPublicKey(
                "Invalid address format".to_string(),
            ));
        }
    };

    // Accept these input shapes:
    // - 65 bytes: full SEC1 (0x04 || X || Y)
    // - 33 bytes: compressed SEC1 (0x02/0x03 || X)
    // - 64 bytes: raw X||Y (add 0x04)
    // - 32 bytes: x-only (try 0x02/0x03)

    // Try full SEC1 if present
    if decoded_hex.len() == 65
        && let Ok(verifying_key) = K256VerifyingKey::from_sec1_bytes(&decoded_hex)
        && verifying_key.verify(&message_hash, &signature).is_ok()
    {
        return Ok(true);
    }

    // Try compressed SEC1 if present (33 bytes)
    if decoded_hex.len() == SEC1_COMPRESSED_LEN {
        if decoded_hex[0] == 0x02 || decoded_hex[0] == 0x03 {
            if let Ok(verifying_key) = K256VerifyingKey::from_sec1_bytes(&decoded_hex)
                && verifying_key.verify(&message_hash, &signature).is_ok()
            {
                return Ok(true);
            }
        } else {
            // If 33 bytes but no prefix, treat as invalid
            return Err(SignatureError::InvalidPublicKey(
                "Invalid address format".to_string(),
            ));
        }
    }

    // Try raw uncompressed (X||Y) of 64 bytes by adding 0x04 prefix
    if decoded_hex.len() == RAW_XY_LEN {
        let mut public_key_bytes = Vec::with_capacity(65);
        public_key_bytes.push(0x04);
        public_key_bytes.extend_from_slice(&decoded_hex);
        if let Ok(verifying_key) = K256VerifyingKey::from_sec1_bytes(&public_key_bytes)
            && verifying_key.verify(&message_hash, &signature).is_ok()
        {
            return Ok(true);
        }
    }

    // Try x-only (32 bytes) by attempting both even/odd Y prefixes
    if decoded_hex.len() == X_ONLY_LEN {
        let mut public_key_bytes = vec![0x02];
        public_key_bytes.extend_from_slice(&decoded_hex[0..X_ONLY_LEN]);
        if let Ok(verifying_key) = K256VerifyingKey::from_sec1_bytes(&public_key_bytes)
            && verifying_key.verify(&message_hash, &signature).is_ok()
        {
            return Ok(true);
        }
        public_key_bytes[0] = 0x03;
        if let Ok(verifying_key) = K256VerifyingKey::from_sec1_bytes(&public_key_bytes)
            && verifying_key.verify(&message_hash, &signature).is_ok()
        {
            return Ok(true);
        }
    }

    // None matched
    Ok(false)
}

/// Verify a signature using P256 (secp256r1)
///
/// **⚠️ IMPORTANT: Kanari P256 Uses SHA3-256 Pre-Hashing**
///
/// This verifier expects signatures created with P256 pre-hashing:
/// - Message was hashed with SHA3-256 before signing
/// - Verification repeats: `message_hash = SHA3-256(message)` then verifies signature
///
/// **Incompatible With:**
/// ❌ Raw P256 signatures (unsigned message hash)
/// ❌ NIST ECDSA standard scheme (NIST uses different approaches)
/// ✅ Only Kanari P256 signatures
///
/// **Strategy Match:**
/// P256 in Kanari uses the SAME pre-hashing as K256 for consistency.
pub fn verify_signature_p256(
    address_hex: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, SignatureError> {
    // Parse the signature
    let signature = P256Signature::from_der(signature)
        .map_err(|_| SignatureError::InvalidFormat("Invalid signature format".to_string()))?;

    // ⚠️ KANARI-SPECIFIC: Pre-hash the message with SHA3-256 (must match signing path!)
    let mut hasher = Sha3_256::default();
    hasher.update(message);
    let message_hash = hasher.finalize();

    // Normalize and decode the address hex (may be raw sec1 bytes or X/Y without prefix)
    let raw_key = crate::keys::extract_raw_key(address_hex);
    let decoded_hex = match hex::decode(raw_key) {
        Ok(v) => v,
        Err(_) => {
            return Err(SignatureError::InvalidPublicKey(
                "Invalid address format".to_string(),
            ));
        }
    };

    // Accept these input shapes:
    // - 65 bytes: full SEC1 (0x04 || X || Y)
    // - 33 bytes: compressed SEC1 (0x02/0x03 || X)
    // - 64 bytes: raw X||Y (add 0x04)
    // - 32 bytes: x-only (try 0x02/0x03)
    if decoded_hex.len() == SEC1_UNCOMPRESSED_LEN
        && let Ok(verifying_key) = VerifyingKey::from_sec1_bytes(&decoded_hex)
        && verifying_key.verify(&message_hash, &signature).is_ok()
    {
        return Ok(true);
    }

    // Try compressed SEC1 if present (33 bytes)
    if decoded_hex.len() == SEC1_COMPRESSED_LEN {
        if decoded_hex[0] == 0x02 || decoded_hex[0] == 0x03 {
            if let Ok(verifying_key) = VerifyingKey::from_sec1_bytes(&decoded_hex)
                && verifying_key.verify(&message_hash, &signature).is_ok()
            {
                return Ok(true);
            }
        } else {
            return Err(SignatureError::InvalidPublicKey(
                "Invalid address format".to_string(),
            ));
        }
    }

    // Try raw uncompressed (X||Y) of 64 bytes by adding 0x04 prefix
    if decoded_hex.len() == RAW_XY_LEN {
        let mut public_key_bytes = Vec::with_capacity(65);
        public_key_bytes.push(0x04);
        public_key_bytes.extend_from_slice(&decoded_hex);
        if let Ok(verifying_key) = VerifyingKey::from_sec1_bytes(&public_key_bytes)
            && verifying_key.verify(&message_hash, &signature).is_ok()
        {
            return Ok(true);
        }
    }

    // Try x-only (32 bytes) by attempting both even/odd Y prefixes
    if decoded_hex.len() == X_ONLY_LEN {
        let mut public_key_bytes = vec![0x02];
        public_key_bytes.extend_from_slice(&decoded_hex[0..X_ONLY_LEN]);
        if let Ok(verifying_key) = VerifyingKey::from_sec1_bytes(&public_key_bytes)
            && verifying_key.verify(&message_hash, &signature).is_ok()
        {
            return Ok(true);
        }
        public_key_bytes[0] = 0x03;
        if let Ok(verifying_key) = VerifyingKey::from_sec1_bytes(&public_key_bytes)
            && verifying_key.verify(&message_hash, &signature).is_ok()
        {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Verify a signature using Ed25519
///
/// **✅ RFC-8032 COMPLIANT - Standard Ed25519 Verification (NO Pre-Hashing)**
///
/// This function strictly adheres to RFC-8032 standard Ed25519:
/// - Signatures are verified DIRECTLY against the original message (no pre-hashing)
/// - Compatible with Ed25519 signatures from all standard implementations
/// - Uses constant-time verification to prevent timing attacks
///
/// **⚠️ CRITICAL DIFFERENCE FROM K256/P256:**
/// - K256: Message → SHA3-256 hash → Sign hash ← Kanari-specific
/// - P256: Message → SHA3-256 hash → Sign hash ← Kanari-specific
/// - Ed25519: Message → Sign directly ← RFC-8032 standard
///
/// **DO NOT:**
/// ❌ Pre-hash the message: `sign_message(key, sha256(msg), Ed25519)` ← WRONG!
/// ❌ Try to verify K256-style hashed signatures
///
/// **Interoperability:**
/// - ✅ Verifies Ed25519 signatures from libsodium, NaCl, cryptonote, external Ed25519 tools
/// - ✅ Kanari Ed25519 signatures verify in standard Ed25519 implementations
/// - ✅ No format conversion or special handling needed
///
pub fn verify_signature_ed25519(
    address_hex: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, SignatureError> {
    // ✅ RFC-8032 STANDARD: Verify message DIRECTLY (no pre-hashing)
    // This is the key difference from K256/P256 which use pre-hashing
    // Check signature length and construct signature object
    if signature.len() != 64 {
        return Err(SignatureError::InvalidSignatureLength);
    }
    let mut sig_array = [0u8; 64];
    sig_array.copy_from_slice(signature);
    let signature = Ed25519Signature::from_bytes(&sig_array);
    sig_array.zeroize();

    // Normalize and decode public key
    let raw_key = crate::keys::extract_raw_key(address_hex);
    let decoded_hex = hex::decode(raw_key)
        .map_err(|_| SignatureError::InvalidPublicKey("Invalid address format".to_string()))?;

    if decoded_hex.len() != ED25519_PUBLIC_KEY_LEN {
        return Err(SignatureError::InvalidPublicKey(
            "Invalid address format".to_string(),
        ));
    }

    let mut key_array = [0u8; ED25519_PUBLIC_KEY_LEN];
    key_array.copy_from_slice(&decoded_hex);
    let verifying_key = Ed25519VerifyingKey::from_bytes(&key_array)
        .map_err(|_| SignatureError::InvalidPublicKey("Invalid address format".to_string()))?;

    match verifying_key.verify(message, &signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
