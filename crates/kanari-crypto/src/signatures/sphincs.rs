use pqcrypto_sphincsplus::sphincssha2256fsimple;

use pqcrypto_traits::sign::SecretKey as PqcSecretKeyTrait;
use pqcrypto_traits::sign::{
    DetachedSignature as PqcDetachedTrait, PublicKey as PqcPublicKeyTrait,
};
use zeroize::Zeroizing;

use crate::SignatureError;

/// Verify a signature using SPHINCS+ (PQC)
///
/// **✅ NIST STANDARD COMPLIANT - Direct Signing (NO Pre-Hashing)**
///
/// This function strictly adheres to SPHINCS+ standard:
/// - Signatures are verified DIRECTLY against the original message (no pre-hashing)
/// - Compatible with SPHINCS+ signatures from NIST-standard implementations
/// - Uses constant-time verification where possible
///
/// **⚠️ CRITICAL DIFFERENCE FROM K256/P256:**
/// - K256: Message → SHA3-256 hash → Sign hash ← Kanari-specific
/// - P256: Message → SHA3-256 hash → Sign hash ← Kanari-specific  
/// - SPHINCS+: Message → Sign directly ← NIST standard
///
/// **Public Key Format:**
/// - Expected: 64-byte public key in hex format
/// - Address derivation: SHA3-256 of public key bytes
pub fn verify_signature_sphincs(
    address_hex: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, SignatureError> {
    // Strip known prefixes (e.g., "kanapqc") then decode
    let pqc_pub_raw = crate::keys::extract_raw_key(address_hex);
    let pub_bytes = hex::decode(pqc_pub_raw)
        .map_err(|_| SignatureError::InvalidPublicKey("Invalid public key hex".to_string()))?;
    // Validate key length (SPHINCS+ public key is 64 bytes)
    if pub_bytes.len() != 64 {
        return Err(SignatureError::InvalidPublicKey(
            "Invalid SPHINCS+ public key".to_string(),
        ));
    }
    let pk = sphincssha2256fsimple::PublicKey::from_bytes(&pub_bytes)
        .map_err(|_| SignatureError::InvalidPublicKey("Invalid SPHINCS+ public key".to_string()))?;
    let sig_obj =
        sphincssha2256fsimple::DetachedSignature::from_bytes(signature).map_err(|_| {
            SignatureError::InvalidFormat("Invalid signature bytes for SPHINCS+".to_string())
        })?;
    match sphincssha2256fsimple::verify_detached_signature(&sig_obj, message, &pk) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Sign a message using SPHINCS+ private key (PQC)
pub fn sign_message_sphincs(
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
    let sk = sphincssha2256fsimple::SecretKey::from_bytes(&sk_bytes).map_err(|_| {
        SignatureError::InvalidPrivateKey("Invalid SPHINCS+ private key".to_string())
    })?;
    let sig = sphincssha2256fsimple::detached_sign(message, &sk);
    Ok(sig.as_bytes().to_vec())
}
