// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Library helpers for the kanari-faucet crate.
//!
//! Exposes an async function `request_from_dev` that other crates (for example
//! the `kanari` binary) can call to request tokens from the Dev wallet.

use anyhow::{Context, ensure};
use kanari_common::get_main_wallet;
use kanari_crypto::wallet::Wallet;
use kanari_rpc_api::{
    BuildNativeCoinConsolidationRequest, BuildNativeTransferRequest, CallFunctionRequest,
    ObjectTransferData, TransactionErrorReason, TransactionStatus,
};
use kanari_rpc_client::{
    RpcClient, transaction_error_details as rpc_transaction_error_details,
    transaction_error_reason as rpc_transaction_error_reason,
};
use kanari_types::{
    address::Address,
    transaction::{SignedTransaction, Transaction},
};
use std::env;
use std::thread;
use std::time::Duration;

fn sign_object_transfer_request(
    mut request: ObjectTransferData,
    wallet: &Wallet,
) -> anyhow::Result<ObjectTransferData> {
    ensure!(
        request
            .signature
            .as_ref()
            .map(|sig| sig.is_empty())
            .unwrap_or(true),
        "Refusing to overwrite existing object-transfer signature"
    );

    let coin_object_ref = request
        .coin_object_ref
        .clone()
        .context("coin_object_ref is required to sign faucet transfer transaction")?;

    let mut transaction = Transaction::new_transfer_with_object_ref_and_gas(
        request.sender.clone(),
        coin_object_ref,
        request.recipient.clone(),
        request.amount,
        request.canonical_nonce().map_err(anyhow::Error::msg)?,
        request.gas_limit,
        request.gas_price,
    );
    if let Transaction::ExecuteFunction { gas_payment, .. } = &mut transaction
        && request.gas_payment.is_some()
    {
        *gas_payment = request.gas_payment.clone();
    }

    let mut signed_tx = SignedTransaction::new(transaction);
    let signing_curve = wallet
        .validated_signing_curve()
        .context("Faucet wallet private key does not match its stored curve/address")?;
    signed_tx
        .sign(&wallet.private_key, signing_curve)
        .context("Failed to sign faucet transfer transaction")?;
    request.signature = Some(signed_tx.signature);
    Ok(request)
}

fn sign_call_function_request(
    mut request: CallFunctionRequest,
    wallet: &Wallet,
) -> anyhow::Result<CallFunctionRequest> {
    ensure!(
        request
            .signature
            .as_ref()
            .map(|sig| sig.is_empty())
            .unwrap_or(true),
        "Refusing to overwrite existing call-function signature"
    );

    let transaction = Transaction::ExecuteFunction {
        sender: request.sender.clone(),
        module: format!("{}::{}", request.package, request.module),
        function: request.function.clone(),
        type_args: request.type_args.clone(),
        args: request.args.clone(),
        object_inputs: request.object_inputs.clone().unwrap_or_default(),
        gas_payment: request.gas_payment.clone(),
        gas_limit: request.gas_limit,
        gas_price: request.gas_price,
        nonce: request.canonical_nonce().map_err(anyhow::Error::msg)?,
    };

    let mut signed_tx = SignedTransaction::new(transaction);
    let signing_curve = wallet
        .validated_signing_curve()
        .context("Faucet wallet private key does not match its stored curve/address")?;
    signed_tx
        .sign(&wallet.private_key, signing_curve)
        .context("Failed to sign faucet call-function transaction")?;
    request.signature = Some(signed_tx.signature);
    Ok(request)
}

async fn sign_and_call_function(
    client: &RpcClient,
    wallet: &Wallet,
    request: CallFunctionRequest,
) -> anyhow::Result<TransactionStatus> {
    let signed_request = sign_call_function_request(request, wallet)?;
    client
        .call_function(signed_request)
        .await
        .context("Failed to submit faucet call-function transaction")
}

async fn wait_for_transaction_commit(
    client: &RpcClient,
    tx_hash: &str,
    timeout: Duration,
    poll_interval: Duration,
) -> anyhow::Result<TransactionStatus> {
    let start = std::time::Instant::now();
    loop {
        let status = client.get_transaction(tx_hash).await?;
        if status.committed || status.status.eq_ignore_ascii_case("committed") {
            return Ok(TransactionStatus {
                hash: status.hash,
                status: status.status,
                block_height: status.block_height,
                gas_used: status.gas_used,
                success: status.success,
                previewed: status.previewed,
                submitted: status.submitted,
                committed: status.committed,
            });
        }

        if start.elapsed() >= timeout {
            anyhow::bail!(
                "Timed out waiting for faucet transaction {} to commit",
                tx_hash
            );
        }

        thread::sleep(poll_interval);
    }
}

