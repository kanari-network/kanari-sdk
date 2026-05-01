// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! ECVRF (Elliptic Curve Verifiable Random Function) implementation
//! Based on [RFC 9381](https://www.rfc-editor.org/rfc/rfc9381.html)
//!
//! This is a production-grade VRF using Ristretto255 curve for cryptographically
//! secure and unpredictable leader election in DAG consensus.

use super::crypto_signatures::{
    CompressedRistretto, RISTRETTO_BASEPOINT_TABLE, RistrettoPoint, Scalar,
};
use kanari_crypto::{hash_data_blake3, hash_data_shake256_custom};
use rand::TryRng;
use rand::rngs::SysRng;
use std::fmt;

/// Convert hash to scalar with proper bias removal (RFC 9381 compliant)
/// Uses 64-byte hash input and wide reduction to eliminate modular bias
fn hash_to_scalar_unbiased(hash: &[u8]) -> Scalar {
    // FIX #4: Use 64 bytes for unbiased scalar generation (RFC 9381)
    let mut bytes = [0u8; 64];
    if hash.len() >= 64 {
        bytes.copy_from_slice(&hash[..64]);
    } else {
        // Fallback: extend with zeros if hash is too short (shouldn't happen with Blake3-512)
        bytes[..hash.len()].copy_from_slice(hash);
    }
    Scalar::from_bytes_mod_order_wide(&bytes)
}

// /// Legacy function kept for backward compatibility (DEPRECATED - use hash_to_scalar_unbiased)
// #[deprecated(since = "0.2.0", note = "Use hash_to_scalar_unbiased for RFC 9381 compliance")]
// fn hash32_to_scalar(hash: &[u8]) -> Scalar {
//     let mut bytes = [0u8; 32];
//     bytes.copy_from_slice(&hash[..32]);
//     Scalar::from_bytes_mod_order(bytes)
// }

fn hash_with_domain(domain: &[u8], parts: &[&[u8]]) -> Vec<u8> {
    let cap = domain.len() + parts.iter().map(|p| p.len()).sum::<usize>();
    let mut data = Vec::with_capacity(cap);
    data.extend_from_slice(domain);
    for part in parts {
        data.extend_from_slice(part);
    }
    // FIX #4: Use SHAKE256 with 64-byte output for unbiased scalar generation (RFC 9381)
    // This prevents lattice attacks on VRF nonce by ensuring uniform distribution
    hash_data_shake256_custom(&data, 64)
}

// ===== VRF Helper Functions (Shared between SecretKey and PublicKey) =====

/// Hash arbitrary input to a curve point (hash-to-curve)
/// Uses try-and-increment approach for Ristretto255
/// Returns Result instead of panicking to prevent liveness failures
fn vrf_hash_to_curve(alpha: &[u8]) -> Result<RistrettoPoint, anyhow::Error> {
    let mut data =
        Vec::with_capacity(b"vrf_h2c_v1".len() + alpha.len() + std::mem::size_of::<u32>());
    data.extend_from_slice(b"vrf_h2c_v1");
    data.extend_from_slice(alpha);
    let base_len = data.len();

    // Try-and-increment approach with proper error handling
    for i in 0u32..256 {
        data.extend_from_slice(&i.to_le_bytes());
        let hash = hash_data_blake3(&data);
        data.truncate(base_len);

        if let Ok(compressed) = CompressedRistretto::from_slice(&hash[..32])
            && let Some(point) = compressed.decompress()
        {
            return Ok(point);
        }
    }

    // FIX #4: Return error instead of panic to prevent node halt (liveness failure)
    // This is extremely unlikely (< 2^-128 probability) but must be handled gracefully
    Err(anyhow::anyhow!(
        "Hash-to-curve failed after 256 attempts. This indicates either a critical cryptographic failure or corrupted input data."
    ))
}

