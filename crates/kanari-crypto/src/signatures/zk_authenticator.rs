// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Canonical zkLogin transaction authenticator.
//!
//! This is the data model + verification entry point the node will call when
//! it starts accepting zkLogin senders (tracked integration, not yet wired:
//! ingestion must additionally enforce replay protection and a ceremony VK).
//! Defining it here — with vectors both sides agree on — lets the CLI
//! produce and the (future) engine consume the exact same bytes.
//!
//! Bundle: ephemeral pubkey (32B) + ephemeral sig (64B, over the tx hash)
//! + max_epoch (u64) + one of:
//! - JWT mode: jwt, jwks, iss, aud, randomness[32], salt[32] (linkable).
//! - Proof mode: vk, inputs (64x32B BE), proof, expected address (private).
//!
//! Verification is fail-closed and total: malformed input => `Err`,
//! well-formed-but-wrong => `Err(VerificationFailed)`. Success returns the
//! recovered `0x` address, which the caller MUST compare against the
//! transaction sender.

use crate::SignatureError;
use crate::signatures::zklogin::{
    EphemeralKeypair, JwksDocument, compute_nonce, derive_zklogin_address_v2, verify_jwt_with_jwks,
};

/// Linkable authenticator: full JWT travels with the transaction.
#[derive(Debug, Clone)]
pub struct JwtAuth {
    /// Compact JWT.
    pub jwt: String,
    /// Provider JWKS (fetched by the submitter; the chain does not dial out).
    pub jwks: JwksDocument,
    /// Expected issuer (e.g. `https://accounts.google.com`).
    pub iss: String,
    /// Expected audience (the OAuth client ID).
    pub aud: String,
    /// Nonce randomness from the login session.
    pub randomness: [u8; 32],
    /// Address salt from the login session.
    pub salt: [u8; 32],
}

/// Private authenticator: Groth16 proof only (plus ephemeral signature).
#[derive(Debug, Clone)]
pub struct ProofAuth {
    /// Serialized verifying key (caller pins its hash out-of-band).
    pub vk: Vec<u8>,
    /// 64 x 32-byte big-endian field elements (address ++ nonce).
    pub inputs: Vec<u8>,
    /// Serialized Groth16 proof.
    pub proof: Vec<u8>,
    /// Address the inputs must encode (checked byte-for-byte).
    pub expected_address: String,
}

/// One of the two authentication modes.
#[derive(Debug, Clone)]
pub enum ZkAuthKind {
    Jwt(JwtAuth),
    Proof(ProofAuth),
}

/// Full transaction authenticator for a zkLogin sender.
#[derive(Debug, Clone)]
pub struct ZkLoginAuthenticator {
    /// Ephemeral session public key (32 bytes).
    pub ephemeral_pubkey: [u8; 32],
    /// Ed25519 signature over the transaction hash (64 bytes).
    pub ephemeral_sig: [u8; 64],
    /// Validity bound from the login nonce.
    pub max_epoch: u64,
    /// Mode-specific payload.
    pub kind: ZkAuthKind,
}

/// Successfully authenticated sender.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedZkLogin {
    /// Recovered `0x`-hex address. Caller compares with tx sender.
    pub address: String,
    /// Validity bound from the login nonce.
    pub max_epoch: u64,
}

