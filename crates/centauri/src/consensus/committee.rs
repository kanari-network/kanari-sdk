// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Dynamic Committee Management
//!
//! Enables runtime changes to the validator set:
//! - Adding new validators
//! - Removing validators
//! - Updating validator stakes
//! - Epoch-based transitions
//!
//! Inspired by Sui's validator set management.

use anyhow::{Result, anyhow};
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::AuthorityId;
use super::crypto_signatures::Ed25519Keypair;

/// Validator information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidatorInfo {
    pub authority_id: AuthorityId,
    pub public_key: Vec<u8>,
    pub network_address: String,
    pub active: bool,
}

/// Committee (set of validators)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Committee {
    pub epoch: u64,
    pub validators: BTreeMap<AuthorityId, ValidatorInfo>,
    pub quorum_size: usize,
}

impl Committee {
    fn compute_quorum_size(validators: &[ValidatorInfo]) -> usize {
        let total = validators.len();
        if total == 0 {
            return 0;
        }
        (2 * total).div_ceil(3)
    }

    pub fn new(epoch: u64, validators: Vec<ValidatorInfo>) -> Self {
        let quorum_size = Self::compute_quorum_size(&validators);

        let validators_map: BTreeMap<AuthorityId, ValidatorInfo> = validators
            .into_iter()
            .map(|v| (v.authority_id.clone(), v))
            .collect();

        Self {
            epoch,
            validators: validators_map,
            quorum_size,
        }
    }

    pub fn get_validator(&self, authority: &str) -> Option<&ValidatorInfo> {
        self.validators.get(authority)
    }

    pub fn contains(&self, authority: &str) -> bool {
        self.validators.contains_key(authority)
    }

