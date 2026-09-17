// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use kanari_crypto::keys::{
    CurveType, KeyPair, generate_keypair, keypair_from_mnemonic, keypair_from_private_key,
    keypair_from_seed,
};
use kanari_crypto::signatures::{sign_message, verify_signature_with_curve};
use kanari_crypto::{hash_data_blake3, hd_wallet, keys};
use serde::{Deserialize, Serialize};

uniffi::setup_scaffolding!();

/// Expose curve names safely for Kotlin/FFI
#[derive(Serialize, Deserialize, Debug, Clone, uniffi::Record)]
pub struct CurveInfo {
    pub name: String,
    pub is_post_quantum: bool,
    pub is_hybrid: bool,
    pub security_level: u8,
}

/// KeyPair data structure safe for FFI transfer
#[derive(Serialize, Deserialize, Debug, Clone, uniffi::Record)]
pub struct KeyPairData {
    pub private_key: String, // format "kanari...", "kanapqc...", "kanahybrid..."
    pub public_key: String,  // hex
    pub address: String,     // 0x...
    pub tagged_address: String, // Curve:0x...
    pub raw_public_key: Vec<u8>, // raw bytes of public key (for PQC verification)
    pub curve_type: String,  // e.g., "K256", "Ed25519Dilithium3"
}

/// Generate a keypair for the specified curve type
#[uniffi::export]
pub fn generate_keypair_api(curve_name: String) -> Result<KeyPairData, String> {
    let curve = parse_curve_type(&curve_name)
        .ok_or_else(|| format!("Unsupported curve type: {}", curve_name))?;

    let kp = generate_keypair(curve).map_err(|e| format!("Key generation failed: {}", e))?;

    Ok(to_keypair_data(&kp, curve))
}

/// Derive a keypair from a mnemonic (BIP39).
///
/// All curves are supported, including post-quantum and hybrid curves, which
/// derive domain-separated sub-seeds deterministically from the BIP39 seed.
#[uniffi::export]
pub fn derive_keypair_from_mnemonic(
    mnemonic: String,
    curve_name: String,
) -> Result<KeyPairData, String> {
    let curve = parse_curve_type(&curve_name)
        .ok_or_else(|| format!("Unsupported curve type: {}", curve_name))?;

    let kp = keypair_from_mnemonic(&mnemonic, curve)
        .map_err(|e| format!("Mnemonic derivation failed: {}", e))?;

    Ok(to_keypair_data(&kp, curve))
}

/// Derive a keypair deterministically from raw seed material.
///
/// `seed` must hold at least 64 bytes (e.g. a BIP39 seed). Classical curves
/// use the first 32 bytes; post-quantum and hybrid curves derive
/// domain-separated sub-seeds, so the same seed always reproduces the same
/// keypair for a given curve.
#[uniffi::export]
pub fn derive_keypair_from_seed_api(
    seed: Vec<u8>,
    curve_name: String,
) -> Result<KeyPairData, String> {
    let curve = parse_curve_type(&curve_name)
        .ok_or_else(|| format!("Unsupported curve type: {}", curve_name))?;

    let kp =
        keypair_from_seed(&seed, curve).map_err(|e| format!("Seed derivation failed: {}", e))?;

    Ok(to_keypair_data(&kp, curve))
}

/// Import a keypair from a provided private key
#[uniffi::export]
pub fn import_keypair_from_private_key(
    private_key: String,
    curve_name: String,
) -> Result<KeyPairData, String> {
    let curve = parse_curve_type(&curve_name)
        .ok_or_else(|| format!("Unsupported curve type: {}", curve_name))?;

    let kp = keypair_from_private_key(&private_key, curve)
        .map_err(|e| format!("Private key import failed: {}", e))?;

    Ok(to_keypair_data(&kp, curve))
}

