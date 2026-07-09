// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result, bail};
use kanari_crypto::wallet::{Wallet, load_wallet};
use kanari_rpc_api::{CallFunctionRequest, ObjectInfo, TransactionStatus};
use kanari_rpc_client::RpcClient;
use kanari_types::GasConfig;
use kanari_types::address::Address;
use kanari_types::kanari::KANARI_TOKEN_TYPE;
use kanari_types::transaction::{SignedTransaction, Transaction};
use log::error;
use reqwest::blocking::Client;
use rpassword;
use std::time::Duration;

/// Normalize and validate an address string to a 0x-prefixed 64-hex format
pub fn normalize_addr(a: &str) -> Result<String> {
    use std::str::FromStr;
    // Use the central Address type to handle tagged addresses, public keys, and hex literals
    let addr = Address::from_str(a).with_context(|| format!("Invalid address: {}", a))?;
    Ok(addr.to_hex_literal())
}

/// Determine the RPC endpoint to use.
pub fn resolve_transaction_gas(gas_limit: Option<u64>, gas_price: Option<u64>) -> (u64, u64) {
    let config = GasConfig::default();
    (
        gas_limit.unwrap_or_else(|| config.default_transaction_gas_limit()),
        gas_price.unwrap_or_else(|| config.default_transaction_gas_price()),
    )
}

pub fn get_rpc_endpoint(rpc_opt: Option<String>) -> String {
    rpc_opt
        .or_else(kanari_common::get_active_rpc)
        .unwrap_or_else(|| "http://127.0.0.1:6767".to_string())
}

/// Resolve the sender address from either an option or the selected wallet.
pub fn resolve_sender(from_opt: Option<String>) -> Result<String> {
    let addr = if let Some(f) = from_opt {
        f
    } else {
        kanari_crypto::wallet::get_selected_wallet().ok_or_else(|| {
            anyhow::anyhow!("No sender provided and no selected wallet set. Use --from or run `kanari keytool load-wallet` to select one.")
        })?
    };
    normalize_addr(&addr)
}

/// Load a wallet for the given normalized address, prompting for a password if not provided.
pub fn load_wallet_for(
    address_normalized: &str,
    password_opt: Option<String>,
) -> Result<kanari_crypto::wallet::Wallet> {
    let password = match password_opt {
        Some(p) => p,
        None => rpassword::prompt_password("Wallet password: ")
            .context("Password required for signing")?,
    };

    let w = load_wallet(address_normalized, &password)
        .context("Failed to load wallet. Make sure the wallet exists and password is correct")?;

    Ok(w)
}

/// Build a blocking HTTP client with optional timeout (seconds)
pub fn build_blocking_client(timeout_secs: u64) -> Result<Client> {
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .context("Failed to build HTTP client")?;
    Ok(client)
}

/// Check node connection and return block height (async)
pub async fn check_node_connection(client: &RpcClient, rpc: &str) -> Result<u64> {
    match client.get_block_height().await {
        Ok(height) => {
            eprintln!("  Connected to node (height: {})", height);
            Ok(height)
        }
        Err(_) => {
            error!("  Cannot connect to RPC server at {}", rpc);
            error!("  Please start the node first: cargo run --bin kanari-node");
            Err(anyhow::anyhow!("RPC server not available"))
        }
    }
}

pub fn sign_call_function_request(
    mut request: CallFunctionRequest,
    wallet: &Wallet,
) -> Result<CallFunctionRequest> {
    let transaction = Transaction::ExecuteFunction {
        sender: request.sender.clone(),
        module: format!("{}::{}", request.package, request.module),
        function: request.function.clone(),
        type_args: request.type_args.clone(),
        args: request.args.clone(),
        gas_limit: request.gas_limit,
        gas_price: request.gas_price,
        sequence_number: request.sequence_number,
    };

    let mut signed_tx = SignedTransaction::new(transaction);
    signed_tx
        .sign(&wallet.private_key, wallet.curve_type)
        .context("Failed to sign transaction")?;
    request.signature = Some(signed_tx.signature);
    Ok(request)
}

pub async fn sign_and_call_function(
    client: &RpcClient,
    wallet: &Wallet,
    request: CallFunctionRequest,
) -> Result<TransactionStatus> {
    let signed_request = sign_call_function_request(request, wallet)?;
    let status = client
        .call_function(signed_request)
        .await
        .context("Failed to submit transaction")?;

    if status.status != "pending" && status.status != "executed" && status.status != "committed" {
        bail!(
            "Transaction was not successful (status: {}). Tx hash: {}",
            status.status,
            status.hash
        );
    }

    Ok(status)
}

fn read_coin_balance(data: &[u8]) -> Option<u64> {
    if data.len() < 40 {
        return None;
    }

    let mut amount_bytes = [0u8; 8];
    amount_bytes.copy_from_slice(&data[32..40]);
    Some(u64::from_le_bytes(amount_bytes))
}

#[derive(Debug, Clone)]
pub struct SelectedCoinObject {
    pub coin_object_id: String,
    pub selected_balance: u64,
    pub total_balance: u64,
}

#[derive(Debug, Clone)]
pub struct SpendableCoinObject {
    pub coin_object_id: String,
    pub balance: u64,
}

pub fn spendable_coin_objects(
    owned_objects: &[ObjectInfo],
    token_type: &str,
) -> Vec<SpendableCoinObject> {
    let coin_type = format!("0x2::coin::Coin<{}>", token_type);
    let mut coins = Vec::new();

    for obj in owned_objects {
        if obj.type_ != coin_type {
            continue;
        }

        let Some(balance) = read_coin_balance(&obj.data) else {
            continue;
        };
        if balance == 0 {
            continue;
        }

        coins.push(SpendableCoinObject {
            coin_object_id: obj.id.clone(),
            balance,
        });
    }

    coins
}