/// Hash points to create challenge scalar for VRF proof
fn vrf_hash_challenge(
    pk: &RistrettoPoint,
    h: &RistrettoPoint,
    gamma: &RistrettoPoint,
    u: &RistrettoPoint,
    v: &RistrettoPoint,
) -> Scalar {
    let hash = hash_with_domain(
        b"vrf_challenge_v1",
        &[
            pk.compress().as_bytes(),
            h.compress().as_bytes(),
            gamma.compress().as_bytes(),
            u.compress().as_bytes(),
            v.compress().as_bytes(),
        ],
    );
    // FIX #4: Use unbiased scalar generation (RFC 9381 compliant)
    hash_to_scalar_unbiased(&hash)
}

/// Convert proof gamma to final VRF output hash
fn vrf_proof_to_hash(gamma: &RistrettoPoint) -> [u8; 32] {
    let hash = hash_with_domain(b"vrf_output_v1", &[gamma.compress().as_bytes()]);
    hash[..32].try_into().unwrap_or([0u8; 32])
}

// ===== VRF Types =====

/// VRF secret key (32 bytes, Ristretto255 scalar)
#[derive(Clone)]
pub struct VrfSecretKey {
    scalar: Scalar,
}

/// VRF public key (32 bytes, Ristretto255 point)
#[derive(Clone, PartialEq, Eq)]
pub struct VrfPublicKey {
    point: RistrettoPoint,
}

impl fmt::Debug for VrfPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VrfPublicKey({})",
            hex::encode(self.point.compress().as_bytes())
        )
    }
}

/// VRF proof (gamma point + challenge + response)
#[derive(Clone)]
pub struct VrfProof {
    gamma: RistrettoPoint, // VRF hash point
    c: Scalar,             // Challenge
    s: Scalar,             // Response
}

impl serde::Serialize for VrfProof {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;

        // Serialize as tuple: (gamma_bytes, c_bytes, s_bytes)
        // gamma: 32 bytes (compressed), c: 32 bytes, s: 32 bytes = 96 bytes total
        let mut tuple = serializer.serialize_tuple(3)?;
        tuple.serialize_element(self.gamma.compress().as_bytes())?;
        tuple.serialize_element(&self.c.to_bytes())?;
        tuple.serialize_element(&self.s.to_bytes())?;
        tuple.end()
    }
}

impl<'de> serde::Deserialize<'de> for VrfProof {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, SeqAccess, Visitor};
        use std::fmt;

        struct VrfProofVisitor;

        impl<'de> Visitor<'de> for VrfProofVisitor {
            type Value = VrfProof;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a tuple of (gamma_bytes, c_bytes, s_bytes)")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<VrfProof, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let gamma_bytes: [u8; 32] = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let c_bytes: [u8; 32] = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let s_bytes: [u8; 32] = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;

                let gamma = CompressedRistretto(gamma_bytes)
                    .decompress()
                    .ok_or_else(|| de::Error::custom("Invalid gamma point"))?;

                let c = Scalar::from_canonical_bytes(c_bytes)
                    .into_option()
                    .ok_or_else(|| de::Error::custom("Invalid c scalar"))?;

                let s = Scalar::from_canonical_bytes(s_bytes)
                    .into_option()
                    .ok_or_else(|| de::Error::custom("Invalid s scalar"))?;

                Ok(VrfProof { gamma, c, s })
            }
        }

        deserializer.deserialize_tuple(3, VrfProofVisitor)
    }
}

impl fmt::Debug for VrfProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VrfProof")
            .field("gamma", &hex::encode(self.gamma.compress().as_bytes()))
            .field("c", &"[scalar]")
            .field("s", &"[scalar]")
            .finish()
    }
}

/// VRF output (pseudorandom value)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VrfOutput {
    pub value: [u8; 32],
}