/// Sign a message
#[uniffi::export]
pub fn sign_message_api(
    private_key: String,
    message: Vec<u8>,
    curve_name: String,
) -> Result<Vec<u8>, String> {
    let curve = parse_curve_type(&curve_name)
        .ok_or_else(|| format!("Unsupported curve type: {}", curve_name))?;

    sign_message(&private_key, &message, curve).map_err(|e| format!("Signing failed: {}", e))
}

/// Hash data using Blake3
#[uniffi::export]
pub fn blake3_hash_api(data: Vec<u8>) -> Vec<u8> {
    hash_data_blake3(&data)
}

/// Verify a signature
#[uniffi::export]
pub fn verify_signature_api(
    address: String,
    message: Vec<u8>,
    signature: Vec<u8>,
    curve_name: String,
) -> Result<bool, String> {
    let curve = parse_curve_type(&curve_name)
        .ok_or_else(|| format!("Unsupported curve type: {}", curve_name))?;

    verify_signature_with_curve(&address, &message, &signature, curve)
        .map_err(|e| format!("Verification failed: {}", e))
}

/// Generate a random mnemonic
#[uniffi::export]
pub fn generate_mnemonic_api(word_count: u32) -> Result<String, String> {
    if word_count != 12 && word_count != 24 {
        return Err("Only 12 or 24-word mnemonics are supported".to_string());
    }

    keys::generate_mnemonic(word_count as usize)
        .map_err(|e| format!("Mnemonic generation failed: {}", e))
}

/// Derive a keypair from a mnemonic at a specific derivation path
#[uniffi::export]
pub fn derive_keypair_from_path_api(
    mnemonic: String,
    derivation_path: String,
    curve_name: String,
) -> Result<KeyPairData, String> {
    let curve = parse_curve_type(&curve_name)
        .ok_or_else(|| format!("Unsupported curve type: {}", curve_name))?;

    let kp = hd_wallet::derive_keypair_from_path(&mnemonic, "", &derivation_path, curve)
        .map_err(|e| format!("Path derivation failed: {}", e))?;

    Ok(to_keypair_data(&kp, curve))
}

/// Derive multiple addresses from a mnemonic using a path template
#[uniffi::export]
pub fn derive_multiple_addresses_api(
    mnemonic: String,
    path_template: String,
    curve_name: String,
    count: u32,
) -> Result<Vec<KeyPairData>, String> {
    let curve = parse_curve_type(&curve_name)
        .ok_or_else(|| format!("Unsupported curve type: {}", curve_name))?;

    let keypairs =
        hd_wallet::derive_multiple_addresses(&mnemonic, "", &path_template, curve, count as usize)
            .map_err(|e| format!("HD derivation failed: {}", e))?;

    Ok(keypairs
        .into_iter()
        .map(|kp| to_keypair_data(&kp, curve))
        .collect())
}

/// ---- zkLogin (Google OIDC bound to an ephemeral key) ----
///
/// Thin FFI over `kanari_crypto::signatures::zklogin`: nonce binding, JWT
/// verification against a provider JWKS, and v2 address derivation. The
/// OAuth browser dance stays in Kotlin; Rust owns the crypto vectors so
/// both sides can never drift.
/// Verified JWT claims returned to Kotlin.
#[derive(Serialize, Deserialize, Debug, Clone, uniffi::Record)]
pub struct ZkLoginClaimsData {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub exp: Option<u64>,
    pub nonce: Option<String>,
}

/// Fresh login material: ephemeral pubkey + randomness + salt + nonce.
#[derive(Serialize, Deserialize, Debug, Clone, uniffi::Record)]
pub struct ZkLoginNonceData {
    pub ephemeral_pubkey: Vec<u8>,
    pub ephemeral_secret: Vec<u8>,
    pub randomness: Vec<u8>,
    pub salt: Vec<u8>,
    pub max_epoch: u64,
    pub nonce: String,
}

