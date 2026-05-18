// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Light Client Support for DAG Consensus
//!
//! Enables lightweight clients to:
//! - Verify transactions without full DAG sync
//! - Verify checkpoints with minimal data
//! - Query account state with Merkle proofs
//!
//! Inspired by Sui's light client design.

use anyhow::{Result, anyhow};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use smt::compute_merkle_root;
use std::collections::BTreeMap;

use super::{AuthorityId, Checkpoint, Round};

/// Light client checkpoint verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightCheckpoint {
    /// Checkpoint sequence number
    pub sequence: u64,

    /// State root hash
    pub state_root: Vec<u8>,

    /// Transaction root hash
    pub tx_root: Vec<u8>,

    /// Round/epoch
    pub epoch: Round,

    /// Quorum signatures (2f+1 authorities)
    pub signatures: Vec<CheckpointSignature>,
}

/// Authority signature on checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointSignature {
    /// Authority ID
    pub authority: AuthorityId,
    /// Signature bytes
    pub signature: Vec<u8>,
}

/// State proof for light client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateProof {
    /// Account address
    pub address: String,
    /// Account state data (value)
    /// If is_membership is false, this should be empty
    pub state_data: Vec<u8>,
    /// Whether this is a membership proof (true) or non-membership proof (false)
    pub is_membership: bool,
    /// Merkle proof siblings (bottom-up from leaf)
    pub siblings: Vec<[u8; 32]>,
    /// State root hash
    pub state_root: Vec<u8>,
    /// Checkpoint this proof is based on
    pub checkpoint: LightCheckpoint,
}

/// Transaction inclusion proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionProof {
    /// Transaction hash
    pub tx_hash: Vec<u8>,

    /// Transaction index in the block/checkpoint
    pub tx_index: usize,

    /// Transaction data
    pub tx_data: Vec<u8>,

    /// Merkle proof path
    pub path: Vec<Vec<u8>>,

    /// Transaction root hash
    pub tx_root: Vec<u8>,

    /// Checkpoint containing this transaction
    pub checkpoint: LightCheckpoint,
}

/// Light client
pub struct LightClient {
    /// Verified checkpoints (sequence -> checkpoint)
    verified_checkpoints: BTreeMap<u64, LightCheckpoint>,

    /// Latest verified checkpoint
    latest_checkpoint: u64,

    /// Authority public keys for signature verification
    authority_keys: BTreeMap<AuthorityId, Vec<u8>>,
}

impl LightClient {
    fn has_verified_checkpoint(&self, sequence: u64) -> bool {
        self.verified_checkpoints.contains_key(&sequence)
    }

    fn ensure_verified_checkpoint(&self, sequence: u64) -> Result<()> {
        if self.has_verified_checkpoint(sequence) {
            Ok(())
        } else {
            Err(anyhow!("Checkpoint {} not verified", sequence))
        }
    }

    fn checkpoint_payload(
        sequence: u64,
        state_root: &[u8],
        tx_root: &[u8],
        epoch: Round,
    ) -> String {
        format!(
            "{}:{}:{}:{}",
            sequence,
            hex::encode(state_root),
            hex::encode(tx_root),
            epoch
        )
    }

