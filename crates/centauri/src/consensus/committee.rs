// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Committee representation for validator membership and quorum checks.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;

use super::AuthorityId;

/// Validator information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidatorInfo {
    pub authority_id: AuthorityId,
    pub public_key: Vec<u8>,
    pub network_address: String,
    pub active: bool,
}

/// Snapshot of current network conditions used by adaptive quorum policy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkHealth {
    pub connectivity_ratio: f64,
    pub delivery_success_ratio: f64,
    pub timeout_ratio: f64,
    pub median_latency_ms: u64,
}

impl NetworkHealth {
    fn clamp_ratio(value: f64) -> f64 {
        value.clamp(0.0, 1.0)
    }

    fn latency_score(&self) -> f64 {
        const LOW_LATENCY_MS: u64 = 100;
        const HIGH_LATENCY_MS: u64 = 5_000;

        match self.median_latency_ms {
            latency if latency <= LOW_LATENCY_MS => 1.0,
            latency if latency >= HIGH_LATENCY_MS => 0.0,
            latency => {
                let span = (HIGH_LATENCY_MS - LOW_LATENCY_MS) as f64;
                1.0 - ((latency - LOW_LATENCY_MS) as f64 / span)
            }
        }
    }

    /// Aggregate health score in `[0, 1]`.
    pub fn score(&self) -> f64 {
        let connectivity = Self::clamp_ratio(self.connectivity_ratio);
        let delivery = Self::clamp_ratio(self.delivery_success_ratio);
        let timeout = 1.0 - Self::clamp_ratio(self.timeout_ratio);
        let latency = self.latency_score();

        (connectivity * 0.35) + (delivery * 0.35) + (timeout * 0.15) + (latency * 0.15)
    }
}

impl Default for NetworkHealth {
    fn default() -> Self {
        Self {
            connectivity_ratio: 1.0,
            delivery_success_ratio: 1.0,
            timeout_ratio: 0.0,
            median_latency_ms: 50,
        }
    }
}

/// Configuration for adaptive quorum and timeout behavior.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdaptiveQuorumConfig {
    pub healthy_threshold: f64,
    pub degraded_threshold: f64,
    pub degraded_extra_votes: usize,
    pub unhealthy_extra_votes: usize,
    pub base_timeout_ms: u64,
    pub max_timeout_ms: u64,
}

impl Default for AdaptiveQuorumConfig {
    fn default() -> Self {
        Self {
            healthy_threshold: 0.85,
            degraded_threshold: 0.60,
            degraded_extra_votes: 1,
            unhealthy_extra_votes: 2,
            base_timeout_ms: 2_000,
            max_timeout_ms: 10_000,
        }
    }
}

/// Adaptive quorum policy driven by network-health snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdaptiveQuorum {
    pub config: AdaptiveQuorumConfig,
    pub health: NetworkHealth,
}

impl AdaptiveQuorum {
    pub fn new(config: AdaptiveQuorumConfig) -> Self {
        Self {
            config,
            health: NetworkHealth::default(),
        }
    }

    pub fn update_health(&mut self, health: NetworkHealth) {
        self.health = health;
    }

    pub fn required_quorum(&self, total_validators: usize, base_quorum: usize) -> usize {
        if total_validators == 0 {
            return 0;
        }

        let extra_capacity = total_validators.saturating_sub(base_quorum);
        let score = self.health.score();
        let extra_votes = if score >= self.config.healthy_threshold {
            0
        } else if score >= self.config.degraded_threshold {
            self.config.degraded_extra_votes.min(extra_capacity)
        } else {
            self.config.unhealthy_extra_votes.min(extra_capacity)
        };

        (base_quorum + extra_votes).min(total_validators)
    }

    pub fn timeout_ms(&self) -> u64 {
        let score = self.health.score();
        if score >= self.config.healthy_threshold {
            return self.config.base_timeout_ms;
        }

        let headroom = self
            .config
            .max_timeout_ms
            .saturating_sub(self.config.base_timeout_ms);

        if headroom == 0 {
            return self.config.base_timeout_ms;
        }

        let degraded_floor = self
            .config
            .degraded_threshold
            .min(self.config.healthy_threshold);
        let severity = if degraded_floor <= 0.0 {
            1.0
        } else {
            (1.0 - (score / degraded_floor)).clamp(0.0, 1.0)
        };

        self.config.base_timeout_ms + ((headroom as f64) * severity).round() as u64
    }
}

