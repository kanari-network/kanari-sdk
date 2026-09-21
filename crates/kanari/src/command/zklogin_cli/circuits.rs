// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! `kanari zklogin setup-circuit` / `kanari zklogin prove`.
//!
//! DEMO GRADE (see `kanari_crypto::signatures::zklogin_circuit`):
//! - `setup-circuit` runs a LOCAL random Groth16 setup. The toxic waste
//!   lets whoever ran it forge proofs, so its VK must NEVER be treated as
//!   canonical. Production uses an MPC ceremony VK pinned on-chain; this
//!   command exists so developers can exercise the full prove/verify
//!   pipeline end to end today.
//! - `prove` needs the matching proving key (PK) file from setup.

use anyhow::{Context, Result};
use ark_std::rand::SeedableRng as _;
use clap::Parser;
use kanari_crypto::signatures::groth16::{
    proof_to_bytes, proving_key_from_bytes, proving_key_to_bytes, vk_fingerprint, vk_to_bytes,
};
use kanari_crypto::signatures::zklogin::{compute_nonce, derive_zklogin_address_v2};
use kanari_crypto::signatures::zklogin_circuit::{
    prove_binding, public_inputs_to_be_bytes, setup_binding_circuit,
};
use serde::{Deserialize, Serialize};

use super::session;

/// Groth16 proof package: everything a verifier (contract or human) needs,
/// self-describing so a wrong-VK proof fails loudly at pin check time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofPackage {
    pub version: u32,
    pub circuit: String,
    pub iss: String,
    pub aud: String,
    pub address: String,
    pub nonce_hex: String,
    pub max_epoch: u64,
    pub vk_hash_hex: String,
    pub vk_hex: String,
    pub inputs_hex: String,
    pub proof_hex: String,
}

#[derive(Parser, Debug)]
pub struct SetupCircuit {
    /// OIDC issuer, baked in as a circuit constant (domain separation).
    #[arg(long, default_value = "https://accounts.google.com")]
    pub iss: String,
    /// OAuth client ID, baked in as a circuit constant.
    #[arg(long)]
    pub aud: String,
    /// Where to write the proving key (tens of MB).
    #[arg(long)]
    pub out_pk: String,
    /// Where to write the verifying key.
    #[arg(long)]
    pub out_vk: String,
}

