// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use sha3::{Digest, Sha3_256};

use k256::{
    SecretKey as K256SecretKey,
    ecdsa::{
        Signature as K256Signature, SigningKey as K256SigningKey, VerifyingKey as K256VerifyingKey,
    },
};

use p256::ecdsa::signature::Verifier;

use ed25519_dalek::Signer;

use zeroize::Zeroize;

use crate::{
    SignatureError,
    signatures::{RAW_XY_LEN, SEC1_COMPRESSED_LEN, X_ONLY_LEN},
};

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
pub fn sign_message_k256(private_key_hex: &str, message: &[u8]) -> Result<Vec<u8>, SignatureError> {
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