/// Generate ephemeral keypair + randomness + salt and bind them into a nonce.
#[uniffi::export]
pub fn zklogin_prepare_nonce(max_epoch: u64) -> Result<ZkLoginNonceData, String> {
    use kanari_crypto::signatures::zklogin::{
        EphemeralKeypair, compute_nonce, generate_randomness, generate_salt,
    };
    let ephemeral =
        EphemeralKeypair::generate().map_err(|e| format!("ephemeral key failed: {e:?}"))?;
    let randomness = generate_randomness().map_err(|e| format!("randomness failed: {e:?}"))?;
    let salt = generate_salt().map_err(|e| format!("salt failed: {e:?}"))?;
    let pubkey = ephemeral.public_bytes();
    let secret = ephemeral.secret_bytes();
    let nonce = compute_nonce(&pubkey, max_epoch, &randomness);
    Ok(ZkLoginNonceData {
        ephemeral_pubkey: pubkey.to_vec(),
        ephemeral_secret: secret.to_vec(),
        randomness: randomness.to_vec(),
        salt: salt.to_vec(),
        max_epoch,
        nonce,
    })
}

/// Verify an id_token against a provider JWKS (RS256 + iss/aud/exp + nonce).
#[uniffi::export]
pub fn zklogin_verify_jwt(
    jwt: String,
    jwks_json: String,
    expected_iss: String,
    expected_aud: String,
    expected_nonce: Option<String>,
    now_secs: u64,
) -> Result<ZkLoginClaimsData, String> {
    use kanari_crypto::signatures::zklogin::verify_jwt_with_jwks;
    let jwks: kanari_crypto::signatures::zklogin::JwksDocument =
        serde_json::from_str(&jwks_json).map_err(|e| format!("bad JWKS JSON: {e}"))?;
    let claims = verify_jwt_with_jwks(
        &jwt,
        &jwks,
        &expected_iss,
        &expected_aud,
        expected_nonce.as_deref(),
        now_secs,
    )
    .map_err(|e| format!("JWT verification failed: {e:?}"))?;
    let aud = claims.aud.as_vec().first().cloned().unwrap_or_default();
    Ok(ZkLoginClaimsData {
        iss: claims.iss,
        aud,
        sub: claims.sub,
        exp: claims.exp,
        nonce: claims.nonce,
    })
}

/// Derive the canonical v2 zkLogin address (matches chain + Android).
#[uniffi::export]
pub fn zklogin_derive_address(
    iss: String,
    aud: String,
    sub: String,
    salt: Vec<u8>,
) -> Result<String, String> {
    use kanari_crypto::signatures::zklogin::derive_zklogin_address_v2;
    derive_zklogin_address_v2(&iss, &aud, &sub, &salt)
        .map_err(|e| format!("address derivation failed: {e:?}"))
}

/// Build the opaque `ZkLogin:` transaction signature bundle (v1 JSON).
/// Returns the exact bytes `encode_zklogin_tx_signature` produces so Kotlin
/// just does `sign(hash)` + this call — no manual JSON.
#[uniffi::export]
pub fn zklogin_build_bundle(
    jwt: String,
    jwks_json: String,
    iss: String,
    aud: String,
    salt: Vec<u8>,
    randomness: Vec<u8>,
    ephemeral_pubkey: Vec<u8>,
    ephemeral_sig: Vec<u8>,
    max_epoch: u64,
) -> Result<Vec<u8>, String> {
    use kanari_crypto::signatures::zk_authenticator::{
        JwtAuth, ZkAuthKind, ZkLoginAuthenticator, encode_zklogin_tx_signature,
    };
    use kanari_crypto::signatures::zklogin::JwksDocument;
    let jwks: JwksDocument =
        serde_json::from_str(&jwks_json).map_err(|e| format!("bad JWKS JSON: {e}"))?;
    let salt_arr: [u8; 32] = salt
        .try_into()
        .map_err(|_| "salt must be 32 bytes".to_string())?;
    let rand_arr: [u8; 32] = randomness
        .try_into()
        .map_err(|_| "randomness must be 32 bytes".to_string())?;
    let pub_arr: [u8; 32] = ephemeral_pubkey
        .try_into()
        .map_err(|_| "ephemeral pubkey must be 32 bytes".to_string())?;
    let sig_arr: [u8; 64] = ephemeral_sig
        .try_into()
        .map_err(|_| "ephemeral sig must be 64 bytes".to_string())?;
    let auth = ZkLoginAuthenticator {
        ephemeral_pubkey: pub_arr,
        ephemeral_sig: sig_arr,
        max_epoch,
        kind: ZkAuthKind::Jwt(JwtAuth {
            jwt,
            jwks,
            iss,
            aud,
            randomness: rand_arr,
            salt: salt_arr,
        }),
    };
    encode_zklogin_tx_signature(&auth).map_err(|e| format!("bundle encode failed: {e:?}"))
}

