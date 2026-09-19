// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Generic Groth16 plumbing over BN254 — circuit-agnostic.
//!
//! Everything here works for ANY circuit: caps, public-input decoding,
//! proof verification, canonical (de)serialization of keys/proofs, and the
//! VK fingerprint contracts pin. zkLogin-specific pieces (the session-
//! binding statement, its setup/prove entry points, its 64-byte-input wire
//! format) live in [`crate::signatures::zklogin_circuit`]; the transaction
//! authenticator that calls this verifier lives in
//! [`crate::signatures::zk_authenticator`]; the chain entry point lives in
//! `kanari_system_natives::crypto::zklogin`.
//!
//! Wire format (all `vector<u8>` on the Move side):
//! - `vk_bytes`: `ark_serialize::CanonicalSerialize::serialize_compressed`
//!   of `ark_groth16::VerifyingKey<ark_bn254::Bn254>`. Capped at
//!   [`MAX_VK_BYTES`].
//! - `proof_bytes`: compressed serialization of
//!   `ark_groth16::Proof<ark_bn254::Bn254>`. Capped at [`MAX_PROOF_BYTES`]
//!   (a BN254 Groth16 proof compresses to ~128 bytes).
//! - `public_inputs_bytes`: concatenation of 32-byte big-endian field
//!   elements (`Fr::from_be_bytes_mod_order`), at most
//!   [`MAX_PUBLIC_INPUTS`] inputs. Empty = zero public inputs.
//!
//! Gas note: a BN254 pairing check is ~2–4 ms (an order of magnitude above
//! an RSA-2048 verify), so the native charges `zklogin_proof_verify`
//! (production 28_000) rather than reusing the RSA-class fee.

