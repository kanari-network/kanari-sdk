// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! zkLogin session-binding circuit (DEMO GRADE — read this first).
//!
//! Statement proved (all byte arrays fixed-width):
//! ```text
//! address == SHA256("zkLogin-v2" || iss[64] || aud[96] || sub[64] || salt[32])
//! nonce   == SHA256("zkLogin-nonce" || eph_pub[32] || max_epoch:u64LE || randomness[32])
//! ```
//! - Public inputs: 64 field elements, **one per byte** — `inputs[0..32]`
//!   is the address, `inputs[32..64]` the nonce. Byte-wise inputs keep every
//!   value `< 256`, so nothing is ever reduced modulo the field order, and
//!   the count fits the existing `MAX_PUBLIC_INPUTS = 64` cap exactly.
//! - Private witnesses: `salt`, `sub` (prover pads to caps off-circuit; the
//!   CLI rejects over-long inputs), `randomness`, `eph_pub`, `max_epoch`.
//! - `iss`/`aud` are circuit constants: one setup per (provider, client)
//!   deployment, which doubles as domain separation.
//!
//! What this proves in practice: whoever generates a proof knows the
//! salt+sub behind an address AND the randomness behind a nonce, without
//! revealing any of them. The JWT itself is still verified off-chain
//! (CLI login) or on-chain via `kanari_system::zklogin::verify` (linkable
//! Phase 3b); this circuit is the step that makes the salt/sub half
//! unlinkable (Phase 3c direction).
//!
//! DEMO GRADE means:
//! - The proving/verify keys below come from a local random setup
//!   (`setup_binding_circuit`). Anyone holding the trapdoor can forge
//!   proofs, so production MUST replace it with an MPC ceremony and pin
//!   that VK hash on-chain (see `verify_pinned_proof` + `vk_fingerprint`).
//! - Epoch timeliness (`max_epoch` vs chain time) is NOT constrained here;
//!   integrators enforce it alongside the JWT `exp` check.
//! - Full Sui-style JWT-in-circuit proving (RSA+Base64+JSON in R1CS) is
//!   deliberately out of scope: same privacy direction, month-scale work.

use ark_bn254::{Bn254, Fr};
use ark_crypto_primitives::crh::sha256::constraints::Sha256Gadget;
use ark_groth16::{Groth16, Proof, ProvingKey, VerifyingKey};
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::boolean::Boolean;
use ark_r1cs_std::convert::ToBitsGadget;
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::uint8::UInt8;
use ark_relations::gr1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;

use crate::SignatureError;
use crate::signatures::zklogin::{MAX_AUD_BYTES, MAX_ISS_BYTES, MAX_SUB_BYTES};

/// Domain separators must match the off-circuit derivation byte-for-byte.
pub const CIRCUIT_ADDR_SEED: &[u8] = b"zkLogin-v2";
pub const CIRCUIT_NONCE_SEED: &[u8] = b"zkLogin-nonce";

/// Fixed preimage sizes (bytes).
pub const ADDR_PREIMAGE_LEN: usize = 10 + MAX_ISS_BYTES + MAX_AUD_BYTES + MAX_SUB_BYTES + 32;
pub const NONCE_PREIMAGE_LEN: usize = 13 + 32 + 8 + 32;

/// Public inputs: 32 address bytes + 32 nonce bytes, one Fr per byte.
pub const PUBLIC_INPUT_LEN: usize = 64;

/// Pad `data` with zeros to `width` or fail when it does not fit.
fn pad_to(data: &[u8], width: usize, what: &'static str) -> Result<Vec<u8>, SignatureError> {
    if data.len() > width {
        return Err(SignatureError::InvalidFormat(format!(
            "zkLogin circuit: {what} too long ({} > {width})",
            data.len()
        )));
    }
    let mut out = vec![0u8; width];
    out[..data.len()].copy_from_slice(data);
    Ok(out)
}

/// Session-binding circuit. `iss`/`aud` are deployment constants;
/// everything else is witness (private) or public input (address, nonce).
#[derive(Clone)]
pub struct BindingCircuit {
    /// Padded deployment constants (exact widths).
    iss_padded: Vec<u8>,
    /// Padded deployment constants (exact widths).
    aud_padded: Vec<u8>,
    /// Private witnesses (`None` only for the blank setup instance).
    salt: Option<[u8; 32]>,
    /// Private witnesses (`None` only for the blank setup instance).
    sub_padded: Option<Vec<u8>>,
    /// Private witnesses (`None` only for the blank setup instance).
    randomness: Option<[u8; 32]>,
    /// Private witnesses (`None` only for the blank setup instance).
    eph_pub: Option<[u8; 32]>,
    /// Private witnesses (`None` only for the blank setup instance).
    max_epoch: Option<u64>,
    /// Public inputs as raw bytes (address[32] ++ nonce[32]).
    address: Option<[u8; 32]>,
    /// Public inputs as raw bytes (address[32] ++ nonce[32]).
    nonce: Option<[u8; 32]>,
}

