// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Library helpers for the kanari-faucet crate.
//!
//! Exposes an async function `request_from_dev` that other crates (for example
//! the `kanari` binary) can call to request tokens from the Dev wallet.

use anyhow::Context;
use kanari_common::get_main_wallet;
use kanari_rpc_api::SignedTransactionData;
use kanari_rpc_client::RpcClient;
use kanari_types::{
    address::Address,
    transaction::{SignedTransaction, Transaction},
};
use std::env;

/// Request KANARI tokens from the Dev wallet.
///
/// Parameters:
/// - `dev_address`: optional dev address hex string (if None, uses `Address::DEV_ADDRESS`).
/// - `dev_password`: optional password string; if None this function will try `KANARI_PASSWORD` env var.
/// - `to`: optional recipient address; if None this function will use `active_address` from kanari config.
/// - `amount`: amount in KANARI (not Mist).
/// - `rpc_url`: RPC server URL.
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

    // Determine recipient
    let recipient = if let Some(r) = to {
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

    let account = client
        .get_account(&dev_address)
        .await
        .context("Failed to fetch Dev account info")?;

    // Convert amount to Mist
    const MIST_PER_KANARI: f64 = 1_000_000_000.0;
    let amount_mist = (amount * MIST_PER_KANARI).round() as u64;

    // If it's a PQC or Hybrid wallet, use the tagged address (Curve:PublicKey)
    // for signing and sender identity to ensure the RPC server can verify it.
    let sender_for_tx = if wallet.curve_type.is_post_quantum() || wallet.curve_type.is_hybrid() {
        // Re-derive public key from private key to get the tagged address format
        let keypair =
            kanari_crypto::keys::keypair_from_private_key(&wallet.private_key, wallet.curve_type)
                .context("Failed to derive public key from wallet")?;
        format!("{:?}:{}", wallet.curve_type, keypair.public_key)
    } else {
        dev_address.clone()
    };

    let tx = Transaction::Transfer {
        from: sender_for_tx.clone(),
        to: recipient.clone(),
        amount: amount_mist,
        gas_limit: 100_000,
        gas_price: 1000,
        sequence_number: account.sequence_number,
    };

    let mut signed_tx = SignedTransaction::new(tx);
    signed_tx
        .sign(&wallet.private_key, wallet.curve_type)
        .context("Failed to sign transaction with Dev wallet")?;

    let tx_data = SignedTransactionData {
        sender: sender_for_tx,
        recipient: Some(recipient),
        amount: Some(amount_mist),
        gas_limit: signed_tx.transaction.gas_limit(),
        gas_price: signed_tx.transaction.gas_price(),
        sequence_number: account.sequence_number,
        signature: Some(signed_tx.signature.clone()),
    };

    let status = client
        .submit_transaction(tx_data)
        .await
        .context("Failed to submit transaction to RPC")?;

    Ok(status)
}