    /// Create new light client
    pub fn new(authority_keys: BTreeMap<AuthorityId, Vec<u8>>) -> Self {
        Self {
            verified_checkpoints: BTreeMap::new(),
            latest_checkpoint: 0,
            authority_keys,
        }
    }
    /// Verify and add checkpoint
    pub fn verify_checkpoint(
        &mut self,
        checkpoint: LightCheckpoint,
        committee: &crate::consensus::committee::Committee,
    ) -> Result<()> {
        // Check if we already have this checkpoint
        if self.has_verified_checkpoint(checkpoint.sequence) {
            return Ok(());
        }

        let checkpoint_hash = self.hash_checkpoint(&checkpoint);
        let mut seen = std::collections::BTreeSet::new();
        let mut trusted_count = 0usize;

        for sig in &checkpoint.signatures {
            if !seen.insert(sig.authority.clone()) {
                return Err(anyhow::anyhow!(
                    "Duplicate signature from authority {}",
                    sig.authority
                ));
            }

            // Verify authority is known and active
            if !self.authority_keys.contains_key(&sig.authority) {
                return Err(anyhow::anyhow!("Unknown authority: {}", sig.authority));
            }

            // Verify authority exists in committee and is active
            let validator = committee.get_validator(&sig.authority).ok_or_else(|| {
                anyhow::anyhow!("Authority {} not found in committee", sig.authority)
            })?;

            if !validator.active {
                return Err(anyhow::anyhow!("Authority {} is not active", sig.authority));
            }

            // Verify signature against the authority public key
            self.verify_signature(&checkpoint_hash, &sig.signature, &sig.authority)?;

            trusted_count += 1;
        }

        // Check count-based quorum (PoA: 2f+1 of authorities)
        if trusted_count < committee.quorum_size {
            return Err(anyhow::anyhow!(
                "Insufficient validators in quorum: {} < {}",
                trusted_count,
                committee.quorum_size
            ));
        }

        // Add verified checkpoint
        self.verified_checkpoints
            .insert(checkpoint.sequence, checkpoint.clone());

        if checkpoint.sequence > self.latest_checkpoint {
            self.latest_checkpoint = checkpoint.sequence;
        }

        tracing::info!(
            "Verified checkpoint {} with {} signatures",
            checkpoint.sequence,
            checkpoint.signatures.len()
        );

        Ok(())
    }

    /// Verify state proof using SMT verification
    pub fn verify_state_proof(&self, proof: &StateProof) -> Result<()> {
        self.ensure_verified_checkpoint(proof.checkpoint.sequence)?;

        // Verify state root matches checkpoint
        if proof.state_root != proof.checkpoint.state_root {
            return Err(anyhow!("State root mismatch"));
        }

        // Verify Merkle proof using SMT library
        let mut root_arr = [0u8; 32];
        if proof.state_root.len() == 32 {
            root_arr.copy_from_slice(&proof.state_root);
        } else {
            return Err(anyhow!("Invalid state root length"));
        }

        // Reconstruct leaf hash (consistent with SMT implementation)
        let key_hash = smt::digest(proof.address.as_bytes());
        let leaf_hash = if proof.is_membership {
            smt::hash_leaf(&key_hash, &proof.state_data)
        } else {
            // Use default leaf hash for non-membership
            // FIX #3: Add bounds checking to prevent panic if library changes
            let default_hashes = smt::default_hashes();
            if default_hashes.len() <= 256 {
                return Err(anyhow!(
                    "SMT default_hashes array too small: expected at least 257 elements, got {}",
                    default_hashes.len()
                ));
            }
            default_hashes[256]
        };

        // verify_proof returns true if valid
        // Proof tuple: (is_member, leaf_hash, siblings)
        if !smt::verify_proof(
            &root_arr,
            proof.address.as_bytes(),
            (proof.is_membership, leaf_hash, proof.siblings.clone()),
        ) {
            return Err(anyhow!("Invalid Merkle proof for state"));
        }

        tracing::debug!(
            "Verified state proof for address {} at checkpoint {}",
            proof.address,
            proof.checkpoint.sequence
        );

        Ok(())
    }

    /// Verify transaction proof using Merkle proof verification
    pub fn verify_transaction_proof(&self, proof: &TransactionProof) -> Result<()> {
        self.ensure_verified_checkpoint(proof.checkpoint.sequence)?;

        // Verify transaction hash matches data
        let tx_hash = kanari_crypto::hash_data_blake3(&proof.tx_data);
        if tx_hash != proof.tx_hash {
            return Err(anyhow!("Transaction hash mismatch"));
        }

        // Verify tx root matches checkpoint
        if proof.tx_root != proof.checkpoint.tx_root {
            return Err(anyhow!("Transaction root mismatch"));
        }

        // Verify Merkle inclusion proof using smt library
        if !smt::verify_merkle_proof(&proof.tx_hash, proof.tx_index, &proof.path, &proof.tx_root) {
            return Err(anyhow!("Invalid Merkle proof for transaction inclusion"));
        }

        tracing::debug!(
            "Verified transaction inclusion at checkpoint {}",
            proof.checkpoint.sequence
        );

        Ok(())
    }

