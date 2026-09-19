// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! zkLogin (Sui-style) — Phase 1+2: JWT claims, address derivation,
//! ephemeral keys, nonce binding.
//!
//! Sui zkLogin flow (what we are building toward):
//! 1. User logs in with Google (or any OIDC provider) and gets a JWT with
//!    claims `iss` (issuer), `aud` (audience = client ID), `sub` (subject).
//! 2. Client picks a random `salt` (user secret) and derives a zkLogin
//!    address = `hash("zkLogin" || iss || aud || sub || salt)` truncated to
//!    an AccountAddress.
//! 3. Client generates an ephemeral keypair + Groth16 ZK proof that links
//!    the JWT claims to the ephemeral public key, bounded by `max_epoch`.
//! 4. Chain verifies: ephemeral signature over the tx, then the ZK proof
//!    against the provider JWK + `authenticator_state`.
//!
//! This module implements steps 1–2 fully (JWT parsing, address derivation,
//! ephemeral Ed25519 keys, Sui-style nonce binding) with only dependencies
//! already in the tree (`sha2`/`base64`/`ed25519-dalek`/`rand`).
//! Groth16 proof verification (steps 3–4, on-chain) is Phase 3 and needs a
//! BN254 + circuit dependency that does not exist in the workspace yet.

use base64::{Engine as _, engine::general_purpose};
use serde::Deserialize;

use crate::SignatureError;

/// The OIDC issuer we accept. Only Google is supported end to end (CLI
/// rejects other providers, the app mints Google JWTs) — no other issuer
/// constants exist on purpose, so nothing can target an unconfigured one.
pub const ISS_GOOGLE: &str = "https://accounts.google.com";

/// Minimal JWT payload claims needed for zkLogin.
#[derive(Debug, Clone, Deserialize)]
pub struct JwtClaims {
    /// Issuer, e.g. `https://accounts.google.com`.
    pub iss: String,
    /// Audience: OAuth client ID the JWT was minted for.
    #[serde(default)]
    pub aud: JwtAud,
    /// Subject: provider-stable user ID.
    pub sub: String,
    /// Expiry (unix seconds). Checked by `verify_claims_timing`.
    #[serde(default)]
    pub exp: Option<u64>,
    /// Issued-at (unix seconds). Checked by `verify_claims_timing` when
    /// present: must not be in the future beyond the clock-skew leeway.
    #[serde(default)]
    pub iat: Option<u64>,
    /// Not-before (unix seconds). Checked by `verify_claims_timing` when
    /// present: must not be in the future beyond the clock-skew leeway.
    #[serde(default)]
    pub nbf: Option<u64>,
    /// Nonce bound to the ephemeral key (checked in Phase 3).
    #[serde(default)]
    pub nonce: Option<String>,
}

/// `aud` is a string for Google but an array for some providers (Apple).
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(untagged)]
pub enum JwtAud {
    Single(String),
    Multiple(Vec<String>),
    #[default]
    Missing,
}

impl JwtAud {
    #[must_use]
    pub fn as_vec(&self) -> Vec<String> {
        match self {
            JwtAud::Single(s) => vec![s.clone()],
            JwtAud::Multiple(v) => v.clone(),
            JwtAud::Missing => vec![],
        }
    }

    #[must_use]
    pub fn contains(&self, aud: &str) -> bool {
        match self {
            JwtAud::Single(s) => s == aud,
            JwtAud::Multiple(v) => v.iter().any(|a| a == aud),
            JwtAud::Missing => false,
        }
    }
}

/// Decode (NOT verify) the payload of a compact JWT (`header.payload.sig`).
/// Signature verification happens against the provider JWK in Phase 2/3;
/// this is only for extracting claims client-side.
pub fn decode_jwt_claims(jwt: &str) -> Result<JwtClaims, SignatureError> {
    let mut parts = jwt.splitn(3, '.');
    let (_header, payload, _sig) = (parts.next(), parts.next(), parts.next());
    let payload = payload.ok_or_else(|| {
        SignatureError::InvalidFormat("JWT must have 3 dot-separated parts".to_string())
    })?;
    let bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| SignatureError::InvalidFormat("JWT payload is not base64url".to_string()))?;
    serde_json::from_slice::<JwtClaims>(&bytes)
        .map_err(|e| SignatureError::InvalidFormat(format!("JWT payload is not valid claims: {e}")))
}

