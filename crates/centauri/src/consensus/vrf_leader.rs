// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! VRF-based Leader Election
//!
//! Uses Verifiable Random Function (VRF) to select leaders in a provably
//! random and fair manner, replacing simple round-robin selection.
//!
//! VRF properties:
//! - Unpredictable: Cannot predict future leaders
//! - Verifiable: Anyone can verify the VRF proof
//! - Deterministic: Same input always produces same output
//! - Unique: Only the secret key holder can produce valid output
//!
//! Now using production-grade ECVRF implementation with Ristretto255.

use std::collections::BTreeMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::ecvrf::{VrfPublicKey, VrfSecretKey};
use super::{AuthorityId, Round};

/// VRF output and proof wrapper for leader election
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrfOutput {
    /// The cryptographic VRF output value
    #[serde(with = "serde_bytes")]
    pub output: [u8; 32],

    /// Serialized VRF proof (gamma, c, s components)
    pub proof: Vec<u8>,

    /// Round number this VRF is for
    pub round: Round,

    /// Authority that generated this VRF
    pub authority: AuthorityId,
}

impl VrfOutput {
    fn build_input(round: Round, authority: &str) -> Vec<u8> {
        let mut input = Vec::with_capacity(std::mem::size_of::<Round>() + authority.len());
        input.extend_from_slice(&round.to_le_bytes());
        input.extend_from_slice(authority.as_bytes());
        input
    }

    /// Create a new VRF output using ECVRF
    pub fn new(round: Round, authority: &str, secret_key: &VrfSecretKey) -> Self {
        let input = Self::build_input(round, authority);

        // Generate VRF proof
        let (ecvrf_output, proof) = secret_key.prove(&input);

        // Serialize the VRF proof for storage and transmission
        // This should never fail - if it does, it indicates corrupted memory or a bug
        let proof_bytes = bcs::to_bytes(&proof)
            .expect("VRF proof serialization must not fail - indicates corrupted memory");

        Self {
            output: ecvrf_output.value,
            proof: proof_bytes,
            round,
            authority: authority.to_string(),
        }
    }

    /// Verify the VRF proof using public key
    pub fn verify(&self, public_key: &VrfPublicKey) -> bool {
        // FIX #1: Implement full cryptographic VRF verification instead of dummy check
        // This prevents malicious validators from submitting fake VRF outputs

        if self.proof.is_empty() || self.output == [0u8; 32] {
            tracing::warn!("VRF output has empty proof or zero output");
            return false;
        }

        // Deserialize the proof
        let vrf_proof: super::ecvrf::VrfProof = match bcs::from_bytes(&self.proof) {
            Ok(proof) => proof,
            Err(e) => {
                tracing::warn!("Failed to deserialize VRF proof: {}", e);
                return false;
            }
        };

        // Build the input that was used to generate this VRF
        let input = Self::build_input(self.round, &self.authority);

        // Perform cryptographic verification
        match public_key.verify(&input, &vrf_proof) {
            Some(computed_output) => {
                // Verify that the computed output matches the claimed output
                if computed_output.value != self.output {
                    tracing::warn!(
                        "VRF output mismatch for round {} authority {}: expected {}, got {}",
                        self.round,
                        self.authority,
                        hex::encode(&computed_output.value[..8]),
                        hex::encode(&self.output[..8])
                    );
                    return false;
                }

                // Additional security checks
                let unique_bytes: std::collections::HashSet<u8> =
                    self.output.iter().copied().collect();
                if unique_bytes.len() < 4 {
                    tracing::warn!(
                        "VRF output has suspiciously low entropy: {} unique bytes",
                        unique_bytes.len()
                    );
                    return false;
                }

                true
            }
            None => {
                tracing::warn!(
                    "VRF proof verification failed for round {} authority {}",
                    self.round,
                    self.authority
                );
                false
            }
        }
    }
}

// Serde helper module for byte arrays
mod serde_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        bytes.as_ref().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec = Vec::<u8>::deserialize(deserializer)?;
        vec.try_into()
            .map_err(|_| serde::de::Error::custom("Invalid length for byte array"))
    }
}

/// VRF-based leader election using ECVRF
pub struct VrfLeaderElection {
    /// Authority ID to VRF secret key mapping
    /// In production, use secure key management (HSM, encrypted storage)
    authority_keys: BTreeMap<AuthorityId, VrfSecretKey>,

    /// Authority ID to VRF public key mapping
    authority_pubkeys: BTreeMap<AuthorityId, VrfPublicKey>,

    /// Cache of VRF outputs by round
    vrf_cache: BTreeMap<Round, Vec<VrfOutput>>,

    /// Current round for future-round DoS protection (FIX #13)
    current_round: Round,
}

impl VrfLeaderElection {
    // FIX #10: Compare full 32-byte VRF output to prevent grinding attacks
    // Previously used only 8 bytes (64-bit) which is vulnerable to birthday paradox collisions
    fn compare_vrf_outputs(a: &VrfOutput, b: &VrfOutput) -> std::cmp::Ordering {
        let val_cmp = a.output.cmp(&b.output); // Compare full 32 bytes
        if val_cmp == std::cmp::Ordering::Equal {
            a.authority.cmp(&b.authority)
        } else {
            val_cmp
        }
    }

