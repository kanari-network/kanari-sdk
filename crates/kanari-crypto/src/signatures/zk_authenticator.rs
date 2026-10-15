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

/// Chain-level admission policy for zkLogin senders (fail-closed).
///
/// The synchronous `verify_signature` dispatch carries no chain context, yet
/// session authentication has no key material to check against — so every
/// mode must be bound to chain trust roots configured here and installed
/// once at node startup with [`install_zklogin_admission_policy`] (see
/// [`zklogin_admission_policy_from_env`]). While no policy is installed,
/// [`verify_zklogin_tx_signature`] rejects every zkLogin bundle.
///
/// - Proof mode: `vk_pin` MUST hold the ceremony VK fingerprint
///   ([`crate::signatures::groth16::vk_fingerprint`] of the MPC-ceremony
///   verifying key). `None` rejects every proof bundle: an attacker can
///   always run their own Groth16 setup, so an unpinned verifier is an
///   impersonation oracle (CWE-347).
/// - JWT mode: `allowed_issuers`/`allowed_audiences` MUST be non-empty and
///   `pinned_jwks` MUST hold the chain-pinned provider JWKS snapshot. The
///   bundle-supplied JWKS is ignored entirely and the bundle `iss`/`aud`
///   must each match the allow-lists; otherwise a bundle submitting its own
///   JWKS could mint arbitrary claims (including a victim's `sub` + `salt`
///   replayed from an earlier on-chain bundle) and impersonate the victim
///   (CWE-345). Empty/missing values reject every JWT bundle.
/// - `current_epoch`: validator epoch clock for the `max_epoch` bound.
///   Advance it via [`set_zklogin_chain_epoch`] as the chain progresses.
///   Proof-mode sessions carry no JWT `exp`, so this is their ONLY
///   timeliness bound at admission (Move `verify_session` re-enforces it
///   against `tx_context::epoch` at execution).
/// - `replay_ttl_secs`: TTL of the process-global admission replay guard
///   ([`record_zklogin_admission`], consulted at mempool admission);
///   `0` disables it. Durable replay safety still comes from the mempool
///   pending-duplicate check, the sender-nonce watermark, and the
///   committed-transaction index.
#[derive(Debug, Clone)]
pub struct ZkLoginAdmissionPolicy {
    /// Ceremony VK fingerprint. REQUIRED to admit proof-mode bundles.
    pub vk_pin: Option<[u8; 32]>,
    /// Chain-approved OIDC issuers. Empty rejects all JWT bundles.
    pub allowed_issuers: Vec<String>,
    /// Chain-approved OAuth client IDs. Empty rejects all JWT bundles.
    pub allowed_audiences: Vec<String>,
    /// Chain-pinned provider JWKS snapshot. `None` rejects all JWT bundles.
    pub pinned_jwks: Option<JwksDocument>,
    /// Validator epoch clock for the `max_epoch` bound.
    pub current_epoch: u64,
    /// TTL of the process-global admission replay guard (`0` = disabled).
    pub replay_ttl_secs: u64,
}

impl ZkLoginAdmissionPolicy {
    /// AdmitOptions carrying exactly this policy's trust roots, for use
    /// with [`admit_zklogin_tx`] and [`verify_zklogin_authenticator_admitted`].
    pub fn admit_options(&self) -> AdmitOptions {
        AdmitOptions {
            vk_pin: self.vk_pin,
            allowed_issuers: self.allowed_issuers.clone(),
            allowed_audiences: self.allowed_audiences.clone(),
            pinned_jwks: self.pinned_jwks.clone(),
        }
    }
}

/// Process-global admission policy (see [`ZkLoginAdmissionPolicy`]).
static ADMISSION_POLICY: std::sync::RwLock<Option<ZkLoginAdmissionPolicy>> =
    std::sync::RwLock::new(None);

/// Process-global admission replay guard, recreated on every
/// [`install_zklogin_admission_policy`] with that policy's TTL.
static ADMISSION_REPLAY: std::sync::OnceLock<std::sync::Mutex<ReplayCache>> =
    std::sync::OnceLock::new();

fn admission_replay() -> &'static std::sync::Mutex<ReplayCache> {
    ADMISSION_REPLAY.get_or_init(|| std::sync::Mutex::new(ReplayCache::new(0)))
}