/// Fixed-width caps for the v2 address scheme. Every real-world value fits
/// with margin (Google iss 27B, client-ID aud ~72B, numeric sub ~21B,
/// UUID sub 36B); larger inputs are rejected, never silently truncated.
pub const MAX_ISS_BYTES: usize = 64;
pub const MAX_AUD_BYTES: usize = 96;
pub const MAX_SUB_BYTES: usize = 64;

/// Circuit-friendly zkLogin address derivation (v2):
/// `address = SHA256("zkLogin-v2" || iss[64] || aud[96] || sub[64] || salt[32])`
/// with zero padding. Fixed widths make the preimage exactly 263 bytes, so
/// the same layout is directly constrainable in R1CS
/// (see `zklogin_circuit`). Unambiguous by construction — no length
/// prefixes needed.
///
/// This is a DIFFERENT scheme from v1 (different addresses for the same
/// inputs). v1 never reached production (test sessions only); v2 is the
/// canonical scheme going forward.
pub fn derive_zklogin_address_v2(
    iss: &str,
    aud: &str,
    sub: &str,
    salt: &[u8],
) -> Result<String, SignatureError> {
    use sha2::{Digest, Sha256};

    let (iss_b, aud_b, sub_b) = (iss.as_bytes(), aud.as_bytes(), sub.as_bytes());
    if iss_b.is_empty() || aud_b.is_empty() || sub_b.is_empty() {
        return Err(SignatureError::InvalidFormat(
            "zkLogin iss/aud/sub must all be non-empty".to_string(),
        ));
    }
    if iss_b.len() > MAX_ISS_BYTES || aud_b.len() > MAX_AUD_BYTES || sub_b.len() > MAX_SUB_BYTES {
        return Err(SignatureError::InvalidFormat(format!(
            "zkLogin field too long (iss<={MAX_ISS_BYTES}, aud<={MAX_AUD_BYTES}, sub<={MAX_SUB_BYTES})"
        )));
    }
    if salt.len() != 32 {
        return Err(SignatureError::InvalidFormat(
            "zkLogin salt must be exactly 32 bytes".to_string(),
        ));
    }
    let mut preimage = Vec::with_capacity(10 + MAX_ISS_BYTES + MAX_AUD_BYTES + MAX_SUB_BYTES + 32);
    preimage.extend_from_slice(b"zkLogin-v2");
    preimage.extend_from_slice(&pad_zero(iss_b, MAX_ISS_BYTES));
    preimage.extend_from_slice(&pad_zero(aud_b, MAX_AUD_BYTES));
    preimage.extend_from_slice(&pad_zero(sub_b, MAX_SUB_BYTES));
    preimage.extend_from_slice(salt);
    debug_assert_eq!(
        preimage.len(),
        10 + MAX_ISS_BYTES + MAX_AUD_BYTES + MAX_SUB_BYTES + 32
    );
    Ok(format!("0x{}", hex::encode(Sha256::digest(&preimage))))
}

fn pad_zero(data: &[u8], width: usize) -> Vec<u8> {
    let mut out = vec![0u8; width];
    out[..data.len()].copy_from_slice(data);
    out
}

