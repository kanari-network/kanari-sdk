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
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::AuthorityId;

/// Validator information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidatorInfo {
    /// Authority ID
    pub authority_id: AuthorityId,

    /// Public key
    pub public_key: Vec<u8>,

    /// Stake amount
    pub stake: u64,

    /// Network address
    pub network_address: String,

    /// Active status
    pub active: bool,
}

/// Committee (set of validators)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Committee {
    /// Epoch number
    pub epoch: u64,

    /// Validators
    pub validators: HashMap<AuthorityId, ValidatorInfo>,

    /// Total stake
    pub total_stake: u64,

    /// Quorum threshold (2f+1)
    pub quorum_threshold: u64,
}

impl Committee {
    /// Create new committee
    pub fn new(epoch: u64, validators: Vec<ValidatorInfo>) -> Self {
        let total_stake: u64 = validators.iter().map(|v| v.stake).sum();

        // Calculate quorum: 2f+1 stake
        // For Byzantine fault tolerance: f = (n-1)/3, so 2f+1 = (2n+1)/3
        let quorum_threshold = (total_stake * 2 / 3) + 1;

        let validators_map: HashMap<AuthorityId, ValidatorInfo> = validators
            .into_iter()
            .map(|v| (v.authority_id.clone(), v))
            .collect();

        Self {
            epoch,
            validators: validators_map,
            total_stake,
            quorum_threshold,
        }
    }

    /// Get validator info
    pub fn get_validator(&self, authority: &str) -> Option<&ValidatorInfo> {
        self.validators.get(authority)
    }

    /// Check if authority is in committee
    pub fn contains(&self, authority: &str) -> bool {
        self.validators.contains_key(authority)
    }

    /// Get active validators
    pub fn active_validators(&self) -> Vec<&ValidatorInfo> {
        self.validators.values().filter(|v| v.active).collect()
    }

    /// Get total active stake
    pub fn active_stake(&self) -> u64 {
        self.validators
            .values()
            .filter(|v| v.active)
            .map(|v| v.stake)
            .sum()
    }

    /// Check if stake amount forms quorum
    pub fn has_quorum(&self, stake: u64) -> bool {
        stake >= self.quorum_threshold
    }

    /// Verify quorum certificates (signatures from 2f+1 validators)
    pub fn verify_quorum_certificate(&self, signers: &[AuthorityId]) -> Result<()> {
        let total_stake: u64 = signers
            .iter()
            .filter_map(|auth| self.validators.get(auth))
            .filter(|v| v.active)
            .map(|v| v.stake)
            .sum();

        if total_stake >= self.quorum_threshold {
            Ok(())
        } else {
            Err(anyhow!(
                "Insufficient stake in quorum certificate: {} < {}",
                total_stake,
                self.quorum_threshold
            ))
        }
    }
}

/// Committee change request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommitteeChange {
    /// Add new validator
    AddValidator(ValidatorInfo),

    /// Remove validator
    RemoveValidator {
        authority_id: AuthorityId,
        reason: String,
    },

    /// Update validator stake
    UpdateStake {
        authority_id: AuthorityId,
        new_stake: u64,
    },

    /// Deactivate validator
    DeactivateValidator { authority_id: AuthorityId },

    /// Reactivate validator
    ReactivateValidator { authority_id: AuthorityId },
}

/// Committee change transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitteeChangeTx {
    /// Change to apply
    pub change: CommitteeChange,

    /// Epoch to apply change in
    pub target_epoch: u64,

    /// Signatures from current committee (quorum)
    pub signatures: Vec<(AuthorityId, Vec<u8>)>,
}

/// Committee manager
pub struct CommitteeManager {
    /// Current committee
    current_committee: Committee,

    /// Pending committee changes (target_epoch -> changes)
    pending_changes: HashMap<u64, Vec<CommitteeChange>>,

    /// Committee history (epoch -> committee)
    committee_history: HashMap<u64, Committee>,
}

impl CommitteeManager {
    /// Create new committee manager
    pub fn new(initial_committee: Committee) -> Self {
        let epoch = initial_committee.epoch;
        let mut history = HashMap::new();
        history.insert(epoch, initial_committee.clone());

        Self {
            current_committee: initial_committee,
            pending_changes: HashMap::new(),
            committee_history: history,
        }
    }

    /// Get current committee
    pub fn current_committee(&self) -> &Committee {
        &self.current_committee
    }

    /// Get committee at specific epoch
    pub fn get_committee(&self, epoch: u64) -> Option<&Committee> {
        self.committee_history.get(&epoch)
    }

    /// Propose committee change
    pub fn propose_change(&mut self, change: CommitteeChange, target_epoch: u64) -> Result<()> {
        if target_epoch <= self.current_committee.epoch {
            return Err(anyhow!(
                "Target epoch {} must be greater than current epoch {}",
                target_epoch,
                self.current_committee.epoch
            ));
        }

        self.pending_changes
            .entry(target_epoch)
            .or_default()
            .push(change);

        tracing::info!("Proposed committee change for epoch {}", target_epoch);

        Ok(())
    }

