// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Mysticeti consensus protocol parameters.
//!
//! This mirrors Mysticeti's protocol and commit semantics: user-facing protocol
//! selection is converted into concrete thresholds and round geometry used by
//! the existing DAG committer.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::num::NonZeroUsize;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "protocol", rename_all = "kebab-case")]
pub enum ConsensusProtocol {
    Mysticeti {
        #[serde(default = "default_leader_count")]
        leader_count: NonZeroUsize,
    },
}

fn default_leader_count() -> NonZeroUsize {
    NonZeroUsize::new(2).expect("default leader count must be non-zero")
}

impl Default for ConsensusProtocol {
    fn default() -> Self {
        Self::Mysticeti {
            leader_count: default_leader_count(),
        }
    }
}

impl fmt::Display for ConsensusProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mysticeti { leader_count } => {
                write!(f, "Mysticeti ({} leaders/round)", leader_count)
            }
        }
    }
}

impl ConsensusProtocol {
    pub fn default_for_committee_size(committee_size: usize) -> Self {
        let leader_count = default_leader_count().get().min(committee_size.max(1));
        Self::Mysticeti {
            leader_count: NonZeroUsize::new(leader_count)
                .expect("committee-adjusted leader count must be non-zero"),
        }
    }

    pub fn to_protocol(&self, committee_size: usize) -> Result<Protocol> {
        match *self {
            Self::Mysticeti { leader_count } => Protocol::mysticeti(committee_size, leader_count),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Protocol {
    pub direct_commit_quorum: usize,
    pub direct_skip_quorum: usize,
    pub anchor_link_size: usize,
    pub wave_length: u64,
    pub leader_count: NonZeroUsize,
    pub pipeline: bool,
    pub leader_wait: bool,
    pub require_crypto: bool,
}

impl Protocol {
    pub fn mysticeti(total_authorities: usize, leader_count: NonZeroUsize) -> Result<Self> {
        if total_authorities == 0 {
            anyhow::bail!("Mysticeti protocol requires at least one authority");
        }
        if leader_count.get() > total_authorities {
            anyhow::bail!(
                "leader_count ({}) exceeds committee size ({})",
                leader_count,
                total_authorities
            );
        }

        let quorum = (2 * total_authorities / 3) + 1;
        Ok(Self {
            direct_commit_quorum: quorum,
            direct_skip_quorum: quorum,
            anchor_link_size: 1,
            wave_length: 3,
            leader_count,
            pipeline: true,
            leader_wait: true,
            require_crypto: true,
        })
    }

    pub fn decision_depth(&self) -> u64 {
        self.wave_length.saturating_sub(1).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mysticeti_protocol_uses_strict_quorum_and_three_round_wave() {
        let protocol = Protocol::mysticeti(4, NonZeroUsize::new(2).unwrap()).unwrap();

        assert_eq!(protocol.direct_commit_quorum, 3);
        assert_eq!(protocol.direct_skip_quorum, 3);
        assert_eq!(protocol.wave_length, 3);
        assert_eq!(protocol.decision_depth(), 2);
        assert!(protocol.pipeline);
        assert!(protocol.leader_wait);
    }

    #[test]
    fn default_protocol_caps_leaders_to_committee_size() {
        let protocol = ConsensusProtocol::default_for_committee_size(1)
            .to_protocol(1)
            .unwrap();

        assert_eq!(protocol.leader_count.get(), 1);
    }
}
