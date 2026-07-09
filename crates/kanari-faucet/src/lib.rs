// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Library helpers for the kanari-faucet crate.
//!
//! Exposes an async function `request_from_dev` that other crates (for example
//! the `kanari` binary) can call to request tokens from the Dev wallet.

use anyhow::Context;
use kanari_common::get_main_wallet;
use kanari_rpc_api::{CallFunctionRequest, TransactionStatus};
use kanari_rpc_client::RpcClient;
use kanari_types::{
    address::Address,
    kanari::KANARI_TOKEN_TYPE,
    transaction::{SignedTransaction, Transaction},
};
use std::env;

fn read_coin_balance(data: &[u8]) -> Option<u64> {
    if data.len() < 40 {
        return None;
    }

    let mut amount_bytes = [0u8; 8];
    amount_bytes.copy_from_slice(&data[32..40]);
    Some(u64::from_le_bytes(amount_bytes))
}

fn select_native_coin_object(
    owned_objects: &[kanari_rpc_api::ObjectInfo],
    required_amount: u64,
) -> Option<(String, u64, u64)> {
    let coin_type = format!("0x2::coin::Coin<{}>", KANARI_TOKEN_TYPE);
    let mut total_balance = 0u64;
    let mut smallest_sufficient: Option<(String, u64)> = None;
    let mut largest_available: Option<(String, u64)> = None;

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

        total_balance = total_balance.saturating_add(balance);

        if balance >= required_amount {
            match &smallest_sufficient {
                Some((_, current)) if *current <= balance => {}
                _ => smallest_sufficient = Some((obj.id.clone(), balance)),
            }
        }

        match &largest_available {
            Some((_, current)) if *current >= balance => {}
            _ => largest_available = Some((obj.id.clone(), balance)),
        }
    }

    smallest_sufficient
        .or(largest_available)
        .map(|(id, balance)| (id, balance, total_balance))
}

fn sign_call_function_request(
    mut request: CallFunctionRequest,
    wallet: &kanari_crypto::wallet::Wallet,
) -> anyhow::Result<CallFunctionRequest> {
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
        .context("Failed to sign function call")?;
    request.signature = Some(signed_tx.signature);
    Ok(request)
}

async fn sign_and_call_function(
    client: &RpcClient,
    wallet: &kanari_crypto::wallet::Wallet,
    request: CallFunctionRequest,
) -> anyhow::Result<TransactionStatus> {
    let signed_request = sign_call_function_request(request, wallet)?;
    client
        .call_function(signed_request)
        .await
        .context("Failed to submit function call")
}