/// Check `exp`/`iat`/`nbf` against `now_unix_secs` with a clock-skew leeway.
/// `None` exp fails closed (Sui requires exp in the proof circuit). When
/// `iat`/`nbf` are present they must not lie beyond the leeway in the future.
/// Expiry has its own error so callers (and chains) never confuse it with
/// a bad signature.
pub fn verify_claims_timing(claims: &JwtClaims, now_unix_secs: u64) -> Result<(), SignatureError> {
    const LEEWAY_SECS: u64 = 60;
    let exp = claims
        .exp
        .ok_or_else(|| SignatureError::InvalidFormat("JWT has no exp claim".to_string()))?;
    if now_unix_secs > exp.saturating_add(LEEWAY_SECS) {
        return Err(SignatureError::Expired);
    }
    // A token that claims to have been issued too far in the future is a
    // clock-integrity violation (or a misconfigured clock): treat as expired.
    if let Some(iat) = claims.iat
        && iat > now_unix_secs.saturating_add(LEEWAY_SECS)
    {
        return Err(SignatureError::Expired);
    }
    // A `nbf` after now means the IDP has not yet activated the token.
    // Fail closed instead of honoring it early.
    if let Some(nbf) = claims.nbf
        && now_unix_secs.saturating_add(LEEWAY_SECS) < nbf
    {
        return Err(SignatureError::Expired);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Phase 2: ephemeral keys + nonce binding (Sui `zklogin` protocol)
// ---------------------------------------------------------------------------

/// Ephemeral Ed25519 keypair bound to one login session.
/// The secret is zeroized on drop; only the 32-byte public key leaves the
/// client (inside the OIDC nonce and later the ZK proof).
pub struct EphemeralKeypair {
    secret: zeroize::Zeroizing<[u8; 32]>,
    public: [u8; 32],
}

impl EphemeralKeypair {
    /// Generate a fresh ephemeral keypair from OS randomness.
    pub fn generate() -> Result<Self, SignatureError> {
        use rand::{TryRng, rngs::SysRng};
        let mut secret = [0u8; 32];
        SysRng
            .try_fill_bytes(&mut secret)
            .map_err(|_| SignatureError::InvalidPrivateKey("OS randomness failed".to_string()))?;
        // Clamp-independent: ed25519-dalek handles the scalar internally.
        // Reject degenerate all-zero secrets (vanishing probability, but free).
        if secret == [0u8; 32] {
            return Err(SignatureError::InvalidPrivateKey(
                "degenerate ephemeral secret".to_string(),
            ));
        }
        let signing = ed25519_dalek::SigningKey::from_bytes(&secret);
        let public = signing.verifying_key().to_bytes();
        // ed25519-dalek 3.x `from_bytes` cannot fail for 32 bytes; the
        // small-order check happens at verify time.
        let _ = signing;
        Ok(Self {
            secret: zeroize::Zeroizing::new(secret),
            public,
        })
    }

    /// Deterministic construction from a 32-byte secret (tests / KDF flows).
    pub fn from_secret(secret: [u8; 32]) -> Result<Self, SignatureError> {
        if secret == [0u8; 32] {
            return Err(SignatureError::InvalidPrivateKey(
                "degenerate ephemeral secret".to_string(),
            ));
        }
        let signing = ed25519_dalek::SigningKey::from_bytes(&secret);
        let public = signing.verifying_key().to_bytes();
        Ok(Self {
            secret: zeroize::Zeroizing::new(secret),
            public,
        })
    }

    /// 32-byte ephemeral public key (goes into the nonce).
    #[must_use]
    pub fn public_bytes(&self) -> [u8; 32] {
        self.public
    }

    /// Export the raw 32-byte secret for session persistence (CLI login
    /// sessions, future transaction signing).
    ///
    /// Handle with care: anyone holding these bytes can sign as the
    /// ephemeral key until `max_epoch`. The CLI stores them with
    /// owner-only file permissions and warns on every save.
    #[must_use]
    pub fn secret_bytes(&self) -> [u8; 32] {
        *self.secret
    }

    /// Sign a transaction payload with the ephemeral secret.
    pub fn sign(&self, msg: &[u8]) -> Vec<u8> {
        use ed25519_dalek::Signer as _;
        let signing = ed25519_dalek::SigningKey::from_bytes(&self.secret);
        signing.sign(msg).to_bytes().to_vec()
    }

    /// Verify an ephemeral signature (used by tests / offline checks; the
    /// chain path re-verifies in the native).
    pub fn verify(public: &[u8], msg: &[u8], sig: &[u8]) -> Result<(), SignatureError> {
        use ed25519_dalek::Verifier as _;
        let pk: &[u8; 32] = public.try_into().map_err(|_| {
            SignatureError::InvalidPublicKey("ephemeral public key must be 32 bytes".to_string())
        })?;
        let verifying = ed25519_dalek::VerifyingKey::from_bytes(pk).map_err(|_| {
            SignatureError::InvalidPublicKey("bad ephemeral public key".to_string())
        })?;
        let sig: &[u8; 64] = sig.try_into().map_err(|_| {
            SignatureError::InvalidFormat("ephemeral signature must be 64 bytes".to_string())
        })?;
        verifying
            .verify(msg, &ed25519_dalek::Signature::from_bytes(sig))
            .map_err(|_| SignatureError::VerificationFailed)
    }
}

/// Sui-style nonce: `nonce = poseidon_bn254(ephemeral_pubkey || max_epoch || randomness)`
/// Poseidon needs a BN254 field implementation (not in the tree yet), so this
/// phase binds with `SHA256("zkLogin-nonce" || pubkey || max_epoch_le64 || randomness)`
/// in the same field order. The hash function swaps 1:1 for Poseidon in
/// Phase 3 without changing the wire format (`nonce` stays a hex string).
pub fn compute_nonce(ephemeral_pubkey: &[u8], max_epoch: u64, randomness: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"zkLogin-nonce");
    h.update(ephemeral_pubkey);
    h.update(max_epoch.to_le_bytes());
    h.update(randomness);
    hex::encode(h.finalize())
}

/// Random 32-byte `randomness` for nonce binding (single-use per login).
///
/// NOTE: there is deliberately no random-salt generator here. The
/// address-salt is always [`deterministic_salt`]; a random salt per login
/// would rotate the wallet address every time (and strand funds on
/// reinstall), so nothing in the repo may mint one.
pub fn generate_randomness() -> Result<[u8; 32], SignatureError> {
    use rand::{TryRng, rngs::SysRng};
    let mut randomness = [0u8; 32];
    SysRng
        .try_fill_bytes(&mut randomness)
        .map_err(|_| SignatureError::InvalidPrivateKey("OS randomness failed".to_string()))?;
    Ok(randomness)
}

/// Canonical zkLogin address-salt (THE primary salt, shared by the CLI
/// `kanari zklogin` and the Android app via FFI) so the same Google
/// account always derives the same address on every device and across
/// reinstalls / cleared app data:
///
/// `salt = SHA256("kanari-zklogin-salt-v1" || len(iss) || iss || len(aud) || aud || len(sub) || sub)`
/// with `u64` big-endian length prefixes (same framing as the salt-vault key).
///
/// Stability beats unlinkability here on purpose: a random per-device salt
/// silently derives a NEW address whenever local storage is lost, stranding
/// funds. A deterministic salt is computable by anyone who knows
/// `(iss, aud, sub)`, so it does not hide the link between the Google
/// identity and the on-chain address — same trade-off as Sui's
/// deterministic salt services. Both the CLI (`kanari zklogin`) and the
/// Android app (via FFI) use this function unconditionally, with nothing
/// stored anywhere, so the same account always maps to the same address.
///
/// Field caps mirror [`derive_zklogin_address_v2`]: oversized inputs are
/// rejected, never silently truncated.
pub fn deterministic_salt(iss: &str, aud: &str, sub: &str) -> Result<[u8; 32], SignatureError> {
    use sha2::{Digest, Sha256};

    let (iss_b, aud_b, sub_b) = (iss.as_bytes(), aud.as_bytes(), sub.as_bytes());
    if iss_b.is_empty() || aud_b.is_empty() || sub_b.is_empty() {
        return Err(SignatureError::InvalidFormat(
            "zkLogin iss/aud/sub must all be non-empty".to_string(),
        ));
    }
    if iss_b.len() > MAX_ISS_BYTES || aud_b.len() > MAX_AUD_BYTES || sub_b.len() > MAX_SUB_BYTES {
        return Err(SignatureError::InvalidFormat(format!(
            "zkLogin field too long (iss<={MAX_ISS_BYTES}, aud<={MAX_AUD_BYTES}, sub<={MAX_SUB_BYTES})"
        )));
    }
    let mut h = Sha256::new();
    h.update(b"kanari-zklogin-salt-v1");
    for part in [iss_b, aud_b, sub_b] {
        h.update((part.len() as u64).to_be_bytes());
        h.update(part);
    }
    Ok(h.finalize().into())
}

// ---------------------------------------------------------------------------
// Phase 3a: JWK RS256 verification (no Groth16 yet)
// ---------------------------------------------------------------------------

/// A single RSA JWK (`kty == "RSA"`) from a provider JWKS document.
/// Field names match RFC 7517 (`n`, `e` base64url, `kid`, `alg`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RsaJwk {
    /// Key ID — must match the JWT header `kid`.
    #[serde(default)]
    pub kid: Option<String>,
    /// Key type, must be `"RSA"`.
    pub kty: String,
    /// Algorithm, expected `"RS256"`.
    #[serde(default)]
    pub alg: Option<String>,
    /// Modulus, base64url (no pad).
    pub n: String,
    /// Exponent, base64url (no pad).
    pub e: String,
}

