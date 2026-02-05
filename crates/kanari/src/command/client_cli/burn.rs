// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use clap::Parser;
use kanari_crypto::wallet::load_wallet;
use kanari_rpc_api::SignedTransactionData;
use kanari_rpc_client::RpcClient;
use kanari_types::transaction::{SignedTransaction, Transaction};
use log::error;

#[derive(Parser, Debug)]
pub struct Burn {
    /// Wallet address to burn from (optional). If omitted, uses selected wallet in config.
    #[arg(short, long)]
    pub from: Option<String>,
    /// Amount in Kanari to burn
    #[arg(short, long)]
    pub amount: f64,
    /// Wallet password
    #[arg(short, long)]
    pub password: String,

    /// RPC endpoint
    #[clap(long = "rpc")]
    pub rpc_endpoint: Option<String>,
}

impl Burn {
    pub async fn execute(&self) -> Result<()> {
        let rpc = self
            .rpc_endpoint
            .clone()
            .or_else(kanari_common::get_active_rpc)
            .unwrap_or_else(|| "http://127.0.0.1:19001".to_string());

        // Determine sender: prefer explicit `--from`, otherwise use selected wallet
        let from_addr = if let Some(f) = self.from.clone() {
            f
        } else {
            kanari_crypto::wallet::get_selected_wallet().ok_or_else(|| {
                anyhow::anyhow!("No sender provided and no selected wallet set. Use --from or run `kanari keytool load-wallet` to select one.")
            })?
        };

        let wallet =
            load_wallet(&from_addr, &self.password).context("Failed to load sender wallet")?;

        eprintln!("Burning Kanari tokens...");
        eprintln!("  From: {}", from_addr);
        eprintln!("  Amount: {} KANARI", self.amount);

        // Convert Kanari to Mist (1 KANARI = 10^9 Mist)
        const MIST_PER_KANARI: f64 = 1_000_000_000.0;
        let amount_mist_f = self.amount * MIST_PER_KANARI;
        let amount_mist = amount_mist_f.round() as u64;
        eprintln!("  Amount (Mist): {}", amount_mist);

        // Connect to RPC server
        let client = RpcClient::new(&rpc);

        match client.get_block_height().await {
            Ok(height) => eprintln!("  Connected to node (height: {})", height),
            Err(_) => {
                error!("  Cannot connect to RPC server at {}", rpc);
                error!("  Please start the node first: cargo run --bin kanari-node");
                return Err(anyhow::anyhow!("RPC server not available"));
            }
        }

        // Get account to get sequence number
        let account = client
            .get_account(&from_addr)
            .await
            .context("Failed to get sender account")?;

        // Create burn transaction
        let tx = Transaction::Burn {
            from: from_addr.clone(),
            amount: amount_mist,
            gas_limit: 100_000,
            gas_price: 1000,
            sequence_number: account.sequence_number,
        };

        eprintln!("  Gas Limit: {}", tx.gas_limit());
        eprintln!("  Gas Price: {} Mist/gas", tx.gas_price());

        // Sign transaction
        let mut signed_tx = SignedTransaction::new(tx);
        signed_tx
            .sign(&wallet.private_key, wallet.curve_type)
            .context("Failed to sign transaction")?;
        eprintln!("  Transaction signed");

        eprintln!("  Submitting burn transaction to node...");

        let tx_data = SignedTransactionData {
            sender: from_addr.clone(),
            recipient: None,
            amount: Some(amount_mist),
            gas_limit: signed_tx.transaction.gas_limit(),
            gas_price: signed_tx.transaction.gas_price(),
            sequence_number: account.sequence_number,
            signature: Some(signed_tx.signature.clone()),
        };

        match client.submit_transaction(tx_data).await {
            Ok(status) => {
                eprintln!("  Burn transaction submitted successfully");
                eprintln!("  Transaction hash: {}", status.hash);
                eprintln!("  Status: {}", status.status);
                eprintln!("  Waiting for block confirmation...");
            }
            Err(e) => {
                error!("  Failed to submit burn transaction: {}", e);
                return Err(e);
            }
        }

        Ok(())
    }
}