    /// Hash checkpoint for signing
    fn hash_checkpoint(&self, checkpoint: &LightCheckpoint) -> Vec<u8> {
        let data = Self::checkpoint_payload(
            checkpoint.sequence,
            &checkpoint.state_root,
            &checkpoint.tx_root,
            checkpoint.epoch,
        );
        kanari_crypto::hash_data_blake3(data.as_bytes())
    }

    /// Verify ed25519 signature.
    fn verify_signature(&self, message: &[u8], signature: &[u8], authority: &str) -> Result<()> {
        let pubkey = self
            .authority_keys
            .get(authority)
            .ok_or_else(|| anyhow!("Authority not found: {}", authority))?;
        let key_bytes: [u8; 32] = pubkey
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("Invalid public key length for {}", authority))?;
        let verifying_key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|e| anyhow!("Invalid public key for {}: {}", authority, e))?;
        let sig_bytes: [u8; 64] = signature
            .try_into()
            .map_err(|_| anyhow!("Invalid signature length for {}", authority))?;
        let sig = Signature::from_bytes(&sig_bytes);
        verifying_key
            .verify_strict(message, &sig)
            .map_err(|e| anyhow!("Invalid signature from {}: {}", authority, e))
    }
}

/// Checkpoint builder (full node only)
pub struct CheckpointBuilder {
    /// Current checkpoint sequence
    sequence: u64,

    /// Authority ID
    authority_id: AuthorityId,

    /// Authority secret key
    secret_key: Vec<u8>,
}

impl CheckpointBuilder {
    /// Create new checkpoint builder
    pub fn new(sequence: u64, authority_id: AuthorityId, secret_key: Vec<u8>) -> Self {
        Self {
            sequence,
            authority_id,
            secret_key,
        }
    }

    /// Build light checkpoint from full checkpoint
    pub fn build_light_checkpoint(
        &self,
        checkpoint: &Checkpoint,
        signatures: Vec<CheckpointSignature>,
    ) -> LightCheckpoint {
        // Calculate tx_root from actual transactions
        let tx_hashes: Vec<Vec<u8>> = checkpoint.transactions.iter().map(|tx| tx.hash()).collect();
        let tx_root = if tx_hashes.is_empty() {
            vec![0u8; 32]
        } else {
            compute_merkle_root(&tx_hashes)
        };

        LightCheckpoint {
            sequence: checkpoint.sequence,
            state_root: checkpoint.state_root.clone(),
            tx_root,
            epoch: checkpoint.sequence, // Use sequence as epoch
            signatures,
        }
    }