fn should_attempt_native_consolidation(error: &anyhow::Error) -> bool {
    matches!(
        rpc_transaction_error_reason(error),
        Some(
            TransactionErrorReason::InsufficientNativeCoinObjects
                | TransactionErrorReason::InsufficientTransferCoinBalance
                | TransactionErrorReason::InsufficientGasCoinBalance
                | TransactionErrorReason::NativeTransferPolicyNotSatisfied
        )
    )
}

fn native_transfer_policy_summary(error: &anyhow::Error) -> Option<String> {
    rpc_transaction_error_details(error)
        .and_then(|details| details.native_transfer_policy)
        .map(|policy| policy.summary().to_string())
}

/// Request KANARI tokens from the Dev wallet.
///
/// Parameters:
/// - `dev_address`: optional dev address hex string (if None, uses `Address::DEV_ADDRESS`).
/// - `dev_password`: optional password string; if None this function will try `KANARI_PASSWORD` env var.
/// - `to`: optional recipient address; if None this function will use `active_address` from kanari config.
/// - `amount`: decimal KANARI amount (e.g. `"12.5"`). Parsed with exact
///   integer math — never float, so large amounts keep every Mist.
/// - `rpc_url`: RPC server URL.
///
/// Returns the `TransactionStatus` returned by the RPC server on success.
pub async fn request_from_dev(
    dev_address: Option<&str>,
    dev_password: Option<&str>,
    to: Option<&str>,
    amount: &str,
    rpc_url: &str,
) -> anyhow::Result<TransactionStatus> {
    let amount_mist = kanari_types::gas_coin::GasModule::parse_kanari_to_mist(amount)
        .map_err(|error| anyhow::anyhow!("Invalid faucet amount: {error}"))?;
    request_from_dev_mist(dev_address, dev_password, to, amount_mist, rpc_url).await
}