use ark_bn254::{Bn254, Fr};
use ark_ff::PrimeField;
use ark_groth16::{Groth16, Proof, ProvingKey, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;

use crate::SignatureError;

/// Maximum serialized verifying-key size we accept (DoS cap).
pub const MAX_VK_BYTES: usize = 32 * 1024; // 32 KiB
/// Maximum serialized proof size we accept (DoS cap).
pub const MAX_PROOF_BYTES: usize = 1024; // 1 KiB (real proofs are ~128 B)
/// Maximum number of public inputs per proof.
pub const MAX_PUBLIC_INPUTS: usize = 64;

/// Decode concatenated 32-byte big-endian field elements into `Fr`.
pub fn public_inputs_from_be_bytes(bytes: &[u8]) -> Result<Vec<Fr>, SignatureError> {
    if !bytes.len().is_multiple_of(32) {
        return Err(SignatureError::InvalidFormat(format!(
            "public inputs must be a multiple of 32 bytes, got {}",
            bytes.len()
        )));
    }
    let n = bytes.len() / 32;
    if n > MAX_PUBLIC_INPUTS {
        return Err(SignatureError::InvalidFormat(format!(
            "too many public inputs: {n} > {MAX_PUBLIC_INPUTS}"
        )));
    }
    Ok(bytes
        .as_chunks::<32>()
        .0
        .iter()
        .map(|chunk: &[u8; 32]| Fr::from_be_bytes_mod_order(chunk))
        .collect())
}

/// Verify a BN254 Groth16 proof.
///
/// - Deserialization failures => `Err(InvalidFormat)` (fail-closed, maps to
///   `E_INVALID_PROOF` on-chain).
/// - A well-formed but unsatisfying proof => `Ok(false)` (non-aborting Move
///   wrapper returns `false`).
/// - Internal SNARK errors => `Err(VerificationFailed)`.
pub fn verify_groth16_proof(
    vk_bytes: &[u8],
    public_inputs_bytes: &[u8],
    proof_bytes: &[u8],
) -> Result<bool, SignatureError> {
    if vk_bytes.is_empty() || vk_bytes.len() > MAX_VK_BYTES {
        return Err(SignatureError::InvalidFormat(format!(
            "bad vk length {}",
            vk_bytes.len()
        )));
    }
    if proof_bytes.is_empty() || proof_bytes.len() > MAX_PROOF_BYTES {
        return Err(SignatureError::InvalidFormat(format!(
            "bad proof length {}",
            proof_bytes.len()
        )));
    }
    let vk = VerifyingKey::<Bn254>::deserialize_compressed(&mut &vk_bytes[..])
        .map_err(|e| SignatureError::InvalidFormat(format!("bad Groth16 verifying key: {e}")))?;
    let proof = Proof::<Bn254>::deserialize_compressed(&mut &proof_bytes[..])
        .map_err(|e| SignatureError::InvalidFormat(format!("bad Groth16 proof: {e}")))?;
    let inputs = public_inputs_from_be_bytes(public_inputs_bytes)?;
    Groth16::<Bn254>::verify(&vk, &inputs, &proof).map_err(|_| SignatureError::VerificationFailed)
}

/// Verify a proof ONLY against a pinned ceremony VK hash.
///
/// `vk_pin` is the 32-byte [`vk_fingerprint`] of the MPC-ceremony VK the
/// deployment trusts. Proofs against any other VK — including locally
/// generated toxic-waste setups — fail closed BEFORE any pairing work, so
/// a demo setup can never pass as canonical. Prefer this over bare
/// [`verify_groth16_proof`] everywhere; bare verification is reserved for
/// callers that pin the VK themselves (e.g. the Move `verify_pinned_proof`
/// wrapper, which checks the hash first).
pub fn verify_pinned_proof(
    vk_pin: &[u8; 32],
    vk_bytes: &[u8],
    public_inputs_bytes: &[u8],
    proof_bytes: &[u8],
) -> Result<bool, SignatureError> {
    let vk = vk_from_bytes(vk_bytes)?;
    if &vk_fingerprint(&vk)? != vk_pin {
        return Err(SignatureError::VerificationFailed);
    }
    verify_groth16_proof(vk_bytes, public_inputs_bytes, proof_bytes)
}

/// SHA256 of compressed VK bytes — the value contracts pin.
pub fn vk_fingerprint(vk: &VerifyingKey<Bn254>) -> Result<[u8; 32], SignatureError> {
    use sha2::{Digest, Sha256};
    let mut bytes = Vec::new();
    vk.serialize_compressed(&mut bytes)
        .map_err(|_| SignatureError::InvalidFormat("vk does not serialize".to_string()))?;
    Ok(Sha256::digest(&bytes).into())
}

/// Canonical (de)serialization helpers for keys and proofs.
pub fn vk_to_bytes(vk: &VerifyingKey<Bn254>) -> Result<Vec<u8>, SignatureError> {
    let mut bytes = Vec::new();
    vk.serialize_compressed(&mut bytes)
        .map_err(|_| SignatureError::InvalidFormat("vk does not serialize".to_string()))?;
    Ok(bytes)
}

pub fn vk_from_bytes(bytes: &[u8]) -> Result<VerifyingKey<Bn254>, SignatureError> {
    VerifyingKey::<Bn254>::deserialize_compressed(&mut &bytes[..])
        .map_err(|e| SignatureError::InvalidFormat(format!("bad vk bytes: {e}")))
}

pub fn proof_to_bytes(proof: &Proof<Bn254>) -> Result<Vec<u8>, SignatureError> {
    let mut bytes = Vec::new();
    proof
        .serialize_compressed(&mut bytes)
        .map_err(|_| SignatureError::InvalidFormat("proof does not serialize".to_string()))?;
    Ok(bytes)
}

/// Canonical (de)serialization for proving keys.
/// Proving keys are large (tens of MB); callers stream them to files.
pub fn proving_key_to_bytes(pk: &ProvingKey<Bn254>) -> Result<Vec<u8>, SignatureError> {
    let mut bytes = Vec::new();
    pk.serialize_compressed(&mut bytes)
        .map_err(|_| SignatureError::InvalidFormat("pk does not serialize".to_string()))?;
    Ok(bytes)
}

pub fn proving_key_from_bytes(bytes: &[u8]) -> Result<ProvingKey<Bn254>, SignatureError> {
    ProvingKey::<Bn254>::deserialize_compressed(&mut &bytes[..])
        .map_err(|e| SignatureError::InvalidFormat(format!("bad pk bytes: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ff::{BigInteger, UniformRand};
    use ark_relations::{
        gr1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError},
        lc,
    };
    use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
    use ark_std::rand::SeedableRng;

    /// Deterministic RNG for tests: ark's `test_rng` returns a versioned
    /// `rand` wrapper the SNARK trait bounds reject, so seed a `StdRng`
    /// from the matching `ark_std::rand` re-export instead.
    fn test_seeded_rng() -> ark_std::rand::rngs::StdRng {
        ark_std::rand::rngs::StdRng::seed_from_u64(0x5eed42)
    }

    /// 32-byte big-endian encoding of a field element (matches
    /// `public_inputs_from_be_bytes`).
    fn fr_to_be_bytes(f: Fr) -> [u8; 32] {
        let bytes = f.into_bigint().to_bytes_be();
        let mut out = [0u8; 32];
        let start = 32usize.saturating_sub(bytes.len());
        out[start..].copy_from_slice(&bytes);
        out
    }

    /// Trivial demo circuit: private `a * b == public c`.
    /// Proves the plumbing (setup/prove/verify + wire format) without any
    /// application circuit. App circuits reuse the same verifier.
    #[derive(Clone, Copy)]
    struct MulCircuit {
        a: Option<Fr>,
        b: Option<Fr>,
        c: Fr,
    }

    impl ConstraintSynthesizer<Fr> for MulCircuit {
        fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
            let a = cs.new_witness_variable(|| self.a.ok_or(SynthesisError::AssignmentMissing))?;
            let b = cs.new_witness_variable(|| self.b.ok_or(SynthesisError::AssignmentMissing))?;
            let c = cs.new_input_variable(|| Ok(self.c))?;
            cs.enforce_r1cs_constraint(|| lc!() + a, || lc!() + b, || lc!() + c)?;
            Ok(())
        }
    }

    fn setup_mul_circuit() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        // Returns (vk_bytes, public_inputs_bytes, proof_bytes) for a=6,b=7,c=42.
        let mut rng = test_seeded_rng();
        let c = Fr::from(42u64);
        let circuit = MulCircuit {
            a: Some(Fr::from(6u64)),
            b: Some(Fr::from(7u64)),
            c,
        };
        let (pk, vk) =
            Groth16::<Bn254>::circuit_specific_setup(circuit, &mut rng).expect("setup works");
        let mut vk_bytes = Vec::new();
        vk.serialize_compressed(&mut vk_bytes)
            .expect("vk serializes");
        let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng).expect("honest proof proves");
        let mut proof_bytes = Vec::new();
        proof
            .serialize_compressed(&mut proof_bytes)
            .expect("proof serializes");
        let mut inputs_bytes = Vec::new();
        inputs_bytes.extend_from_slice(&fr_to_be_bytes(c));
        (vk_bytes, inputs_bytes, proof_bytes)
    }

    #[test]
    fn verifies_honest_proof() {
        let (vk, inputs, proof) = setup_mul_circuit();
        assert!(verify_groth16_proof(&vk, &inputs, &proof).unwrap());
    }

    #[test]
    fn rejects_wrong_public_input() {
        let (vk, _, proof) = setup_mul_circuit();
        // Claim c=43 instead of 42: well-formed, but unsatisfying => Ok(false).
        let bad = fr_to_be_bytes(Fr::from(43u64));
        assert!(!verify_groth16_proof(&vk, &bad, &proof).unwrap());
    }

    #[test]
    fn rejects_malformed_bytes() {
        assert!(verify_groth16_proof(&[], &[0u8; 32], &[1u8; 64]).is_err());
        assert!(verify_groth16_proof(&[1u8; 64], &[0u8; 32], &[]).is_err());
        // Public inputs not a multiple of 32.
        let (vk, _, proof) = setup_mul_circuit();
        assert!(verify_groth16_proof(&vk, &[9u8; 31], &proof).is_err());
        // Garbage proof bytes.
        assert!(verify_groth16_proof(&vk, &[0u8; 32], &[7u8; 128]).is_err());
    }

    #[test]
    fn rejects_tampered_proof() {
        let (vk, inputs, mut proof) = setup_mul_circuit();
        let last = proof.len() - 1;
        proof[last] ^= 0xFF;
        // Either a decode error or a failed verification — both fail closed.
        if let Ok(valid) = verify_groth16_proof(&vk, &inputs, &proof) {
            assert!(!valid);
        }
    }

    #[test]
    fn pinned_proof_accepts_pin_and_rejects_everything_else() {
        let (vk_bytes, inputs, proof) = setup_mul_circuit();
        let vk = VerifyingKey::<Bn254>::deserialize_compressed(&mut &vk_bytes[..]).unwrap();
        let pin = vk_fingerprint(&vk).unwrap();
        // Ceremony pin + honest proof verifies.
        assert!(verify_pinned_proof(&pin, &vk_bytes, &inputs, &proof).unwrap());
        // Right proof, wrong trust root: fail closed, no pairing work leaks.
        assert!(verify_pinned_proof(&[0u8; 32], &vk_bytes, &inputs, &proof).is_err());
        // Malformed VK fails closed (InvalidFormat, not a silent false).
        assert!(verify_pinned_proof(&pin, &[7u8; 64], &inputs, &proof).is_err());
    }

    #[test]
    fn caps_are_sane() {
        assert_eq!(MAX_VK_BYTES, 32 * 1024);
        assert_eq!(MAX_PROOF_BYTES, 1024);
        assert_eq!(MAX_PUBLIC_INPUTS, 64);
        // Random field roundtrips through BE bytes.
        let mut rng = test_seeded_rng();
        let f = Fr::rand(&mut rng);
        let be = fr_to_be_bytes(f);
        let back = public_inputs_from_be_bytes(&be).unwrap();
        assert_eq!(back, vec![f]);
    }
}
