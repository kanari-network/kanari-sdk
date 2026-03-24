// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// PQC crates
use pqcrypto_dilithium::dilithium5;

use pqcrypto_traits::sign::SecretKey as PqcSecretKeyTrait;
use pqcrypto_traits::sign::{
    DetachedSignature as PqcDetachedTrait, PublicKey as PqcPublicKeyTrait,
};
use zeroize::Zeroizing;

use crate::SignatureError;

/// Verify a signature using Dilithium5 (PQC)
///
/// **✅ NIST STANDARD COMPLIANT - Direct Signing (NO Pre-Hashing)**
///
/// This function strictly adheres to Dilithium5 standard:
/// - Signatures are verified DIRECTLY against the original message (no pre-hashing)
/// - Compatible with Dilithium5 signatures from NIST-standard implementations
/// - Uses constant-time verification where possible
///
/// **Public Key Format:**
/// - Expected: 2592-byte public key in hex format
/// - Address derivation: SHA3-256 of public key bytes
pub fn verify_signature_dilithium5(
    address_hex: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, SignatureError> {
    // Strip known prefixes (e.g., "kanapqc") then decode
    let pqc_pub_raw = crate::keys::extract_raw_key(address_hex);
    let pub_bytes = hex::decode(pqc_pub_raw)
        .map_err(|_| SignatureError::InvalidPublicKey("Invalid public key hex".to_string()))?;
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
    match dilithium5::verify_detached_signature(&sig_obj, message, &pk) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Sign a message using Dilithium5 private key (PQC)
pub fn sign_message_dilithium5(
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