    pub fn verify_quorum_certificate(&self, signers: &[AuthorityId]) -> Result<()> {
        let unique_signers: std::collections::HashSet<&str> =
            signers.iter().map(|s| s.as_str()).collect();
        let trusted_count = unique_signers
            .iter()
            .filter(|auth| {
                self.validators
                    .get(**auth) // Dereference &&str to &str
                    .map(|v| v.active)
                    .unwrap_or(false)
            })
            .count();

        if trusted_count >= self.quorum_size {
            Ok(())
        } else {
            Err(anyhow!(
                "Insufficient validators in quorum certificate: {} < {}",
                trusted_count,
                self.quorum_size
            ))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommitteeChange {
    AddValidator(ValidatorInfo),
    RemoveValidator {
        authority_id: AuthorityId,
        reason: String,
    },
    DeactivateValidator {
        authority_id: AuthorityId,
    },
    ReactivateValidator {
        authority_id: AuthorityId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitteeChangeTx {
    pub change: CommitteeChange,
    pub target_epoch: u64,
    pub signatures: Vec<(AuthorityId, Vec<u8>)>,
}

const MAX_PENDING_EPOCHS: u64 = 100;
const MAX_COMMITTEE_HISTORY: usize = 1000;

pub struct CommitteeManager {
    current_committee: Committee,
    pending_changes: BTreeMap<u64, Vec<CommitteeChange>>,
    committee_history: BTreeMap<u64, Committee>,
}

// FIX #17: Constants for committee change limits
const MAX_CHANGES_PER_EPOCH: usize = 1000; // Maximum pending changes per epoch to prevent OOM

impl CommitteeManager {
    fn ensure_future_epoch(current_epoch: u64, target_epoch: u64, action: &str) -> Result<()> {
        if target_epoch <= current_epoch {
            return Err(anyhow!(
                "{} requires a future epoch: current={}, target={}",
                action,
                current_epoch,
                target_epoch
            ));
        }
        Ok(())
    }

    fn apply_change(
        new_validators: &mut BTreeMap<AuthorityId, ValidatorInfo>,
        change: CommitteeChange,
    ) {
        match change {
            CommitteeChange::AddValidator(info) => {
                tracing::info!("Adding validator: {}", info.authority_id);
                new_validators.insert(info.authority_id.clone(), info);
            }
            CommitteeChange::RemoveValidator {
                authority_id,
                reason,
            } => {
                tracing::info!("Removing validator {}: {}", authority_id, reason);
                new_validators.remove(&authority_id);
            }
            CommitteeChange::DeactivateValidator { authority_id } => {
                if let Some(validator) = new_validators.get_mut(&authority_id) {
                    tracing::info!("Deactivating validator: {}", authority_id);
                    validator.active = false;
                }
            }
            CommitteeChange::ReactivateValidator { authority_id } => {
                if let Some(validator) = new_validators.get_mut(&authority_id) {
                    tracing::info!("Reactivating validator: {}", authority_id);
                    validator.active = true;
                }
            }
        }
    }

    fn signature_signers(tx: &CommitteeChangeTx) -> Vec<AuthorityId> {
        tx.signatures.iter().map(|(auth, _)| auth.clone()).collect()
    }

    pub fn new(initial_committee: Committee) -> Self {
        let epoch = initial_committee.epoch;
        let mut history = BTreeMap::new();
        history.insert(epoch, initial_committee.clone());

        Self {
            current_committee: initial_committee,
            pending_changes: BTreeMap::new(),
            committee_history: history,
        }
    }

    pub fn propose_change(&mut self, change: CommitteeChange, target_epoch: u64) -> Result<()> {
        Self::ensure_future_epoch(self.current_committee.epoch, target_epoch, "Propose change")?;

        // FIX #17: CRITICAL - Prevent memory exhaustion via committee change spam
        // Previously allowed unlimited changes per epoch, enabling OOM attacks
        let changes = self.pending_changes.entry(target_epoch).or_default();

        if changes.len() >= MAX_CHANGES_PER_EPOCH {
            anyhow::bail!(
                "Too many pending changes for epoch {} (max: {}). Rejecting new proposal.",
                target_epoch,
                MAX_CHANGES_PER_EPOCH
            );
        }

        // FIX #17: Deduplicate changes to prevent storing identical proposals multiple times
        // Check if this exact change already exists in the pending list
        let is_duplicate = changes.iter().any(|existing| match (&change, existing) {
            (
                CommitteeChange::AddValidator(new_info),
                CommitteeChange::AddValidator(existing_info),
            ) => new_info.authority_id == existing_info.authority_id,
            (
                CommitteeChange::RemoveValidator {
                    authority_id: id1, ..
                },
                CommitteeChange::RemoveValidator {
                    authority_id: id2, ..
                },
            ) => id1 == id2,
            _ => false,
        });

        if is_duplicate {
            tracing::debug!(
                "Duplicate committee change ignored for epoch {}",
                target_epoch
            );
            return Ok(()); // Silently ignore duplicates instead of error
        }

        changes.push(change);
        tracing::info!("Proposed committee change for epoch {}", target_epoch);
        Ok(())
    }

    pub fn advance_epoch(&mut self, new_epoch: u64) -> Result<Committee> {
        Self::ensure_future_epoch(self.current_committee.epoch, new_epoch, "Advance epoch")?;

        let mut new_validators: BTreeMap<AuthorityId, ValidatorInfo> =
            self.current_committee.validators.clone();

        if let Some(changes) = self.pending_changes.remove(&new_epoch) {
            for change in changes {
                Self::apply_change(&mut new_validators, change);
            }
        }

        let validators: Vec<ValidatorInfo> = new_validators.into_values().collect();
        let new_committee = Committee::new(new_epoch, validators);

        self.committee_history
            .insert(new_epoch, new_committee.clone());
        self.current_committee = new_committee.clone();

        tracing::info!(
            "Advanced to epoch {} with {} validators",
            new_epoch,
            self.current_committee.validators.len()
        );

        Ok(new_committee)
    }

    pub fn prune_old_data(&mut self) {
        let current_epoch = self.current_committee.epoch;
        let max_future_epoch = current_epoch + MAX_PENDING_EPOCHS;
        self.pending_changes
            .retain(|&epoch, _| epoch >= current_epoch && epoch <= max_future_epoch);

        if self.committee_history.len() > MAX_COMMITTEE_HISTORY {
            let cutoff_epoch = current_epoch.saturating_sub(MAX_COMMITTEE_HISTORY as u64);
            self.committee_history
                .retain(|&epoch, _| epoch >= cutoff_epoch);
        }
    }

    pub fn verify_change_tx(&self, tx: &CommitteeChangeTx, chain_id: &str) -> Result<()> {
        let signers = Self::signature_signers(tx);
        let unique_count = signers
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        if unique_count != signers.len() {
            return Err(anyhow!("Duplicate signers in committee change transaction"));
        }

        self.current_committee.verify_quorum_certificate(&signers)?;

        // FIX #6: Add chain_id to SignPayload to prevent cross-chain replay attacks
        // Without chain_id, attackers can capture signatures from Testnet and replay them on Mainnet
        #[derive(Serialize)]
        struct SignPayload<'a> {
            chain_id: &'a str,
            change: &'a CommitteeChange,
            target_epoch: u64,
        }

        let payload = bcs::to_bytes(&SignPayload {
            chain_id,
            change: &tx.change,
            target_epoch: tx.target_epoch,
        })
        .map_err(|_| anyhow!("Failed to serialize tx payload"))?;

        for (authority, signature) in &tx.signatures {
            let validator = self
                .current_committee
                .get_validator(authority)
                .ok_or_else(|| anyhow!("Signer {} not in current committee", authority))?;

            if signature.is_empty() {
                return Err(anyhow!("Invalid signature from {}", authority));
            }

            let pub_key_bytes: [u8; 32] = validator
                .public_key
                .clone()
                .try_into()
                .map_err(|_| anyhow!("Invalid public key length for {}", authority))?;

            let pub_key = VerifyingKey::from_bytes(&pub_key_bytes)
                .map_err(|e| anyhow!("Invalid public key for {}: {}", authority, e))?;

            Ed25519Keypair::verify(&pub_key, &payload, signature)
                .map_err(|e| anyhow!("Signature verification failed for {}: {}", authority, e))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_validator(id: &str) -> ValidatorInfo {
        ValidatorInfo {
            authority_id: id.to_string(),
            public_key: vec![0u8; 32],
            network_address: format!("127.0.0.1:{}", 9000 + id.len()),
            active: true,
        }
    }

    fn create_test_committee() -> Committee {
        let validators = vec![
            create_test_validator("auth1"),
            create_test_validator("auth2"),
            create_test_validator("auth3"),
            create_test_validator("auth4"),
        ];
        Committee::new(0, validators)
    }

    #[test]
    fn test_committee_creation() {
        let committee = create_test_committee();
        assert_eq!(committee.epoch, 0);
        assert_eq!(committee.validators.len(), 4);
        // 4 validators require a 2/3 supermajority quorum of 3
        assert_eq!(committee.quorum_size, 3);
    }

    #[test]
    fn test_three_validator_quorum_is_not_single_vote() {
        let validators = vec![
            create_test_validator("auth1"),
            create_test_validator("auth2"),
            create_test_validator("auth3"),
        ];
        let committee = Committee::new(0, validators);
        assert_eq!(committee.quorum_size, 2);
    }

    #[test]
    fn test_quorum_verification() {
        let committee = create_test_committee();
        assert!(3 >= committee.quorum_size);
        assert!(2 < committee.quorum_size);
    }

    #[test]
    fn test_verify_quorum_certificate_rejects_duplicate_signers() {
        let committee = create_test_committee();
        let signers = vec![
            "auth1".to_string(),
            "auth1".to_string(),
            "auth2".to_string(),
        ];
        // Only 2 unique signers, quorum is 3
        assert!(committee.verify_quorum_certificate(&signers).is_err());
    }

    #[test]
    fn test_add_validator() {
        let committee = create_test_committee();
        let mut manager = CommitteeManager::new(committee);
        let new_validator = create_test_validator("auth5");
        let change = CommitteeChange::AddValidator(new_validator);

        manager.propose_change(change, 1).unwrap();
        let new_committee = manager.advance_epoch(1).unwrap();
        assert_eq!(new_committee.validators.len(), 5);
        // 5 validators require a 2/3 supermajority quorum of 4
        assert_eq!(new_committee.quorum_size, 4);
    }
}
