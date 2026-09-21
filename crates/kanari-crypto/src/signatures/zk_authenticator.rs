// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Canonical zkLogin transaction authenticator.
//!
//! Ingestion (mempool/node) entry point: [`admit_zklogin_tx`]. It bundles
//! every check the raw pieces leave to the caller — bundle version, session
//! expiry (`max_epoch`), replay guard, mode verification (with a REQUIRED
//! ceremony-VK pin for proof mode), and sender match — so future wiring
//! cannot forget one. The lower-level verifiers below stay available for
//! offline tooling (CLI) and the Move natives, which enforce pinning and
//! expiry in their own layer (`kanari_system::zklogin`).
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

/// Hex serde for fixed byte arrays (serde only implements arrays to 32).
/// Lengths are enforced on decode: wrong size fails closed, never pads.
mod hex_fixed {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<const N: usize, S: Serializer>(
        bytes: &[u8; N],
        s: S,
    ) -> Result<S::Ok, S::Error> {
        hex::encode(bytes).serialize(s)
    }

    pub fn deserialize<'de, const N: usize, D: Deserializer<'de>>(
        d: D,
    ) -> Result<[u8; N], D::Error> {
        let s = String::deserialize(d)?;
        let v = hex::decode(s.trim_start_matches("0x")).map_err(serde::de::Error::custom)?;
        v.try_into()
            .map_err(|_| serde::de::Error::custom(format!("expected {N} bytes")))
    }
}

/// Linkable authenticator: full JWT travels with the transaction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    #[serde(with = "hex_fixed")]
    pub randomness: [u8; 32],
    /// Address salt from the login session.
    #[serde(with = "hex_fixed")]
    pub salt: [u8; 32],
}

/// Private authenticator: Groth16 proof only (plus ephemeral signature).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ZkAuthKind {
    Jwt(JwtAuth),
    Proof(ProofAuth),
}

/// Full transaction authenticator for a zkLogin sender.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ZkLoginAuthenticator {
    /// Ephemeral session public key (32 bytes).
    #[serde(with = "hex_fixed")]
    pub ephemeral_pubkey: [u8; 32],
    /// Ed25519 signature over the transaction hash (64 bytes).
    #[serde(with = "hex_fixed")]
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
            // Nonce must bind THIS session key (not just any valid login).
            let expected = compute_nonce(&auth.ephemeral_pubkey, auth.max_epoch, &jwt.randomness);
            let claims = verify_jwt_with_jwks(
                &jwt.jwt,
                &jwt.jwks,
                &jwt.iss,
                &jwt.aud,
                Some(expected.as_str()),
                now_secs,
            )?;
            let address = derive_zklogin_address_v2(&claims.iss, &jwt.aud, &claims.sub, &jwt.salt)?;
            Ok(VerifiedZkLogin {
                address,
                max_epoch: auth.max_epoch,
            })
        }
        ZkAuthKind::Proof(proof) => verify_proof_kind(
            &auth.ephemeral_pubkey,
            &auth.ephemeral_sig,
            tx_hash,
            auth.max_epoch,
            proof,
            None,
        ),
    }
}

/// Proof-mode verification shared by the bare verifier above (unpinned —
/// offline tooling and the Move `verify_proof` native, which pins in its
/// own layer) and [`admit_zklogin_tx`] (pinned — REQUIRED).
fn verify_proof_kind(
    ephemeral_pubkey: &[u8; 32],
    ephemeral_sig: &[u8; 64],
    tx_hash: &[u8],
    max_epoch: u64,
    proof: &ProofAuth,
    vk_pin: Option<&[u8; 32]>,
) -> Result<VerifiedZkLogin, SignatureError> {
    EphemeralKeypair::verify(ephemeral_pubkey, tx_hash, ephemeral_sig)?;
    let valid = match vk_pin {
        Some(pin) => crate::signatures::groth16::verify_pinned_proof(
            pin,
            &proof.vk,
            &proof.inputs,
            &proof.proof,
        )?,
        None => crate::signatures::groth16::verify_groth16_proof(
            &proof.vk,
            &proof.inputs,
            &proof.proof,
        )?,
    };
    if !valid {
        return Err(SignatureError::VerificationFailed);
    }
    let address = address_from_inputs(&proof.inputs)?;
    if !addresses_equal(&address, &proof.expected_address) {
        return Err(SignatureError::VerificationFailed);
    }
    Ok(VerifiedZkLogin { address, max_epoch })
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

/// Wire encoding of a transaction signature for `ZkLogin:` senders.
///
/// JSON object with a version tag so future bundle formats fail closed on
/// old verifiers instead of mis-parsing:
/// `{"v":1,"auth":{...ZkLoginAuthenticator...}}`.
/// Sizes stay small (JWT mode ~2KB, proof mode ~5KB with VK) and under the
/// 64KiB signature cap enforced by `validate_signature_bytes`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ZkLoginTxSignature {
    /// Bundle format version. Only `1` is accepted.
    pub v: u32,
    /// The authenticator itself.
    pub auth: ZkLoginAuthenticator,
}

