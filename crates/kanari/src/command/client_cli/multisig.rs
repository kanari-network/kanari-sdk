// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Multisig wallet CLI: create/propose/approve/execute/cancel/inspect a
//! `0x2::multisig` wallet. Thin wrapper over `kanari call`: every subcommand
//! builds one Move entry call (`multisig::*_entry`) with object inputs
//! resolved from on-chain state.

use crate::command::common::{
    build_blocking_client, check_node_connection, get_rpc_endpoint, get_sender_for_tx,
    load_wallet_for, normalize_addr, resolve_sender, resolve_transaction_gas,
};
use crate::command::rpc_helpers::{
    object_input_from_object_id, render_transaction_submission, require_rpc_result,
    sign_call_function_request, submit_blocking_rpc,
};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use kanari_rpc_api::{BuildCallFunctionRequest, CallFunctionRequest, methods};
use kanari_rpc_client::RpcClient;
use kanari_types::address::Address as KanariAddress;
use move_core_types::runtime_value::MoveValue;

const MULTISIG_MODULE: &str = "multisig";
const KANARI_COIN_TYPE: &str = "0x2::kanari::KANARI";

fn system_package() -> String {
    KanariAddress::KANARI_SYSTEM_ADDRESS.to_string()
}

fn bcs_addr(addr_normalized: &str) -> Result<Vec<u8>> {
    let addr = move_core_types::account_address::AccountAddress::from_hex_literal(addr_normalized)
        .map_err(|e| anyhow::anyhow!("Invalid address {}: {:?}", addr_normalized, e))?;
    Ok(addr.to_vec())
}

fn bcs_u64(v: u64) -> Result<Vec<u8>> {
    bcs::to_bytes(&v).context("Failed to BCS-encode u64")
}

fn bcs_bytes(v: &[u8]) -> Result<Vec<u8>> {
    bcs::to_bytes(&v.to_vec()).context("Failed to BCS-encode bytes")
}

fn bcs_string(s: &str) -> Result<Vec<u8>> {
    bcs::to_bytes(&s.as_bytes().to_vec()).context("Failed to BCS-encode string")
}

fn bcs_address_vec(addrs: &[String]) -> Result<Vec<u8>> {
    let mut vec = Vec::with_capacity(addrs.len());
    for a in addrs {
        vec.push(
            move_core_types::account_address::AccountAddress::from_hex_literal(a)
                .map_err(|e| anyhow::anyhow!("Invalid owner address {}: {:?}", a, e))?,
        );
    }
    bcs::to_bytes(&MoveValue::Vector(
        vec.into_iter().map(MoveValue::Address).collect(),
    ))
    .context("Failed to BCS-encode address vector")
}