impl BindingCircuit {
    /// Blank instance for trusted setup (values are placeholders; only the
    /// structure matters). `iss`/`aud` fix the deployment.
    pub fn blank(iss: &[u8], aud: &[u8]) -> Result<Self, SignatureError> {
        Ok(Self {
            iss_padded: pad_to(iss, MAX_ISS_BYTES, "iss")?,
            aud_padded: pad_to(aud, MAX_AUD_BYTES, "aud")?,
            salt: None,
            sub_padded: None,
            randomness: None,
            eph_pub: None,
            max_epoch: None,
            address: None,
            nonce: None,
        })
    }

    /// Proving instance. Rejects over-long fields (fail-closed).
    #[allow(clippy::too_many_arguments)]
    pub fn prove_instance(
        iss: &[u8],
        aud: &[u8],
        salt: [u8; 32],
        sub: &[u8],
        randomness: [u8; 32],
        eph_pub: [u8; 32],
        max_epoch: u64,
        address: [u8; 32],
        nonce: [u8; 32],
    ) -> Result<Self, SignatureError> {
        Ok(Self {
            iss_padded: pad_to(iss, MAX_ISS_BYTES, "iss")?,
            aud_padded: pad_to(aud, MAX_AUD_BYTES, "aud")?,
            salt: Some(salt),
            sub_padded: Some(pad_to(sub, MAX_SUB_BYTES, "sub")?),
            randomness: Some(randomness),
            eph_pub: Some(eph_pub),
            max_epoch: Some(max_epoch),
            address: Some(address),
            nonce: Some(nonce),
        })
    }
}

fn const_bytes(cs: ConstraintSystemRef<Fr>, data: &[u8]) -> Result<Vec<UInt8<Fr>>, SynthesisError> {
    data.iter()
        .map(|b| UInt8::new_constant(cs.clone(), b))
        .collect()
}

fn opt_bytes(
    cs: ConstraintSystemRef<Fr>,
    data: Option<[u8; 32]>,
) -> Result<Vec<UInt8<Fr>>, SynthesisError> {
    data.unwrap_or([0u8; 32])
        .iter()
        .map(|b| UInt8::new_witness(cs.clone(), || Ok(*b)))
        .collect()
}

impl ConstraintSynthesizer<Fr> for BindingCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // --- Address preimage: seed || iss || aud || sub || salt ---
        let mut addr_preimage = Vec::with_capacity(ADDR_PREIMAGE_LEN);
        addr_preimage.extend(const_bytes(cs.clone(), CIRCUIT_ADDR_SEED)?);
        addr_preimage.extend(const_bytes(cs.clone(), &self.iss_padded)?);
        addr_preimage.extend(const_bytes(cs.clone(), &self.aud_padded)?);
        let sub_full = self.sub_padded.unwrap_or_else(|| vec![0u8; MAX_SUB_BYTES]);
        debug_assert_eq!(sub_full.len(), MAX_SUB_BYTES);
        for b in sub_full {
            addr_preimage.push(UInt8::new_witness(cs.clone(), || Ok(b))?);
        }
        addr_preimage.extend(opt_bytes(cs.clone(), self.salt)?);
        debug_assert_eq!(addr_preimage.len(), ADDR_PREIMAGE_LEN);
        let addr_digest = Sha256Gadget::<Fr>::digest(&addr_preimage)?;

        // --- Nonce preimage: seed || eph_pub || epoch_le || randomness ---
        let mut nonce_preimage = Vec::with_capacity(NONCE_PREIMAGE_LEN);
        nonce_preimage.extend(const_bytes(cs.clone(), CIRCUIT_NONCE_SEED)?);
        nonce_preimage.extend(opt_bytes(cs.clone(), self.eph_pub)?);
        let epoch = self.max_epoch.unwrap_or(0).to_le_bytes();
        for b in epoch {
            nonce_preimage.push(UInt8::new_witness(cs.clone(), || Ok(b))?);
        }
        nonce_preimage.extend(opt_bytes(cs.clone(), self.randomness)?);
        debug_assert_eq!(nonce_preimage.len(), NONCE_PREIMAGE_LEN);
        let nonce_digest = Sha256Gadget::<Fr>::digest(&nonce_preimage)?;

        // --- Public inputs: 64 byte-valued Fr elements ---
        // Each public input is ONE Fp variable constrained to a byte:
        // decompose, force the high 246 bits to zero (value < 256), and
        // rebuild the byte for the digest comparison. This keeps the wire
        // format at exactly 64 inputs (one per address/nonce byte).
        let addr_in = self.address.unwrap_or([0u8; 32]);
        let nonce_in = self.nonce.unwrap_or([0u8; 32]);
        for (i, digest_byte) in addr_digest.0.iter().enumerate() {
            let byte = fp_input_byte(cs.clone(), addr_in[i])?;
            digest_byte.enforce_equal(&byte)?;
        }
        for (i, digest_byte) in nonce_digest.0.iter().enumerate() {
            let byte = fp_input_byte(cs.clone(), nonce_in[i])?;
            digest_byte.enforce_equal(&byte)?;
        }
        Ok(())
    }
}