pub fn encode_zklogin_tx_signature(auth: &ZkLoginAuthenticator) -> Result<Vec<u8>, SignatureError> {
    serde_json::to_vec(&ZkLoginTxSignature {
        v: 1,
        auth: auth.clone(),
    })
    .map_err(|e| SignatureError::InvalidFormat(format!("cannot encode zkLogin bundle: {e}")))
}

/// Verify a `ZkLogin:` sender's transaction signature.
///
/// This is the function the mempool admission path reaches through
/// `verify_signature_with_curve(..., CurveType::ZkLogin)`: `message` is the
/// transaction hash, `signature` the JSON bundle above. Returns `Ok(true)`
/// only when the ephemeral signature checks out, the mode payload verifies,
/// AND the recovered address matches `address_hex`.
pub fn verify_zklogin_tx_signature(
    address_hex: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, SignatureError> {
    let bundle: ZkLoginTxSignature = serde_json::from_slice(signature).map_err(|_| {
        SignatureError::InvalidFormat("zkLogin bundle is not valid JSON".to_string())
    })?;
    if bundle.v != 1 {
        return Err(SignatureError::InvalidFormat(format!(
            "unsupported zkLogin bundle version {}",
            bundle.v
        )));
    }
    // Wall-clock here matches the CLI/JWT world (Google `exp`). A future
    // consensus rule may additionally pin `max_epoch` to chain time; until
    // then the JWT `exp` check inside is the timeliness bound for JWT mode.
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| SignatureError::VerificationFailed)?;
    let verified = verify_zklogin_authenticator(message, &bundle.auth, now_secs)?;
    Ok(addresses_equal(&verified.address, address_hex))
}

/// Fail unless the session is still valid at `current_epoch` (chain time).
///
/// JWT mode additionally enforces JWT `exp` during verification; proof mode
/// has NO `exp`, so this is its ONLY timeliness bound — never skip it.
/// `max_epoch` comes from the login nonce binding; `current_epoch` is
/// `tx_context::epoch` (Move) or the validator's epoch clock (mempool).
pub fn check_max_epoch(max_epoch: u64, current_epoch: u64) -> Result<(), SignatureError> {
    if max_epoch < current_epoch {
        return Err(SignatureError::Expired);
    }
    Ok(())
}

/// Replay-guard cache for admitted zkLogin bundles.
///
/// Keyed by `SHA256(tx_hash)` with per-entry expiry; [`ReplayCache::check_and_insert`]
/// reports whether a transaction is fresh. Best-effort in-memory layer for
/// the admission path: it bounds rapid resubmission, but a restart wipes
/// it — the execution layer MUST additionally reject re-execution (object
/// versions / sender nonce), and JWT/proof lifetimes bound the residual
/// window.
pub struct ReplayCache {
    /// Digest -> expiry (unix secs). Swept lazily on insert.
    seen: std::collections::HashMap<[u8; 32], u64>,
    ttl_secs: u64,
}