/// Faucet drip in integer Mist. Splits the drip into two coin objects so the
/// recipient always holds at least two distinct coins (native transfers
/// require one transfer coin plus a SEPARATE gas coin).
pub async fn request_from_dev_mist(
    dev_address: Option<&str>,
    dev_password: Option<&str>,
    to: Option<&str>,
    amount_mist: u64,
    rpc_url: &str,
) -> anyhow::Result<TransactionStatus> {
    let dev_address = dev_address
        .map(|s| s.to_string())
        .unwrap_or_else(|| Address::DEV_ADDRESS.to_string());

    if std::path::Path::new("crates/kanari-faucet/.env").exists() {
        let _ = dotenvy::from_filename("crates/kanari-faucet/.env");
    }
    dotenvy::dotenv().ok();

    let password = if let Some(p) = dev_password {
        p.to_string()
    } else if let Ok(envp) = env::var("KANARI_PASSWORD") {
        envp
    } else {
        anyhow::bail!("Dev password not provided to library and KANARI_PASSWORD not set")
    };

    let recipient = if let Some(r) = to {
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

    let wallet = kanari_crypto::wallet::load_wallet(&dev_address, &password)
        .context("Failed to load Dev wallet; check address and password")?;

    let client = RpcClient::new(rpc_url);
    client
        .get_block_height()
        .await
        .context("Cannot connect to RPC server")?;

    let signing_curve = wallet
        .validated_signing_curve()
        .context("Faucet wallet private key does not match its stored curve/address")?;
    let keypair = kanari_crypto::keys::keypair_from_private_key(&wallet.private_key, signing_curve)
        .context("Failed to derive public key from wallet")?;
    let sender_for_tx = keypair.tagged_address();
    let gas_limit = 100_000;
    let gas_price = 1_000;

    // Split the drip into two coin objects so the recipient always holds at
    // least two distinct coins: native transfers require one transfer coin
    // plus a SEPARATE gas coin, and a single-coin wallet can never send.
    let drip_parts: Vec<u64> = if amount_mist >= 2 {
        let half = amount_mist / 2;
        vec![half, amount_mist - half]
    } else {
        vec![amount_mist]
    };
    let mut last_status: Option<TransactionStatus> = None;
    for (leg, part) in drip_parts.iter().enumerate() {
        eprintln!(
            "Faucet drip {}/{}: {} KANARI",
            leg + 1,
            drip_parts.len(),
            kanari_types::gas_coin::GasModule::format_mist_to_kanari(*part)
        );
        let prepared = {
            let mut last_build_error = None;
            let mut built_transfer = None;

            for step in 0..8 {
                match client
                    .build_native_transfer(BuildNativeTransferRequest {
                        sender: sender_for_tx.clone(),
                        recipient: recipient.clone(),
                        amount: *part,
                        gas_limit,
                        gas_price,
                        excluded_object_ids: Vec::new(),
                        nonce: None,
                        execute_immediate: Some(false),
                    })
                    .await
                {
                    Ok(prepared) => {
                        built_transfer = Some(prepared);
                        break;
                    }
                    Err(error) => {
                        if !should_attempt_native_consolidation(&error) {
                            let policy_suffix = native_transfer_policy_summary(&error)
                                .map(|summary| format!(" Policy: {}.", summary))
                                .unwrap_or_default();
                            return Err(anyhow::anyhow!(
                                "Failed to build faucet transfer for sender {}: {:#}{}",
                                dev_address,
                                error,
                                policy_suffix
                            ));
                        }
                        last_build_error = Some(error);
                    }
                }

                let consolidate_request = BuildNativeCoinConsolidationRequest {
                    sender: sender_for_tx.clone(),
                    required_amount: *part,
                    gas_limit,
                    gas_price,
                    nonce: None,
                    execute_immediate: Some(false),
                };
                let join_request = match client
                    .build_native_coin_consolidation(consolidate_request)
                    .await
                {
                    Ok(request) => request,
                    Err(join_error) => {
                        let build_error = last_build_error
                            .take()
                            .context("missing previous native transfer build failure")?;
                        let policy_suffix = native_transfer_policy_summary(&build_error)
                            .map(|summary| format!(" Policy: {}.", summary))
                            .unwrap_or_default();
                        return Err(anyhow::anyhow!(
                            "Failed to build faucet transfer for sender {}. API reported a native-transfer coin-layout issue and no safe consolidation step is available: {:#}; consolidation error: {:#}{}",
                            dev_address,
                            build_error,
                            join_error,
                            policy_suffix
                        ));
                    }
                };

                eprintln!(
                    "  Consolidation step {}/8: joining native coin objects via API...",
                    step + 1
                );
                let status = sign_and_call_function(&client, &wallet, join_request).await?;
                eprintln!("    Join tx hash: {}", status.hash);
                let _ = wait_for_transaction_commit(
                    &client,
                    &status.hash,
                    Duration::from_secs(20),
                    Duration::from_millis(400),
                )
                .await?;
            }

            built_transfer.ok_or_else(|| {
            let message = if let Some(error) = last_build_error {
                format!(
                    "Failed to build faucet transfer after API-driven coin-shape preparation attempts: {:#}",
                    error
                )
            } else {
                "Failed to build faucet transfer after API-driven coin-shape preparation attempts"
                    .to_string()
            };
            anyhow::anyhow!(message)
        })?
        };

        eprintln!("Submitting faucet transaction...");
        eprintln!("  From: {}", dev_address);
        eprintln!("  To: {}", recipient);
        eprintln!("  Coin Object: {}", prepared.coin_object_id);
        if let Some(gas_payment) = &prepared.gas_payment
            && let Some(gas_object) = gas_payment.payment_objects.first()
        {
            eprintln!("  Gas payment object: {}", gas_object.object_id);
        }
        eprintln!(
            "  Amount: {} KANARI",
            kanari_types::gas_coin::GasModule::format_mist_to_kanari(*part)
        );

        let signed = sign_object_transfer_request(prepared, &wallet)?;
        let status = client
            .submit_object_transfer(signed)
            .await
            .context("Failed to submit faucet transaction to RPC")?;

        eprintln!("  Transaction hash: {}", status.hash);
        eprintln!("  Status: {}", status.status);
        // Wait for commit before building the next drip: both drips draw
        // from the same faucet wallet, and building on pre-commit state
        // would select the same coins twice (version conflict on execution).
        if leg + 1 < drip_parts.len()
            && let Err(error) = wait_for_transaction_commit(
                &client,
                &status.hash,
                Duration::from_secs(30),
                Duration::from_millis(400),
            )
            .await
        {
            eprintln!(
                "  Warning: drip {}/{} commit wait failed (continuing): {:#}",
                leg + 1,
                drip_parts.len(),
                error
            );
        }
        last_status = Some(status);
    }

    last_status.context("Faucet drip produced no transactions")
}
