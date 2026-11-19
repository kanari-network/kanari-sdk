// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Governance CLI: propose, vote, execute, and withdraw on the `0x3::governance`
//! staked-voting system. Thin wrapper over `kanari call`: every subcommand
//! builds one Move entry call (`governance::*`) with object inputs resolved
//! from on-chain state.

use crate::command::client_cli::tx_submit::{
    self, EntryObject, ResolvedTx, bcs_bool, bcs_string, bcs_u64, resolve_tx,
};
use anyhow::Result;
use clap::{Parser, Subcommand};
use kanari_rpc_client::RpcClient;

const GOVERNANCE_MODULE: &str = kanari_types::governance::GovernanceModule::GOVERNANCE_MODULE;

fn governance_package() -> String {
    kanari_types::governance::GovernanceModule::GOVERNANCE_PACKAGE.to_string()
}

#[allow(clippy::too_many_arguments)]
async fn submit_entry(
    tx: &ResolvedTx,
    client: &RpcClient,
    function: &str,
    type_args: Vec<String>,
    args: Vec<Vec<u8>>,
    objects: Vec<EntryObject>,
) -> Result<()> {
    tx_submit::submit_entry(
        tx,
        client,
        &governance_package(),
        GOVERNANCE_MODULE,
        function,
        type_args,
        args,
        objects,
    )
    .await
}
#[derive(Subcommand, Debug)]
pub enum GovernanceCommand {
    /// Create a new proposal by escrowing a deposit coin
    CreateProposal {
        /// Shared GovernanceState object ID
        #[arg(long)]
        state: String,
        /// Coin object ID to use as the proposal deposit
        #[arg(long)]
        coin: String,
        /// Proposal title
        #[arg(long)]
        title: String,
        /// Proposal description
        #[arg(long, default_value = "")]
        description: String,
        /// Voting period in milliseconds
        #[arg(long, default_value_t = 604_800_000)]
        voting_period: u64,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
    /// Cast a vote by staking a coin behind yes/no
    CastVote {
        /// Shared GovernanceState object ID
        #[arg(long)]
        state: String,
        /// Shared Proposal object ID
        #[arg(long)]
        proposal: String,
        /// Vote in favour (true) or against (false)
        #[arg(long)]
        support: bool,
        /// Coin object ID to stake
        #[arg(long)]
        coin: String,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
    /// Execute a proposal after its voting window closes
    ExecuteProposal {
        /// Shared GovernanceState object ID
        #[arg(long)]
        state: String,
        /// Shared Proposal object ID
        #[arg(long)]
        proposal: String,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
    /// Withdraw your staked coins after the voting window closes
    WithdrawStake {
        /// Shared Proposal object ID
        #[arg(long)]
        proposal: String,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
    /// Withdraw the creator's deposit after execution
    WithdrawDeposit {
        /// Shared Proposal object ID
        #[arg(long)]
        proposal: String,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
    /// Admin: pause governance
    Pause {
        /// GovernanceCap object ID
        #[arg(long)]
        cap: String,
        /// Shared GovernanceState object ID
        #[arg(long)]
        state: String,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
    /// Admin: unpause governance
    Unpause {
        #[arg(long)]
        cap: String,
        #[arg(long)]
        state: String,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
    /// Admin: set the voting period (ms)
    SetVotingPeriod {
        #[arg(long)]
        cap: String,
        #[arg(long)]
        state: String,
        #[arg(long)]
        period_ms: u64,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
    /// Admin: set the quorum threshold (basis points)
    SetQuorumThreshold {
        #[arg(long)]
        cap: String,
        #[arg(long)]
        state: String,
        #[arg(long)]
        threshold_bps: u64,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
    /// Admin: sync total_supply from the Kanari treasury cap (supply oracle)
    SetTotalSupply {
        #[arg(long)]
        cap: String,
        #[arg(long)]
        state: String,
        #[arg(long)]
        treasury_cap: String,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
}

#[derive(Parser, Debug)]
#[clap(name = "governance", about = "Governance (0x3) operations")]
pub struct Governance {
    #[command(subcommand)]
    pub command: GovernanceCommand,
    #[clap(long = "gas-limit")]
    pub gas_limit: Option<u64>,
    #[clap(long = "gas-price")]
    pub gas_price: Option<u64>,
}

impl Governance {
    pub async fn execute(&self) -> Result<()> {
        match &self.command {
            GovernanceCommand::CreateProposal {
                state,
                coin,
                title,
                description,
                voting_period,
                from,
                password,
                rpc_endpoint,
            } => {
                let (tx, client) = resolve_tx(
                    from.clone(),
                    password.clone(),
                    rpc_endpoint.clone(),
                    self.gas_limit,
                    self.gas_price,
                )
                .await?;
                eprintln!("Creating proposal...");
                submit_entry(
                    &tx,
                    &client,
                    kanari_types::governance::GovernanceModule::entry_functions().create_proposal,
                    vec![],
                    vec![bcs_string(title)?, bcs_string(description)?, bcs_u64(*voting_period)?],
                    vec![
                        EntryObject { id: state.clone(), mutable: true, owner_check: false },
                        EntryObject { id: coin.clone(), mutable: true, owner_check: false },
                    ],
                )
                .await
            }
            GovernanceCommand::CastVote {
                state,
                proposal,
                support,
                coin,
                from,
                password,
                rpc_endpoint,
            } => {
                let (tx, client) = resolve_tx(
                    from.clone(),
                    password.clone(),
                    rpc_endpoint.clone(),
                    self.gas_limit,
                    self.gas_price,
                )
                .await?;
                eprintln!("Casting vote (support={})...", support);
                submit_entry(
                    &tx,
                    &client,
                    kanari_types::governance::GovernanceModule::entry_functions().cast_vote,
                    vec![],
                    vec![bcs_bool(*support)?],
                    vec![
                        EntryObject { id: state.clone(), mutable: true, owner_check: false },
                        EntryObject { id: proposal.clone(), mutable: true, owner_check: false },
                        EntryObject { id: coin.clone(), mutable: true, owner_check: false },
                    ],
                )
                .await
            }
            GovernanceCommand::ExecuteProposal {
                state,
                proposal,
                from,
                password,
                rpc_endpoint,
            } => {
                let (tx, client) = resolve_tx(
                    from.clone(),
                    password.clone(),
                    rpc_endpoint.clone(),
                    self.gas_limit,
                    self.gas_price,
                )
                .await?;
                eprintln!("Executing proposal...");
                submit_entry(
                    &tx,
                    &client,
                    kanari_types::governance::GovernanceModule::entry_functions().execute_proposal,
                    vec![],
                    vec![],
                    vec![
                        EntryObject { id: state.clone(), mutable: true, owner_check: false },
                        EntryObject { id: proposal.clone(), mutable: true, owner_check: false },
                    ],
                )
                .await
            }
            GovernanceCommand::WithdrawStake { proposal, from, password, rpc_endpoint } => {
                let (tx, client) = resolve_tx(
                    from.clone(),
                    password.clone(),
                    rpc_endpoint.clone(),
                    self.gas_limit,
                    self.gas_price,
                )
                .await?;
                eprintln!("Withdrawing stake...");
                submit_entry(
                    &tx,
                    &client,
                    kanari_types::governance::GovernanceModule::entry_functions().withdraw_stake,
                    vec![],
                    vec![],
                    vec![EntryObject { id: proposal.clone(), mutable: true, owner_check: false }],
                )
                .await
            }
            GovernanceCommand::WithdrawDeposit { proposal, from, password, rpc_endpoint } => {
                let (tx, client) = resolve_tx(
                    from.clone(),
                    password.clone(),
                    rpc_endpoint.clone(),
                    self.gas_limit,
                    self.gas_price,
                )
                .await?;
                eprintln!("Withdrawing deposit...");
                submit_entry(
                    &tx,
                    &client,
                    kanari_types::governance::GovernanceModule::entry_functions().withdraw_deposit,
                    vec![],
                    vec![],
                    vec![EntryObject { id: proposal.clone(), mutable: true, owner_check: false }],
                )
                .await
            }
            GovernanceCommand::Pause { cap, state, from, password, rpc_endpoint } => {
                let (tx, client) = resolve_tx(
                    from.clone(),
                    password.clone(),
                    rpc_endpoint.clone(),
                    self.gas_limit,
                    self.gas_price,
                )
                .await?;
                eprintln!("Pausing governance...");
                submit_entry(
                    &tx,
                    &client,
                    kanari_types::governance::GovernanceModule::entry_functions().pause,
                    vec![],
                    vec![],
                    vec![
                        EntryObject { id: cap.clone(), mutable: false, owner_check: true },
                        EntryObject { id: state.clone(), mutable: true, owner_check: false },
                    ],
                )
                .await
            }
            GovernanceCommand::Unpause { cap, state, from, password, rpc_endpoint } => {
                let (tx, client) = resolve_tx(
                    from.clone(),
                    password.clone(),
                    rpc_endpoint.clone(),
                    self.gas_limit,
                    self.gas_price,
                )
                .await?;
                eprintln!("Unpausing governance...");
                submit_entry(
                    &tx,
                    &client,
                    kanari_types::governance::GovernanceModule::entry_functions().unpause,
                    vec![],
                    vec![],
                    vec![
                        EntryObject { id: cap.clone(), mutable: false, owner_check: true },
                        EntryObject { id: state.clone(), mutable: true, owner_check: false },
                    ],
                )
                .await
            }
            GovernanceCommand::SetVotingPeriod {
                cap,
                state,
                period_ms,
                from,
                password,
                rpc_endpoint,
            } => {
                let (tx, client) = resolve_tx(
                    from.clone(),
                    password.clone(),
                    rpc_endpoint.clone(),
                    self.gas_limit,
                    self.gas_price,
                )
                .await?;
                eprintln!("Setting voting period...");
                submit_entry(
                    &tx,
                    &client,
                    kanari_types::governance::GovernanceModule::entry_functions().set_voting_period,
                    vec![],
                    vec![bcs_u64(*period_ms)?],
                    vec![
                        EntryObject { id: cap.clone(), mutable: false, owner_check: true },
                        EntryObject { id: state.clone(), mutable: true, owner_check: false },
                    ],
                )
                .await
            }
            GovernanceCommand::SetQuorumThreshold {
                cap,
                state,
                threshold_bps,
                from,
                password,
                rpc_endpoint,
            } => {
                let (tx, client) = resolve_tx(
                    from.clone(),
                    password.clone(),
                    rpc_endpoint.clone(),
                    self.gas_limit,
                    self.gas_price,
                )
                .await?;
                eprintln!("Setting quorum threshold...");
                submit_entry(
                    &tx,
                    &client,
                    kanari_types::governance::GovernanceModule::entry_functions()
                        .set_quorum_threshold,
                    vec![],
                    vec![bcs_u64(*threshold_bps)?],
                    vec![
                        EntryObject { id: cap.clone(), mutable: false, owner_check: true },
                        EntryObject { id: state.clone(), mutable: true, owner_check: false },
                    ],
                )
                .await
            }
            GovernanceCommand::SetTotalSupply {
                cap,
                state,
                treasury_cap,
                from,
                password,
                rpc_endpoint,
            } => {
                let (tx, client) = resolve_tx(
                    from.clone(),
                    password.clone(),
                    rpc_endpoint.clone(),
                    self.gas_limit,
                    self.gas_price,
                )
                .await?;
                eprintln!("Syncing total supply from treasury cap...");
                submit_entry(
                    &tx,
                    &client,
                    kanari_types::governance::GovernanceModule::entry_functions().set_total_supply,
                    vec![],
                    vec![],
                    vec![
                        EntryObject { id: cap.clone(), mutable: false, owner_check: true },
                        EntryObject { id: state.clone(), mutable: true, owner_check: false },
                        EntryObject {
                            id: treasury_cap.clone(),
                            mutable: false,
                            owner_check: false,
                        },
                    ],
                )
                .await
            }
        }
    }
}