/// Committee (set of validators)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Committee {
    pub epoch: u64,
    pub validators: BTreeMap<AuthorityId, ValidatorInfo>,
    pub quorum_size: usize,
    pub adaptive_quorum: Option<AdaptiveQuorum>,
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
            adaptive_quorum: None,
        }
    }

    pub fn get_validator(&self, authority: &str) -> Option<&ValidatorInfo> {
        self.validators.get(authority)
    }

    pub fn contains(&self, authority: &str) -> bool {
        self.validators.contains_key(authority)
    }

    pub fn enable_adaptive_quorum(&mut self, config: AdaptiveQuorumConfig) {
        self.adaptive_quorum = Some(AdaptiveQuorum::new(config));
    }

    pub fn disable_adaptive_quorum(&mut self) {
        self.adaptive_quorum = None;
    }

    pub fn update_network_health(&mut self, health: NetworkHealth) {
        if let Some(policy) = self.adaptive_quorum.as_mut() {
            policy.update_health(health);
        }
    }

    pub fn required_quorum(&self) -> usize {
        self.adaptive_quorum
            .as_ref()
            .map(|policy| policy.required_quorum(self.validators.len(), self.quorum_size))
            .unwrap_or(self.quorum_size)
    }

    pub fn quorum_timeout(&self) -> Duration {
        let timeout_ms = self
            .adaptive_quorum
            .as_ref()
            .map(AdaptiveQuorum::timeout_ms)
            .unwrap_or(AdaptiveQuorumConfig::default().base_timeout_ms);
        Duration::from_millis(timeout_ms)
    }

    pub fn verify_quorum_certificate(&self, signers: &[AuthorityId]) -> Result<()> {
        let unique_signers: std::collections::HashSet<&str> =
            signers.iter().map(|s| s.as_str()).collect();
        let trusted_count = unique_signers
            .iter()
            .filter(|auth| {
                self.validators
                    .get(**auth)
                    .map(|v| v.active)
                    .unwrap_or(false)
            })
            .count();
        let required = self.required_quorum();

        if trusted_count >= required {
            Ok(())
        } else {
            Err(anyhow!(
                "Insufficient validators in quorum certificate: {} < {}",
                trusted_count,
                required
            ))
        }
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
        assert_eq!(committee.quorum_size, 3);
        assert_eq!(committee.required_quorum(), 3);
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
    fn test_network_health_score_degrades_with_timeouts() {
        let healthy = NetworkHealth::default();
        let degraded = NetworkHealth {
            connectivity_ratio: 0.65,
            delivery_success_ratio: 0.60,
            timeout_ratio: 0.40,
            median_latency_ms: 2_500,
        };

        assert!(healthy.score() > degraded.score());
    }

    #[test]
    fn test_adaptive_quorum_increases_threshold_when_unhealthy() {
        let mut committee = create_test_committee();
        committee.enable_adaptive_quorum(AdaptiveQuorumConfig::default());
        committee.update_network_health(NetworkHealth {
            connectivity_ratio: 0.45,
            delivery_success_ratio: 0.50,
            timeout_ratio: 0.35,
            median_latency_ms: 3_000,
        });

        assert_eq!(committee.quorum_size, 3);
        assert_eq!(committee.required_quorum(), 4);
    }

    #[test]
    fn test_adaptive_timeout_expands_when_network_degrades() {
        let mut committee = create_test_committee();
        committee.enable_adaptive_quorum(AdaptiveQuorumConfig::default());
        let healthy_timeout = committee.quorum_timeout();

        committee.update_network_health(NetworkHealth {
            connectivity_ratio: 0.40,
            delivery_success_ratio: 0.55,
            timeout_ratio: 0.45,
            median_latency_ms: 3_500,
        });

        assert!(committee.quorum_timeout() > healthy_timeout);
    }

    #[test]
    fn test_disable_adaptive_quorum_restores_static_threshold_and_timeout() {
        let mut committee = create_test_committee();
        let static_quorum = committee.quorum_size;
        let static_timeout = committee.quorum_timeout();

        committee.enable_adaptive_quorum(AdaptiveQuorumConfig::default());
        committee.update_network_health(NetworkHealth {
            connectivity_ratio: 0.30,
            delivery_success_ratio: 0.35,
            timeout_ratio: 0.55,
            median_latency_ms: 4_200,
        });

        assert!(committee.required_quorum() > static_quorum);
        assert!(committee.quorum_timeout() > static_timeout);

        committee.disable_adaptive_quorum();

        assert_eq!(committee.required_quorum(), static_quorum);
        assert_eq!(committee.quorum_timeout(), static_timeout);
    }

    #[test]
    fn test_verify_quorum_certificate_ignores_inactive_validators() {
        let mut committee = create_test_committee();
        committee
            .validators
            .get_mut("auth3")
            .expect("validator must exist")
            .active = false;

        let signers = vec![
            "auth1".to_string(),
            "auth2".to_string(),
            "auth3".to_string(),
        ];

        assert!(committee.verify_quorum_certificate(&signers).is_err());
    }

    #[test]
    fn test_quorum_verification() {
        let committee = create_test_committee();
        assert!(3 >= committee.required_quorum());
        assert!(2 < committee.required_quorum());
    }

    #[test]
    fn test_verify_quorum_certificate_rejects_duplicate_signers() {
        let committee = create_test_committee();
        let signers = vec![
            "auth1".to_string(),
            "auth1".to_string(),
            "auth2".to_string(),
        ];
        assert!(committee.verify_quorum_certificate(&signers).is_err());
    }

    #[test]
    fn test_verify_quorum_certificate_uses_adaptive_threshold() {
        let mut committee = create_test_committee();
        committee.enable_adaptive_quorum(AdaptiveQuorumConfig::default());
        committee.update_network_health(NetworkHealth {
            connectivity_ratio: 0.30,
            delivery_success_ratio: 0.40,
            timeout_ratio: 0.50,
            median_latency_ms: 4_000,
        });

        let signers = vec![
            "auth1".to_string(),
            "auth2".to_string(),
            "auth3".to_string(),
        ];
        assert!(committee.verify_quorum_certificate(&signers).is_err());
    }
}
