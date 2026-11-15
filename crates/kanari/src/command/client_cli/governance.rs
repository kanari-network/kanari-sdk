// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Governance CLI: propose, vote, execute, and withdraw on the `0x3::governance`
//! staked-voting system. Thin wrapper over `kanari call`: every subcommand
//! builds one Move entry call (`governance::*`) with object inputs resolved
//! from on-chain state.

use crate::command::common::{
    check_node_connection, get_rpc_endpoint, get_sender_for_tx, load_wallet_for, normalize_addr,
    resolve_sender, resolve_transaction_gas,
};
use crate::command::rpc_helpers::{object_input_from_object_id, sign_and_call_function};
use crate::command::tx_output::print_transaction_status;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use kanari_rpc_api::BuildCallFunctionRequest;
use kanari_rpc_client::RpcClient;

const GOVERNANCE_MODULE: &str = kanari_types::governance::GovernanceModule::GOVERNANCE_MODULE;

fn governance_package() -> String {
    kanari_types::governance::GovernanceModule::GOVERNANCE_PACKAGE.to_string()
}

fn bcs_bool(v: bool) -> Result<Vec<u8>> {
    bcs::to_bytes(&v).context("Failed to BCS-encode bool")
}

fn bcs_u64(v: u64) -> Result<Vec<u8>> {
    bcs::to_bytes(&v).context("Failed to BCS-encode u64")
}

fn bcs_string(s: &str) -> Result<Vec<u8>> {
    bcs::to_bytes(&s.as_bytes().to_vec()).context("Failed to BCS-encode string")
}

struct EntryObject {
    id: String,
    mutable: bool,
    owner_check: bool,
}

async fn submit_entry(
    tx: &ResolvedTx,
    client: &RpcClient,
    function: &str,
    type_args: Vec<String>,
    args: Vec<Vec<u8>>,
    objects: Vec<EntryObject>,
) -> Result<()> {
    let mut object_inputs = Vec::with_capacity(objects.len());
    for entry in &objects {
        let normalized = normalize_addr(&entry.id)?;
        let expected = if entry.owner_check { Some(tx.sender_normalized.as_str()) } else { None };
        let mut input = object_input_from_object_id(client, &normalized, expected).await?;
        input.mutable = entry.mutable;
        object_inputs.push(input);
    }

    let prepared = client
        .build_call_function(BuildCallFunctionRequest {
            sender: tx.sender_tagged.clone(),
            package: governance_package(),
            module: GOVERNANCE_MODULE.to_string(),
            function: function.to_string(),
            type_args,
            args,
            object_inputs: Some(object_inputs),
            gas_limit: tx.gas_limit,
            gas_price: tx.gas_price,
            nonce: None,
            execute_immediate: Some(true),
        })
        .await
        .context("Failed to build call function transaction")?;

    if let Some(inputs) = &prepared.object_inputs {
        for input in inputs {
            eprintln!("  Object input: {}", input.object_ref.object_id);
        }
    }
    eprintln!("  Gas Limit: {}", prepared.gas_limit);
    eprintln!("  Gas Price: {} Mist/gas", prepared.gas_price);

    let status = sign_and_call_function(client, &tx.wallet, prepared)
        .await
        .context("Failed to submit transaction")?;
    print_transaction_status("  ", &status);
    if !status.success {
        anyhow::bail!(
            "Transaction failed (status: {}, hash: {}). Preview did not commit; retry with --gas-limit higher or check node logs for the Move abort code.",
            status.status,
            status.hash
        );
    }
    Ok(())
}

struct ResolvedTx {
    sender_normalized: String,
    sender_tagged: String,
    wallet: kanari_crypto::wallet::Wallet,
    gas_limit: u64,
    gas_price: u64,
}

async fn resolve_tx(
    from: Option<String>,
    password: Option<String>,
    rpc_endpoint: Option<String>,
    gas_limit: Option<u64>,
    gas_price: Option<u64>,
) -> Result<(ResolvedTx, RpcClient)> {
    let rpc = get_rpc_endpoint(rpc_endpoint);
    let sender_normalized = resolve_sender(from)?;
    let wallet = load_wallet_for(&sender_normalized, password)?;
    let (gas_limit, gas_price) = resolve_transaction_gas(gas_limit, gas_price);
    let client = RpcClient::new(&rpc);
    check_node_connection(&client, &rpc).await?;
    let sender_tagged = get_sender_for_tx(&wallet, &sender_normalized)?;
    Ok((ResolvedTx { sender_normalized, sender_tagged, wallet, gas_limit, gas_price }, client))
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
    /// Admin: set the total supply used for quorum (mist)
    SetTotalSupply {
        #[arg(long)]
        cap: String,
        #[arg(long)]
        state: String,
        #[arg(long)]
        supply: u64,
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
                supply,
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
                eprintln!("Setting total supply...");
                submit_entry(
                    &tx,
                    &client,
                    kanari_types::governance::GovernanceModule::entry_functions().set_total_supply,
                    vec![],
                    vec![bcs_u64(*supply)?],
                    vec![
                        EntryObject { id: cap.clone(), mutable: false, owner_check: true },
                        EntryObject { id: state.clone(), mutable: true, owner_check: false },
                    ],
                )
                .await
            }
        }
    }
}