/// Sign bytes with an ephemeral secret (32 raw bytes from prepare).
#[uniffi::export]
pub fn zklogin_sign_ephemeral(secret: Vec<u8>, message: Vec<u8>) -> Result<Vec<u8>, String> {
    use kanari_crypto::signatures::zklogin::EphemeralKeypair;
    let raw: [u8; 32] = secret
        .try_into()
        .map_err(|_| "ephemeral secret must be 32 bytes".to_string())?;
    let kp = EphemeralKeypair::from_secret(raw).map_err(|e| format!("bad secret: {e:?}"))?;
    Ok(kp.sign(&message))
}

/// Verify an ephemeral Ed25519 signature.
#[uniffi::export]
pub fn zklogin_verify_ephemeral(
    pubkey: Vec<u8>,
    message: Vec<u8>,
    signature: Vec<u8>,
) -> Result<bool, String> {
    use kanari_crypto::signatures::zklogin::EphemeralKeypair;
    match EphemeralKeypair::verify(&pubkey, &message, &signature) {
        Ok(()) => Ok(true),
        Err(kanari_crypto::signatures::SignatureError::VerificationFailed) => Ok(false),
        Err(e) => Err(format!("malformed ephemeral input: {e:?}")),
    }
}

/// List all supported curves
#[uniffi::export]
pub fn list_supported_curves() -> Vec<CurveInfo> {
    use CurveType::*;
    let curves = [
        K256,
        P256,
        Ed25519,
        Dilithium2,
        Dilithium3,
        Dilithium5,
        SphincsPlusSha256Robust,
        Falcon512,
        Falcon1024,
        Ed25519Dilithium3,
        K256Dilithium3,
    ];

    curves
        .iter()
        .map(|&c| CurveInfo {
            name: format!("{:?}", c),
            is_post_quantum: c.is_post_quantum(),
            is_hybrid: c.is_hybrid(),
            security_level: c.security_level(),
        })
        .collect()
}

// --- Helper Internals ---
fn to_keypair_data(kp: &KeyPair, curve: CurveType) -> KeyPairData {
    let public_key = kp.public_key.clone();
    let raw_public_key = hex::decode(
        kp.pqc_public_key
            .clone()
            .unwrap_or_else(|| public_key.clone()),
    )
    .unwrap_or_default();

    KeyPairData {
        private_key: kp.private_key.to_string(),
        public_key,
        address: kp.address.clone(),
        tagged_address: kp.tagged_address(),
        raw_public_key,
        curve_type: format!("{:?}", curve),
    }
}

fn parse_curve_type(name: &str) -> Option<CurveType> {
    match name {
        "K256" => Some(CurveType::K256),
        "P256" => Some(CurveType::P256),
        "Ed25519" => Some(CurveType::Ed25519),
        "Dilithium2" => Some(CurveType::Dilithium2),
        "Dilithium3" => Some(CurveType::Dilithium3),
        "Dilithium5" => Some(CurveType::Dilithium5),
        "SphincsPlusSha256Robust" => Some(CurveType::SphincsPlusSha256Robust),
        "Falcon512" | "FnDsa512" | "FN-DSA-512" => Some(CurveType::Falcon512),
        "Falcon1024" | "FnDsa1024" | "FN-DSA-1024" => Some(CurveType::Falcon1024),
        "Ed25519Dilithium3" => Some(CurveType::Ed25519Dilithium3),
        "K256Dilithium3" => Some(CurveType::K256Dilithium3),
        _ => None,
    }
}