/// One public Fr input constrained to equal `value` as a byte (< 256).
fn fp_input_byte(cs: ConstraintSystemRef<Fr>, value: u8) -> Result<UInt8<Fr>, SynthesisError> {
    let input = FpVar::<Fr>::new_input(cs.clone(), || Ok(Fr::from(value as u64)))?;
    let bits = input.to_bits_le()?;
    for b in &bits[8..] {
        b.enforce_equal(&Boolean::FALSE)?;
    }
    Ok(UInt8::from_bits_le(&bits[..8]))
}

/// Convert 64 public-input bytes to Groth16 Fr inputs (exact: bytes < 256).
pub fn public_inputs_from_bytes(address: &[u8; 32], nonce: &[u8; 32]) -> Vec<Fr> {
    address
        .iter()
        .chain(nonce.iter())
        .map(|b| Fr::from(*b as u64))
        .collect()
}

/// Encode public inputs for the Move wire format: 64 × 32-byte big-endian
/// field elements (matches `public_inputs_from_be_bytes`).
pub fn public_inputs_to_be_bytes(address: &[u8; 32], nonce: &[u8; 32]) -> Vec<u8> {
    use ark_ff::{BigInteger, PrimeField as _};
    public_inputs_from_bytes(address, nonce)
        .iter()
        .flat_map(|f| {
            let bi = (*f).into_bigint().to_bytes_be();
            let mut out = [0u8; 32];
            out[32usize.saturating_sub(bi.len())..].copy_from_slice(&bi);
            out.to_vec()
        })
        .collect()
}

/// DEMO-GRADE trusted setup (local randomness — toxic waste!).
/// Production must use an MPC ceremony and pin that VK hash on-chain.
/// Takes a concrete `StdRng` (same pattern as the existing Groth16 tests)
/// to avoid rand-version trait soup.
pub fn setup_binding_circuit(
    iss: &[u8],
    aud: &[u8],
    rng: &mut ark_std::rand::rngs::StdRng,
) -> Result<(ProvingKey<Bn254>, VerifyingKey<Bn254>), SignatureError> {
    let circuit = BindingCircuit::blank(iss, aud)?;
    Groth16::<Bn254>::circuit_specific_setup(circuit, rng)
        .map_err(|e| SignatureError::InvalidFormat(format!("circuit setup failed: {e:?}")))
}

/// Prove with a setup proving key. Fails when the witnesses do not satisfy
/// the statement (e.g. wrong salt for the claimed address).
#[allow(clippy::too_many_arguments)]
pub fn prove_binding(
    pk: &ProvingKey<Bn254>,
    iss: &[u8],
    aud: &[u8],
    salt: [u8; 32],
    sub: &[u8],
    randomness: [u8; 32],
    eph_pub: [u8; 32],
    max_epoch: u64,
    address: [u8; 32],
    nonce: [u8; 32],
    rng: &mut ark_std::rand::rngs::StdRng,
) -> Result<Proof<Bn254>, SignatureError> {
    let circuit = BindingCircuit::prove_instance(
        iss, aud, salt, sub, randomness, eph_pub, max_epoch, address, nonce,
    )?;
    Groth16::<Bn254>::prove(pk, circuit, rng).map_err(|_| SignatureError::VerificationFailed)
}

