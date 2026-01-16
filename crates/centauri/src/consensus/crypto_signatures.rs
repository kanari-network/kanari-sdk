use anyhow::Result;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

// Re-export curve25519-dalek types for VRF usage
pub use curve25519_dalek::constants::RISTRETTO_BASEPOINT_TABLE;
pub use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
pub use curve25519_dalek::scalar::Scalar;

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
        let mut csprng = OsRng {};
        let signing_key = SigningKey::generate(&mut csprng);
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
        if sig_bytes.len() != 64 {
            return Err(anyhow::anyhow!(
                "invalid signature length: {}",
                sig_bytes.len()
            ));
        }
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&sig_bytes[0..64]);
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
