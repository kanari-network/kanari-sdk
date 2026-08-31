// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use sha3::{Digest, Sha3_256};

use p256::{
    SecretKey as P256SecretKey,
    ecdsa::{Signature as P256Signature, SigningKey, VerifyingKey, signature::Verifier},
};

use ed25519_dalek::Signer;

use zeroize::Zeroize;

use crate::{
    SignatureError,
    signatures::{RAW_XY_LEN, SEC1_COMPRESSED_LEN, SEC1_UNCOMPRESSED_LEN, X_ONLY_LEN},
};

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

    if bool::from(p256::elliptic_curve::scalar::IsHigh::is_high(
        &signature.s(),
    )) {
        return Err(SignatureError::InvalidFormat(
            "High S value - malleable signature".to_string(),
        ));
    }

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
pub fn sign_message_p256(private_key_hex: &str, message: &[u8]) -> Result<Vec<u8>, SignatureError> {
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

    // Sign and enforce low-S
    let mut signature: P256Signature = signing_key.sign(&message_hash);
    if bool::from(p256::elliptic_curve::scalar::IsHigh::is_high(
        &signature.s(),
    )) {
        let r_bytes = signature.r().to_bytes();
        let s_low = (-signature.s()).to_bytes();
        signature = P256Signature::from_scalars(r_bytes, s_low)
            .map_err(|_| SignatureError::InvalidFormat("Failed to normalize S".to_string()))?;
    }

    // Convert DER signature to bytes correctly
    let der_bytes = signature.to_der();
    Ok(der_bytes.as_bytes().to_vec())
}