impl VrfSecretKey {
    /// Generate a new random VRF secret key
    pub fn generate() -> Self {
        // Keep deterministic behavior under Miri for testing, but use a
        // secure OS RNG in all other environments.
        if cfg!(miri) {
            // deterministic seed under Miri
            Self {
                scalar: Scalar::from_bytes_mod_order([0u8; 32]),
            }
        } else {
            let mut bytes = [0u8; 32];
            SysRng
                .try_fill_bytes(&mut bytes)
                .expect("Failed to get OS randomness");

            Self {
                scalar: Scalar::from_bytes_mod_order(bytes),
            }
        }
    }

    /// Create from raw bytes (clamped to valid scalar)
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self {
            scalar: Scalar::from_bytes_mod_order(bytes),
        }
    }

    /// Get raw bytes of the scalar
    pub fn to_bytes(&self) -> [u8; 32] {
        self.scalar.to_bytes()
    }

    /// Get the corresponding public key (scalar * basepoint)
    pub fn public_key(&self) -> VrfPublicKey {
        VrfPublicKey {
            point: &self.scalar * RISTRETTO_BASEPOINT_TABLE,
        }
    }

    /// Prove VRF evaluation for given input
    pub fn prove(&self, alpha: &[u8]) -> (VrfOutput, VrfProof) {
        // ECVRF-PROVE algorithm (simplified, using Ristretto255)

        // 1. Hash to curve: H = hash_to_curve(alpha)
        let h = vrf_hash_to_curve(alpha)
            .expect("Hash-to-curve should not fail with valid input - this indicates a critical cryptographic error");

        // 2. Compute gamma = sk * H (the VRF hash point)
        let gamma = self.scalar * h;

        // 3. Generate deterministic nonce k from sk and alpha
        let k = self.generate_nonce(alpha, &gamma);

        // 4. Compute commitment points: U = k*G, V = k*H
        let u = &k * RISTRETTO_BASEPOINT_TABLE;
        let v = k * h;

        // 5. Compute challenge c = Hash(pk, H, gamma, U, V)
        let pk = self.public_key();
        let c = vrf_hash_challenge(&pk.point, &h, &gamma, &u, &v);

        // 6. Compute response s = k + c*sk (mod order)
        let s = k + (c * self.scalar);

        // 7. Compute output = Hash(gamma)
        let output = vrf_proof_to_hash(&gamma);

        let proof = VrfProof { gamma, c, s };

        (VrfOutput { value: output }, proof)
    }

    /// Generate deterministic nonce from secret key and input
    fn generate_nonce(&self, alpha: &[u8], gamma: &RistrettoPoint) -> Scalar {
        let scalar_bytes = self.scalar.to_bytes();
        let hash = hash_with_domain(
            b"vrf_nonce_v1",
            &[&scalar_bytes, alpha, gamma.compress().as_bytes()],
        );
        // FIX #4: Use unbiased scalar generation for nonce (RFC 9381 compliant)
        // This prevents lattice attacks that could recover the VRF secret key
        hash_to_scalar_unbiased(&hash)
    }
}

impl VrfPublicKey {
    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Option<Self> {
        let compressed = CompressedRistretto::from_slice(&bytes).ok()?;
        let point = compressed.decompress()?;
        Some(Self { point })
    }

    /// Get raw bytes
    pub fn as_bytes(&self) -> [u8; 32] {
        *self.point.compress().as_bytes()
    }

    /// Verify VRF proof and recover output
    ///
    /// This implements the verification equation:
    /// s*G == U + c*PK  and  s*H == V + c*Gamma
    pub fn verify(&self, alpha: &[u8], proof: &VrfProof) -> Option<VrfOutput> {
        // 1. Recompute H = hash_to_curve(alpha)
        let h = vrf_hash_to_curve(alpha).ok()?;

        // 2. Recompute challenge c' = Hash(pk, H, gamma, U', V')
        //    where U' = s*G - c*PK  and  V' = s*H - c*Gamma
        let s_g = &proof.s * RISTRETTO_BASEPOINT_TABLE;
        let c_pk = proof.c * self.point;
        let u = s_g - c_pk;

        let s_h = proof.s * h;
        let c_gamma = proof.c * proof.gamma;
        let v = s_h - c_gamma;

        let c_prime = vrf_hash_challenge(&self.point, &h, &proof.gamma, &u, &v);

        // 3. Verify that c == c'
        if proof.c != c_prime {
            return None;
        }

        // 4. Compute and return output = Hash(gamma)
        let output = vrf_proof_to_hash(&proof.gamma);
        Some(VrfOutput { value: output })
    }
}

