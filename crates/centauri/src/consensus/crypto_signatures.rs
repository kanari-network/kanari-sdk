// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::TryRng;
use rand::rngs::SysRng;
use serde::{Deserialize, Serialize};

const ED25519_SIGNATURE_LEN: usize = 64;

/// Ed25519 signature wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureScheme(pub Vec<u8>);

/// Ed25519 keypair wrapper
pub struct Ed25519Keypair {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

impl Ed25519Keypair {
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        SysRng
            .try_fill_bytes(&mut bytes)
            .expect("Failed to get OS randomness");

        let signing_key = SigningKey::from_bytes(&bytes);
        let verifying_key = signing_key.verifying_key();

        Self {
            signing_key,
            verifying_key,
        }
    }

    pub fn public(&self) -> VerifyingKey {
        self.verifying_key
    }

    pub fn sign(&self, msg: &[u8]) -> Vec<u8> {
        let sig: Signature = self.signing_key.sign(msg);
        sig.to_bytes().to_vec()
    }

    pub fn verify(pubkey: &VerifyingKey, msg: &[u8], sig_bytes: &[u8]) -> Result<()> {
        if sig_bytes.len() != ED25519_SIGNATURE_LEN {
            return Err(anyhow::anyhow!(
                "invalid signature length: {}",
                sig_bytes.len()
            ));
        }
        let mut sig_arr = [0u8; ED25519_SIGNATURE_LEN];
        sig_arr.copy_from_slice(sig_bytes);
        let sig = Signature::from_bytes(&sig_arr);
        pubkey.verify(msg, &sig).map_err(|e| anyhow::anyhow!(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_ed25519_sign_verify() -> Result<()> {
        let kp = Ed25519Keypair::generate();
        let pk = kp.public();
        let msg = b"hello kanari";
        let sig = kp.sign(msg);
        Ed25519Keypair::verify(&pk, msg, &sig)?;
        Ok(())
    }
}