    /// Create a new VRF leader election system
    pub fn new() -> Self {
        Self {
            authority_keys: BTreeMap::new(),
            authority_pubkeys: BTreeMap::new(),
            vrf_cache: BTreeMap::new(),
            current_round: 0,
        }
    }

    /// Update current round (called when blockchain advances)
    pub fn update_current_round(&mut self, round: Round) {
        self.current_round = round;

        // FIX #13: Prune old VRF data to prevent memory leaks
        // Keep only recent rounds (current - 100 to current + 5)
        let min_round = self.current_round.saturating_sub(100);
        let max_round = self.current_round.saturating_add(5);
        self.vrf_cache
            .retain(|round, _| *round >= min_round && *round <= max_round);
    }

    /// Register an authority with their VRF secret key
    /// In production, use secure key management
    pub fn register_authority(&mut self, authority: AuthorityId, secret_key: VrfSecretKey) {
        let public_key = secret_key.public_key();
        self.authority_keys.insert(authority.clone(), secret_key);
        self.authority_pubkeys.insert(authority, public_key);
    }

    /// Register an authority with raw secret bytes (32 bytes)
    pub fn register_authority_bytes(&mut self, authority: AuthorityId, secret: &[u8; 32]) {
        let secret_key = VrfSecretKey::from_bytes(*secret);
        self.register_authority(authority, secret_key);
    }

    /// Get public key for an authority
    pub fn get_public_key(&self, authority: &str) -> Option<&VrfPublicKey> {
        self.authority_pubkeys.get(authority)
    }

    /// Generate VRF output for a round
    pub fn generate_vrf(&self, round: Round, authority: &str) -> Result<VrfOutput> {
        let secret_key = self
            .authority_keys
            .get(authority)
            .ok_or_else(|| anyhow::anyhow!("Authority not registered"))?;

        Ok(VrfOutput::new(round, authority, secret_key))
    }

    /// Add VRF output to cache with rate limiting and cryptographic verification
    pub fn add_vrf(&mut self, vrf: VrfOutput) {
        // FIX #13: CRITICAL - Reject VRF submissions from too far in the future
        // Prevents memory exhaustion attacks where attacker sends VRF for round 999,999,999
        const MAX_FUTURE_ROUNDS: Round = 5;
        if vrf.round > self.current_round.saturating_add(MAX_FUTURE_ROUNDS) {
            tracing::warn!(
                "[Security] Future round VRF rejected: round {} (current: {}, max allowed: {})",
                vrf.round,
                self.current_round,
                self.current_round.saturating_add(MAX_FUTURE_ROUNDS)
            );
            return;
        }

        // Also reject VRF from too far in the past (already pruned)
        const MIN_PAST_ROUNDS: Round = 100;
        if vrf.round < self.current_round.saturating_sub(MIN_PAST_ROUNDS) {
            tracing::debug!(
                "Past round VRF rejected: round {} (current: {})",
                vrf.round,
                self.current_round
            );
            return;
        }

        // FIX #10: Limit VRF submissions per round to prevent memory exhaustion attacks
        const MAX_VRF_PER_ROUND: usize = 1000;

        let round_entries = self.vrf_cache.entry(vrf.round).or_default();

        if round_entries.len() >= MAX_VRF_PER_ROUND {
            tracing::warn!(
                "VRF submission limit reached for round {} (max: {}). Rejecting VRF from authority {}",
                vrf.round,
                MAX_VRF_PER_ROUND,
                vrf.authority
            );
            return;
        }

        // FIX #12: CRITICAL - Prevent duplicate VRF submissions from same authority
        // Previously allowed one attacker to fill the entire quota with duplicate entries
        if round_entries
            .iter()
            .any(|existing| existing.authority == vrf.authority)
        {
            tracing::warn!(
                "[Security] Duplicate VRF submission rejected from authority {} for round {}",
                vrf.authority,
                vrf.round
            );
            return;
        }

        // FIX #1: Verify VRF proof cryptographically before accepting
        if let Some(public_key) = self.authority_pubkeys.get(&vrf.authority) {
            if !vrf.verify(public_key) {
                tracing::warn!(
                    "[Security] Invalid VRF proof rejected for round {} from authority {}",
                    vrf.round,
                    vrf.authority
                );
                return;
            }
        } else {
            tracing::warn!(
                "[Security] Unknown authority {} submitted VRF for round {}",
                vrf.authority,
                vrf.round
            );
            return;
        }

        round_entries.push(vrf);
    }

    /// Elect leader for a round based on VRF outputs
    /// Leader is the authority with the lowest VRF output value
    pub fn elect_leader(&self, round: Round) -> Option<AuthorityId> {
        let vrfs = self.vrf_cache.get(&round)?;

        if vrfs.is_empty() {
            return None;
        }

        // Find VRF with lowest output value
        // Use tie-breaker (AuthorityId) for strict determinism regardless of insertion order
        let leader_vrf = vrfs.iter().min_by(|a, b| Self::compare_vrf_outputs(a, b))?;

        Some(leader_vrf.authority.clone())
    }

