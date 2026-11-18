// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Key Encapsulation Mechanism (KEM) support.
//!
//! Provides a KEM built on X25519 key agreement (via `aws-lc-rs`).

use aws_lc_rs::agreement::{self, PrivateKey, UnparsedPublicKey};
use rand::Rng;

pub struct KemKeyPair {
    pub private_key: Vec<u8>, // 32 raw bytes
    pub public_key: Vec<u8>,  // 32 raw bytes
}

pub struct EncapsulationResult {
    pub ciphertext: Vec<u8>, // ephemeral public key (32 bytes)
    pub shared_secret: Vec<u8>,
}

/// Generate an X25519 KEM keypair.
pub fn generate_keypair() -> Result<KemKeyPair, String> {
    let mut private_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut private_bytes);

    let private_key = PrivateKey::from_private_key(&agreement::X25519, &private_bytes)
        .map_err(|e| format!("invalid private key: {e}"))?;
    let public_key = private_key
        .compute_public_key()
        .map_err(|e| format!("public key computation failed: {e}"))?;

    Ok(KemKeyPair { private_key: private_bytes.to_vec(), public_key: public_key.as_ref().to_vec() })
}

/// Encapsulate a shared secret to a recipient's public key.
pub fn encapsulate(recipient_public_key: &[u8]) -> Result<EncapsulationResult, String> {
    let mut ephemeral_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut ephemeral_bytes);

    let ephemeral_private = PrivateKey::from_private_key(&agreement::X25519, &ephemeral_bytes)
        .map_err(|e| format!("ephemeral key generation failed: {e}"))?;
    let ephemeral_public = ephemeral_private
        .compute_public_key()
        .map_err(|e| format!("ephemeral public key failed: {e}"))?;

    let peer_key = UnparsedPublicKey::new(&agreement::X25519, recipient_public_key);
    let shared_secret =
        agreement::agree(&ephemeral_private, peer_key, (), |secret| Ok(secret.to_vec()))
            .map_err(|_| "key agreement failed".to_string())?;

    Ok(EncapsulationResult { ciphertext: ephemeral_public.as_ref().to_vec(), shared_secret })
}

/// Decapsulate a shared secret using the recipient's private key and the ciphertext.
pub fn decapsulate(private_key: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    let private_key = PrivateKey::from_private_key(&agreement::X25519, private_key)
        .map_err(|e| format!("invalid private key: {e}"))?;
    let peer_key = UnparsedPublicKey::new(&agreement::X25519, ciphertext);
    let shared_secret = agreement::agree(&private_key, peer_key, (), |secret| Ok(secret.to_vec()))
        .map_err(|_| "decapsulation failed".to_string())?;
    Ok(shared_secret)
}