    /// Apply pending changes for new epoch
    pub fn advance_epoch(&mut self, new_epoch: u64) -> Result<Committee> {
        if new_epoch <= self.current_committee.epoch {
            return Err(anyhow!("Cannot advance to past epoch"));
        }

        // Start with current validators
        let mut new_validators: HashMap<AuthorityId, ValidatorInfo> =
            self.current_committee.validators.clone();

        // Apply pending changes
        if let Some(changes) = self.pending_changes.remove(&new_epoch) {
            for change in changes {
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
                    CommitteeChange::UpdateStake {
                        authority_id,
                        new_stake,
                    } => {
                        if let Some(validator) = new_validators.get_mut(&authority_id) {
                            tracing::info!(
                                "Updating stake for {}: {} -> {}",
                                authority_id,
                                validator.stake,
                                new_stake
                            );
                            validator.stake = new_stake;
                        }
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
        }

        // Create new committee
        let validators: Vec<ValidatorInfo> = new_validators.into_values().collect();
        let new_committee = Committee::new(new_epoch, validators);

        // Store in history
        self.committee_history
            .insert(new_epoch, new_committee.clone());
        self.current_committee = new_committee.clone();

        tracing::info!(
            "Advanced to epoch {} with {} validators (total stake: {})",
            new_epoch,
            self.current_committee.validators.len(),
            self.current_committee.total_stake
        );

        Ok(new_committee)
    }

    /// Get pending changes for epoch
    pub fn get_pending_changes(&self, epoch: u64) -> Option<&[CommitteeChange]> {
        self.pending_changes.get(&epoch).map(|v| v.as_slice())
    }

    /// Verify committee change transaction
    pub fn verify_change_tx(&self, tx: &CommitteeChangeTx) -> Result<()> {
        // Extract signers
        let signers: Vec<AuthorityId> =
            tx.signatures.iter().map(|(auth, _)| auth.clone()).collect();

        // Verify quorum from current committee
        self.current_committee.verify_quorum_certificate(&signers)?;

        // Verify signatures (simplified)
        for (authority, signature) in &tx.signatures {
            if !self.current_committee.contains(authority) {
                return Err(anyhow!("Signer {} not in current committee", authority));
            }

            if signature.is_empty() {
                return Err(anyhow!("Invalid signature from {}", authority));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_validator(id: &str, stake: u64) -> ValidatorInfo {
        ValidatorInfo {
            authority_id: id.to_string(),
            public_key: vec![0u8; 32],
            stake,
            network_address: format!("127.0.0.1:{}", 9000 + id.len()),
            active: true,
        }
    }

    fn create_test_committee() -> Committee {
        let validators = vec![
            create_test_validator("auth1", 100),
            create_test_validator("auth2", 100),
            create_test_validator("auth3", 100),
            create_test_validator("auth4", 100),
        ];
        Committee::new(0, validators)
    }

    #[test]
    fn test_committee_creation() {
        let committee = create_test_committee();

        assert_eq!(committee.epoch, 0);
        assert_eq!(committee.validators.len(), 4);
        assert_eq!(committee.total_stake, 400);
        assert_eq!(committee.quorum_threshold, (400 * 2 / 3) + 1); // 267
    }

    #[test]
    fn test_quorum_verification() {
        let committee = create_test_committee();

        // 3 out of 4 validators = 300 stake (>= 267) = quorum
        assert!(committee.has_quorum(300));

        // 2 out of 4 validators = 200 stake (< 267) = no quorum
        assert!(!committee.has_quorum(200));
    }

    #[test]
    fn test_add_validator() {
        let committee = create_test_committee();
        let mut manager = CommitteeManager::new(committee);

        let new_validator = create_test_validator("auth5", 100);
        let change = CommitteeChange::AddValidator(new_validator);

        manager.propose_change(change, 1).unwrap();

        let new_committee = manager.advance_epoch(1).unwrap();
        assert_eq!(new_committee.validators.len(), 5);
        assert_eq!(new_committee.total_stake, 500);
    }

    #[test]
    fn test_remove_validator() {
        let committee = create_test_committee();
        let mut manager = CommitteeManager::new(committee);

        let change = CommitteeChange::RemoveValidator {
            authority_id: "auth4".to_string(),
            reason: "Inactive".to_string(),
        };

        manager.propose_change(change, 1).unwrap();

        let new_committee = manager.advance_epoch(1).unwrap();
        assert_eq!(new_committee.validators.len(), 3);
        assert_eq!(new_committee.total_stake, 300);
    }

    #[test]
    fn test_update_stake() {
        let committee = create_test_committee();
        let mut manager = CommitteeManager::new(committee);

        let change = CommitteeChange::UpdateStake {
            authority_id: "auth1".to_string(),
            new_stake: 200,
        };

        manager.propose_change(change, 1).unwrap();

        let new_committee = manager.advance_epoch(1).unwrap();
        assert_eq!(new_committee.get_validator("auth1").unwrap().stake, 200);
        assert_eq!(new_committee.total_stake, 500);
    }

    #[test]
    fn test_deactivate_validator() {
        let committee = create_test_committee();
        let mut manager = CommitteeManager::new(committee);

        let change = CommitteeChange::DeactivateValidator {
            authority_id: "auth2".to_string(),
        };

        manager.propose_change(change, 1).unwrap();

        let new_committee = manager.advance_epoch(1).unwrap();
        assert!(!new_committee.get_validator("auth2").unwrap().active);
        assert_eq!(new_committee.active_stake(), 300); // 100*3
    }
}
