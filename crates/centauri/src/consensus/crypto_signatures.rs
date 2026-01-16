use anyhow::Result;

use bls_signatures::Serialize as BlsSerialize;
use bls_signatures::{
    PrivateKey as BlsPrivateKey, PublicKey as BlsPublicKey, Signature as BlsSignature,
};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

/// A simple abstraction over signature schemes. BLS is a placeholder here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignatureScheme {
    Ed25519(Vec<u8>),
    Bls(Vec<u8>), // BLS signature bytes (may be aggregated)
}

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

/// Lightweight BLS keypair wrapper (bytes-backed).
#[allow(dead_code)]
pub struct BlsKeypair {
    privkey: BlsPrivateKey,
    pubkey: BlsPublicKey,
}

#[allow(dead_code)]
impl BlsKeypair {
    pub fn generate() -> Self {
        let mut rng = OsRng {};
        let privkey = BlsPrivateKey::generate(&mut rng);
        let pubkey = privkey.public_key();
        Self { privkey, pubkey }
    }

    pub fn public(&self) -> BlsPublicKey {
        self.pubkey
    }

    pub fn sign(&self, msg: &[u8]) -> Vec<u8> {
        let sig: BlsSignature = self.privkey.sign(msg);
        sig.as_bytes()
    }

    pub fn verify(pubkey: &BlsPublicKey, msg: &[u8], sig_bytes: &[u8]) -> Result<()> {
        let sig = BlsSignature::from_bytes(sig_bytes)
            .map_err(|e| anyhow::anyhow!("bls from_bytes: {}", e))?;
        let ok = pubkey.verify(sig, msg);
        if ok {
            Ok(())
        } else {
            Err(anyhow::anyhow!("bls verify failed"))
        }
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

    #[cfg_attr(miri, ignore)]
    #[test]
    fn test_bls_sign_verify() -> Result<()> {
        let kp = BlsKeypair::generate();
        let pk = kp.public();
        let msg = b"hello kanari bls";
        let sig = kp.sign(msg);
        BlsKeypair::verify(&pk, msg, &sig)?;
        Ok(())
    }
}
