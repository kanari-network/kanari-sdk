// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::command::common::{
    check_node_connection, get_rpc_endpoint, get_sender_for_tx, load_wallet_for,
    native_transfer_policy_hint, normalize_addr, resolve_sender, resolve_transaction_gas,
};
use crate::command::rpc_helpers::{
    should_wait_for_commit, sign_object_transfer_request, wait_for_transaction_commit,
};
use crate::command::tx_output::print_transaction_status;
use anyhow::{Context, Result};
use clap::Parser;
use kanari_rpc_api::BuildNativeTransferRequest;
use kanari_rpc_client::RpcClient;
use std::time::Duration;

#[derive(Parser, Debug)]
pub struct Transfer {
    #[arg(short, long)]
    pub from: Option<String>,
    #[arg(short, long)]
    pub to: String,
    #[arg(short, long)]
    pub amount: f64,
    #[arg(short, long)]
    pub password: String,
    #[clap(long = "rpc")]
    pub rpc_endpoint: Option<String>,
}

impl Transfer {
    pub async fn execute(&self) -> Result<()> {
        let rpc = get_rpc_endpoint(self.rpc_endpoint.clone());
        let from_addr = resolve_sender(self.from.clone())?;
        let to_addr = normalize_addr(&self.to)?;
        let wallet = load_wallet_for(&from_addr, Some(self.password.clone()))?;
        let (gas_limit, gas_price) = resolve_transaction_gas(None, None);

        eprintln!("Transferring Kanari tokens...");
        eprintln!("  From: {}", from_addr);
        eprintln!("  To: {}", to_addr);
        eprintln!("  Amount: {} KANARI", self.amount);

        const MIST_PER_KANARI: f64 = 1_000_000_000.0;
        let amount_mist = (self.amount * MIST_PER_KANARI).round() as u64;
        eprintln!("  Amount (Mist): {}", amount_mist);

        let client = RpcClient::new(&rpc);
        check_node_connection(&client, &rpc).await?;

        let sender_tagged = get_sender_for_tx(&wallet, &from_addr)?;
        let prepared = client
            .build_native_transfer(BuildNativeTransferRequest {
                sender: sender_tagged.clone(),
                recipient: to_addr,
                amount: amount_mist,
                gas_limit,
                gas_price,
                execute_immediate: None,
            })
            .await
            .map_err(|error| {
                let policy_suffix = native_transfer_policy_hint(&error)
                    .map(|summary| format!(" Policy: {}.", summary))
                    .unwrap_or_default();
                anyhow::anyhow!(
                    "Failed to build native transfer transaction for sender {}: {:#}{}",
                    from_addr,
                    error,
                    policy_suffix
                )
            })?;

        eprintln!("  Using coin object: {}", prepared.coin_object_id);
        if let Some(gas_payment) = &prepared.gas_payment
            && let Some(gas_object) = gas_payment.payment_objects.first()
        {
            eprintln!("  Gas payment object: {}", gas_object.object_id);
        }
        eprintln!("  Gas Limit: {}", prepared.gas_limit);
        eprintln!("  Gas Price: {} Mist/gas", prepared.gas_price);

        let signed = sign_object_transfer_request(prepared, &wallet)?;
        eprintln!("  Transaction signed");
        eprintln!("  Submitting transaction to node...");

        let status = client
            .submit_object_transfer(signed)
            .await
            .context("Failed to submit transaction")?;
        print_transaction_status("  ", &status);
        if should_wait_for_commit(
            status.success,
            status.previewed,
            status.submitted,
            status.committed,
        ) {
            eprintln!("  Waiting for transaction commit...");
            let committed = wait_for_transaction_commit(
                &client,
                &status.hash,
                Duration::from_secs(20),
                Duration::from_millis(400),
            )
            .await?;
            print_transaction_status("  Final: ", &committed);
        }

        Ok(())
    }
}