/// Verify a transaction authenticator against a transaction hash.
///
/// Order matters (cheap checks first, fail-closed throughout):
/// 1.Ephemeral Ed25519 signature over `tx_hash`.
/// 2a.JWT mode: signature/kid/iss/aud/exp, nonce binding, address derivation.
/// 2b.Proof mode: Groth16 verify, address extraction + expected match.
///
/// What this does NOT do (caller's job): compare `address` with the tx
/// sender, enforce `max_epoch` against chain time, or replay protection.
pub fn verify_zklogin_authenticator(
    tx_hash: &[u8],
    auth: &ZkLoginAuthenticator,
    now_secs: u64,
) -> Result<VerifiedZkLogin, SignatureError> {
    EphemeralKeypair::verify(&auth.ephemeral_pubkey, tx_hash, &auth.ephemeral_sig)?;
    match &auth.kind {
        ZkAuthKind::Jwt(jwt) => {
            let claims = verify_jwt_with_jwks(&jwt.jwt, &jwt.jwks, &jwt.iss, &jwt.aud, now_secs)?;
            // Nonce must bind THIS session key (not just any valid login).
            let expected = compute_nonce(&auth.ephemeral_pubkey, auth.max_epoch, &jwt.randomness);
            match &claims.nonce {
                Some(n) if *n == expected => {}
                _ => return Err(SignatureError::VerificationFailed),
            }
            let address = derive_zklogin_address_v2(&claims.iss, &jwt.aud, &claims.sub, &jwt.salt)?;
            Ok(VerifiedZkLogin {
                address,
                max_epoch: auth.max_epoch,
            })
        }
        ZkAuthKind::Proof(proof) => {
            let valid = crate::signatures::zklogin_proof::verify_groth16_proof(
                &proof.vk,
                &proof.inputs,
                &proof.proof,
            )?;
            if !valid {
                return Err(SignatureError::VerificationFailed);
            }
            let address = address_from_inputs(&proof.inputs)?;
            if !addresses_equal(&address, &proof.expected_address) {
                return Err(SignatureError::VerificationFailed);
            }
            Ok(VerifiedZkLogin {
                address,
                max_epoch: auth.max_epoch,
            })
        }
    }
}

/// Decode the address (first 32 inputs) from 64x32B public-input bytes.
/// Each input must be a bare byte (top 31 bytes zero) — otherwise the
/// encoding is not ours and we refuse to interpret it.
fn address_from_inputs(inputs: &[u8]) -> Result<String, SignatureError> {
    if inputs.len() != 64 * 32 {
        return Err(SignatureError::InvalidFormat(format!(
            "proof inputs must be 64x32 bytes, got {}",
            inputs.len()
        )));
    }
    let mut addr = [0u8; 32];
    for (i, chunk) in inputs[..32 * 32].as_chunks::<32>().0.iter().enumerate() {
        if chunk[..31].iter().any(|&b| b != 0) {
            return Err(SignatureError::InvalidFormat(
                "address input is not a byte".to_string(),
            ));
        }
        addr[i] = chunk[31];
    }
    Ok(format!("0x{}", hex::encode(addr)))
}

