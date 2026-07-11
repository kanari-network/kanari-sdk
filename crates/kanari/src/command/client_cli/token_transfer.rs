// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::command::common::{
    check_node_connection, get_rpc_endpoint, get_sender_for_tx, load_wallet_for, normalize_addr,
    resolve_sender, resolve_transaction_gas,
};
use crate::command::rpc_helpers::{
    should_wait_for_commit, sign_and_call_function, wait_for_transaction_commit,
};
use crate::command::tx_output::print_transaction_status;
use anyhow::{Context, Result};
use clap::Parser;
use kanari_rpc_api::BuildTokenTransferRequest;
use kanari_rpc_client::RpcClient;
use std::time::Duration;

#[derive(Parser, Debug)]
pub struct TokenTransfer {
    #[arg(short, long)]
    pub from: Option<String>,
    #[arg(short, long)]
    pub to: String,
    #[arg(long)]
    pub token: String,
    #[arg(long)]
    pub amount: u64,
    #[arg(short, long)]
    pub password: String,
    #[clap(long = "rpc")]
    pub rpc_endpoint: Option<String>,
}

impl TokenTransfer {
    pub async fn execute(&self) -> Result<()> {
        let rpc = get_rpc_endpoint(self.rpc_endpoint.clone());
        let from_addr = resolve_sender(self.from.clone())?;
        let to_addr = normalize_addr(&self.to)?;
        let wallet = load_wallet_for(&from_addr, Some(self.password.clone()))?;
        let (gas_limit, gas_price) = resolve_transaction_gas(None, None);

        eprintln!(
            "Transferring {} units of {} token...",
            self.amount, self.token
        );
        eprintln!("  From: {}", from_addr);
        eprintln!("  To: {}", to_addr);

        let client = RpcClient::new(&rpc);
        check_node_connection(&client, &rpc).await?;
        let sender_tagged = get_sender_for_tx(&wallet, &from_addr)?;

        let prepared = client
            .build_token_transfer(BuildTokenTransferRequest {
                sender: sender_tagged.clone(),
                recipient: to_addr,
                token_type: self.token.clone(),
                amount: self.amount,
                gas_limit,
                gas_price,
                client_nonce: None,
                execute_immediate: None,
            })
            .await
            .context("Failed to build token transfer transaction")?;

        if let Some(object_inputs) = &prepared.object_inputs
            && let Some(primary) = object_inputs.first()
        {
            eprintln!("  Using coin object: {}", primary.object_ref.object_id);
        }
        if let Some(gas_payment) = &prepared.gas_payment
            && let Some(gas_object) = gas_payment.payment_objects.first()
        {
            eprintln!("  Gas payment object: {}", gas_object.object_id);
        }
        eprintln!("  Gas Limit: {}", prepared.gas_limit);
        eprintln!("  Gas Price: {} Mist/gas", prepared.gas_price);
        eprintln!("  Submitting transaction to node...");

        let status = sign_and_call_function(&client, &wallet, prepared)
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
