// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Multisig wallet CLI: create/propose/approve/execute/cancel/inspect a
//! `0x2::multisig` wallet. Thin wrapper over `kanari call`: every subcommand
//! builds one Move entry call (`multisig::*_entry`) with object inputs
//! resolved from on-chain state.

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
use kanari_types::address::Address as KanariAddress;

const MULTISIG_MODULE: &str = "multisig";
const KANARI_COIN_TYPE: &str = "0x2::kanari::KANARI";

fn system_package() -> String {
    KanariAddress::KANARI_SYSTEM_ADDRESS.to_string()
}

fn parse_addr(addr_normalized: &str) -> Result<move_core_types::account_address::AccountAddress> {
    move_core_types::account_address::AccountAddress::from_hex_literal(addr_normalized)
        .map_err(|e| anyhow::anyhow!("Invalid address {}: {:?}", addr_normalized, e))
}

fn bcs_addr(addr_normalized: &str) -> Result<Vec<u8>> {
    // Object/reference args must be full BCS (the runtime test-suite passes
    // `bcs::to_bytes(&AccountAddress)`); raw 32 bytes fail with
    // FAILED_TO_DESERIALIZE_ARGUMENT.
    bcs::to_bytes(&parse_addr(addr_normalized)?).context("Failed to BCS-encode address")
}

fn bcs_u64(v: u64) -> Result<Vec<u8>> {
    bcs::to_bytes(&v).context("Failed to BCS-encode u64")
}

/// Parse a human amount into base units using the coin's decimals
/// (fetched from owner balances via the API).
///
/// `"1000"` means 1000 whole coins (scaled by decimals), `"1.5"` means one
/// and a half. Raw base-unit integers are unambiguous only when they carry
/// a `mist:` prefix (e.g. `mist:1000`).
async fn parse_human_amount(
    client: &RpcClient,
    owner_normalized: &str,
    coin_type: &str,
    raw: &str,
) -> Result<u64> {
    let s = raw.trim();
    // Explicit escape hatch: raw base units.
    if let Some(mist_str) = s.strip_prefix("mist:") {
        return mist_str
            .parse::<u64>()
            .with_context(|| format!("Invalid mist amount '{}'; use mist:<integer>", s));
    }
    let decimals = coin_decimals(client, owner_normalized, coin_type).await?;
    let (whole, frac) = match s.split_once('.') {
        Some((w, f)) => (w, f),
        // Bare integer = whole coins (NOT mist). Use `mist:` for raw units.
        None => (s, ""),
    };
    if frac.is_empty() && whole.parse::<u64>().is_err() {
        anyhow::bail!(
            "Invalid amount '{}'; use whole coins (e.g. \"1000\"), decimal (e.g. \"1.5\"), or mist:<integer>",
            s
        );
    }
    if frac.len() > decimals as usize {
        anyhow::bail!(
            "Amount '{}' exceeds {} decimals for {}",
            s,
            decimals,
            coin_type
        );
    }
    let scale = 10u64
        .checked_pow(decimals as u32)
        .context("Decimals overflow")?;
    let whole: u64 = whole
        .parse()
        .with_context(|| format!("Invalid amount '{}'", s))?;
    let mut frac_padded = frac.to_string();
    while frac_padded.len() < decimals as usize {
        frac_padded.push('0');
    }
    let frac: u64 = if frac_padded.is_empty() {
        0
    } else {
        frac_padded
            .parse()
            .with_context(|| format!("Invalid amount '{}'", s))?
    };
    whole
        .checked_mul(scale)
        .and_then(|w| w.checked_add(frac))
        .context("Amount overflow")
}

/// Look up a coin's decimals from the owner's balances (the node tags every
/// balance with its decimals from on-chain metadata).
///
/// Compares canonicalized type strings: the API may return
/// `0x2::coin::Coin<...>`-wrapped or legacy spellings while the caller
/// passes the bare inner `T`, so normalize both sides before comparing.
async fn coin_decimals(client: &RpcClient, owner_normalized: &str, coin_type: &str) -> Result<u8> {
    let want = kanari_types::coin::CoinModule::normalize_token_type(coin_type.trim());
    let value = client
        .get_owner_balances(owner_normalized)
        .await
        .context("Failed to fetch owner balances for decimals")?;
    let balances = value
        .get("balances")
        .and_then(|b| b.as_array())
        .context("Malformed balances response")?;
    for entry in balances {
        let entry_type = entry
            .get("token_type")
            .and_then(|t| t.as_str())
            .unwrap_or("");
        let have = kanari_types::coin::CoinModule::normalize_token_type(entry_type);
        // Accept both the bare `T` and the `Coin<T>` wrapper spelling.
        let entry_inner = have
            .split_once('<')
            .and_then(|(_, rest)| rest.strip_suffix('>'))
            .unwrap_or(&have);
        let type_matches = have == want || entry_inner == want || have.contains(&want);
        if type_matches && let Some(d) = entry.get("decimals").and_then(|d| d.as_u64()) {
            return Ok(d as u8);
        }
    }
    anyhow::bail!(
        "No {} balance for {} — decimals unknown; pass mist integer instead",
        coin_type,
        owner_normalized
    )
}

