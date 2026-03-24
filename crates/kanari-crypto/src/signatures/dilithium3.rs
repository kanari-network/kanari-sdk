// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use pqcrypto_dilithium::dilithium3;
use pqcrypto_traits::sign::SecretKey as PqcSecretKeyTrait;
use pqcrypto_traits::sign::{
    DetachedSignature as PqcDetachedTrait, PublicKey as PqcPublicKeyTrait,
};
use zeroize::Zeroizing;

use crate::SignatureError;

/// Verify a signature using Dilithium3 (PQC)
///
/// **✅ NIST STANDARD COMPLIANT - Direct Signing (NO Pre-Hashing)**
///
/// This function strictly adheres to Dilithium3 standard:
/// - Signatures are verified DIRECTLY against the original message (no pre-hashing)
/// - Compatible with Dilithium3 signatures from NIST-standard implementations
/// - Uses constant-time verification where possible
///
/// **Public Key Format:**
/// - Expected: 1952-byte public key in hex format
/// - Address derivation: SHA3-256 of public key bytes
pub fn verify_signature_dilithium3(
    address_hex: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, SignatureError> {
    // Strip known prefixes (e.g., "kanapqc") then decode
    let pqc_pub_raw = crate::keys::extract_raw_key(address_hex);
    let pub_bytes = hex::decode(pqc_pub_raw)
        .map_err(|_| SignatureError::InvalidPublicKey("Invalid public key hex".to_string()))?;
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
    match dilithium3::verify_detached_signature(&sig_obj, message, &pk) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Sign a message using Dilithium3 private key (PQC)
pub fn sign_message_dilithium3(
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