pub fn select_coin_object(
    owned_objects: &[ObjectInfo],
    token_type: &str,
    required_amount: u64,
) -> Result<SelectedCoinObject> {
    let coins = spendable_coin_objects(owned_objects, token_type);
    let mut total_balance = 0u64;
    let mut smallest_sufficient: Option<(String, u64)> = None;
    let mut largest_available: Option<(String, u64)> = None;

    for coin in coins {
        let balance = coin.balance;
        total_balance = total_balance.saturating_add(balance);

        if balance >= required_amount {
            match &smallest_sufficient {
                Some((_, current)) if *current <= balance => {}
                _ => smallest_sufficient = Some((coin.coin_object_id.clone(), balance)),
            }
        }

        match &largest_available {
            Some((_, current)) if *current >= balance => {}
            _ => largest_available = Some((coin.coin_object_id.clone(), balance)),
        }
    }

    let (coin_object_id, selected_balance) = smallest_sufficient
        .or(largest_available)
        .context("No spendable native coin object found")?;

    Ok(SelectedCoinObject {
        coin_object_id,
        selected_balance,
        total_balance,
    })
}

pub fn select_native_coin_object(
    owned_objects: &[ObjectInfo],
    required_amount: u64,
) -> Result<SelectedCoinObject> {
    select_coin_object(owned_objects, KANARI_TOKEN_TYPE, required_amount)
}

pub async fn consolidate_coin_objects(
    client: &RpcClient,
    wallet: &Wallet,
    sender_tagged: &str,
    token_type: &str,
    spendable_coins: &[SpendableCoinObject],
    required_balance: u64,
    starting_sequence: u64,
    gas_limit: u64,
    gas_price: u64,
) -> Result<(SelectedCoinObject, u64)> {
    let mut coins = spendable_coins.to_vec();
    coins.sort_by(|a, b| b.balance.cmp(&a.balance));

    let Some(primary) = coins.first().cloned() else {
        bail!("No spendable Coin<{}> object found", token_type);
    };

    let mut accumulated = primary.balance;
    let mut sequence_number = starting_sequence;

    for coin in coins.iter().skip(1) {
        if accumulated >= required_balance {
            break;
        }

        let join_req = CallFunctionRequest {
            sender: sender_tagged.to_string(),
            package: "0x2".to_string(),
            module: "coin".to_string(),
            function: "join_entry".to_string(),
            type_args: vec![token_type.to_string()],
            args: vec![
                move_core_types::account_address::AccountAddress::from_hex_literal(
                    &primary.coin_object_id,
                )
                .context("Invalid primary coin object ID")?
                .to_vec(),
                move_core_types::account_address::AccountAddress::from_hex_literal(
                    &coin.coin_object_id,
                )
                .context("Invalid merge coin object ID")?
                .to_vec(),
            ],
            gas_limit,
            gas_price,
            sequence_number,
            signature: None,
            execute_immediate: Some(true),
        };
        let status = sign_and_call_function(client, wallet, join_req).await?;
        eprintln!(
            "  Consolidated coin {} into {} (tx: {})",
            coin.coin_object_id, primary.coin_object_id, status.hash
        );
        sequence_number = sequence_number.saturating_add(1);
        accumulated = accumulated.saturating_add(coin.balance);
    }

    Ok((
        SelectedCoinObject {
            coin_object_id: primary.coin_object_id,
            selected_balance: accumulated,
            total_balance: spendable_coins
                .iter()
                .fold(0u64, |sum, coin| sum.saturating_add(coin.balance)),
        },
        sequence_number,
    ))
}

/// Query owner sequence number from RPC for a sender address (normalized).
pub fn get_owner_sequence(
    client: &Client,
    rpc_endpoint: &str,
    sender_normalized: &str,
) -> Result<u64> {
    use kanari_rpc_api::{RpcRequest, RpcResponse, methods};

    let acct_req = RpcRequest {
        jsonrpc: "2.0".to_string(),
        method: methods::GET_OWNER.to_string(),
        params: serde_json::to_value(sender_normalized)
            .context("Failed to serialize sender for RPC")?,
        id: 1,
    };

    let resp = client
        .post(rpc_endpoint)
        .json(&acct_req)
        .send()
        .context("Failed to query owner sequence number from RPC")?;

    let rpc_resp: RpcResponse = resp.json().context("Failed to parse owner RPC response")?;

    if let Some(result) = rpc_resp.result {
        if let Some(sn) = result.get("sequence_number").and_then(|v| v.as_u64()) {
            Ok(sn)
        } else {
            bail!("Owner RPC result missing 'sequence_number'");
        }
    } else {
        bail!("RPC did not return owner info for sender");
    }
}


/// Determine the sender address string for a transaction.
/// For all wallets, this returns the tagged address (Curve:PublicKey)
/// which is required for signature verification by the node.
pub fn get_sender_for_tx(
    wallet: &kanari_crypto::wallet::Wallet,
    _address_normalized: &str,
) -> Result<String> {
    // Re-derive public key from private key to get the tagged address format
    // Tagged addresses are required for signature verification in ALL scenarios
    let keypair =
        kanari_crypto::keys::keypair_from_private_key(&wallet.private_key, wallet.curve_type)
            .context("Failed to derive public key from wallet")?;
    Ok(keypair.tagged_address())
}