impl SetupCircuit {
    pub fn execute(&self) -> Result<()> {
        anyhow::ensure!(!self.aud.is_empty(), "pass --aud <oauth-client-id>");
        eprintln!(
            "WARNING: demo-grade local setup. The toxic waste in this run can forge proofs; \
             NEVER pin this VK as canonical. Production requires an MPC ceremony."
        );
        let mut rng = ark_std::rand::rngs::StdRng::from_entropy();
        let (pk, vk) = setup_binding_circuit(self.iss.as_bytes(), self.aud.as_bytes(), &mut rng)
            .map_err(|e| anyhow::anyhow!("setup failed: {e:?}"))?;
        let pk_bytes = proving_key_to_bytes(&pk).map_err(|e| anyhow::anyhow!("{e:?}"))?;
        std::fs::write(&self.out_pk, &pk_bytes)
            .with_context(|| format!("cannot write {}", self.out_pk))?;
        let vk_bytes = vk_to_bytes(&vk).map_err(|e| anyhow::anyhow!("{e:?}"))?;
        std::fs::write(&self.out_vk, &vk_bytes)
            .with_context(|| format!("cannot write {}", self.out_vk))?;
        eprintln!(
            "wrote PK ({} bytes) and VK ({} bytes); VK sha256 = {}",
            pk_bytes.len(),
            vk_bytes.len(),
            hex::encode(vk_fingerprint(&vk).map_err(|e| anyhow::anyhow!("{e:?}"))?)
        );
        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct Prove {
    /// Address of the logged-in session to prove for (default: latest).
    #[arg(long)]
    pub address: Option<String>,
    /// Proving key from `setup-circuit` (must match the pinned VK).
    #[arg(long)]
    pub pk: String,
    /// Where to write the proof package JSON.
    #[arg(long)]
    pub out: String,
}

impl Prove {
    pub fn execute(&self) -> Result<()> {
        let session = match &self.address {
            Some(addr) => session::load_session(addr)?,
            None => session::latest_session()?
                .context("no zkLogin session found (run `kanari zklogin login` first)")?,
        };
        // Decode session secrets (all fixed widths, fail-closed).
        let salt: [u8; 32] = hex::decode(&session.salt_hex)
            .context("session salt corrupt")?
            .try_into()
            .map_err(|_| anyhow::anyhow!("session salt is not 32 bytes"))?;
        let randomness: [u8; 32] = hex::decode(&session.randomness_hex)
            .context("session randomness corrupt")?
            .try_into()
            .map_err(|_| anyhow::anyhow!("session randomness is not 32 bytes"))?;
        let eph_pub: [u8; 32] = hex::decode(&session.ephemeral_pubkey_hex)
            .context("session pubkey corrupt")?
            .try_into()
            .map_err(|_| anyhow::anyhow!("session pubkey is not 32 bytes"))?;
        let sub = session.sub.as_bytes();
        // Re-derive and cross-check: a stale/edited session must fail here,
        // not produce a proof for the wrong statement.
        let addr_hex = derive_zklogin_address_v2(&session.iss, &session.aud, &session.sub, &salt)
            .map_err(|e| anyhow::anyhow!("address re-derivation failed: {e:?}"))?;
        anyhow::ensure!(
            addr_hex.eq_ignore_ascii_case(&session.address),
            "session address does not re-derive (stale or edited session?)"
        );
        let address: [u8; 32] = hex::decode(&addr_hex[2..])
            .context("derived address not hex")?
            .try_into()
            .map_err(|_| anyhow::anyhow!("derived address is not 32 bytes"))?;
        let nonce_hex = compute_nonce(&eph_pub, session.max_epoch, &randomness);
        let nonce: [u8; 32] = hex::decode(&nonce_hex)
            .context("nonce not hex")?
            .try_into()
            .map_err(|_| anyhow::anyhow!("nonce is not 32 bytes"))?;

        let pk_bytes = std::fs::read(&self.pk)
            .with_context(|| format!("cannot read proving key {}", self.pk))?;
        let pk = proving_key_from_bytes(&pk_bytes)
            .map_err(|e| anyhow::anyhow!("proving key is corrupt (wrong file?): {e:?}"))?;
        // Fresh randomness per proof (deterministic RNG would reuse k).
        let mut rng = ark_std::rand::rngs::StdRng::from_entropy();
        let proof = prove_binding(
            &pk,
            session.iss.as_bytes(),
            session.aud.as_bytes(),
            salt,
            sub,
            randomness,
            eph_pub,
            session.max_epoch,
            address,
            nonce,
            &mut rng,
        )
        .map_err(|e| anyhow::anyhow!("proving failed (witnesses do not satisfy?): {e:?}"))?;

        // Derive the VK fingerprint from the PK? No — the VK travels with
        // the proof package separately; here we recompute inputs and let
        // the verifier supply/check the VK. Package carries vk_hash only
        // once the operator provides the VK file; instead we store the
        // inputs + proof and let `verify` take vk explicitly.
        let package = ProofPackage {
            version: 1,
            circuit: "kanari-binding-v1".to_string(),
            iss: session.iss.clone(),
            aud: session.aud.clone(),
            address: addr_hex,
            nonce_hex,
            max_epoch: session.max_epoch,
            vk_hash_hex: String::new(),
            vk_hex: String::new(),
            inputs_hex: hex::encode(public_inputs_to_be_bytes(&address, &nonce)),
            proof_hex: hex::encode(proof_to_bytes(&proof).map_err(|e| anyhow::anyhow!("{e:?}"))?),
        };
        // Fill VK fields from a sidecar `<pk>.vk` file when present (the
        // setup command writes `<out-vk>`; operators keep them adjacent).
        let mut package = package;
        let vk_sidecar = format!("{}.vk", self.pk.trim_end_matches(".pk"));
        if let Ok(vk_bytes) = std::fs::read(&vk_sidecar)
            && let Ok(vk) = kanari_crypto::signatures::groth16::vk_from_bytes(&vk_bytes)
            && let Ok(fp) = vk_fingerprint(&vk)
        {
            package.vk_hash_hex = hex::encode(fp);
            package.vk_hex = hex::encode(vk_bytes);
        }
        std::fs::write(&self.out, serde_json::to_string_pretty(&package)?)
            .with_context(|| format!("cannot write {}", self.out))?;
        eprintln!("proof package written to {}", self.out);
        if package.vk_hash_hex.is_empty() {
            eprintln!(
                "NOTE: no VK sidecar found next to the PK; add the VK bytes + hash before \
                 submitting to a pinning verifier."
            );
        }
        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct VerifyProof {
    /// Proof package from `prove`.
    #[arg(long)]
    pub package: String,
    /// REQUIRED ceremony VK hash (64 hex chars): the package VK must
    /// fingerprint to this. Unpinned verification is refused — a proof
    /// against any other VK (including toxic-waste demo setups) is
    /// meaningless here.
    #[arg(long)]
    pub pin: String,
}

impl VerifyProof {
    pub fn execute(&self) -> Result<()> {
        use kanari_crypto::signatures::groth16::verify_pinned_proof;

        let json = std::fs::read_to_string(&self.package)
            .with_context(|| format!("cannot read {}", self.package))?;
        let package: ProofPackage =
            serde_json::from_str(&json).context("proof package is corrupt")?;
        anyhow::ensure!(package.version == 1, "unsupported package version");
        let pin: [u8; 32] = hex::decode(self.pin.trim().trim_start_matches("0x"))
            .context("pin is not valid hex")?
            .try_into()
            .map_err(|_| anyhow::anyhow!("pin must be exactly 32 bytes (64 hex chars)"))?;
        let vk = hex::decode(&package.vk_hex).context("package vk not hex")?;
        let inputs = hex::decode(&package.inputs_hex).context("package inputs not hex")?;
        let proof = hex::decode(&package.proof_hex).context("package proof not hex")?;
        // Pinned verification only: pin mismatch or malformed VK fails
        // closed inside the helper (no pairing work on untrusted keys).
        let valid = verify_pinned_proof(&pin, &vk, &inputs, &proof)
            .map_err(|e| anyhow::anyhow!("proof rejected: {e:?}"))?;
        eprintln!("circuit: {}", package.circuit);
        eprintln!("address: {}", package.address);
        eprintln!("valid:   {valid}");
        anyhow::ensure!(valid, "proof does not verify");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_package_json_roundtrip() {
        let p = ProofPackage {
            version: 1,
            circuit: "kanari-binding-v1".to_string(),
            iss: "https://accounts.google.com".to_string(),
            aud: "cid".to_string(),
            address: "0xabc".to_string(),
            nonce_hex: "00".repeat(32),
            max_epoch: 1000,
            vk_hash_hex: "11".repeat(32),
            vk_hex: "22".repeat(64),
            inputs_hex: "33".repeat(2048),
            proof_hex: "44".repeat(128),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: ProofPackage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.circuit, "kanari-binding-v1");
        assert_eq!(back.max_epoch, 1000);
    }
}