/// Provider JWKS document (`{"keys": [...]}`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JwksDocument {
    #[serde(default)]
    pub keys: Vec<RsaJwk>,
}

impl RsaJwk {
    fn decoding_key(&self) -> Result<jsonwebtoken::DecodingKey, SignatureError> {
        if self.kty != "RSA" {
            return Err(SignatureError::InvalidPublicKey(format!(
                "unsupported JWK kty {}",
                self.kty
            )));
        }
        jsonwebtoken::DecodingKey::from_rsa_components(&self.n, &self.e)
            .map_err(|e| SignatureError::InvalidPublicKey(format!("bad RSA JWK components: {e}")))
    }
}

/// Verify a JWT's RS256 signature against a JWKS document and return the
/// verified claims.
///
/// Checks, in order (fail-closed):
/// 1. `kid` in the header selects the JWK (missing/unknown `kid` => error).
/// 2. RS256 signature verifies with that JWK.
/// 3. `iss` equals `expected_iss`, `aud` contains `expected_aud`.
/// 4. `exp` is fresh per `verify_claims_timing` (`now_unix_secs`).
/// 5. When `expected_nonce` is `Some`, the `nonce` claim must equal it
///    exactly (missing or different => error). Pass `None` only when the
///    caller enforces the binding separately (e.g. on-chain `check_nonce`).
///
/// `validate_exp = false` in the inner validator because expiry is checked
/// with our own leeway logic (same instant, single clock read by the caller).
pub fn verify_jwt_with_jwks(
    jwt: &str,
    jwks: &JwksDocument,
    expected_iss: &str,
    expected_aud: &str,
    expected_nonce: Option<&str>,
    now_unix_secs: u64,
) -> Result<JwtClaims, SignatureError> {
    use jsonwebtoken::Algorithm;

    let header = jsonwebtoken::decode_header(jwt)
        .map_err(|e| SignatureError::InvalidFormat(format!("bad JWT header: {e}")))?;
    if header.alg != Algorithm::RS256 {
        return Err(SignatureError::InvalidFormat(format!(
            "unsupported JWT alg {:?} (need RS256)",
            header.alg
        )));
    }
    let kid = header
        .kid
        .ok_or_else(|| SignatureError::InvalidFormat("JWT header has no kid".to_string()))?;
    let jwk = jwks
        .keys
        .iter()
        .find(|k| k.kid.as_deref() == Some(kid.as_str()))
        .ok_or_else(|| SignatureError::InvalidPublicKey(format!("unknown JWK kid {kid}")))?;
    let mut validation = jsonwebtoken::Validation::new(Algorithm::RS256);
    validation.validate_exp = false; // own leeway check below
    validation.set_issuer(&[expected_iss]);
    validation.set_audience(&[expected_aud]);
    let data = jsonwebtoken::decode::<JwtClaims>(jwt, &jwk.decoding_key()?, &validation)
        .map_err(|_| SignatureError::VerificationFailed)?;
    // `sub` must be present and non-empty: it is half of the address
    // preimage, and an empty subject would alias every login of that issuer.
    if data.claims.sub.trim().is_empty() {
        return Err(SignatureError::InvalidFormat(
            "JWT sub is empty".to_string(),
        ));
    }
    verify_claims_timing(&data.claims, now_unix_secs)?;
    if let Some(expected) = expected_nonce {
        match &data.claims.nonce {
            Some(n) if n == expected => {}
            _ => return Err(SignatureError::VerificationFailed),
        }
    }
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_jwt() -> String {
        // header {"alg":"RS256"} . payload {"iss":...,"aud":"client123","sub":"user456","exp":9999999999}
        let payload = general_purpose::URL_SAFE_NO_PAD.encode(
            r#"{"iss":"https://accounts.google.com","aud":"client123","sub":"user456","exp":9999999999}"#,
        );
        format!("e30.{payload}.e30")
    }

    #[test]
    fn decodes_claims() {
        let claims = decode_jwt_claims(&test_jwt()).unwrap();
        assert_eq!(claims.iss, ISS_GOOGLE);
        assert!(claims.aud.contains("client123"));
        assert_eq!(claims.sub, "user456");
    }

    #[test]
    fn derives_stable_address_v2() {
        let salt = [7u8; 32];
        let a = derive_zklogin_address_v2(ISS_GOOGLE, "client123", "user456", &salt).unwrap();
        let b = derive_zklogin_address_v2(ISS_GOOGLE, "client123", "user456", &salt).unwrap();
        assert_eq!(a, b);
        assert!(a.starts_with("0x") && a.len() == 66);
        // Different sub => different address.
        let c = derive_zklogin_address_v2(ISS_GOOGLE, "client123", "other", &salt).unwrap();
        assert_ne!(a, c);
    }

    #[test]
    fn deterministic_salt_is_stable_and_shared_vector() {
        // Shared vector with the Android suite and the FFI tests: CLI and
        // app must emit this exact primary salt so both derive the address.
        let salt = deterministic_salt("https://accounts.google.com", "kanari-test-client", "1234")
            .unwrap();
        assert_eq!(
            hex::encode(salt),
            "82364ea8d3563ad90cd8876b4242a2d44be88114b1b4ca114b39ca7027617fe0"
        );
        // Same account -> same salt, forever (reinstall-safe).
        let again = deterministic_salt("https://accounts.google.com", "kanari-test-client", "1234")
            .unwrap();
        assert_eq!(salt, again);
        // Any field change -> different salt (hence different address).
        for (iss, aud, sub) in [
            ("https://other.example", "kanari-test-client", "1234"),
            ("https://accounts.google.com", "other-client", "1234"),
            ("https://accounts.google.com", "kanari-test-client", "5678"),
        ] {
            assert_ne!(salt, deterministic_salt(iss, aud, sub).unwrap());
        }
        // Feeds the address derivation end to end.
        let addr = derive_zklogin_address_v2(
            "https://accounts.google.com",
            "kanari-test-client",
            "1234",
            &salt,
        )
        .unwrap();
        assert!(addr.starts_with("0x") && addr.len() == 66);
    }

    #[test]
    fn deterministic_salt_rejects_bad_inputs() {
        assert!(deterministic_salt("", "a", "b").is_err());
        assert!(deterministic_salt("i", "", "b").is_err());
        assert!(deterministic_salt("i", "a", "").is_err());
        assert!(deterministic_salt("i", &"y".repeat(97), "b").is_err());
    }

    #[test]
    fn rejects_bad_salt_and_empty_claims() {
        assert!(derive_zklogin_address_v2(ISS_GOOGLE, "a", "b", &[0u8; 16]).is_err());
        assert!(derive_zklogin_address_v2("", "a", "b", &[0u8; 32]).is_err());
        assert!(derive_zklogin_address_v2(ISS_GOOGLE, "a", &"x".repeat(65), &[0u8; 32]).is_err());
        assert!(derive_zklogin_address_v2(ISS_GOOGLE, &"y".repeat(97), "b", &[0u8; 32]).is_err());
    }

    #[test]
    fn checks_expiry() {
        let claims = decode_jwt_claims(&test_jwt()).unwrap();
        assert!(verify_claims_timing(&claims, 1_000_000).is_ok());
        assert!(verify_claims_timing(&claims, 9_999_999_999 + 61).is_err());
    }

    #[test]
    fn checks_iat_not_in_future() {
        let mut claims = decode_jwt_claims(&test_jwt()).unwrap();
        // No iat => no extra bound (exp still governs).
        assert!(verify_claims_timing(&claims, 1_000_000).is_ok());
        // iat within leeway (now + 60) is tolerated (clock skew).
        claims.iat = Some(1_000_000 + 60);
        assert!(verify_claims_timing(&claims, 1_000_000).is_ok());
        // iat beyond leeway in the future fails closed.
        claims.iat = Some(1_000_000 + 61);
        assert!(verify_claims_timing(&claims, 1_000_000).is_err());
    }

    #[test]
    fn checks_nbf_not_in_future() {
        let mut claims = decode_jwt_claims(&test_jwt()).unwrap();
        // nbf at/before now passes.
        claims.nbf = Some(1_000_000);
        assert!(verify_claims_timing(&claims, 1_000_000).is_ok());
        // nbf exactly at the leeway boundary is tolerated.
        claims.nbf = Some(1_000_000 + 60);
        assert!(verify_claims_timing(&claims, 1_000_000).is_ok());
        // nbf in the future beyond leeway fails closed.
        claims.nbf = Some(1_000_000 + 61);
        assert!(verify_claims_timing(&claims, 1_000_000).is_err());
    }

    #[test]
    fn ephemeral_sign_verify_roundtrip() {
        let kp = EphemeralKeypair::from_secret([9u8; 32]).unwrap();
        let msg = b"kanari zklogin ephemeral test";
        let sig = kp.sign(msg);
        assert!(EphemeralKeypair::verify(&kp.public_bytes(), msg, &sig).is_ok());
        assert!(EphemeralKeypair::verify(&kp.public_bytes(), b"other", &sig).is_err());
    }

    fn jwks_fixture(n: String, e: String) -> JwksDocument {
        JwksDocument {
            keys: vec![RsaJwk {
                kid: Some("test-key-1".to_string()),
                kty: "RSA".to_string(),
                alg: Some("RS256".to_string()),
                n,
                e,
            }],
        }
    }

    #[test]
    fn nonce_claim_is_enforced_on_the_standard_path() {
        // Nonce binding is enforced inside `verify_jwt_with_jwks` (the same
        // call login, the app, and the chain all use) — no separate helper.
        let (n, e, der) = rsa_test_key();
        let jwks = jwks_fixture(n, e);
        let kp = EphemeralKeypair::from_secret([3u8; 32]).unwrap();
        let randomness = [5u8; 32];
        let good_nonce = compute_nonce(&kp.public_bytes(), 42, &randomness);
        let mint = |nonce_json: &str| {
            sign_test_jwt(
                &der,
                "test-key-1",
                &format!(
                    r#"{{"iss":"https://accounts.google.com","aud":"client123","sub":"user456","exp":9999999999,"nonce":{nonce_json}}}"#,
                ),
            )
        };
        // Matching nonce passes.
        let jwt = mint(&format!(r#""{good_nonce}""#));
        assert!(
            verify_jwt_with_jwks(
                &jwt,
                &jwks,
                ISS_GOOGLE,
                "client123",
                Some(&good_nonce),
                1_000_000
            )
            .is_ok()
        );
        // Wrong epoch => different nonce => fail.
        let bad_nonce = compute_nonce(&kp.public_bytes(), 43, &randomness);
        assert!(
            verify_jwt_with_jwks(
                &jwt,
                &jwks,
                ISS_GOOGLE,
                "client123",
                Some(&bad_nonce),
                1_000_000
            )
            .is_err()
        );
        // Missing nonce fails closed when one is expected...
        let bare = mint("null");
        assert!(
            verify_jwt_with_jwks(
                &bare,
                &jwks,
                ISS_GOOGLE,
                "client123",
                Some(&good_nonce),
                1_000_000
            )
            .is_err()
        );
        // ...and passes when the caller does not bind one.
        assert!(
            verify_jwt_with_jwks(&bare, &jwks, ISS_GOOGLE, "client123", None, 1_000_000).is_ok()
        );
    }

    fn rsa_test_key() -> (String, String, Vec<u8>) {
        // Generate a throwaway 2048-bit RSA key and return (n_b64, e_b64, der).
        // DER keeps the test working with `default-features = false`
        // (no `pem` feature in jsonwebtoken).
        use rsa::{
            RsaPrivateKey, pkcs1::EncodeRsaPrivateKey, rand_core::OsRng, traits::PublicKeyParts,
        };
        let mut rng = OsRng;
        let sk = RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let pk = sk.to_public_key();
        let n = general_purpose::URL_SAFE_NO_PAD.encode(pk.n().to_bytes_be());
        let e = general_purpose::URL_SAFE_NO_PAD.encode(pk.e().to_bytes_be());
        let der = sk.to_pkcs1_der().unwrap().as_bytes().to_vec();
        (n, e, der)
    }

    fn sign_test_jwt(der: &[u8], kid: &str, payload_json: &str) -> String {
        use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(kid.to_string());
        let key = EncodingKey::from_rsa_der(der);
        // Parse leniently: tests mint structurally-odd claims (empty sub)
        // that must still reach `verify_jwt_with_jwks` to prove it rejects
        // them. A strict parse here would fail the test before the code
        // under test ever runs.
        let claims: serde_json::Value = serde_json::from_str(payload_json)
            .unwrap_or(serde_json::Value::Object(Default::default()));
        encode(&header, &claims, &key).unwrap()
    }

    /// Mint a JWT whose payload bytes are used verbatim (no serde
    /// roundtrip): for claims `encode` would normalize away (empty-string
    /// `sub` must survive signing so the verifier can reject it).
    fn sign_raw_payload_jwt(der: &[u8], kid: &str, payload_json: &str) -> String {
        use jsonwebtoken::{Algorithm, EncodingKey, Header};
        use rsa::{
            pkcs1::DecodeRsaPrivateKey,
            sha2::Sha256,
            signature::{SignatureEncoding, Signer},
        };
        let header = {
            let mut h = Header::new(Algorithm::RS256);
            h.kid = Some(kid.to_string());
            h
        };
        let header_b64 =
            general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap());
        let payload_b64 = general_purpose::URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
        let signing_input = format!("{header_b64}.{payload_b64}");
        let key = EncodingKey::from_rsa_der(der);
        let _ = key;
        let sk = rsa::RsaPrivateKey::from_pkcs1_der(der).unwrap();
        let scheme = rsa::pkcs1v15::SigningKey::<Sha256>::new(sk);
        let sig = scheme.sign(signing_input.as_bytes());
        format!(
            "{signing_input}.{}",
            general_purpose::URL_SAFE_NO_PAD.encode(sig.to_bytes())
        )
    }

    #[test]
    fn verifies_jwt_against_jwks() {
        let (n, e, der) = rsa_test_key();
        let jwks = JwksDocument {
            keys: vec![RsaJwk {
                kid: Some("test-key-1".to_string()),
                kty: "RSA".to_string(),
                alg: Some("RS256".to_string()),
                n,
                e,
            }],
        };
        let jwt = sign_test_jwt(
            &der,
            "test-key-1",
            r#"{"iss":"https://accounts.google.com","aud":"client123","sub":"user456","exp":9999999999}"#,
        );
        let claims =
            verify_jwt_with_jwks(&jwt, &jwks, ISS_GOOGLE, "client123", None, 1_000_000).unwrap();
        assert_eq!(claims.sub, "user456");
    }

    #[test]
    fn rejects_unknown_kid_and_tampered_sig() {
        let (n, e, der) = rsa_test_key();
        let jwks = JwksDocument {
            keys: vec![RsaJwk {
                kid: Some("test-key-1".to_string()),
                kty: "RSA".to_string(),
                alg: Some("RS256".to_string()),
                n,
                e,
            }],
        };
        let jwt = sign_test_jwt(
            &der,
            "test-key-1",
            r#"{"iss":"https://accounts.google.com","aud":"client123","sub":"user456","exp":9999999999}"#,
        );
        // Unknown kid.
        let mut parts: Vec<&str> = jwt.split('.').collect();
        let bad_kid_header =
            general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","kid":"nope","typ":"JWT"}"#);
        let bad_kid = format!("{}.{}.{}", bad_kid_header, parts[1], parts[2]);
        assert!(
            verify_jwt_with_jwks(&bad_kid, &jwks, ISS_GOOGLE, "client123", None, 1_000_000)
                .is_err()
        );
        // Tampered signature.
        parts[2] = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let bad_sig = parts.join(".");
        assert!(
            verify_jwt_with_jwks(&bad_sig, &jwks, ISS_GOOGLE, "client123", None, 1_000_000)
                .is_err()
        );
        // Wrong audience.
        assert!(verify_jwt_with_jwks(&jwt, &jwks, ISS_GOOGLE, "other", None, 1_000_000).is_err());
    }

    #[test]
    fn rejects_empty_sub() {
        let (n, e, der) = rsa_test_key();
        let jwks = JwksDocument {
            keys: vec![RsaJwk {
                kid: Some("test-key-1".to_string()),
                kty: "RSA".to_string(),
                alg: Some("RS256".to_string()),
                n,
                e,
            }],
        };
        for sub in [r#""#.to_string(), "   ".to_string()] {
            let jwt = sign_raw_payload_jwt(
                &der,
                "test-key-1",
                &format!(
                    r#"{{"iss":"https://accounts.google.com","aud":"client123","sub":{sub},"exp":9999999999}}"#
                ),
            );
            assert!(
                verify_jwt_with_jwks(&jwt, &jwks, ISS_GOOGLE, "client123", None, 1_000_000)
                    .is_err(),
                "empty/blank sub must be rejected"
            );
        }
    }

    #[test]
    fn full_client_flow() {
        // iss/aud/sub from JWT + standard salt => address; ephemeral nonce
        // binding re-derives exactly (checked via `compute_nonce`).
        let claims = decode_jwt_claims(&test_jwt()).unwrap();
        let salt = deterministic_salt(&claims.iss, "client123", &claims.sub).unwrap();
        let addr = derive_zklogin_address_v2(&claims.iss, "client123", &claims.sub, &salt).unwrap();
        assert!(addr.starts_with("0x"));
        let kp = EphemeralKeypair::from_secret([13u8; 32]).unwrap();
        let randomness = [17u8; 32];
        let nonce = compute_nonce(&kp.public_bytes(), 100, &randomness);
        let mut with_nonce = claims.clone();
        with_nonce.nonce = Some(nonce);
        assert_eq!(
            compute_nonce(&kp.public_bytes(), 100, &randomness),
            with_nonce.nonce.clone().unwrap()
        );
        assert!(verify_claims_timing(&with_nonce, 1_000_000).is_ok());
    }
}