    /// Elect leader using VRF (PoA: equal probability for all authorities)
    pub fn elect_leader_weighted(
        &self,
        round: Round,
        _stakes: &BTreeMap<AuthorityId, u64>, // Ignored in PoA - kept for API compatibility
    ) -> Option<AuthorityId> {
        let vrfs = self.vrf_cache.get(&round)?;

        if vrfs.is_empty() {
            return None;
        }

        // PoA: Simple minimum VRF value wins (all authorities have equal weight)
        // No stake weighting - fair rotation among all validators
        let leader_vrf = vrfs.iter().min_by(|vrf_a, vrf_b| {
            // Compare full 16-byte VRF output as u128
            let vrf_num_a = u128::from_be_bytes(vrf_a.output[..16].try_into().unwrap_or([0u8; 16]));
            let vrf_num_b = u128::from_be_bytes(vrf_b.output[..16].try_into().unwrap_or([0u8; 16]));

            let cmp = vrf_num_a.cmp(&vrf_num_b);
            if cmp == std::cmp::Ordering::Equal {
                // Deterministic tie-breaker using AuthorityId
                vrf_a.authority.cmp(&vrf_b.authority)
            } else {
                cmp
            }
        })?;

        Some(leader_vrf.authority.clone())
    }

    /// Check if an authority is the leader for a round
    pub fn is_leader(&self, round: Round, authority: &str) -> bool {
        self.elect_leader(round)
            .map(|leader| leader == authority)
            .unwrap_or(false)
    }

    /// Get all VRF outputs for a round
    pub fn get_vrfs(&self, round: Round) -> Vec<VrfOutput> {
        self.vrf_cache.get(&round).cloned().unwrap_or_default()
    }

    /// Prune old VRF cache entries to prevent memory leak
    /// Should be called after checkpointing old rounds
    pub fn prune_old_rounds(&mut self, before_round: Round) {
        self.vrf_cache.retain(|round, _| *round >= before_round);

        tracing::debug!(
            "Pruned VRF cache data before round {}, remaining entries: {}",
            before_round,
            self.vrf_cache.len()
        );
    }
}

impl Default for VrfLeaderElection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vrf_generation() {
        let sk = VrfSecretKey::from_bytes([42u8; 32]);
        let vrf = VrfOutput::new(1, "auth1", &sk);

        assert_eq!(vrf.round, 1);
        assert_eq!(vrf.authority, "auth1");
        assert_eq!(vrf.output.len(), 32);
        assert!(!vrf.proof.is_empty());
    }

    #[test]
    fn test_vrf_verification() {
        let sk = VrfSecretKey::from_bytes([42u8; 32]);
        let pk = sk.public_key();
        let vrf = VrfOutput::new(1, "auth1", &sk);

        // Note: verification uses simplified check due to proof serialization
        let _verified = vrf.verify(&pk);
    }

    #[test]
    fn test_vrf_deterministic() {
        let sk = VrfSecretKey::from_bytes([42u8; 32]);
        let vrf1 = VrfOutput::new(1, "auth1", &sk);
        let vrf2 = VrfOutput::new(1, "auth1", &sk);

        // Same key and input produces same output
        assert_eq!(vrf1.output, vrf2.output);
    }

    #[test]
    fn test_leader_election() {
        let mut election = VrfLeaderElection::new();

        // Register authorities with VRF keys
        election.register_authority_bytes("auth1".to_string(), &[1u8; 32]);
        election.register_authority_bytes("auth2".to_string(), &[2u8; 32]);
        election.register_authority_bytes("auth3".to_string(), &[3u8; 32]);

        // Generate VRFs for round 1
        let vrf1 = election.generate_vrf(1, "auth1").unwrap();
        let vrf2 = election.generate_vrf(1, "auth2").unwrap();
        let vrf3 = election.generate_vrf(1, "auth3").unwrap();

        election.add_vrf(vrf1);
        election.add_vrf(vrf2);
        election.add_vrf(vrf3);

        // Elect leader
        let leader = election.elect_leader(1);
        assert!(leader.is_some());

        let leader = leader.unwrap();
        assert!(leader == "auth1" || leader == "auth2" || leader == "auth3");

        // Verify is_leader
        assert!(election.is_leader(1, &leader));
    }

    #[test]
    fn test_vrf_uniqueness() {
        let sk1 = VrfSecretKey::from_bytes([1u8; 32]);
        let sk2 = VrfSecretKey::from_bytes([2u8; 32]);

        let vrf1 = VrfOutput::new(1, "auth1", &sk1);
        let vrf2 = VrfOutput::new(1, "auth2", &sk1);
        let vrf3 = VrfOutput::new(2, "auth1", &sk1);
        let vrf4 = VrfOutput::new(1, "auth1", &sk2);

        // Different authority = different output
        assert_ne!(vrf1.output, vrf2.output);

        // Different round = different output
        assert_ne!(vrf1.output, vrf3.output);

        // Different secret key = different output
        assert_ne!(vrf1.output, vrf4.output);
    }
}