#[derive(Subcommand, Debug)]
pub enum MultisigCommand {
    /// Create a wallet funded with a coin object you own
    Create {
        /// Owner addresses (comma-separated)
        #[arg(long)]
        owners: String,
        #[arg(long)]
        threshold: u64,
        /// Coin object ID to fund the wallet with
        #[arg(long)]
        coin: String,
        /// Coin type (default 0x2::kanari::KANARI)
        #[arg(long, default_value = KANARI_COIN_TYPE)]
        coin_type: String,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
    /// Propose a transfer from the wallet (proposer auto-approves)
    Propose {
        #[arg(long)]
        wallet: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: u64,
        #[arg(long, default_value = "")]
        description: String,
        /// Proposal TTL in ms (0 = never expires)
        #[arg(long, default_value_t = 0)]
        ttl_ms: u64,
        #[arg(long)]
        coin_type: Option<String>,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
    /// Approve someone else's proposal
    Approve {
        #[arg(long)]
        wallet: String,
        #[arg(long)]
        proposal: String,
        #[arg(long)]
        coin_type: Option<String>,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
    /// Execute a proposal whose threshold is met (consumes the proposal)
    Execute {
        #[arg(long)]
        wallet: String,
        #[arg(long)]
        proposal: String,
        #[arg(long)]
        coin_type: Option<String>,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
    /// Cancel your own live proposal (consumes the proposal)
    Cancel {
        #[arg(long)]
        wallet: String,
        #[arg(long)]
        proposal: String,
        #[arg(long)]
        coin_type: Option<String>,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
    /// Deposit a coin object into the wallet
    Deposit {
        #[arg(long)]
        wallet: String,
        #[arg(long)]
        coin: String,
        #[arg(long)]
        coin_type: Option<String>,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
}

#[derive(Parser, Debug)]
#[clap(name = "multisig", about = "Multisig wallet operations")]
pub struct Multisig {
    #[command(subcommand)]
    pub command: MultisigCommand,
    #[clap(long = "gas-limit")]
    pub gas_limit: Option<u64>,
    #[clap(long = "gas-price")]
    pub gas_price: Option<u64>,
}

struct ResolvedTx {
    sender_normalized: String,
    sender_tagged: String,
    wallet: kanari_crypto::wallet::Wallet,
    rpc: String,
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
    // Password prompt is sync; do it before creating the async client.
    let wallet = load_wallet_for(&sender_normalized, password)?;
    let (gas_limit, gas_price) = resolve_transaction_gas(gas_limit, gas_price);
    let client = RpcClient::new(&rpc);
    check_node_connection(&client, &rpc).await?;
    let sender_tagged = get_sender_for_tx(&wallet, &sender_normalized)?;
    Ok((
        ResolvedTx {
            sender_normalized,
            sender_tagged,
            wallet,
            rpc,
            gas_limit,
            gas_price,
        },
        client,
    ))
}

#[allow(clippy::too_many_arguments)]
async fn submit_entry(
    tx: &ResolvedTx,
    client: &RpcClient,
    function: &str,
    type_args: Vec<String>,
    args: Vec<Vec<u8>>,
    object_ids: Vec<String>,
    label: &str,
) -> Result<()> {
    // Resolve object inputs (wallet/proposal/coin refs with version+digest).
    let mut object_inputs = Vec::with_capacity(object_ids.len());
    for id in &object_ids {
        let normalized = normalize_addr(id)?;
        object_inputs.push(
            object_input_from_object_id(client, &normalized, Some(&tx.sender_normalized)).await?,
        );
    }

    let prepared: CallFunctionRequest = serde_json::from_value(require_rpc_result(
        submit_blocking_rpc(
            &build_blocking_client(30)?,
            &tx.rpc,
            methods::BUILD_CALL_FUNCTION,
            serde_json::to_value(BuildCallFunctionRequest {
                sender: tx.sender_tagged.clone(),
                package: system_package(),
                module: MULTISIG_MODULE.to_string(),
                function: function.to_string(),
                type_args,
                args,
                object_inputs: Some(object_inputs),
                gas_limit: tx.gas_limit,
                gas_price: tx.gas_price,
                nonce: None,
                execute_immediate: Some(true),
            })
            .context("Failed to serialize build call request")?,
        )?,
        "RPC did not return prepared call request",
    )?)
    .context("Failed to decode prepared call request")?;

    let signed = sign_call_function_request(prepared, &tx.wallet)?;
    eprintln!("  Transaction signed");
    let rpc_response = submit_blocking_rpc(
        &build_blocking_client(30)?,
        &tx.rpc,
        methods::CALL_FUNCTION,
        serde_json::to_value(signed).context("Failed to serialize call request")?,
    )?;
    render_transaction_submission(
        &build_blocking_client(30)?,
        &tx.rpc,
        rpc_response,
        label,
        true,
    )?;
    Ok(())
}

impl Multisig {
    pub async fn execute(&self) -> Result<()> {
        match &self.command {
            MultisigCommand::Create {
                owners,
                threshold,
                coin,
                coin_type,
                from,
                password,
                rpc_endpoint,
            } => {
                let parsed_owners: Vec<String> = owners
                    .split(',')
                    .map(|s| normalize_addr(s.trim()))
                    .collect::<Result<_>>()
                    .context("Invalid owner address list")?;
                if parsed_owners.is_empty() {
                    anyhow::bail!("Provide at least one owner via --owners");
                }
                let (tx, client) = resolve_tx(
                    from.clone(),
                    password.clone(),
                    rpc_endpoint.clone(),
                    self.gas_limit,
                    self.gas_price,
                )
                .await?;
                eprintln!(
                    "Creating multisig wallet ({} owners, threshold {})...",
                    parsed_owners.len(),
                    threshold
                );
                submit_entry(
                    &tx,
                    &client,
                    "create_wallet_entry",
                    vec![coin_type.clone()],
                    vec![
                        bcs_address_vec(&parsed_owners)?,
                        bcs_u64(*threshold)?,
                        bcs_addr(&normalize_addr(coin)?)?,
                    ],
                    vec![coin.clone()],
                    "",
                )
                .await
            }
            MultisigCommand::Propose {
                wallet,
                to,
                amount,
                description,
                ttl_ms,
                coin_type,
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
                eprintln!("Proposing transfer of {} to {}...", amount, to);
                submit_entry(
                    &tx,
                    &client,
                    "propose_transfer_entry",
                    vec![
                        coin_type
                            .clone()
                            .unwrap_or_else(|| KANARI_COIN_TYPE.to_string()),
                    ],
                    vec![
                        bcs_addr(&normalize_addr(wallet)?)?,
                        bcs_addr(&normalize_addr(to)?)?,
                        bcs_u64(*amount)?,
                        bcs_string(description)?,
                        bcs_u64(*ttl_ms)?,
                    ],
                    vec![wallet.clone()],
                    "",
                )
                .await
            }
            MultisigCommand::Approve {
                wallet,
                proposal,
                coin_type,
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
                eprintln!("Approving proposal {}...", proposal);
                submit_entry(
                    &tx,
                    &client,
                    "approve_entry",
                    vec![
                        coin_type
                            .clone()
                            .unwrap_or_else(|| KANARI_COIN_TYPE.to_string()),
                    ],
                    vec![
                        bcs_addr(&normalize_addr(wallet)?)?,
                        bcs_addr(&normalize_addr(proposal)?)?,
                    ],
                    vec![wallet.clone(), proposal.clone()],
                    "",
                )
                .await
            }
            MultisigCommand::Execute {
                wallet,
                proposal,
                coin_type,
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
                eprintln!("Executing proposal {}...", proposal);
                submit_entry(
                    &tx,
                    &client,
                    "execute_entry",
                    vec![
                        coin_type
                            .clone()
                            .unwrap_or_else(|| KANARI_COIN_TYPE.to_string()),
                    ],
                    vec![
                        bcs_addr(&normalize_addr(wallet)?)?,
                        bcs_addr(&normalize_addr(proposal)?)?,
                    ],
                    vec![wallet.clone(), proposal.clone()],
                    "",
                )
                .await
            }
            MultisigCommand::Cancel {
                wallet,
                proposal,
                coin_type,
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
                eprintln!("Cancelling proposal {}...", proposal);
                submit_entry(
                    &tx,
                    &client,
                    "cancel_entry",
                    vec![
                        coin_type
                            .clone()
                            .unwrap_or_else(|| KANARI_COIN_TYPE.to_string()),
                    ],
                    vec![
                        bcs_addr(&normalize_addr(wallet)?)?,
                        bcs_addr(&normalize_addr(proposal)?)?,
                    ],
                    vec![wallet.clone(), proposal.clone()],
                    "",
                )
                .await
            }
            MultisigCommand::Deposit {
                wallet,
                coin,
                coin_type,
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
                eprintln!("Depositing coin {} into wallet {}...", coin, wallet);
                submit_entry(
                    &tx,
                    &client,
                    "deposit_entry",
                    vec![
                        coin_type
                            .clone()
                            .unwrap_or_else(|| KANARI_COIN_TYPE.to_string()),
                    ],
                    vec![
                        bcs_addr(&normalize_addr(wallet)?)?,
                        bcs_bytes(&hex::decode(
                            normalize_addr(coin)?.trim_start_matches("0x"),
                        )?)?,
                    ],
                    vec![wallet.clone(), coin.clone()],
                    "",
                )
                .await
            }
        }
    }
}
