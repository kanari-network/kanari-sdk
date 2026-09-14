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
        /// Coin object ID, `auto:<mist_amount>`, or omit to auto-pick the
        /// richest coin (resolved from the API)
        #[arg(long, default_value = "")]
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
        /// Coin object ID, `auto:<mist_amount>`, or omit to auto-pick the
        /// richest coin of the wallet's type (resolved from the API)
        #[arg(long, default_value = "")]
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
    /// List your coin objects (from the API) to pick --coin/--wallet IDs
    Coins {
        #[arg(long)]
        from: Option<String>,
        /// Filter by coin type (default 0x2::kanari::KANARI)
        #[arg(long, default_value = KANARI_COIN_TYPE)]
        coin_type: String,
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

/// A coin object with its decoded balance, listed from the API.
struct ListedCoin {
    id: String,
    balance: u64,
    version: u64,
}

fn coin_balance_from_data(data: &[u8]) -> Option<u64> {
    // Coin<T> BCS: UID (32 bytes addr) ++ Balance(u64 LE). Balance sits in
    // the last 8 bytes.
    if data.len() < 8 {
        return None;
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&data[data.len() - 8..]);
    Some(u64::from_le_bytes(buf))
}

/// List owned coin objects of `coin_type` via the API (same source as
/// `kanari client objects`, but filtered + balance-decoded).
async fn list_coins(
    client: &RpcClient,
    owner_normalized: &str,
    coin_type: &str,
) -> Result<Vec<ListedCoin>> {
    let response = client
        .get_objects(kanari_rpc_api::GetObjectsRequest {
            owner: Some(owner_normalized.to_string()),
            owner_kind: None,
            object_type: Some(coin_type.to_string()),
            min_version: None,
            max_version: None,
        })
        .await
        .context("Failed to list coin objects from API")?;
    let mut coins = Vec::new();
    for obj in response.objects {
        if !obj.type_.contains("::coin::Coin<") {
            continue;
        }
        let balance = coin_balance_from_data(&obj.data).unwrap_or(0);
        coins.push(ListedCoin {
            id: obj.id,
            balance,
            version: obj.version,
        });
    }
    coins.sort_by_key(|c| std::cmp::Reverse(c.balance));
    Ok(coins)
}

/// Extract the `T` in `0x2::multisig::MultisigWallet<T>` from a wallet
/// object so callers don't have to pass `--coin-type` by hand.
async fn wallet_coin_type(client: &RpcClient, wallet_id: &str) -> Result<String> {
    let normalized = normalize_addr(wallet_id)?;
    let obj = client
        .get_object(&normalized)
        .await
        .with_context(|| format!("Failed to fetch wallet object {}", normalized))?;
    // Type looks like `0x2::multisig::MultisigWallet<0xABC::james::JAMES>`.
    let inner = obj
        .type_
        .split_once('<')
        .and_then(|(_, rest)| rest.strip_suffix('>'))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Object {} is not a multisig wallet (type: {})",
                normalized,
                obj.type_
            )
        })?;
    Ok(inner.trim().to_string())
}

/// Prefer an explicit `--coin-type`; otherwise read `T` from the wallet
/// object via the API.
async fn resolve_wallet_coin_type(
    client: &RpcClient,
    wallet_id: &str,
    coin_type_opt: &Option<String>,
) -> Result<String> {
    if let Some(t) = coin_type_opt
        && !t.trim().is_empty()
    {
        return Ok(t.trim().to_string());
    }
    let detected = wallet_coin_type(client, wallet_id).await?;
    eprintln!("  Detected wallet coin type: {}", detected);
    Ok(detected)
}