impl ReplayCache {
    /// `ttl_secs` should cover the maximum bundle lifetime (the JWT `exp`
    /// window for JWT mode; for proof mode size it to the deployment's
    /// epoch length x allowed `max_epoch` span). Zero disables caching
    /// (every admission is "fresh") — tests only.
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            seen: std::collections::HashMap::new(),
            ttl_secs,
        }
    }

    /// True when fresh (records it); false when the same transaction was
    /// already admitted inside the TTL window. Expired entries are swept
    /// on each call so memory stays proportional to the live window.
    pub fn check_and_insert(&mut self, tx_hash: &[u8], now_secs: u64) -> bool {
        use sha2::{Digest, Sha256};
        self.seen.retain(|_, exp| *exp > now_secs);
        let digest: [u8; 32] = Sha256::digest(tx_hash).into();
        if self.seen.contains_key(&digest) {
            return false;
        }
        self.seen
            .insert(digest, now_secs.saturating_add(self.ttl_secs));
        true
    }

    /// Live entries (metrics/tests).
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    /// True when no entries are cached.
    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

/// Options for [`admit_zklogin_tx`].
pub struct AdmitOptions {
    /// Pinned ceremony VK hash. REQUIRED when a Proof-mode bundle arrives;
    /// `None` + Proof bundle => rejection, so demo toxic-waste setups can
    /// never pass admission. Unused for JWT mode.
    pub vk_pin: Option<[u8; 32]>,
}

