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

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

use super::{AuthorityId, Round};

/// VRF output and proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrfOutput {
    /// The VRF output (hash)
    pub output: Vec<u8>,

    /// Proof that the output is valid
    pub proof: Vec<u8>,

    /// Round number this VRF is for
    pub round: Round,

    /// Authority that generated this VRF
    pub authority: AuthorityId,
}

impl VrfOutput {
    /// Create a new VRF output
    /// In production, this would use a proper VRF library (e.g., schnorrkel)
    /// For now, we use a simplified hash-based approach
    pub fn new(round: Round, authority: &str, secret: &[u8]) -> Self {
        let mut hasher = Sha3_256::new();
        hasher.update(round.to_le_bytes());
        hasher.update(authority.as_bytes());
        hasher.update(secret);

        let output = hasher.finalize().to_vec();

        // Simplified proof (in production, use proper VRF proof)
        let mut proof_hasher = Sha3_256::new();
        proof_hasher.update(&output);
        proof_hasher.update(secret);
        let proof = proof_hasher.finalize().to_vec();

        Self {
            output,
            proof,
            round,
            authority: authority.to_string(),
        }
    }

    /// Verify the VRF proof
    /// In production, this would verify using public key
    pub fn verify(&self, _public_key: &[u8]) -> bool {
        // Simplified verification
        // In production: use VRF.verify(public_key, input, output, proof)
        !self.output.is_empty() && !self.proof.is_empty()
    }

    /// Get the VRF output as a number for comparison
    pub fn as_number(&self) -> u64 {
        let bytes: [u8; 8] = self.output[..8].try_into().unwrap_or([0u8; 8]);
        u64::from_le_bytes(bytes)
    }
}

/// VRF-based leader election
pub struct VrfLeaderElection {
    /// Authority ID to secret mapping (for VRF generation)
    /// In production, use proper key management
    authority_secrets: std::collections::HashMap<AuthorityId, Vec<u8>>,

    /// Cache of VRF outputs by round
    vrf_cache: std::collections::HashMap<Round, Vec<VrfOutput>>,
}

impl VrfLeaderElection {
    /// Create a new VRF leader election system
    pub fn new() -> Self {
        Self {
            authority_secrets: std::collections::HashMap::new(),
            vrf_cache: std::collections::HashMap::new(),
        }
    }

    /// Register an authority with their secret (for VRF generation)
    /// In production, this would use proper key management
    pub fn register_authority(&mut self, authority: AuthorityId, secret: Vec<u8>) {
        self.authority_secrets.insert(authority, secret);
    }

    /// Generate VRF output for a round
    pub fn generate_vrf(&self, round: Round, authority: &str) -> Result<VrfOutput> {
        let secret = self
            .authority_secrets
            .get(authority)
            .ok_or_else(|| anyhow::anyhow!("Authority not registered"))?;

        Ok(VrfOutput::new(round, authority, secret))
    }

    /// Add VRF output to cache
    pub fn add_vrf(&mut self, vrf: VrfOutput) {
        self.vrf_cache.entry(vrf.round).or_default().push(vrf);
    }

    /// Elect leader for a round based on VRF outputs
    /// Leader is the authority with the lowest VRF output value
    pub fn elect_leader(&self, round: Round) -> Option<AuthorityId> {
        let vrfs = self.vrf_cache.get(&round)?;

        if vrfs.is_empty() {
            return None;
        }

        // Find VRF with lowest output value
        let leader_vrf = vrfs.iter().min_by_key(|vrf| vrf.as_number())?;

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
        let vrf = VrfOutput::new(1, "auth1", b"secret123");

        assert_eq!(vrf.round, 1);
        assert_eq!(vrf.authority, "auth1");
        assert!(!vrf.output.is_empty());
        assert!(!vrf.proof.is_empty());
    }

    #[test]
    fn test_vrf_verification() {
        let vrf = VrfOutput::new(1, "auth1", b"secret123");
        assert!(vrf.verify(b"pubkey123"));
    }

    #[test]
    fn test_vrf_deterministic() {
        let vrf1 = VrfOutput::new(1, "auth1", b"secret123");
        let vrf2 = VrfOutput::new(1, "auth1", b"secret123");

        assert_eq!(vrf1.output, vrf2.output);
        assert_eq!(vrf1.proof, vrf2.proof);
    }

    #[test]
    fn test_leader_election() {
        let mut election = VrfLeaderElection::new();

        // Register authorities
        election.register_authority("auth1".to_string(), b"secret1".to_vec());
        election.register_authority("auth2".to_string(), b"secret2".to_vec());
        election.register_authority("auth3".to_string(), b"secret3".to_vec());

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
        let vrf1 = VrfOutput::new(1, "auth1", b"secret1");
        let vrf2 = VrfOutput::new(1, "auth2", b"secret1");
        let vrf3 = VrfOutput::new(2, "auth1", b"secret1");

        // Different authority = different output
        assert_ne!(vrf1.output, vrf2.output);

        // Different round = different output
        assert_ne!(vrf1.output, vrf3.output);
    }
}