/// Resolve `--coin`: an object ID verbatim, `auto:<amount>` to pick the
/// smallest owned coin holding at least `amount` (from the API), or empty
/// (omitted flag) to pick the richest coin outright.
async fn resolve_coin_arg(
    client: &RpcClient,
    owner_normalized: &str,
    coin_arg: &str,
    coin_type: &str,
) -> Result<String> {
    if coin_arg.trim().is_empty() {
        let coins = list_coins(client, owner_normalized, coin_type).await?;
        let picked = coins.first().ok_or_else(|| {
            anyhow::anyhow!("No owned {} coin found; check `multisig coins`", coin_type)
        })?;
        eprintln!(
            "  Auto-picked coin {} (balance {} mist)",
            picked.id, picked.balance
        );
        return Ok(picked.id.clone());
    }
    if let Some(amount_str) = coin_arg.strip_prefix("auto:") {
        let amount: u64 = amount_str
            .parse()
            .context("Invalid auto amount; use auto:<mist_amount>")?;
        let mut coins = list_coins(client, owner_normalized, coin_type).await?;
        coins.retain(|c| c.balance >= amount);
        let picked = coins.pop().ok_or_else(|| {
            anyhow::anyhow!(
                "No owned {} coin holds >= {} mist; check `multisig coins`",
                coin_type,
                amount
            )
        })?;
        eprintln!(
            "  Auto-picked coin {} (balance {} mist)",
            picked.id, picked.balance
        );
        return Ok(picked.id);
    }
    Ok(normalize_addr(coin_arg)?)
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
                let coin_id =
                    resolve_coin_arg(&client, &tx.sender_normalized, coin, coin_type).await?;
                submit_entry(
                    &tx,
                    &client,
                    "create_wallet_entry",
                    vec![coin_type.clone()],
                    vec![
                        bcs_address_vec(&parsed_owners)?,
                        bcs_u64(*threshold)?,
                        bcs_addr(&coin_id)?,
                    ],
                    vec![coin_id],
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
                let resolved_coin_type =
                    resolve_wallet_coin_type(&client, wallet, coin_type).await?;
                submit_entry(
                    &tx,
                    &client,
                    "propose_transfer_entry",
                    vec![resolved_coin_type],
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
                let resolved_coin_type =
                    resolve_wallet_coin_type(&client, wallet, coin_type).await?;
                submit_entry(
                    &tx,
                    &client,
                    "approve_entry",
                    vec![resolved_coin_type],
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
                let resolved_coin_type =
                    resolve_wallet_coin_type(&client, wallet, coin_type).await?;
                submit_entry(
                    &tx,
                    &client,
                    "execute_entry",
                    vec![resolved_coin_type],
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
                let resolved_coin_type =
                    resolve_wallet_coin_type(&client, wallet, coin_type).await?;
                submit_entry(
                    &tx,
                    &client,
                    "cancel_entry",
                    vec![resolved_coin_type],
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
                let resolved_coin_type = coin_type
                    .clone()
                    .unwrap_or_else(|| KANARI_COIN_TYPE.to_string());
                let coin_id =
                    resolve_coin_arg(&client, &tx.sender_normalized, coin, &resolved_coin_type)
                        .await?;
                eprintln!("Depositing coin {} into wallet {}...", coin_id, wallet);
                submit_entry(
                    &tx,
                    &client,
                    "deposit_entry",
                    vec![resolved_coin_type],
                    vec![bcs_addr(&normalize_addr(wallet)?)?, bcs_addr(&coin_id)?],
                    vec![wallet.clone(), coin_id],
                    "",
                )
                .await
            }
            MultisigCommand::Coins {
                from,
                coin_type,
                rpc_endpoint,
            } => {
                let sender_normalized = resolve_sender(from.clone())?;
                let rpc = get_rpc_endpoint(rpc_endpoint.clone());
                let client = RpcClient::new(&rpc);
                check_node_connection(&client, &rpc).await?;
                let coins = list_coins(&client, &sender_normalized, coin_type).await?;
                if coins.is_empty() {
                    eprintln!("No {} coins owned by {}", coin_type, sender_normalized);
                    return Ok(());
                }
                eprintln!("COINS ({}):", coin_type);
                for (i, c) in coins.iter().enumerate() {
                    eprintln!(
                        "  #{} ID: {}  balance: {} mist  version: {}",
                        i + 1,
                        c.id,
                        c.balance,
                        c.version
                    );
                }
                eprintln!("Use an ID as --coin, or --coin auto:<mist_amount>.");
                Ok(())
            }
        }
    }
}