fn addresses_equal(a: &str, b: &str) -> bool {
    a.trim_start_matches("0x")
        .eq_ignore_ascii_case(b.trim_start_matches("0x"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signatures::zklogin::{ISS_GOOGLE, decode_jwt_claims, derive_address_from_claims};

    const TEST_AUD: &str = "kanari-test-client";

    fn fixture_ephemeral() -> ([u8; 32], Vec<u8>) {
        // Same deterministic fixture as the Move tests: secret [42u8; 32].
        let kp = EphemeralKeypair::from_secret([42u8; 32]).unwrap();
        let msg = b"kanari ephemeral fixture";
        let sig = kp.sign(msg);
        (kp.public_bytes(), sig)
    }

    /// Deterministic RSA-2048 key (xorshift RNG): same key every run, no OS
    /// entropy, no fixture files. Returns (n_b64url, e_b64url, pkcs1_der).
    fn local_rsa() -> (String, String, Vec<u8>) {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

        struct Xor(u64);
        impl rsa::rand_core::RngCore for Xor {
            fn next_u32(&mut self) -> u32 {
                self.next_u64() as u32
            }
            fn next_u64(&mut self) -> u64 {
                let mut x = self.0 | 1;
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                self.0 = x;
                x
            }
            fn fill_bytes(&mut self, dest: &mut [u8]) {
                let mut i = 0;
                while i < dest.len() {
                    let b = self.next_u64().to_le_bytes();
                    let n = core::cmp::min(8, dest.len() - i);
                    dest[i..i + n].copy_from_slice(&b[..n]);
                    i += n;
                }
            }
            fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rsa::rand_core::Error> {
                self.fill_bytes(dest);
                Ok(())
            }
        }
        impl rsa::rand_core::CryptoRng for Xor {}

        // Cached: ~1s keygen runs once per test process.
        static KEY: std::sync::OnceLock<(String, String, Vec<u8>)> = std::sync::OnceLock::new();
        KEY.get_or_init(|| {
            use rsa::pkcs1::EncodeRsaPrivateKey as _;
            use rsa::traits::PublicKeyParts as _;
            let mut rng = Xor(0xBEEF);
            let sk = rsa::RsaPrivateKey::new(&mut rng, 2048).unwrap();
            let pk = rsa::RsaPublicKey::from(&sk);
            let n = URL_SAFE_NO_PAD.encode(pk.n().to_bytes_be());
            let e = URL_SAFE_NO_PAD.encode(pk.e().to_bytes_be());
            let der = sk.to_pkcs1_der().unwrap().as_bytes().to_vec();
            (n, e, der)
        })
        .clone()
    }

    fn fixture_jwt_auth() -> JwtAuth {
        let (n_b64, e_b64, _) = local_rsa();
        let jwks: JwksDocument = serde_json::from_str(&format!(
            "{{\"keys\":[{{\"kty\":\"RSA\",\"kid\":\"local-1\",\"alg\":\"RS256\",\"n\":\"{n_b64}\",\"e\":\"{e_b64}\"}}]}}"
        ))
        .unwrap();
        JwtAuth {
            jwt: mint_local_jwt(),
            jwks,
            iss: ISS_GOOGLE.to_string(),
            aud: TEST_AUD.to_string(),
            randomness: [7u8; 32],
            salt: [9u8; 32],
        }
    }

    fn mint_local_jwt() -> String {
        let (_, _, der) = local_rsa();
        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        header.kid = Some("local-1".to_string());
        // Nonce binds the deterministic ephemeral fixture (epoch 1000).
        let (eph_pub, _) = fixture_ephemeral();
        let nonce = crate::signatures::zklogin::compute_nonce(&eph_pub, 1000, &[7u8; 32]);
        let claims = serde_json::json!({
            "iss": ISS_GOOGLE,
            "aud": TEST_AUD,
            "sub": "1234",
            "exp": 2000000000u64,
            "nonce": nonce,
        });
        let key = jsonwebtoken::EncodingKey::from_rsa_der(&der);
        jsonwebtoken::encode(&header, &claims, &key).unwrap()
    }

    #[test]
    fn jwt_authenticator_roundtrip() {
        let (eph_pub, _) = fixture_ephemeral();
        let tx_hash = b"kanari-tx-hash-fixture-000000000001";
        // Sign the tx hash with the fixture ephemeral secret.
        let kp = EphemeralKeypair::from_secret([42u8; 32]).unwrap();
        let sig: [u8; 64] = kp.sign(tx_hash).try_into().unwrap();
        let auth = ZkLoginAuthenticator {
            ephemeral_pubkey: eph_pub,
            ephemeral_sig: sig,
            max_epoch: 1000,
            kind: ZkAuthKind::Jwt(fixture_jwt_auth()),
        };
        let verified = verify_zklogin_authenticator(tx_hash, &auth, 1_700_000_000).unwrap();
        // Cross-check against the direct derivation (no drift).
        let expected = derive_address_from_claims(
            &decode_jwt_claims(&mint_local_jwt()).unwrap(),
            TEST_AUD,
            &[9u8; 32],
        )
        .unwrap();
        assert_eq!(verified.address, expected);
        assert_eq!(verified.max_epoch, 1000);
    }

    #[test]
    fn jwt_authenticator_rejects_wrong_tx_sig() {
        let (eph_pub, _) = fixture_ephemeral();
        let kp = EphemeralKeypair::from_secret([42u8; 32]).unwrap();
        // Signature over a DIFFERENT hash than the one being verified.
        let sig: [u8; 64] = kp
            .sign(b"something else entirely........")
            .try_into()
            .unwrap();
        let auth = ZkLoginAuthenticator {
            ephemeral_pubkey: eph_pub,
            ephemeral_sig: sig,
            max_epoch: 1000,
            kind: ZkAuthKind::Jwt(fixture_jwt_auth()),
        };
        assert!(
            verify_zklogin_authenticator(
                b"kanari-tx-hash-fixture-000000000001",
                &auth,
                1_700_000_000
            )
            .is_err()
        );
    }

    #[test]
    fn proof_authenticator_roundtrip_and_rejects() {
        use crate::signatures::zklogin::{compute_nonce, derive_zklogin_address_v2};
        use crate::signatures::zklogin_circuit::{
            proof_to_bytes, prove_binding, public_inputs_to_be_bytes, setup_binding_circuit,
            vk_to_bytes,
        };
        use ark_std::rand::SeedableRng;

        let mut rng = ark_std::rand::rngs::StdRng::seed_from_u64(0xB10C5EED);
        let salt = [9u8; 32];
        let (eph_pub, _) = fixture_ephemeral();
        let kp = EphemeralKeypair::from_secret([42u8; 32]).unwrap();
        let tx_hash = b"kanari-proof-tx-000000000000000001";
        let sig: [u8; 64] = kp.sign(tx_hash).try_into().unwrap();
        let addr_hex = derive_zklogin_address_v2(ISS_GOOGLE, TEST_AUD, "1234", &salt).unwrap();
        let address: [u8; 32] = hex::decode(&addr_hex[2..]).unwrap().try_into().unwrap();
        let nonce_hex = compute_nonce(&eph_pub, 1000, &[7u8; 32]);
        let nonce: [u8; 32] = hex::decode(&nonce_hex).unwrap().try_into().unwrap();

        let (pk, vk) =
            setup_binding_circuit(ISS_GOOGLE.as_bytes(), TEST_AUD.as_bytes(), &mut rng).unwrap();
        let proof = prove_binding(
            &pk,
            ISS_GOOGLE.as_bytes(),
            TEST_AUD.as_bytes(),
            salt,
            b"1234",
            [7u8; 32],
            eph_pub,
            1000,
            address,
            nonce,
            &mut rng,
        )
        .unwrap();
        let auth = ZkLoginAuthenticator {
            ephemeral_pubkey: eph_pub,
            ephemeral_sig: sig,
            max_epoch: 1000,
            kind: ZkAuthKind::Proof(ProofAuth {
                vk: vk_to_bytes(&vk).unwrap(),
                inputs: public_inputs_to_be_bytes(&address, &nonce),
                proof: proof_to_bytes(&proof).unwrap(),
                expected_address: addr_hex.clone(),
            }),
        };
        let verified = verify_zklogin_authenticator(tx_hash, &auth, 1_700_000_000).unwrap();
        assert_eq!(verified.address, addr_hex);

        // Tampered expected address must fail.
        let mut bad = auth.clone();
        if let ZkAuthKind::Proof(p) = &mut bad.kind {
            p.expected_address =
                "0x0000000000000000000000000000000000000000000000000000000000000000".to_string();
        }
        assert!(verify_zklogin_authenticator(tx_hash, &bad, 1_700_000_000).is_err());
    }

    #[test]
    fn address_from_inputs_rejects_non_bytes() {
        let mut bad = vec![0u8; 64 * 32];
        bad[0] = 1; // top byte of first input nonzero => not a byte
        assert!(address_from_inputs(&bad).is_err());
        assert!(address_from_inputs(&[0u8; 10]).is_err());
    }
}