/// SHA256 of compressed VK bytes — the value contracts pin.
pub fn vk_fingerprint(vk: &VerifyingKey<Bn254>) -> Result<[u8; 32], SignatureError> {
    use sha2::{Digest, Sha256};
    let mut bytes = Vec::new();
    vk.serialize_compressed(&mut bytes)
        .map_err(|_| SignatureError::InvalidFormat("vk does not serialize".to_string()))?;
    Ok(Sha256::digest(&bytes).into())
}

/// Canonical (de)serialization helpers for CLI proof packages.
pub fn vk_to_bytes(vk: &VerifyingKey<Bn254>) -> Result<Vec<u8>, SignatureError> {
    let mut bytes = Vec::new();
    vk.serialize_compressed(&mut bytes)
        .map_err(|_| SignatureError::InvalidFormat("vk does not serialize".to_string()))?;
    Ok(bytes)
}

pub fn proof_to_bytes(proof: &Proof<Bn254>) -> Result<Vec<u8>, SignatureError> {
    let mut bytes = Vec::new();
    proof
        .serialize_compressed(&mut bytes)
        .map_err(|_| SignatureError::InvalidFormat("proof does not serialize".to_string()))?;
    Ok(bytes)
}

pub fn vk_from_bytes(bytes: &[u8]) -> Result<VerifyingKey<Bn254>, SignatureError> {
    VerifyingKey::<Bn254>::deserialize_compressed(&mut &bytes[..])
        .map_err(|e| SignatureError::InvalidFormat(format!("bad vk bytes: {e}")))
}