    /// Sign checkpoint
    pub fn sign_checkpoint(&self, checkpoint: &LightCheckpoint) -> CheckpointSignature {
        // Verify sequence is expected
        if checkpoint.sequence != self.sequence {
            tracing::warn!(
                "Checkpoint sequence mismatch: expected {}, got {}",
                self.sequence,
                checkpoint.sequence
            );
        }

        let message = LightClient::checkpoint_payload(
            checkpoint.sequence,
            &checkpoint.state_root,
            &checkpoint.tx_root,
            checkpoint.epoch,
        );
        let message_hash = kanari_crypto::hash_data_blake3(message.as_bytes());

        let seed: [u8; 32] = self.secret_key.as_slice().try_into().unwrap_or([0u8; 32]);
        let signing_key = SigningKey::from_bytes(&seed);
        let sig: Signature = ed25519_dalek::Signer::sign(&signing_key, &message_hash);
        let signature = sig.to_bytes().to_vec();

        CheckpointSignature {
            authority: self.authority_id.clone(),
            signature,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::consensus::{Committee, ValidatorInfo};

    use super::*;
    use std::collections::BTreeMap;

    fn create_test_authorities() -> (
        BTreeMap<AuthorityId, Vec<u8>>,
        BTreeMap<AuthorityId, [u8; 32]>,
    ) {
        let mut authorities = BTreeMap::new();
        let mut secrets = BTreeMap::new();
        for i in 0..4 {
            let secret = [i as u8 + 1; 32];
            let signing_key = SigningKey::from_bytes(&secret);
            authorities.insert(
                format!("auth{}", i),
                signing_key.verifying_key().to_bytes().to_vec(),
            );
            secrets.insert(format!("auth{}", i), secret);
        }
        (authorities, secrets)
    }

    fn sign_checkpoint_for(
        authority: &str,
        secret: &[u8; 32],
        checkpoint: &LightCheckpoint,
    ) -> CheckpointSignature {
        let payload = LightClient::checkpoint_payload(
            checkpoint.sequence,
            &checkpoint.state_root,
            &checkpoint.tx_root,
            checkpoint.epoch,
        );
        let payload_hash = kanari_crypto::hash_data_blake3(payload.as_bytes());
        let signing_key = SigningKey::from_bytes(secret);
        let sig: Signature = ed25519_dalek::Signer::sign(&signing_key, &payload_hash);
        CheckpointSignature {
            authority: authority.to_string(),
            signature: sig.to_bytes().to_vec(),
        }
    }

    fn create_test_checkpoint(sequence: u64) -> LightCheckpoint {
        LightCheckpoint {
            sequence,
            state_root: vec![1u8; 32],
            tx_root: vec![2u8; 32],
            epoch: sequence,
            signatures: vec![],
        }
    }

    // Helper function to create Committee for testing
    fn create_test_committee(authorities: &BTreeMap<AuthorityId, Vec<u8>>) -> Committee {
        let validators: Vec<ValidatorInfo> = authorities
            .iter()
            .map(|(id, pk)| ValidatorInfo {
                authority_id: id.clone(),
                public_key: pk.clone(),
                network_address: "127.0.0.1:0".to_string(),
                active: true,
            })
            .collect();
        Committee::new(0, validators)
    }

    #[test]
    fn test_light_client_quorum() {
        let (authorities, secrets) = create_test_authorities();
        let committee = create_test_committee(&authorities); // Create committee
        let mut client = LightClient::new(authorities);

        let mut checkpoint = create_test_checkpoint(1);

        for i in 0..3 {
            let authority = format!("auth{}", i);
            checkpoint.signatures.push(sign_checkpoint_for(
                &authority,
                secrets.get(&authority).unwrap(),
                &checkpoint,
            ));
        }

        // FIX: Pass &committee to the test
        assert!(client.verify_checkpoint(checkpoint, &committee).is_ok());
    }

    #[test]
    fn test_light_client_insufficient_signatures() {
        let (authorities, secrets) = create_test_authorities();
        let committee = create_test_committee(&authorities);
        let mut client = LightClient::new(authorities);

        let mut checkpoint = create_test_checkpoint(1);

        for i in 0..2 {
            let authority = format!("auth{}", i);
            checkpoint.signatures.push(sign_checkpoint_for(
                &authority,
                secrets.get(&authority).unwrap(),
                &checkpoint,
            ));
        }

        // FIX: Pass &committee to the test
        assert!(client.verify_checkpoint(checkpoint, &committee).is_err());
    }

    #[test]
    fn test_checkpoint_builder() {
        let builder = CheckpointBuilder::new(1, "auth1".to_string(), vec![0u8; 32]);

        let checkpoint = create_test_checkpoint(1);
        let signature = builder.sign_checkpoint(&checkpoint);

        assert_eq!(signature.authority, "auth1");
        assert!(!signature.signature.is_empty());
    }
}