async fn consolidate_native_coins(
    client: &RpcClient,
    wallet: &kanari_crypto::wallet::Wallet,
    sender_tagged: &str,
    owned_objects: &[kanari_rpc_api::ObjectInfo],
    required_amount: u64,
    starting_sequence: u64,
    gas_limit: u64,
    gas_price: u64,
) -> anyhow::Result<(String, u64, u64, u64)> {
    let coin_type = format!("0x2::coin::Coin<{}>", KANARI_TOKEN_TYPE);
    let mut coins: Vec<(String, u64)> = owned_objects
        .iter()
        .filter(|obj| obj.type_ == coin_type)
        .filter_map(|obj| read_coin_balance(&obj.data).map(|balance| (obj.id.clone(), balance)))
        .filter(|(_, balance)| *balance > 0)
        .collect();
    coins.sort_by(|a, b| b.1.cmp(&a.1));

    let Some((primary_id, primary_balance)) = coins.first().cloned() else {
        anyhow::bail!("No spendable native coin object found in dev wallet");
    };

    let total_balance = coins
        .iter()
        .fold(0u64, |sum, (_, balance)| sum.saturating_add(*balance));
    let mut accumulated = primary_balance;
    let mut sequence_number = starting_sequence;

    for (coin_id, balance) in coins.iter().skip(1) {
        if accumulated >= required_amount {
            break;
        }

        let join_req = CallFunctionRequest {
            sender: sender_tagged.to_string(),
            package: "0x2".to_string(),
            module: "coin".to_string(),
            function: "join_entry".to_string(),
            type_args: vec![KANARI_TOKEN_TYPE.to_string()],
            args: vec![
                Address::from_hex_literal(&primary_id)
                    .context("Invalid primary coin object ID")?
                    .to_vec(),
                Address::from_hex_literal(coin_id)
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
            coin_id, primary_id, status.hash
        );
        sequence_number = sequence_number.saturating_add(1);
        accumulated = accumulated.saturating_add(*balance);
    }

    Ok((primary_id, accumulated, total_balance, sequence_number))
}

/// Request KANARI tokens from the Dev wallet.
///
/// Parameters:
/// - `dev_address`: optional dev address hex string (if None, uses `Address::DEV_ADDRESS`).
/// - `dev_password`: optional password string; if None this function will try `KANARI_PASSWORD` env var.
/// - `to`: optional recipient address; if None this function will use `active_address` from kanari config.
/// - `amount`: amount in KANARI (not Mist).
/// - `rpc_url`: RPC server URL.
/// - `wait_for_confirmation`: optional flag to wait for transaction confirmation (default: false).
///
/// Returns the `TransactionStatus` returned by the RPC server on success.
pub async fn request_from_dev(
    dev_address: Option<&str>,
    dev_password: Option<&str>,
    to: Option<&str>,
    amount: f64,
    rpc_url: &str,
) -> anyhow::Result<kanari_rpc_api::TransactionStatus> {
    // Determine dev address
    let dev_address = dev_address
        .map(|s| s.to_string())
        .unwrap_or_else(|| Address::DEV_ADDRESS.to_string());

    // Try loading workspace-local faucet .env first (useful when called from kanari root),
    // then fall back to normal dotenv search.
    if std::path::Path::new("crates/kanari-faucet/.env").exists() {
        let _ = dotenvy::from_filename("crates/kanari-faucet/.env");
    }
    dotenvy::dotenv().ok();

    // Determine password: prefer provided, else env var
    let password = if let Some(p) = dev_password {
        p.to_string()
    } else if let Ok(envp) = env::var("KANARI_PASSWORD") {
        envp
    } else {
        anyhow::bail!("Dev password not provided to library and KANARI_PASSWORD not set")
    };

    // Determine recipient with validation
    let recipient = if let Some(r) = to {
        // Validate recipient address format
        if !r.starts_with("0x") || r.len() != 66 {
            anyhow::bail!(
                "Invalid recipient address format: {}. Expected 0x-prefixed 64 hex characters",
                r
            );
        }
        r.to_string()
    } else if let Some(m) = get_main_wallet() {
        m
    } else {
        anyhow::bail!("No recipient specified and no active_address configured")
    };

    // Load wallet
    let wallet = kanari_crypto::wallet::load_wallet(&dev_address, &password)
        .context("Failed to load Dev wallet; check address and password")?;

    // RPC client
    let client = RpcClient::new(rpc_url);
    client
        .get_block_height()
        .await
        .context("Cannot connect to RPC server")?;

    // Check if dev account exists on-chain
    let owner = match client.get_owner(&dev_address).await {
        Ok(acct) => acct,
        Err(e) => {
            anyhow::bail!(
                "Dev wallet owner state not found on-chain: {}\n\
                 The dev wallet must be initialized first by receiving a transaction.\n\
                 Error: {}",
                dev_address,
                e
            );
        }
    };

    // Check dev wallet balance before transfer
    const MIST_PER_KANARI: f64 = 1_000_000_000.0;
    let amount_mist = (amount * MIST_PER_KANARI).round() as u64;

    // Estimate gas cost (transfer typically costs ~1000 mist/gas * 100000 gas = 100M mist = 0.1 KANARI)
    let estimated_gas_cost = 100_000 * 1000;
    let total_required = amount_mist + estimated_gas_cost;
    let native_balance = owner.balances.get(KANARI_TOKEN_TYPE).copied().unwrap_or(0);

    if native_balance < total_required {
        anyhow::bail!(
            "Insufficient balance in dev wallet: {} KANARI (required: {} KANARI)\n\
             Available: {:.9} KANARI\n\
             Required: {:.9} KANARI (transfer: {:.9} + gas: {:.9})",
            native_balance as f64 / MIST_PER_KANARI,
            total_required as f64 / MIST_PER_KANARI,
            native_balance as f64 / MIST_PER_KANARI,
            total_required as f64 / MIST_PER_KANARI,
            amount_mist as f64 / MIST_PER_KANARI,
            estimated_gas_cost as f64 / MIST_PER_KANARI
        );
    }

    // Re-derive keypair from private key to get the tagged address format
    // Tagged addresses are required for signature verification
    let keypair =
        kanari_crypto::keys::keypair_from_private_key(&wallet.private_key, wallet.curve_type)
            .context("Failed to derive public key from wallet")?;
    let sender_for_tx = keypair.tagged_address();

    let owned_objects = owner
        .owned_objects
        .as_ref()
        .context("Dev wallet owner state has no owned object list from RPC")?;
    let mut selected_coin = select_native_coin_object(owned_objects, total_required)
            .context("No spendable native coin object found in dev wallet")?;
    let mut next_sequence = owner.sequence_number;
    if selected_coin.1 < total_required {
        if selected_coin.2 < total_required {
            anyhow::bail!(
                "No single native coin object in the dev wallet can cover this faucet transfer: required {} Mist, best coin object {} Mist, total object-backed balance {} Mist",
                total_required,
                selected_coin.1,
                selected_coin.2
            );
        }

        eprintln!("  Consolidating dev wallet coin objects before faucet transfer...");
        selected_coin = consolidate_native_coins(
            &client,
            &wallet,
            &sender_for_tx,
            owned_objects,
            total_required,
            owner.sequence_number,
            100_000,
            1_000,
        )
        .await
        .map(|(id, selected_balance, total_balance, sequence)| {
            next_sequence = sequence;
            (id, selected_balance, total_balance)
        })?;
    }
    let (coin_object_id, selected_coin_balance, _total_coin_balance) = selected_coin;

    let call_req = CallFunctionRequest {
        sender: sender_for_tx,
        package: "0x2".to_string(),
        module: "kanari".to_string(),
        function: "transfer".to_string(),
        type_args: vec![],
        args: vec![
            move_core_types::account_address::AccountAddress::from_hex_literal(&coin_object_id)
                .context("Invalid coin object ID")?
                .to_vec(),
            bcs::to_bytes(&amount_mist).context("Failed to serialize amount")?,
            bcs::to_bytes(
                &move_core_types::account_address::AccountAddress::from_hex_literal(&recipient)?,
            )
            .context("Failed to serialize recipient address")?,
        ],
        gas_limit: 100_000,
        gas_price: 1_000,
        sequence_number: next_sequence,
        signature: None,
        execute_immediate: Some(false),
    };

    eprintln!("Submitting faucet transaction...");
    eprintln!("  From: {}", dev_address);
    eprintln!("  To: {}", recipient);
    eprintln!("  Coin Object: {}", coin_object_id);
    eprintln!("  Selected Coin Balance: {} Mist", selected_coin_balance);
    eprintln!(
        "  Amount: {:.9} KANARI",
        amount_mist as f64 / MIST_PER_KANARI
    );

    let status = sign_and_call_function(&client, &wallet, call_req)
        .await
        .context("Failed to submit transaction to RPC")?;

    eprintln!("  Transaction hash: {}", status.hash);
    eprintln!("  Status: {}", status.status);

    Ok(status)
}