fn bcs_string(s: &str) -> Result<Vec<u8>> {
    bcs::to_bytes(&s.as_bytes().to_vec()).context("Failed to BCS-encode string")
}

fn bcs_address_vec(addrs: &[String]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    // ULEB128 length prefix, then raw 32-byte addresses. (The RPC layer
    // BCS-decodes each arg once for Move `vector<address>`; wrapping in an
    // extra `MoveValue::Vector` layer double-encodes and fails with
    // FAILED_TO_DESERIALIZE_ARGUMENT.)
    out.extend_from_slice(&encode_uleb128(addrs.len() as u64));
    for a in addrs {
        let addr = parse_addr(a).with_context(|| format!("Invalid owner address {}", a))?;
        out.extend_from_slice(&addr.to_vec());
    }
    Ok(out)
}

fn encode_uleb128(mut value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
    out
}

#[derive(Subcommand, Debug)]
pub enum MultisigCommand {
    /// Create a wallet funded with `amount` split from a coin object you own
    Create {
        /// Owner addresses (comma-separated)
        #[arg(long)]
        owners: String,
        #[arg(long)]
        threshold: u64,
        /// Funding amount: whole coins (e.g. `1000`), decimal (e.g. `1.5`),
        /// or raw base units (`mist:1000`). Decimals come from the coin.
        #[arg(long)]
        amount: String,
        /// Funding coin object ID, `auto:<amount>`, or omit to
        /// auto-pick the richest coin (resolved from the API)
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
        /// Transfer amount: whole coins (e.g. `1000`), decimal (e.g. `1.5`),
        /// or raw base units (`mist:1000`)
        #[arg(long)]
        amount: String,
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
    /// Deposit `amount` split from a coin object you own into the wallet
    Deposit {
        #[arg(long)]
        wallet: String,
        /// Deposit amount: whole coins (e.g. `1000`), decimal (e.g. `1.5`),
        /// or raw base units (`mist:1000`)
        #[arg(long)]
        amount: String,
        /// Funding coin object ID, `auto:<amount>`, or omit to
        /// auto-pick the richest coin of the wallet's type (from the API)
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
///
/// The state layer matches `object_type` exactly, and stored coin objects
/// are typed `0x2::coin::Coin<T>` — so the filter must be the full wrapper
/// type, not the bare inner `T`.
async fn list_coins(
    client: &RpcClient,
    owner_normalized: &str,
    coin_type: &str,
) -> Result<Vec<ListedCoin>> {
    let wrapper = format!("0x2::coin::Coin<{}>", coin_type.trim());
    let response = client
        .get_objects(kanari_rpc_api::GetObjectsRequest {
            owner: Some(owner_normalized.to_string()),
            owner_kind: None,
            object_type: Some(wrapper),
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
        let amount = parse_human_amount(client, owner_normalized, coin_type, amount_str).await?;
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
    normalize_addr(coin_arg)
}

/// An object used by a multisig entry call, with the mutability the Move
/// signature actually needs. Marking an immutable-borrowed object as mutable
/// makes the runtime demand sender-ownership and fail co-owner calls with
/// "Ownership verification failed".
struct EntryObject {
    id: String,
    mutable: bool,
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
    // Resolve object inputs (wallet/proposal/coin refs with version+digest).
    // Multisig is cross-owner by design: the wallet/proposal are owned by
    // whoever created them, while any owner can approve/execute. Enforcing
    // sender-ownership here would block co-owners, so skip the check (the
    // Move entry still enforces `is_owner` on-chain).
    let mut object_inputs = Vec::with_capacity(objects.len());
    for entry in &objects {
        let normalized = normalize_addr(&entry.id)?;
        let mut input = object_input_from_object_id(client, &normalized, None).await?;
        input.mutable = entry.mutable;
        object_inputs.push(input);
    }

    let prepared = client
        .build_call_function(BuildCallFunctionRequest {
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
        .await
        .context("Failed to build call function transaction")?;

    if let Some(inputs) = &prepared.object_inputs {
        for input in inputs {
            eprintln!("  Object input: {}", input.object_ref.object_id);
        }
    }
    eprintln!("  Gas Limit: {}", prepared.gas_limit);
    eprintln!("  Gas Price: {} Mist/gas", prepared.gas_price);

    // Reuse the token-transfer submission path (async RpcClient, commit wait
    // included) instead of the blocking reqwest client, which panics when
    // dropped inside the async runtime.
    let status = sign_and_call_function(client, &tx.wallet, prepared)
        .await
        .context("Failed to submit transaction")?;
    print_transaction_status("  ", &status);
    if !status.success {
        // Preview failures don't land on chain (no tx to query), so surface
        // the status verbatim and hint how to get the VM error.
        anyhow::bail!(
            "Transaction failed (status: {}, hash: {}). Preview did not commit; retry with --gas-limit higher or check node logs for the Move abort code.",
            status.status,
            status.hash
        );
    }
    Ok(())
}

impl Multisig {
    pub async fn execute(&self) -> Result<()> {
        match &self.command {
            MultisigCommand::Create {
                owners,
                threshold,
                amount,
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
                let amount_mist =
                    parse_human_amount(&client, &tx.sender_normalized, coin_type, amount).await?;
                eprintln!("  Funding wallet with {} mist...", amount_mist);
                submit_entry(
                    &tx,
                    &client,
                    "create_wallet_entry",
                    vec![coin_type.clone()],
                    vec![
                        bcs_address_vec(&parsed_owners)?,
                        bcs_u64(*threshold)?,
                        bcs_addr(&coin_id)?,
                        bcs_u64(amount_mist)?,
                    ],
                    // Coin stays owned: split off `amount`, remainder saved
                    // back. Declare mutable so the runtime binds &mut Coin.
                    vec![EntryObject {
                        id: coin_id,
                        mutable: true,
                    }],
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
                let resolved_coin_type =
                    resolve_wallet_coin_type(&client, wallet, coin_type).await?;
                let amount_mist =
                    parse_human_amount(&client, &tx.sender_normalized, &resolved_coin_type, amount)
                        .await?;
                eprintln!("Proposing transfer of {} mist to {}...", amount_mist, to);
                submit_entry(
                    &tx,
                    &client,
                    "propose_transfer_entry",
                    vec![resolved_coin_type],
                    vec![
                        bcs_addr(&normalize_addr(wallet)?)?,
                        bcs_addr(&normalize_addr(to)?)?,
                        bcs_u64(amount_mist)?,
                        bcs_string(description)?,
                        bcs_u64(*ttl_ms)?,
                    ],
                    // propose takes &wallet (read-only): immutable so
                    // co-owners can propose without owning the wallet.
                    vec![EntryObject {
                        id: wallet.clone(),
                        mutable: false,
                    }],
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
                    // approve takes (&wallet, &mut proposal).
                    vec![
                        EntryObject {
                            id: wallet.clone(),
                            mutable: false,
                        },
                        EntryObject {
                            id: proposal.clone(),
                            mutable: true,
                        },
                    ],
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
                    // execute consumes the proposal and mutates the wallet.
                    vec![
                        EntryObject {
                            id: wallet.clone(),
                            mutable: true,
                        },
                        EntryObject {
                            id: proposal.clone(),
                            mutable: true,
                        },
                    ],
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
                    // cancel reads the wallet, consumes the proposal.
                    vec![
                        EntryObject {
                            id: wallet.clone(),
                            mutable: false,
                        },
                        EntryObject {
                            id: proposal.clone(),
                            mutable: true,
                        },
                    ],
                )
                .await
            }
            MultisigCommand::Deposit {
                wallet,
                amount,
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
                let amount_mist =
                    parse_human_amount(&client, &tx.sender_normalized, &resolved_coin_type, amount)
                        .await?;
                eprintln!(
                    "Depositing {} mist from coin {} into wallet {}...",
                    amount_mist, coin_id, wallet
                );
                submit_entry(
                    &tx,
                    &client,
                    "deposit_entry",
                    vec![resolved_coin_type],
                    vec![
                        bcs_addr(&normalize_addr(wallet)?)?,
                        bcs_addr(&coin_id)?,
                        bcs_u64(amount_mist)?,
                    ],
                    // deposit mutates both wallet and source coin.
                    vec![
                        EntryObject {
                            id: wallet.clone(),
                            mutable: true,
                        },
                        EntryObject {
                            id: coin_id,
                            mutable: true,
                        },
                    ],
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
                eprintln!("Use an ID as --coin, or --coin auto:<amount> (e.g. auto:1000).");
                Ok(())
            }
        }
    }
}