fn read_policy() -> Option<ZkLoginAdmissionPolicy> {
    ADMISSION_POLICY
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

/// Install (or replace) the process-global zkLogin admission policy.
///
/// Nodes call this once at startup from chain config (see
/// [`zklogin_admission_policy_from_env`]). Re-installing resets the
/// process-global replay guard with the new TTL.
pub fn install_zklogin_admission_policy(policy: ZkLoginAdmissionPolicy) {
    if let Ok(mut guard) = ADMISSION_POLICY.write() {
        *guard = Some(policy.clone());
    }
    if let Ok(mut replay) = admission_replay().lock() {
        *replay = ReplayCache::new(policy.replay_ttl_secs);
    }
}

/// Remove the process-global admission policy (tests / reconfiguration).
/// While unset, [`verify_zklogin_tx_signature`] rejects every bundle.
pub fn clear_zklogin_admission_policy() {
    if let Ok(mut guard) = ADMISSION_POLICY.write() {
        *guard = None;
    }
    if let Ok(mut replay) = admission_replay().lock() {
        *replay = ReplayCache::new(0);
    }
}

/// Snapshot of the currently installed admission policy, if any.
pub fn zklogin_admission_policy() -> Option<ZkLoginAdmissionPolicy> {
    read_policy()
}

/// Advance the validator epoch clock used for the `max_epoch` bound.
/// Nodes should call this as the chain progresses; while unset, dispatch
/// enforces against epoch `0` (any session is live).
pub fn set_zklogin_chain_epoch(epoch: u64) {
    if let Ok(mut guard) = ADMISSION_POLICY.write()
        && let Some(policy) = guard.as_mut()
    {
        policy.current_epoch = epoch;
    }
}

/// Build an admission policy from process environment (node startup).
///
/// - `KANARI_ZKLOGIN_VK_PIN`: 64-hex (0x-tolerant) ceremony VK fingerprint.
/// - `KANARI_ZKLOGIN_ALLOWED_ISSUERS` / `KANARI_ZKLOGIN_ALLOWED_AUDIENCES`:
///   comma-separated allow-lists.
/// - `KANARI_ZKLOGIN_JWKS_JSON` or `KANARI_ZKLOGIN_JWKS_PATH`: chain-pinned
///   provider JWKS snapshot (setting both is an error).
/// - `KANARI_ZKLOGIN_REPLAY_TTL_SECS`: replay-guard TTL (default 3600).
/// - `KANARI_ZKLOGIN_CHAIN_EPOCH`: starting validator epoch (default 0).
///
/// Returns `Ok(None)` when none of the variables are set (caller should
/// keep admission disabled and log that zkLogin stays rejected). Returns
/// `Err` on any malformed value so startup fails closed instead of running
/// with a half-configured trust root.
pub fn zklogin_admission_policy_from_env() -> Result<Option<ZkLoginAdmissionPolicy>, String> {
    zklogin_admission_policy_from_map(|name| std::env::var(name).ok().map(|v| v.trim().to_string()))
}

/// Build an admission policy from a key-value source (the `_env` wrapper
/// above passes process env; tests pass a map — no env mutation needed).
pub fn zklogin_admission_policy_from_map(
    get: impl Fn(&str) -> Option<String>,
) -> Result<Option<ZkLoginAdmissionPolicy>, String> {
    let vk_pin_raw = get("KANARI_ZKLOGIN_VK_PIN");
    let iss_raw = get("KANARI_ZKLOGIN_ALLOWED_ISSUERS");
    let aud_raw = get("KANARI_ZKLOGIN_ALLOWED_AUDIENCES");
    let jwks_json = get("KANARI_ZKLOGIN_JWKS_JSON");
    let jwks_path = get("KANARI_ZKLOGIN_JWKS_PATH");
    let ttl_raw = get("KANARI_ZKLOGIN_REPLAY_TTL_SECS");
    let epoch_raw = get("KANARI_ZKLOGIN_CHAIN_EPOCH");

    if vk_pin_raw.is_none()
        && iss_raw.is_none()
        && aud_raw.is_none()
        && jwks_json.is_none()
        && jwks_path.is_none()
        && ttl_raw.is_none()
        && epoch_raw.is_none()
    {
        return Ok(None);
    }

    let vk_pin = vk_pin_raw
        .map(|s| {
            let hex = s.trim_start_matches("0x").trim_start_matches("0X");
            hex::decode(hex)
                .map_err(|e| format!("KANARI_ZKLOGIN_VK_PIN is not hex: {e}"))?
                .try_into()
                .map_err(|_| "KANARI_ZKLOGIN_VK_PIN must be 32 bytes (64 hex chars)".to_string())
        })
        .transpose()?;

    let split_list = |s: String| {
        s.split(',')
            .map(|part| part.trim().to_string())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
    };
    let allowed_issuers = iss_raw.map(split_list).unwrap_or_default();
    let allowed_audiences = aud_raw.map(split_list).unwrap_or_default();

    if jwks_json.is_some() && jwks_path.is_some() {
        return Err(
            "set only one of KANARI_ZKLOGIN_JWKS_JSON / KANARI_ZKLOGIN_JWKS_PATH".to_string(),
        );
    }
    let pinned_jwks = jwks_json
        .map(|json| {
            serde_json::from_str::<JwksDocument>(&json)
                .map_err(|e| format!("KANARI_ZKLOGIN_JWKS_JSON is not a JWKS document: {e}"))
        })
        .transpose()?
        .or(jwks_path
            .map(|path| {
                let bytes = std::fs::read(&path)
                    .map_err(|e| format!("cannot read KANARI_ZKLOGIN_JWKS_PATH {path}: {e}"))?;
                serde_json::from_slice::<JwksDocument>(&bytes).map_err(|e| {
                    format!("KANARI_ZKLOGIN_JWKS_PATH {path} is not a JWKS document: {e}")
                })
            })
            .transpose()?);

    let replay_ttl_secs = ttl_raw
        .map(|s| {
            s.parse::<u64>()
                .map_err(|_| "KANARI_ZKLOGIN_REPLAY_TTL_SECS must be a u64".to_string())
        })
        .transpose()?
        .unwrap_or(3600);
    let current_epoch = epoch_raw
        .map(|s| {
            s.parse::<u64>()
                .map_err(|_| "KANARI_ZKLOGIN_CHAIN_EPOCH must be a u64".to_string())
        })
        .transpose()?
        .unwrap_or(0);

    Ok(Some(ZkLoginAdmissionPolicy {
        vk_pin,
        allowed_issuers,
        allowed_audiences,
        pinned_jwks,
        current_epoch,
        replay_ttl_secs,
    }))
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
///
/// SECURITY: this raw verifier is NOT admission-safe — proof mode trusts
/// the bundle-supplied VK and JWT mode trusts the bundle-supplied JWKS
/// and `iss`/`aud`. It exists for offline tooling and tests. Ingestion
/// (mempool/node) MUST use [`admit_zklogin_tx`] or the `verify_signature`
/// dispatch ([`verify_zklogin_tx_signature`]), which bind both modes to
/// chain trust roots.
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

/// Admission-grade verification: like [`verify_zklogin_authenticator`] but
/// bound to chain trust roots from `opts` (used by [`admit_zklogin_tx`]
/// and the `verify_signature` dispatch).
///
/// - Proof mode: `opts.vk_pin` is REQUIRED; `None` rejects before any
///   pairing work, so toxic-waste setups can never pass.
/// - JWT mode: `opts.pinned_jwks` is REQUIRED (the bundle JWKS is ignored
///   entirely) and the bundle `iss`/`aud` must each appear in the
///   corresponding allow-list; empty/missing values reject. This closes
///   the bundle-JWKS impersonation: without a chain-pinned key set, a
///   bundle could mint arbitrary claims (victim `sub` + replayed `salt`)
///   under its own key and derive the victim address.
///
/// Ephemeral-signature, nonce-binding, `exp`/`sub` and address-derivation
/// checks are unchanged from the raw verifier. `max_epoch`, replay and
/// sender-match stay the caller's job (see [`admit_zklogin_tx`]).
pub fn verify_zklogin_authenticator_admitted(
    tx_hash: &[u8],
    auth: &ZkLoginAuthenticator,
    now_secs: u64,
    opts: &AdmitOptions,
) -> Result<VerifiedZkLogin, SignatureError> {
    EphemeralKeypair::verify(&auth.ephemeral_pubkey, tx_hash, &auth.ephemeral_sig)?;
    match &auth.kind {
        ZkAuthKind::Jwt(jwt) => {
            if !opts.allowed_issuers.iter().any(|iss| iss == &jwt.iss) {
                return Err(SignatureError::VerificationFailed);
            }
            if !opts.allowed_audiences.iter().any(|aud| aud == &jwt.aud) {
                return Err(SignatureError::VerificationFailed);
            }
            let pinned = opts
                .pinned_jwks
                .as_ref()
                .ok_or(SignatureError::VerificationFailed)?;
            // Nonce must bind THIS session key (not just any valid login).
            let expected = compute_nonce(&auth.ephemeral_pubkey, auth.max_epoch, &jwt.randomness);
            let claims = verify_jwt_with_jwks(
                &jwt.jwt,
                pinned,
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
        ZkAuthKind::Proof(proof) => {
            let pin = opts
                .vk_pin
                .as_ref()
                .ok_or(SignatureError::VerificationFailed)?;
            verify_proof_kind(
                &auth.ephemeral_pubkey,
                &auth.ephemeral_sig,
                tx_hash,
                auth.max_epoch,
                proof,
                Some(pin),
            )
        }
    }
}

/// Proof-mode verification shared by the bare verifier above (unpinned —
/// offline tooling only; NEVER admission) and the admitted verifier above
/// plus [`admit_zklogin_tx`] (pinned — REQUIRED).
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
/// This is the function every node admission path reaches through
/// `verify_signature_with_curve(..., CurveType::ZkLogin)` (mempool,
/// consensus vertex checks, block verification): `message` is the
/// transaction hash, `signature` the JSON bundle above.
///
/// Fail-closed admission through the process-global
/// [`ZkLoginAdmissionPolicy`]: while no policy is installed, EVERY bundle
/// is rejected. With a policy, this enforces bundle v1, `max_epoch`
/// against the policy epoch, mode verification bound to chain trust
/// roots (pinned ceremony VK for proof mode; allow-listed `iss`/`aud`
/// plus the chain-pinned JWKS for JWT mode), and finally the
/// recovered-address match. Returns `Ok(true)` only on full success; a
/// well-formed bundle for a different sender returns `Ok(false)` (like
/// other curves), malformed input returns `Err`.
///
/// The replay guard deliberately does NOT run here: this function is
/// re-executed for every stage that sees a fresh deserialized copy of a
/// transaction (simulation → mempool inside one RPC request, gossip
/// ingress, vertex validation, commit), and a guard inside verification
/// would reject those legitimate re-verifications and stall the node on
/// the first zkLogin transaction. Resubmission is bounded at the
/// admission boundary instead — see [`record_zklogin_admission`], which
/// mempool admission calls once per fully verified hash.
///
/// Nodes install the policy once at startup (see
/// [`zklogin_admission_policy_from_env`]) and advance its epoch with
/// [`set_zklogin_chain_epoch`]. Offline tooling that has no chain context
/// must use [`verify_zklogin_authenticator`] or [`admit_zklogin_tx`] with
/// explicit options instead — never this dispatch for proof bundles.
pub fn verify_zklogin_tx_signature(
    address_hex: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, SignatureError> {
    let policy = read_policy().ok_or(SignatureError::VerificationFailed)?;
    let bundle: ZkLoginTxSignature = serde_json::from_slice(signature).map_err(|_| {
        SignatureError::InvalidFormat("zkLogin bundle is not valid JSON".to_string())
    })?;
    if bundle.v != 1 {
        return Err(SignatureError::InvalidFormat(format!(
            "unsupported zkLogin bundle version {}",
            bundle.v
        )));
    }
    // Wall-clock here matches the CLI/JWT world (Google `exp`); `max_epoch`
    // is enforced against the policy (validator) epoch below.
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| SignatureError::VerificationFailed)?;
    check_max_epoch(bundle.auth.max_epoch, policy.current_epoch)?;
    let verified = verify_zklogin_authenticator_admitted(
        message,
        &bundle.auth,
        now_secs,
        &policy.admit_options(),
    )?;
    Ok(addresses_equal(&verified.address, address_hex))
}

/// Record a fully verified zkLogin transaction at the ADMISSION boundary
/// (mempool ingress) — the single choke point of the process-global
/// replay guard.
///
/// WHY HERE AND NOT INSIDE [`verify_zklogin_tx_signature`]: signature
/// verification legitimately runs several times per transaction on fresh
/// deserialized copies — simulation → mempool in one RPC request, gossip
/// ingress from other validators, vertex validation, and commit. A guard
/// inside verification rejects those later legitimate verifications and
/// stalls the node. Every ingress path funnels through mempool
/// admission, which calls this once per fully verified hash, so
/// resubmission of an already-admitted transaction stays bounded.
///
/// Returns `false` when the hash was already admitted within the policy
/// TTL, or when no policy is installed (fail closed). `replay_ttl_secs
/// == 0` (tests only) disables the guard — every hash counts as fresh.
pub fn record_zklogin_admission(tx_hash: &[u8]) -> bool {
    let Some(policy) = read_policy() else {
        return false;
    };
    if policy.replay_ttl_secs == 0 {
        return true;
    }
    let Ok(now_secs) = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
    else {
        // Clock before the epoch: fail closed, same as verification.
        return false;
    };
    admission_replay()
        .lock()
        .map(|mut replay| replay.check_and_insert(tx_hash, now_secs))
        .unwrap_or(false)
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
    /// Chain-approved OIDC issuers. A JWT bundle whose `iss` is not listed
    /// is rejected. Empty rejects all JWT bundles.
    pub allowed_issuers: Vec<String>,
    /// Chain-approved OAuth client IDs. A JWT bundle whose `aud` is not
    /// listed is rejected. Empty rejects all JWT bundles.
    pub allowed_audiences: Vec<String>,
    /// Chain-pinned provider JWKS snapshot. REQUIRED for JWT mode; the
    /// bundle-supplied JWKS is ignored entirely. `None` + JWT bundle =>
    /// rejection, so bundles minting claims under their own key can never
    /// pass admission.
    pub pinned_jwks: Option<JwksDocument>,
}

/// Fail-closed admission for a `ZkLogin:` sender — THE function ingestion
/// (mempool/node) must call instead of the raw pieces. Order (cheap,
/// state-light checks first):
/// 1. Bundle parses as v1 (else `InvalidFormat`).
/// 2. `max_epoch >= current_epoch` (else `Expired`) — the ONLY timeliness
///    bound proof mode has.
/// 3. Replay: same `tx_hash` admitted inside the cache TTL
///    => `VerificationFailed`.
/// 4. Mode verify: JWT (pinned JWKS sig/kid, allow-listed iss/aud, exp,
///    nonce, address) or Proof (pinned VK REQUIRED + proof + expected
///    address).
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
    let verified = verify_zklogin_authenticator_admitted(tx_hash, &bundle.auth, now_secs, opts)?;
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

    /// Serializes tests that install the process-global admission policy.
    static POLICY_TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn fixture_jwks() -> JwksDocument {
        let (n_b64, e_b64, _) = local_rsa();
        serde_json::from_str(&format!(
            "{{\"keys\":[{{\"kty\":\"RSA\",\"kid\":\"local-1\",\"alg\":\"RS256\",\"n\":\"{n_b64}\",\"e\":\"{e_b64}\"}}]}}"
        ))
        .unwrap()
    }

    /// Strict admission options for the JWT fixture: chain-pinned local
    /// JWKS plus allow-lists covering exactly the fixture claims.
    fn admit_jwt_opts() -> AdmitOptions {
        AdmitOptions {
            vk_pin: None,
            allowed_issuers: vec![ISS_GOOGLE.to_string()],
            allowed_audiences: vec![TEST_AUD.to_string()],
            pinned_jwks: Some(fixture_jwks()),
        }
    }

    /// Install the fixture JWT admission policy globally (dispatch path)
    /// and return the serial-test guard. Callers MUST
    /// [`clear_zklogin_admission_policy`] before dropping the guard.
    fn install_fixture_jwt_policy() -> std::sync::MutexGuard<'static, ()> {
        let guard = POLICY_TEST_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        install_zklogin_admission_policy(ZkLoginAdmissionPolicy {
            vk_pin: None,
            allowed_issuers: vec![ISS_GOOGLE.to_string()],
            allowed_audiences: vec![TEST_AUD.to_string()],
            pinned_jwks: Some(fixture_jwks()),
            current_epoch: 0,
            // The dispatch path never consults the replay guard; TTL 0
            // keeps `record_zklogin_admission` (the admission boundary)
            // inert unless a test opts into the guard explicitly.
            replay_ttl_secs: 0,
        });
        guard
    }

    fn fixture_jwt_auth() -> JwtAuth {
        JwtAuth {
            jwt: mint_local_jwt(),
            jwks: fixture_jwks(),
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
    /// bundle for `ZkLogin:0x...` verifies under the installed chain
    /// policy (pinned JWKS + allow-lists), and every tamper fails closed.
    #[test]
    fn tagged_dispatch_accepts_and_rejects() {
        use crate::signatures::verify_signature;

        let _guard = install_fixture_jwt_policy();
        // Fresh bundle per tx hash (the ephemeral signature binds the hash).
        let (bundle, sender) = admit_jwt_case(b"kanari-tx-hash-fixture-000000000002");
        let tagged = format!("ZkLogin:{sender}");

        // The exact path the mempool takes (tagged dispatch).
        assert!(
            verify_signature(&tagged, b"kanari-tx-hash-fixture-000000000002", &bundle).unwrap()
        );

        // Wrong sender address must fail (binding enforced). Well-formed
        // but non-matching verifies return Ok(false), like other curves.
        // (Fresh hash: the replay guard must not be what rejects this.)
        let (bundle2, _) = admit_jwt_case(b"kanari-tx-hash-fixture-000000000007");
        assert!(
            !verify_signature(
                "ZkLogin:0x0000000000000000000000000000000000000000000000000000000000000000",
                b"kanari-tx-hash-fixture-000000000007",
                &bundle2
            )
            .unwrap()
        );
        // Corrupt JSON must fail, not default-accept.
        assert!(
            verify_signature(&tagged, b"kanari-tx-hash-fixture-000000000002", b"{nope").is_err()
        );
        // Future bundle version must fail closed.
        let mut v2 = serde_json::from_slice::<serde_json::Value>(&bundle).unwrap();
        v2["v"] = serde_json::json!(2);
        assert!(
            verify_signature(
                &tagged,
                b"kanari-tx-hash-fixture-000000000002",
                serde_json::to_vec(&v2).unwrap().as_slice()
            )
            .is_err()
        );
        // Untagged address never routes here.
        assert!(
            verify_signature(&sender, b"kanari-tx-hash-fixture-000000000002", &bundle).is_err()
        );
        clear_zklogin_admission_policy();
    }

    /// With no chain policy installed, the dispatch rejects every zkLogin
    /// bundle — there is no trust root to verify against.
    #[test]
    fn dispatch_rejects_everything_without_policy() {
        use crate::signatures::verify_signature;

        let _guard = POLICY_TEST_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        clear_zklogin_admission_policy();
        let (jwt_bundle, jwt_sender) = admit_jwt_case(b"kanari-nopolicy-jwt-000000000001");
        assert!(
            verify_signature(
                &format!("ZkLogin:{jwt_sender}"),
                b"kanari-nopolicy-jwt-000000000001",
                &jwt_bundle
            )
            .is_err()
        );
        let proof_bundle = admit_proof_nosetup_bundle(b"kanari-nopolicy-proof-000000000001");
        assert!(
            verify_signature(
                "ZkLogin:0xabc",
                b"kanari-nopolicy-proof-000000000001",
                &proof_bundle
            )
            .is_err()
        );
    }

    /// Rogue-JWKS impersonation (CWE-345): the attacker mints claims under
    /// their OWN key — victim `iss`/`sub`, victim `salt` scraped from an
    /// earlier on-chain bundle, nonce bound to the attacker's session key —
    /// and derives the victim address. The raw offline verifier accepts the
    /// self-consistent bundle, but admission under the chain-pinned JWKS
    /// rejects it (unknown `kid`).
    #[test]
    fn dispatch_rejects_rogue_jwks_impersonation() {
        use crate::signatures::verify_signature;

        // Attacker RSA key: DIFFERENT seed from the fixture (pinned) key.
        let (a_n, a_e, a_der) = {
            use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
            // Reuse the deterministic generator via a fresh OnceLock-free
            // path: duplicate the xorshift construction with another seed.
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
            use rsa::pkcs1::EncodeRsaPrivateKey as _;
            use rsa::traits::PublicKeyParts as _;
            let mut rng = Xor(0xBAD);
            let sk = rsa::RsaPrivateKey::new(&mut rng, 2048).unwrap();
            let pk = rsa::RsaPublicKey::from(&sk);
            (
                URL_SAFE_NO_PAD.encode(pk.n().to_bytes_be()),
                URL_SAFE_NO_PAD.encode(pk.e().to_bytes_be()),
                sk.to_pkcs1_der().unwrap().as_bytes().to_vec(),
            )
        };
        let attacker_jwks: JwksDocument = serde_json::from_str(&format!(
            "{{\"keys\":[{{\"kty\":\"RSA\",\"kid\":\"attacker-1\",\"alg\":\"RS256\",\"n\":\"{a_n}\",\"e\":\"{a_e}\"}}]}}"
        ))
        .unwrap();

        // Attacker's session key; victim's sub + salt (public on-chain).
        let attacker_kp = EphemeralKeypair::from_secret([43u8; 32]).unwrap();
        let attacker_pub = attacker_kp.public_bytes();
        let attacker_rand = [11u8; 32];
        let victim_salt = [9u8; 32];
        let victim_address =
            derive_zklogin_address_v2(ISS_GOOGLE, TEST_AUD, "1234", &victim_salt).unwrap();
        let nonce = crate::signatures::zklogin::compute_nonce(&attacker_pub, 1000, &attacker_rand);
        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        header.kid = Some("attacker-1".to_string());
        let claims = serde_json::json!({
            "iss": ISS_GOOGLE,
            "aud": TEST_AUD,
            "sub": "1234",
            "exp": 2000000000u64,
            "nonce": nonce,
        });
        let attacker_jwt = jsonwebtoken::encode(
            &header,
            &claims,
            &jsonwebtoken::EncodingKey::from_rsa_der(&a_der),
        )
        .unwrap();

        let tx_hash = b"kanari-rogue-jwks-attack-000000000001";
        let auth = ZkLoginAuthenticator {
            ephemeral_pubkey: attacker_pub,
            ephemeral_sig: attacker_kp.sign(tx_hash).as_slice().try_into().unwrap(),
            max_epoch: 1000,
            kind: ZkAuthKind::Jwt(JwtAuth {
                jwt: attacker_jwt,
                jwks: attacker_jwks,
                iss: ISS_GOOGLE.to_string(),
                aud: TEST_AUD.to_string(),
                randomness: attacker_rand,
                salt: victim_salt,
            }),
        };
        // The raw offline verifier sees a self-consistent bundle and
        // recovers the VICTIM address: this is the impersonation primitive.
        let raw = verify_zklogin_authenticator(tx_hash, &auth, 1_700_000_000).unwrap();
        assert_eq!(raw.address, victim_address);
        let bundle = encode_zklogin_tx_signature(&auth).unwrap();

        // Admission under the chain-pinned JWKS rejects it.
        let _guard = install_fixture_jwt_policy();
        assert!(verify_signature(&format!("ZkLogin:{victim_address}"), tx_hash, &bundle).is_err());
        clear_zklogin_admission_policy();
    }

    /// Allow-list enforcement: fixture-valid bundles are rejected when the
    /// chain policy does not list their `iss`/`aud`.
    #[test]
    fn dispatch_rejects_disallowed_issuer_and_audience() {
        use crate::signatures::verify_signature;

        let _guard = POLICY_TEST_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // Policy allows a DIFFERENT issuer/audience than the bundle's.
        install_zklogin_admission_policy(ZkLoginAdmissionPolicy {
            vk_pin: None,
            allowed_issuers: vec!["https://other.example".to_string()],
            allowed_audiences: vec!["other-client".to_string()],
            pinned_jwks: Some(fixture_jwks()),
            current_epoch: 0,
            replay_ttl_secs: 0,
        });
        let (bundle, sender) = admit_jwt_case(b"kanari-allowlist-000000000000000001");
        assert!(
            verify_signature(
                &format!("ZkLogin:{sender}"),
                b"kanari-allowlist-000000000000000001",
                &bundle
            )
            .is_err()
        );
        clear_zklogin_admission_policy();
    }

    /// `max_epoch` is enforced on the dispatch path (an expired session
    /// never admits), while RE-verification of the same bundle must keep
    /// succeeding there: every later stage (mempool after simulation,
    /// gossip, vertex validation, commit) verifies a fresh deserialized
    /// copy. The replay guard lives at the admission boundary instead
    /// (`record_zklogin_admission`): first record fresh, second rejected.
    #[test]
    fn dispatch_enforces_expiry_and_admission_records_replay() {
        use crate::signatures::verify_signature;

        let _guard = POLICY_TEST_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        install_zklogin_admission_policy(ZkLoginAdmissionPolicy {
            vk_pin: None,
            allowed_issuers: vec![ISS_GOOGLE.to_string()],
            allowed_audiences: vec![TEST_AUD.to_string()],
            pinned_jwks: Some(fixture_jwks()),
            // Fixture bundles carry max_epoch 1000: already expired here.
            current_epoch: 1001,
            replay_ttl_secs: 3600,
        });
        let (expired_bundle, expired_sender) = admit_jwt_case(b"kanari-expired-000000000000000001");
        assert!(matches!(
            verify_signature(
                &format!("ZkLogin:{expired_sender}"),
                b"kanari-expired-000000000000000001",
                &expired_bundle
            ),
            Err(SignatureError::Expired)
        ));

        // Live epoch: verification succeeds on every call — no false replay.
        set_zklogin_chain_epoch(1000);
        let (bundle, sender) = admit_jwt_case(b"kanari-replay-dispatch-000000000001");
        let tagged = format!("ZkLogin:{sender}");
        let tx_hash = b"kanari-replay-dispatch-000000000001";
        assert!(verify_signature(&tagged, tx_hash, &bundle).unwrap());
        assert!(verify_signature(&tagged, tx_hash, &bundle).unwrap());

        // The admission boundary records once and rejects the resubmission.
        assert!(record_zklogin_admission(tx_hash));
        assert!(!record_zklogin_admission(tx_hash));
        clear_zklogin_admission_policy();
    }

    /// Attacker-ceremony impersonation (CWE-347): the attacker runs their
    /// own Groth16 setup, encodes the VICTIM address in the public inputs,
    /// and proves under the attacker VK. The pairing math checks out, but
    /// admission rejects it: the VK fingerprint is not the ceremony pin —
    /// with no pin configured at all, and with the wrong pin.
    #[test]
    fn proof_mode_rejects_attacker_ceremony_end_to_end() {
        use crate::signatures::groth16::{
            proof_to_bytes, verify_groth16_proof, vk_fingerprint, vk_to_bytes,
        };
        use crate::signatures::zklogin::{compute_nonce, derive_zklogin_address_v2};
        use crate::signatures::zklogin_circuit::{
            prove_binding, public_inputs_to_be_bytes, setup_binding_circuit,
        };
        use ark_std::rand::SeedableRng;

        let _guard = POLICY_TEST_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        clear_zklogin_admission_policy();

        let salt = [9u8; 32];
        let victim_address =
            derive_zklogin_address_v2(ISS_GOOGLE, TEST_AUD, "1234", &salt).unwrap();
        let address: [u8; 32] = hex::decode(&victim_address[2..])
            .unwrap()
            .try_into()
            .unwrap();
        // Ceremony setup (the deployment's trusted one) vs attacker setup.
        let mut ceremony_rng = ark_std::rand::rngs::StdRng::seed_from_u64(0xC0FFEE);
        let (_ceremony_pk, ceremony_vk) = setup_binding_circuit(
            ISS_GOOGLE.as_bytes(),
            TEST_AUD.as_bytes(),
            &mut ceremony_rng,
        )
        .unwrap();
        let ceremony_pin = vk_fingerprint(&ceremony_vk).unwrap();
        // Attacker setup (same circuit, independent toxic-waste randomness).
        // Its proving key proves, its VK verifies the math — but its
        // fingerprint differs from the ceremony pin, which is what saves us.
        let mut attacker_rng = ark_std::rand::rngs::StdRng::seed_from_u64(0xBAD5EED);
        let (attacker_pk, attacker_vk) = setup_binding_circuit(
            ISS_GOOGLE.as_bytes(),
            TEST_AUD.as_bytes(),
            &mut attacker_rng,
        )
        .unwrap();
        assert_ne!(
            vk_fingerprint(&attacker_vk).unwrap(),
            ceremony_pin,
            "distinct setups must have distinct fingerprints"
        );
        // Attacker proves VICTIM address + attacker-session nonce under the
        // attacker VK with the matching attacker proving key.
        let attacker_eph = EphemeralKeypair::from_secret([43u8; 32]).unwrap();
        let attacker_pub = attacker_eph.public_bytes();
        let nonce_hex = compute_nonce(&attacker_pub, 1000, &[7u8; 32]);
        let nonce: [u8; 32] = hex::decode(&nonce_hex).unwrap().try_into().unwrap();
        let attacker_proof = prove_binding(
            &attacker_pk,
            ISS_GOOGLE.as_bytes(),
            TEST_AUD.as_bytes(),
            salt,
            b"1234",
            [7u8; 32],
            attacker_pub,
            1000,
            address,
            nonce,
            &mut attacker_rng,
        )
        .unwrap();
        let attacker_vk_bytes = vk_to_bytes(&attacker_vk).unwrap();
        let inputs = public_inputs_to_be_bytes(&address, &nonce);
        let proof_bytes = proof_to_bytes(&attacker_proof).unwrap();
        // The math verifies under the attacker VK: without pinning, the
        // node would accept this as the victim. This assertion pins down
        // the exploit primitive the fix defends against.
        assert!(verify_groth16_proof(&attacker_vk_bytes, &inputs, &proof_bytes).unwrap());

        let tx_hash = b"kanari-attacker-ceremony-000000000001";
        let auth = ZkLoginAuthenticator {
            ephemeral_pubkey: attacker_pub,
            ephemeral_sig: attacker_eph.sign(tx_hash).as_slice().try_into().unwrap(),
            max_epoch: 1000,
            kind: ZkAuthKind::Proof(ProofAuth {
                vk: attacker_vk_bytes,
                inputs,
                proof: proof_bytes,
                expected_address: victim_address.clone(),
            }),
        };
        let bundle = encode_zklogin_tx_signature(&auth).unwrap();
        let tagged = format!("ZkLogin:{victim_address}");

        // No policy installed: rejected (fail-closed).
        assert!(crate::signatures::verify_signature(&tagged, tx_hash, &bundle).is_err());

        // Ceremony pin installed, attacker VK presented: rejected.
        install_zklogin_admission_policy(ZkLoginAdmissionPolicy {
            vk_pin: Some(ceremony_pin),
            allowed_issuers: vec![],
            allowed_audiences: vec![],
            pinned_jwks: None,
            current_epoch: 1000,
            replay_ttl_secs: 0,
        });
        assert!(crate::signatures::verify_signature(&tagged, tx_hash, &bundle).is_err());

        // Direct admission with the wrong pin: rejected as well.
        let mut cache = ReplayCache::new(3600);
        assert!(
            admit_zklogin_tx(
                &tagged,
                b"kanari-attacker-ceremony-000000000002",
                &encode_zklogin_tx_signature(&ZkLoginAuthenticator {
                    ephemeral_pubkey: attacker_pub,
                    ephemeral_sig: attacker_eph
                        .sign(b"kanari-attacker-ceremony-000000000002")
                        .as_slice()
                        .try_into()
                        .unwrap(),
                    max_epoch: 1000,
                    kind: auth.kind.clone(),
                })
                .unwrap(),
                &AdmitOptions {
                    vk_pin: Some([9u8; 32]),
                    allowed_issuers: vec![],
                    allowed_audiences: vec![],
                    pinned_jwks: None,
                },
                &mut cache,
                1_700_000_000,
                1000,
            )
            .is_err()
        );
        clear_zklogin_admission_policy();
    }

    /// Admission-grade JWT checks without the global: missing trust roots
    /// fail closed even when the bundle is self-consistent.
    #[test]
    fn admit_jwt_mode_requires_pinned_jwks_and_allow_lists() {
        let tx = b"kanari-admit-jwt-strict-000000000001";
        let (bundle, sender) = admit_jwt_case(tx);
        let mut cache = ReplayCache::new(3600);
        // No trust roots at all: rejected.
        assert!(
            admit_zklogin_tx(
                &sender,
                tx,
                &bundle,
                &AdmitOptions {
                    vk_pin: None,
                    allowed_issuers: vec![],
                    allowed_audiences: vec![],
                    pinned_jwks: None,
                },
                &mut cache,
                1_700_000_000,
                1000,
            )
            .is_err()
        );
        // Pinned JWKS but issuer not listed: rejected.
        let mut cache = ReplayCache::new(3600);
        assert!(
            admit_zklogin_tx(
                &sender,
                tx,
                &bundle,
                &AdmitOptions {
                    vk_pin: None,
                    allowed_issuers: vec!["https://other.example".to_string()],
                    allowed_audiences: vec![TEST_AUD.to_string()],
                    pinned_jwks: Some(fixture_jwks()),
                },
                &mut cache,
                1_700_000_000,
                1000,
            )
            .is_err()
        );
        // Pinned JWKS but audience not listed: rejected.
        let mut cache = ReplayCache::new(3600);
        assert!(
            admit_zklogin_tx(
                &sender,
                tx,
                &bundle,
                &AdmitOptions {
                    vk_pin: None,
                    allowed_issuers: vec![ISS_GOOGLE.to_string()],
                    allowed_audiences: vec!["other-client".to_string()],
                    pinned_jwks: Some(fixture_jwks()),
                },
                &mut cache,
                1_700_000_000,
                1000,
            )
            .is_err()
        );
        // Fully pinned (chain JWKS + listed iss/aud): accepted.
        let mut cache = ReplayCache::new(3600);
        assert!(
            admit_zklogin_tx(
                &sender,
                tx,
                &bundle,
                &admit_jwt_opts(),
                &mut cache,
                1_700_000_000,
                1000
            )
            .is_ok()
        );
    }

    /// Chain policy wiring: env/map parsing is fail-closed (malformed pins
    /// error, nothing configured yields no policy) and install/set-epoch
    /// round-trip through the global.
    #[test]
    fn admission_policy_config_roundtrip() {
        use std::collections::HashMap;

        // Nothing configured: no policy (dispatch stays fully rejecting).
        let empty: HashMap<String, String> = HashMap::new();
        assert!(
            zklogin_admission_policy_from_map(|k| empty.get(k).cloned())
                .unwrap()
                .is_none()
        );
        // Malformed pin fails closed instead of installing half a policy.
        let mut bad: HashMap<String, String> = HashMap::new();
        bad.insert("KANARI_ZKLOGIN_VK_PIN".to_string(), "0x1234".to_string());
        assert!(zklogin_admission_policy_from_map(|k| bad.get(k).cloned()).is_err());
        // Conflicting JWKS sources fail closed.
        let mut both: HashMap<String, String> = HashMap::new();
        both.insert(
            "KANARI_ZKLOGIN_JWKS_JSON".to_string(),
            "{\"keys\":[]}".to_string(),
        );
        both.insert(
            "KANARI_ZKLOGIN_JWKS_PATH".to_string(),
            "/tmp/whatever.json".to_string(),
        );
        assert!(zklogin_admission_policy_from_map(|k| both.get(k).cloned()).is_err());

        // Full config parses: pin + allow-lists + inline JWKS + ttl/epoch.
        let jwks_json = serde_json::to_string(&fixture_jwks()).unwrap();
        let mut full: HashMap<String, String> = HashMap::new();
        full.insert(
            "KANARI_ZKLOGIN_VK_PIN".to_string(),
            format!("0x{}", "ab".repeat(32)),
        );
        full.insert(
            "KANARI_ZKLOGIN_ALLOWED_ISSUERS".to_string(),
            "https://accounts.google.com, https://appleid.apple.com".to_string(),
        );
        full.insert(
            "KANARI_ZKLOGIN_ALLOWED_AUDIENCES".to_string(),
            "client-a".to_string(),
        );
        full.insert("KANARI_ZKLOGIN_JWKS_JSON".to_string(), jwks_json);
        full.insert(
            "KANARI_ZKLOGIN_REPLAY_TTL_SECS".to_string(),
            "60".to_string(),
        );
        full.insert("KANARI_ZKLOGIN_CHAIN_EPOCH".to_string(), "7".to_string());
        let policy = zklogin_admission_policy_from_map(|k| full.get(k).cloned())
            .unwrap()
            .expect("full config must parse");
        assert_eq!(policy.vk_pin, Some([0xabu8; 32]));
        assert_eq!(
            policy.allowed_issuers,
            vec![
                "https://accounts.google.com".to_string(),
                "https://appleid.apple.com".to_string()
            ]
        );
        assert_eq!(policy.allowed_audiences, vec!["client-a".to_string()]);
        assert!(policy.pinned_jwks.is_some());
        assert_eq!(policy.replay_ttl_secs, 60);
        assert_eq!(policy.current_epoch, 7);

        // Install + epoch advance round-trip through the global.
        let _guard = POLICY_TEST_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        install_zklogin_admission_policy(policy);
        assert!(zklogin_admission_policy().is_some());
        set_zklogin_chain_epoch(9);
        assert_eq!(zklogin_admission_policy().unwrap().current_epoch, 9);
        set_zklogin_chain_epoch(0);
        clear_zklogin_admission_policy();
        assert!(zklogin_admission_policy().is_none());
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
            &admit_jwt_opts(),
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
        let opts = admit_jwt_opts();
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
        let opts = admit_jwt_opts();
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
                &AdmitOptions {
                    vk_pin: None,
                    allowed_issuers: vec![],
                    allowed_audiences: vec![],
                    pinned_jwks: None,
                },
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
                    vk_pin: Some([1u8; 32]),
                    allowed_issuers: vec![],
                    allowed_audiences: vec![],
                    pinned_jwks: None,
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