/// Fail-closed admission for a `ZkLogin:` sender — THE function ingestion
/// (mempool/node) must call instead of the raw pieces. Order (cheap,
/// state-light checks first):
/// 1. Bundle parses as v1 (else `InvalidFormat`).
/// 2. `max_epoch >= current_epoch` (else `Expired`) — the ONLY timeliness
///    bound proof mode has.
/// 3. Replay: same `tx_hash` admitted inside the cache TTL
///    => `VerificationFailed`.
/// 4. Mode verify: JWT (sig/kid/iss/aud/exp/nonce/address) or Proof
///    (pinned VK REQUIRED + proof + expected address).
/// 5. Recovered address == `expected_sender` (0x-tolerant,
///    case-insensitive).
///
/// What this does NOT do (engine's job): persist the replay log across
/// restarts (see [`ReplayCache`]), or compare `address` with anything
/// beyond the passed sender.
pub fn admit_zklogin_tx(
    expected_sender: &str,
    tx_hash: &[u8],
    bundle_bytes: &[u8],
    opts: &AdmitOptions,
    cache: &mut ReplayCache,
    now_secs: u64,
    current_epoch: u64,
) -> Result<VerifiedZkLogin, SignatureError> {
    let bundle: ZkLoginTxSignature = serde_json::from_slice(bundle_bytes).map_err(|_| {
        SignatureError::InvalidFormat("zkLogin bundle is not valid JSON".to_string())
    })?;
    if bundle.v != 1 {
        return Err(SignatureError::InvalidFormat(format!(
            "unsupported zkLogin bundle version {}",
            bundle.v
        )));
    }
    check_max_epoch(bundle.auth.max_epoch, current_epoch)?;
    if !cache.check_and_insert(tx_hash, now_secs) {
        return Err(SignatureError::VerificationFailed);
    }
    let verified = match &bundle.auth.kind {
        ZkAuthKind::Jwt(_) => verify_zklogin_authenticator(tx_hash, &bundle.auth, now_secs)?,
        ZkAuthKind::Proof(proof) => {
            let pin = opts
                .vk_pin
                .as_ref()
                .ok_or(SignatureError::VerificationFailed)?;
            verify_proof_kind(
                &bundle.auth.ephemeral_pubkey,
                &bundle.auth.ephemeral_sig,
                tx_hash,
                bundle.auth.max_epoch,
                proof,
                Some(pin),
            )?
        }
    };
    if !addresses_equal(&verified.address, expected_sender) {
        return Err(SignatureError::VerificationFailed);
    }
    Ok(verified)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signatures::zklogin::{ISS_GOOGLE, decode_jwt_claims, derive_zklogin_address_v2};

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
        let claims = decode_jwt_claims(&mint_local_jwt()).unwrap();
        let expected =
            derive_zklogin_address_v2(&claims.iss, TEST_AUD, &claims.sub, &[9u8; 32]).unwrap();
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
        use crate::signatures::groth16::{proof_to_bytes, vk_to_bytes};
        use crate::signatures::zklogin::{compute_nonce, derive_zklogin_address_v2};
        use crate::signatures::zklogin_circuit::{
            prove_binding, public_inputs_to_be_bytes, setup_binding_circuit,
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

    /// End-to-end through the mempool admission function
    /// (`SignedTransaction::verify_signature` -> tagged dispatch): a JWT
    /// bundle for `ZkLogin:0x...` verifies, and every tamper fails closed.
    #[test]
    fn tagged_dispatch_accepts_and_rejects() {
        use crate::signatures::verify_signature;

        let (eph_pub, _) = fixture_ephemeral();
        let tx_hash = b"kanari-tx-hash-fixture-000000000002";
        let kp = EphemeralKeypair::from_secret([42u8; 32]).unwrap();
        let sig: [u8; 64] = kp.sign(tx_hash).try_into().unwrap();
        let auth = ZkLoginAuthenticator {
            ephemeral_pubkey: eph_pub,
            ephemeral_sig: sig,
            max_epoch: 1000,
            kind: ZkAuthKind::Jwt(fixture_jwt_auth()),
        };
        let bundle = encode_zklogin_tx_signature(&auth).unwrap();
        let verified = verify_zklogin_authenticator(tx_hash, &auth, 1_700_000_000).unwrap();
        let tagged = format!("ZkLogin:{}", verified.address);

        // The exact path the mempool takes (tagged dispatch).
        assert!(verify_signature(&tagged, tx_hash, &bundle).unwrap());

        // Wrong sender address must fail (binding enforced). Well-formed
        // but non-matching verifies return Ok(false), like other curves.
        assert!(
            !verify_signature(
                "ZkLogin:0x0000000000000000000000000000000000000000000000000000000000000000",
                tx_hash,
                &bundle
            )
            .unwrap()
        );
        // Corrupt JSON must fail, not default-accept.
        assert!(verify_signature(&tagged, tx_hash, b"{nope").is_err());
        // Future bundle version must fail closed.
        let mut v2 = serde_json::from_slice::<serde_json::Value>(&bundle).unwrap();
        v2["v"] = serde_json::json!(2);
        assert!(
            verify_signature(
                &tagged,
                tx_hash,
                serde_json::to_vec(&v2).unwrap().as_slice()
            )
            .is_err()
        );
        // Untagged address never routes here.
        assert!(verify_signature(&verified.address, tx_hash, &bundle).is_err());
    }

    fn admit_jwt_case(tx_hash: &[u8]) -> (Vec<u8>, String) {
        let (eph_pub, _) = fixture_ephemeral();
        let kp = EphemeralKeypair::from_secret([42u8; 32]).unwrap();
        let sig: [u8; 64] = kp.sign(tx_hash).try_into().unwrap();
        let auth = ZkLoginAuthenticator {
            ephemeral_pubkey: eph_pub,
            ephemeral_sig: sig,
            max_epoch: 1000,
            kind: ZkAuthKind::Jwt(fixture_jwt_auth()),
        };
        let verified = verify_zklogin_authenticator(tx_hash, &auth, 1_700_000_000).unwrap();
        let bundle = encode_zklogin_tx_signature(&auth).unwrap();
        (bundle, verified.address)
    }

    /// Proof bundle with a VALID ephemeral sig but garbage VK: reaches the
    /// pin gate without an expensive real setup.
    fn admit_proof_nosetup_bundle(tx_hash: &[u8]) -> Vec<u8> {
        let (eph_pub, _) = fixture_ephemeral();
        let kp = EphemeralKeypair::from_secret([42u8; 32]).unwrap();
        let sig: [u8; 64] = kp.sign(tx_hash).try_into().unwrap();
        let auth = ZkLoginAuthenticator {
            ephemeral_pubkey: eph_pub,
            ephemeral_sig: sig,
            max_epoch: 1000,
            kind: ZkAuthKind::Proof(ProofAuth {
                vk: vec![7u8; 64],
                inputs: vec![0u8; 64 * 32],
                proof: vec![9u8; 128],
                expected_address: "0xabc".to_string(),
            }),
        };
        encode_zklogin_tx_signature(&auth).unwrap()
    }

    #[test]
    fn admit_accepts_fresh_matching_sender() {
        let tx = b"kanari-admit-tx-000000000000000001";
        let (bundle, sender) = admit_jwt_case(tx);
        let mut cache = ReplayCache::new(3600);
        let out = admit_zklogin_tx(
            &sender,
            tx,
            &bundle,
            &AdmitOptions { vk_pin: None },
            &mut cache,
            1_700_000_000,
            1000,
        )
        .unwrap();
        assert_eq!(out.address, sender);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn admit_rejects_replay_and_still_takes_new_tx() {
        let tx1 = b"kanari-admit-tx-000000000000000002";
        let tx2 = b"kanari-admit-tx-000000000000000003";
        let (bundle1, sender1) = admit_jwt_case(tx1);
        let (bundle2, sender2) = admit_jwt_case(tx2);
        let mut cache = ReplayCache::new(3600);
        let opts = AdmitOptions { vk_pin: None };
        assert!(
            admit_zklogin_tx(
                &sender1,
                tx1,
                &bundle1,
                &opts,
                &mut cache,
                1_700_000_000,
                1000
            )
            .is_ok()
        );
        // Same tx again inside the TTL: replay, even though it is valid.
        assert!(
            admit_zklogin_tx(
                &sender1,
                tx1,
                &bundle1,
                &opts,
                &mut cache,
                1_700_000_001,
                1000
            )
            .is_err()
        );
        // A different tx still admits.
        assert!(
            admit_zklogin_tx(
                &sender2,
                tx2,
                &bundle2,
                &opts,
                &mut cache,
                1_700_000_001,
                1000
            )
            .is_ok()
        );
    }

    #[test]
    fn admit_rejects_expired_epoch_and_wrong_sender() {
        let tx = b"kanari-admit-tx-000000000000000004";
        let (bundle, sender) = admit_jwt_case(tx);
        let mut cache = ReplayCache::new(3600);
        let opts = AdmitOptions { vk_pin: None };
        // max_epoch (1000) < current epoch: the ONLY timeliness bound proof
        // mode has, enforced for both modes here.
        assert!(matches!(
            admit_zklogin_tx(&sender, tx, &bundle, &opts, &mut cache, 1_700_000_000, 1001),
            Err(SignatureError::Expired)
        ));
        assert!(check_max_epoch(1000, 1000).is_ok());
        assert!(check_max_epoch(999, 1000).is_err());
        // Right bundle, wrong sender never admits.
        assert!(
            admit_zklogin_tx(
                "0x0000000000000000000000000000000000000000000000000000000000000000",
                tx,
                &bundle,
                &opts,
                &mut cache,
                1_700_000_000,
                1000,
            )
            .is_err()
        );
    }

    #[test]
    fn admit_proof_mode_requires_pin() {
        let tx = b"kanari-admit-tx-000000000000000005";
        let bundle = admit_proof_nosetup_bundle(tx);
        let mut cache = ReplayCache::new(3600);
        // No pin: rejected at the gate before any VK bytes are touched.
        assert!(
            admit_zklogin_tx(
                "0xabc",
                tx,
                &bundle,
                &AdmitOptions { vk_pin: None },
                &mut cache,
                1_700_000_000,
                1000,
            )
            .is_err()
        );
        // Pin present: flow continues into VK parsing, which fails closed
        // on the garbage VK (InvalidFormat, not a silent false).
        let tx2 = b"kanari-admit-tx-000000000000000006";
        let bundle2 = admit_proof_nosetup_bundle(tx2);
        assert!(matches!(
            admit_zklogin_tx(
                "0xabc",
                tx2,
                &bundle2,
                &AdmitOptions {
                    vk_pin: Some([1u8; 32])
                },
                &mut cache,
                1_700_000_000,
                1000,
            ),
            Err(SignatureError::InvalidFormat(_))
        ));
    }

    #[test]
    fn replay_cache_ttl_sweeps_expired_entries() {
        let mut cache = ReplayCache::new(10);
        assert!(cache.check_and_insert(b"tx-a", 100));
        assert_eq!(cache.len(), 1);
        assert!(!cache.check_and_insert(b"tx-a", 105));
        assert!(cache.check_and_insert(b"tx-b", 105));
        assert_eq!(cache.len(), 2);
        // tx-a expired at 110: sweep drops it, re-admission succeeds.
        assert!(cache.check_and_insert(b"tx-a", 111));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn zklogin_has_no_signing_path() {
        use crate::keys::CurveType;
        use crate::signatures::sign_message;
        assert!(sign_message("00", b"m", CurveType::ZkLogin).is_err());
        assert!(crate::keys::generate_keypair(CurveType::ZkLogin).is_err());
    }
}
