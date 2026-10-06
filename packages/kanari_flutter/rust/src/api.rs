// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use kanari_crypto::keys::{
    CurveType, KeyPair, generate_keypair, keypair_from_mnemonic, keypair_from_private_key,
    keypair_from_seed,
};
use kanari_crypto::signatures::{sign_message, verify_signature_with_curve};
use kanari_crypto::{hash_data_blake3, hd_wallet, keys};
use serde::{Deserialize, Serialize};

/// Expose curve names safely for Dart
#[derive(Serialize, Deserialize, Debug)]
pub struct CurveInfo {
    pub name: String,
    pub is_post_quantum: bool,
    pub is_hybrid: bool,
    pub security_level: u8,
}

/// KeyPair data structure safe for FFI transfer
#[derive(Serialize, Deserialize, Debug)]
pub struct KeyPairData {
    pub private_key: String, // format "kanari...", "kanapqc...", "kanahybrid..."
    pub public_key: String,  // hex
    pub address: String,     // 0x...
    pub tagged_address: String, // Curve:0x...
    pub raw_public_key: Vec<u8>, // raw bytes of public key (for PQC verification)
    pub curve_type: String,  // e.g., "K256", "Ed25519Dilithium3"
}

/// Generate a keypair for the specified curve type
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
pub fn blake3_hash_api(data: Vec<u8>) -> Vec<u8> {
    hash_data_blake3(&data)
}

/// Verify a signature
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
pub fn generate_mnemonic_api(word_count: usize) -> Result<String, String> {
    if word_count != 12 && word_count != 24 {
        return Err("Only 12 or 24-word mnemonics are supported".to_string());
    }

    keys::generate_mnemonic(word_count).map_err(|e| format!("Mnemonic generation failed: {}", e))
}

/// Derive a keypair from a mnemonic at a specific derivation path
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
pub fn derive_multiple_addresses_api(
    mnemonic: String,
    path_template: String,
    curve_name: String,
    count: usize,
) -> Result<Vec<KeyPairData>, String> {
    let curve = parse_curve_type(&curve_name)
        .ok_or_else(|| format!("Unsupported curve type: {}", curve_name))?;

    let keypairs =
        hd_wallet::derive_multiple_addresses(&mnemonic, "", &path_template, curve, count)
            .map_err(|e| format!("HD derivation failed: {}", e))?;

    Ok(keypairs
        .into_iter()
        .map(|kp| to_keypair_data(&kp, curve))
        .collect())
}

/// zkLogin nonce material: ephemeral pubkey + randomness + nonce.
///
/// NOTE: no salt here - the address-salt is the kanari-crypto standard
/// (`deterministic_salt`, exposed as `zklogin_deterministic_salt`), derived
/// after the JWT is known. A random salt per prepare would rotate the wallet
/// address every login, so prepare must not mint one.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkLoginNonceData {
    pub ephemeral_pubkey: Vec<u8>,
    pub ephemeral_secret: Vec<u8>,
    pub randomness: Vec<u8>,
    pub max_epoch: u64,
    pub nonce: String,
}

/// Verified JWT claims needed for zkLogin.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkLoginClaimsData {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub exp: Option<u64>,
    pub nonce: Option<String>,
}

/// Generate ephemeral keypair + randomness and bind them into a nonce.
pub fn zklogin_prepare_nonce(max_epoch: u64) -> Result<ZkLoginNonceData, String> {
    use kanari_crypto::signatures::zklogin::{
        EphemeralKeypair, compute_nonce, generate_randomness,
    };
    let ephemeral =
        EphemeralKeypair::generate().map_err(|e| format!("ephemeral key failed: {e:?}"))?;
    let randomness = generate_randomness().map_err(|e| format!("randomness failed: {e:?}"))?;
    let pubkey = ephemeral.public_bytes();
    let secret = ephemeral.secret_bytes();
    let nonce = compute_nonce(&pubkey, max_epoch, &randomness);
    Ok(ZkLoginNonceData {
        ephemeral_pubkey: pubkey.to_vec(),
        ephemeral_secret: secret.to_vec(),
        randomness: randomness.to_vec(),
        max_epoch,
        nonce,
    })
}

/// Verify an id_token against a provider JWKS (RS256 + iss/aud/exp + nonce).
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

/// Canonical zkLogin address-salt (THE primary salt, shared by the CLI and
/// the Android app): deterministic per (iss, aud, sub), stable across
/// reinstalls and devices.
pub fn zklogin_deterministic_salt(
    iss: String,
    aud: String,
    sub: String,
) -> Result<Vec<u8>, String> {
    use kanari_crypto::signatures::zklogin::deterministic_salt;
    deterministic_salt(&iss, &aud, &sub)
        .map(|s| s.to_vec())
        .map_err(|e| format!("salt derivation failed: {e:?}"))
}

/// Build the opaque `ZkLogin:` transaction signature bundle (v1 JSON).
/// Returns the exact bytes `encode_zklogin_tx_signature` produces.
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
pub fn zklogin_sign_ephemeral(secret: Vec<u8>, message: Vec<u8>) -> Result<Vec<u8>, String> {
    use kanari_crypto::signatures::zklogin::EphemeralKeypair;
    let raw: [u8; 32] = secret
        .try_into()
        .map_err(|_| "ephemeral secret must be 32 bytes".to_string())?;
    let kp = EphemeralKeypair::from_secret(raw).map_err(|e| format!("bad secret: {e:?}"))?;
    Ok(kp.sign(&message))
}

/// Verify an ephemeral Ed25519 signature.
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
