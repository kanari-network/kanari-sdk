// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};

/// Governance module constants and helpers for the `0x3::governance` package.
pub struct GovernanceModule;

impl GovernanceModule {
    /// Package (framework) address of the governance module (`0x3`).
    pub const GOVERNANCE_PACKAGE: &'static str = Address::KANARI_GOVERNANCE_ADDRESS;

    /// Move module name.
    pub const GOVERNANCE_MODULE: &'static str = "governance";

    /// Type-name suffixes used to locate governance objects on chain.
    pub const GOVERNANCE_STATE_TYPE: &'static str = "::governance::GovernanceState";
    pub const GOVERNANCE_CAP_TYPE: &'static str = "::governance::GovernanceCap";
    pub const PROPOSAL_TYPE: &'static str = "::governance::Proposal";

    /// Fully qualified Move module path for `0x3::governance`.
    pub fn module_path() -> String {
        format!("{}::{}", Address::KANARI_GOVERNANCE_ADDRESS, Self::GOVERNANCE_MODULE)
    }

    /// Get the module ID for `0x3::governance`.
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_GOVERNANCE_ADDRESS)
            .context("Invalid governance address")?;
        let module_name =
            Identifier::new(Self::GOVERNANCE_MODULE).context("Invalid governance module name")?;
        Ok(ModuleId::new(address, module_name))
    }

    /// Entry function names.
    pub fn entry_functions() -> GovernanceEntryFunctions {
        GovernanceEntryFunctions {
            create_proposal: "create_proposal",
            cast_vote: "cast_vote",
            execute_proposal: "execute_proposal",
            withdraw_stake: "withdraw_stake",
            withdraw_deposit: "withdraw_deposit",
            pause: "pause",
            unpause: "unpause",
            set_voting_period: "set_voting_period",
            set_quorum_threshold: "set_quorum_threshold",
            set_total_supply: "set_total_supply",
        }
    }

    /// View function names.
    pub fn view_functions() -> GovernanceViewFunctions {
        GovernanceViewFunctions {
            get_proposal_count: "get_proposal_count",
            get_voting_period: "get_voting_period",
            get_quorum_threshold: "get_quorum_threshold",
            get_total_supply: "get_total_supply",
            is_paused: "is_paused",
            vote_tally: "vote_tally",
            stake_of: "stake_of",
            is_executed: "is_executed",
            execution_result: "execution_result",
            deposit_of: "deposit_of",
        }
    }
}

pub struct GovernanceEntryFunctions {
    pub create_proposal: &'static str,
    pub cast_vote: &'static str,
    pub execute_proposal: &'static str,
    pub withdraw_stake: &'static str,
    pub withdraw_deposit: &'static str,
    pub pause: &'static str,
    pub unpause: &'static str,
    pub set_voting_period: &'static str,
    pub set_quorum_threshold: &'static str,
    pub set_total_supply: &'static str,
}

pub struct GovernanceViewFunctions {
    pub get_proposal_count: &'static str,
    pub get_voting_period: &'static str,
    pub get_quorum_threshold: &'static str,
    pub get_total_supply: &'static str,
    pub is_paused: &'static str,
    pub vote_tally: &'static str,
    pub stake_of: &'static str,
    pub is_executed: &'static str,
    pub execution_result: &'static str,
    pub deposit_of: &'static str,
}