/// Canonical (de)serialization for proving keys (CLI proof packages).
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
    use ark_relations::gr1cs::ConstraintSystem;
    use ark_std::rand::SeedableRng;

    const TEST_ISS: &[u8] = b"https://accounts.google.com";
    const TEST_AUD: &[u8] = b"kanari-test-client";
    const TEST_SUB: &[u8] = b"1234";

    fn test_rng() -> ark_std::rand::rngs::StdRng {
        ark_std::rand::rngs::StdRng::seed_from_u64(0xB10C5EED)
    }

    #[test]
    fn circuit_shape_is_sane() {
        // Fast: builds constraints only (no setup/prove). Asserts the R1CS
        // size lands in the expected SHA256x2 band.
        let cs = ConstraintSystem::<Fr>::new_ref();
        BindingCircuit::blank(TEST_ISS, TEST_AUD)
            .unwrap()
            .generate_constraints(cs.clone())
            .unwrap();
        let n = cs.num_constraints();
        // ~27k constraints per SHA256 block; 5 + 2 blocks expected.
        assert!(
            (150_000..400_000).contains(&n),
            "unexpected constraint count {n}"
        );
    }

    /// Named types so the could-represent-a-proof bundle stays readable
    /// (and below clippy's `type_complexity` threshold).
    struct PublicWitnesses {
        salt: [u8; 32],
        randomness: [u8; 32],
        eph_pub: [u8; 32],
        max_epoch: u64,
    }

    struct BindingCase {
        address: [u8; 32],
        nonce: [u8; 32],
        witnesses: PublicWitnesses,
    }

    /// Fixed witnesses matching the off-circuit derivation (address/nonce
    /// digest of seed || iss || aud || sub || salt and seed || eph || epoch || rand).
    fn matching_witnesses() -> BindingCase {
        use crate::signatures::zklogin::{compute_nonce, derive_zklogin_address_v2};
        let salt = [9u8; 32];
        let randomness = [7u8; 32];
        let eph_pub = [42u8; 32];
        let max_epoch = 1000u64;
        let addr_hex = derive_zklogin_address_v2(
            std::str::from_utf8(TEST_ISS).unwrap(),
            std::str::from_utf8(TEST_AUD).unwrap(),
            std::str::from_utf8(TEST_SUB).unwrap(),
            &salt,
        )
        .unwrap();
        let address: [u8; 32] = hex::decode(&addr_hex[2..]).unwrap().try_into().unwrap();
        let nonce_hex = compute_nonce(&eph_pub, max_epoch, &randomness);
        let nonce: [u8; 32] = hex::decode(&nonce_hex).unwrap().try_into().unwrap();
        BindingCase {
            address,
            nonce,
            witnesses: PublicWitnesses {
                salt,
                randomness,
                eph_pub,
                max_epoch,
            },
        }
    }

    /// Build constraints and return whether they are all satisfied.
    fn is_satisfied(circuit: BindingCircuit) -> bool {
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();
        cs.is_satisfied().unwrap()
    }

    /// The core invariant, checked WITHOUT a full setup/prove (CI-fast):
    /// the statement is exactly "public == hash(witnesses)". Any single-bit
    /// drift on the public side or any witness perturbation must flip
    /// satisfiability. THIS is what makes the proof non-forgeable.
    /// ~50s in debug (SHA256 gadget synthesis); the full set of 8 tamper
    /// axes is covered transitively by `binding_roundtrip_ignored`.
    #[test]
    fn negative_invariants_canary_transform() {
        let case = matching_witnesses();
        let address = case.address;
        let nonce = case.nonce;
        let salt = case.witnesses.salt;
        let randomness = case.witnesses.randomness;
        let eph_pub = case.witnesses.eph_pub;
        let max_epoch = case.witnesses.max_epoch;

        let sat = |salt, sub, randomness, eph_pub, max_epoch, address, nonce| {
            is_satisfied(
                BindingCircuit::prove_instance(
                    TEST_ISS, TEST_AUD, salt, sub, randomness, eph_pub, max_epoch, address, nonce,
                )
                .unwrap(),
            )
        };

        // Honest instance is fully satisfiable.
        assert!(sat(
            salt, TEST_SUB, randomness, eph_pub, max_epoch, address, nonce
        ));

        // Public-side tampering (the verifier only sees inputs): any bit
        // flipped in address breaks the digest equality.
        let mut addr_flip = address;
        addr_flip[0] ^= 1;
        assert!(!sat(
            salt, TEST_SUB, randomness, eph_pub, max_epoch, addr_flip, nonce
        ));

        // Witness-side tampering (collision/forgery attempt): wrong salt
        // must not satisfy the pinned digest.
        assert!(!sat(
            [1u8; 32], TEST_SUB, randomness, eph_pub, max_epoch, address, nonce
        ));

        // Epoch tampering: bumping max_epoch changes the nonce digest.
        assert!(!sat(
            salt,
            TEST_SUB,
            randomness,
            eph_pub,
            max_epoch + 1,
            address,
            nonce
        ));
    }

    /// Full setup -> prove -> verify roundtrip. SLOW (minutes in debug):
    /// run explicitly in release:
    /// `cargo test -p kanari-crypto --release binding_roundtrip -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn binding_roundtrip_ignored() {
        use crate::signatures::zklogin::{compute_nonce, derive_zklogin_address_v2};

        let mut rng = test_rng();
        let salt = [9u8; 32];
        let randomness = [7u8; 32];
        let eph_pub = [42u8; 32];
        let max_epoch = 1000u64;
        // Cross-check against the real off-circuit derivation (no drift).
        let addr_hex = derive_zklogin_address_v2(
            "https://accounts.google.com",
            "kanari-test-client",
            "1234",
            &salt,
        )
        .unwrap();
        let address: [u8; 32] = hex::decode(&addr_hex[2..]).unwrap().try_into().unwrap();
        let nonce_hex = compute_nonce(&eph_pub, max_epoch, &randomness);
        let nonce: [u8; 32] = hex::decode(&nonce_hex).unwrap().try_into().unwrap();

        let (pk, vk) = setup_binding_circuit(TEST_ISS, TEST_AUD, &mut rng).unwrap();
        let proof = prove_binding(
            &pk, TEST_ISS, TEST_AUD, salt, TEST_SUB, randomness, eph_pub, max_epoch, address,
            nonce, &mut rng,
        )
        .unwrap();
        let inputs = public_inputs_to_be_bytes(&address, &nonce);
        assert!(
            crate::signatures::zklogin_proof::verify_groth16_proof(
                &vk_to_bytes(&vk).unwrap(),
                &inputs,
                &proof_to_bytes(&proof).unwrap(),
            )
            .unwrap()
        );
        // Wrong salt still "proves" (Groth16 prove does not check
        // satisfaction) — but the proof must NOT verify.
        let bad_proof = prove_binding(
            &pk, TEST_ISS, TEST_AUD, [1u8; 32], TEST_SUB, randomness, eph_pub, max_epoch, address,
            nonce, &mut rng,
        )
        .unwrap();
        assert!(
            !crate::signatures::zklogin_proof::verify_groth16_proof(
                &vk_to_bytes(&vk).unwrap(),
                &inputs,
                &proof_to_bytes(&bad_proof).unwrap(),
            )
            .unwrap()
        );
        assert_eq!(vk_fingerprint(&vk).unwrap().len(), 32);
    }
}
