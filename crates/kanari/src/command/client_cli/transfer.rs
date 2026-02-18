// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::command::common::{
    check_node_connection, get_rpc_endpoint, get_sender_for_tx, load_wallet_for, normalize_addr,
    resolve_sender, sign_and_submit_transaction,
};
use anyhow::{Context, Result};
use clap::Parser;
use kanari_rpc_client::RpcClient;
use kanari_types::transaction::Transaction;

#[derive(Parser, Debug)]
pub struct Transfer {
    /// Sender wallet address (optional). If omitted, uses selected wallet in config.
    #[arg(short, long)]
    pub from: Option<String>,
    /// Recipient address
    #[arg(short, long)]
    pub to: String,
    /// Amount in Kanari (will be converted to Mist)
    #[arg(short, long)]
    pub amount: f64,
    /// Wallet password
    #[arg(short, long)]
    pub password: String,

    /// RPC endpoint
    #[clap(long = "rpc")]
    pub rpc_endpoint: Option<String>,
}

impl Transfer {
    pub async fn execute(&self) -> Result<()> {
        let rpc = get_rpc_endpoint(self.rpc_endpoint.clone());
        let from_addr = resolve_sender(self.from.clone())?;
        let to_addr = normalize_addr(&self.to)?;
        let wallet = load_wallet_for(&from_addr, Some(self.password.clone()))?;

        eprintln!("Transferring Kanari tokens...");
        eprintln!("  From: {}", from_addr);
        eprintln!("  To: {}", to_addr);
        eprintln!("  Amount: {} KANARI", self.amount);

        // Convert Kanari to Mist (1 KANARI = 10^9 Mist)
        const MIST_PER_KANARI: f64 = 1_000_000_000.0;
        let amount_mist = (self.amount * MIST_PER_KANARI).round() as u64;
        eprintln!("  Amount (Mist): {}", amount_mist);

        // Connect to RPC server
        let client = RpcClient::new(&rpc);
        check_node_connection(&client, &rpc).await?;

        // Get account to get sequence number
        let account = client
            .get_account(&from_addr)
            .await
            .context("Failed to get sender account")?;

        let sender_tagged = get_sender_for_tx(&wallet, &from_addr)?;

        let tx = Transaction::Transfer {
            from: sender_tagged.clone(),
            to: to_addr.clone(),
            amount: amount_mist,
            gas_limit: 100_000,
            gas_price: 1000,
            sequence_number: account.sequence_number,
        };

        sign_and_submit_transaction(
            &client,
            tx,
            &wallet,
            sender_tagged,
            Some(to_addr),
            Some(amount_mist),
        )
        .await?;

        Ok(())
    }
}