impl VrfOutput {
    /// Convert to u64 for leader election (use first 8 bytes)
    pub fn to_u64(&self) -> u64 {
        u64::from_le_bytes(self.value[..8].try_into().unwrap_or([0u8; 8]))
    }
}

impl fmt::Display for VrfOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.value[..8]))
    }
}

impl fmt::Debug for VrfSecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VrfSecretKey([REDACTED])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecvrf_keygen() {
        let sk = VrfSecretKey::generate();
        let pk1 = sk.public_key();

        // Public key should be deterministic
        let pk2 = sk.public_key();
        assert_eq!(pk1, pk2);
    }

    #[test]
    fn test_ecvrf_prove_verify() {
        let sk = VrfSecretKey::generate();
        let pk = sk.public_key();

        let alpha = b"test_input";
        let (output, proof) = sk.prove(alpha);

        // Verify proof
        let verified_output = pk.verify(alpha, &proof);
        assert!(verified_output.is_some());
        assert_eq!(output, verified_output.unwrap());
    }

    #[test]
    fn test_ecvrf_different_inputs() {
        let sk = VrfSecretKey::generate();
        let _pk = sk.public_key();

        let (output1, _) = sk.prove(b"input1");
        let (output2, _) = sk.prove(b"input2");

        // Different inputs should produce different outputs
        assert_ne!(output1, output2);
    }

    #[test]
    fn test_ecvrf_deterministic() {
        let sk = VrfSecretKey::from_bytes([42u8; 32]);

        let alpha = b"deterministic_test";
        let (output1, _) = sk.prove(alpha);
        let (output2, _) = sk.prove(alpha);

        // Same key and input should produce same output
        assert_eq!(output1, output2);
    }

    #[test]
    fn test_ecvrf_output_to_u64() {
        let sk = VrfSecretKey::generate();
        let (output, _) = sk.prove(b"test");

        let value = output.to_u64();
        // Value might be zero, so we just check it's valid
        let _ = value;
    }

    #[test]
    fn test_ecvrf_uniqueness() {
        let sk = VrfSecretKey::generate();

        let mut outputs = std::collections::HashSet::new();
        for i in 0..100 {
            let input = format!("input_{}", i);
            let (output, _) = sk.prove(input.as_bytes());
            outputs.insert(output.to_u64());
        }

        // Most outputs should be unique (allowing some collisions in u64 space)
        assert!(outputs.len() > 90);
    }

    #[test]
    fn test_ecvrf_invalid_proof() {
        let sk = VrfSecretKey::generate();
        let pk = sk.public_key();

        let alpha = b"test_input";
        let (_output, mut proof) = sk.prove(alpha);

        // Tamper with the proof
        proof.c = Scalar::from(999u64);

        // Verification should fail
        let verified = pk.verify(alpha, &proof);
        assert!(verified.is_none());
    }

    #[test]
    fn test_ecvrf_serialization() {
        let sk = VrfSecretKey::from_bytes([42u8; 32]);
        let pk = sk.public_key();

        // Test public key serialization
        let pk_bytes = pk.as_bytes();
        let pk_restored = VrfPublicKey::from_bytes(pk_bytes);
        assert!(pk_restored.is_some());
        assert_eq!(pk, pk_restored.unwrap());

        // Test secret key serialization
        let sk_bytes = sk.to_bytes();
        let sk_restored = VrfSecretKey::from_bytes(sk_bytes);
        assert_eq!(sk.to_bytes(), sk_restored.to_bytes());
    }
}