#[cfg(test)]
mod zklogin_ffi_tests {
    use super::*;

    /// Cross-language vectors shared with the Android suite and Move tests.
    /// If any of these change, every consumer must update in lockstep.
    const NONCE_FIXTURE_PUBKEY: &str =
        "197f6b23e16c8532c6abc838facd5ea789be0c76b2920334039bfa8b3d368d61";
    const NONCE_FIXTURE_RAND: &str =
        "0707070707070707070707070707070707070707070707070707070707070707";
    const NONCE_FIXTURE_EXPECTED: &str =
        "1bb486c02deef0a4accf5f7c4ca2e7e54ce18cb13b7fa4e193398ebe1b8580fb";
    const ADDR_FIXTURE_EXPECTED: &str =
        "0x3bc27fa23ddd2177cb1914572052272ce060182745a576019095c05a70971181";

    #[test]
    fn ffi_vectors_match_chain() {
        use kanari_crypto::signatures::zklogin::compute_nonce;
        let pubkey = hex::decode(NONCE_FIXTURE_PUBKEY).unwrap();
        let rand = hex::decode(NONCE_FIXTURE_RAND).unwrap();
        assert_eq!(compute_nonce(&pubkey, 1000, &rand), NONCE_FIXTURE_EXPECTED);
        let salt = hex::decode("0909090909090909090909090909090909090909090909090909090909090909")
            .unwrap();
        let addr = zklogin_derive_address(
            "https://accounts.google.com".into(),
            "kanari-test-client".into(),
            "1234".into(),
            salt,
        )
        .unwrap();
        assert_eq!(addr, ADDR_FIXTURE_EXPECTED);
    }

    #[test]
    fn ffi_prepare_sign_verify_roundtrip() {
        let prep = zklogin_prepare_nonce(1000).unwrap();
        assert_eq!(prep.ephemeral_pubkey.len(), 32);
        assert_eq!(prep.ephemeral_secret.len(), 32);
        assert_eq!(prep.randomness.len(), 32);
        assert_eq!(prep.salt.len(), 32);
        // Nonce re-derives from its own parts.
        use kanari_crypto::signatures::zklogin::compute_nonce;
        assert_eq!(
            compute_nonce(&prep.ephemeral_pubkey, 1000, &prep.randomness),
            prep.nonce
        );
        let msg = b"kanari kotlin ffi";
        let sig = zklogin_sign_ephemeral(prep.ephemeral_secret.clone(), msg.to_vec()).unwrap();
        assert_eq!(sig.len(), 64);
        assert!(zklogin_verify_ephemeral(prep.ephemeral_pubkey, msg.to_vec(), sig).unwrap());
        // Tampered message fails (false, not error).
        let sig2 = zklogin_sign_ephemeral(prep.ephemeral_secret, msg.to_vec()).unwrap();
        assert!(
            !zklogin_verify_ephemeral(
                zklogin_prepare_nonce(1000).unwrap().ephemeral_pubkey,
                msg.to_vec(),
                sig2
            )
            .unwrap_or(true)
        );
    }

    #[test]
    fn ffi_rejects_bad_inputs() {
        assert!(zklogin_derive_address("".into(), "a".into(), "b".into(), vec![0u8; 32]).is_err());
        assert!(zklogin_derive_address("i".into(), "a".into(), "b".into(), vec![0u8; 16]).is_err());
        assert!(zklogin_sign_ephemeral(vec![0u8; 16], b"m".to_vec()).is_err());
        assert!(
            zklogin_verify_jwt("bad".into(), "{}".into(), "i".into(), "a".into(), None, 0).is_err()
        );
    }
}
