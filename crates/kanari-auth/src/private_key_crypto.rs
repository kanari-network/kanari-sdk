use aes_gcm::{
    Aes256Gcm, KeyInit,
    aead::{Aead, AeadCore, OsRng},
};
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{AuthError, AuthResult};

const KDF_ITERATIONS: u32 = 120_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPrivateKeyPayload {
    pub ciphertext: String,
    pub nonce: String,
    pub salt: String,
    pub iterations: u32,
}

pub fn encrypt_private_key(private_key: &str, password: &str) -> AuthResult<String> {
    validate_password(password)?;

    let salt = random_bytes(16);
    let key = derive_key(password.as_bytes(), &salt, KDF_ITERATIONS);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| AuthError::CryptoError(format!("Failed to create cipher: {e}")))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, private_key.as_bytes())
        .map_err(|e| AuthError::CryptoError(format!("Private key encryption failed: {e}")))?;

    let payload = EncryptedPrivateKeyPayload {
        ciphertext: general_purpose::STANDARD.encode(ciphertext),
        nonce: general_purpose::STANDARD.encode(nonce),
        salt: general_purpose::STANDARD.encode(salt),
        iterations: KDF_ITERATIONS,
    };

    serde_json::to_string(&payload)
        .map_err(|e| AuthError::SerializationError(format!("Encryption payload serialize failed: {e}")))
}

pub fn decrypt_private_key(encrypted_payload: &str, password: &str) -> AuthResult<String> {
    validate_password(password)?;

    let payload: EncryptedPrivateKeyPayload = serde_json::from_str(encrypted_payload)
        .map_err(|e| AuthError::SerializationError(format!("Invalid encrypted key payload: {e}")))?;

    let ciphertext = general_purpose::STANDARD
        .decode(payload.ciphertext)
        .map_err(|e| AuthError::CryptoError(format!("Invalid ciphertext encoding: {e}")))?;
    let nonce = general_purpose::STANDARD
        .decode(payload.nonce)
        .map_err(|e| AuthError::CryptoError(format!("Invalid nonce encoding: {e}")))?;
    let salt = general_purpose::STANDARD
        .decode(payload.salt)
        .map_err(|e| AuthError::CryptoError(format!("Invalid salt encoding: {e}")))?;

    if nonce.len() != 12 {
        return Err(AuthError::CryptoError("Invalid nonce length".to_string()));
    }

    let key = derive_key(password.as_bytes(), &salt, payload.iterations);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| AuthError::CryptoError(format!("Failed to create cipher: {e}")))?;

    let decrypted = cipher
        .decrypt(aes_gcm::Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| AuthError::AuthenticationFailed)?;

    String::from_utf8(decrypted)
        .map_err(|e| AuthError::SerializationError(format!("Invalid UTF-8 private key: {e}")))
}

fn derive_key(password: &[u8], salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(password);
    hasher.update(salt);
    let mut block = hasher.finalize().to_vec();

    for _ in 1..iterations {
        let mut round = Sha256::new();
        round.update(&block);
        round.update(password);
        round.update(salt);
        block = round.finalize().to_vec();
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&block[..32]);
    key
}

fn random_bytes(len: usize) -> Vec<u8> {
    use aes_gcm::aead::rand_core::RngCore;

    let mut bytes = vec![0u8; len];
    OsRng.fill_bytes(&mut bytes);
    bytes
}

fn validate_password(password: &str) -> AuthResult<()> {
    if password.is_empty() {
        return Err(AuthError::InvalidPassword("Password cannot be empty".to_string()));
    }
    Ok(())
}
