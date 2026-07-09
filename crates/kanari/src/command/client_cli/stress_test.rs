// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Stress test command: faucet tokens then send N transfer transactions.
//!
//! Usage:
//! ```bash
//! kanari client stress-test --to 0x... --amount 0.1 --faucet --count 1000
//! ```

use crate::command::common::{
    get_rpc_endpoint, get_sender_for_tx, load_wallet_for, normalize_addr, resolve_sender,
};
use anyhow::{Context, Result};
use clap::Parser;
use kanari_rpc_client::RpcClient;
use kanari_types::{
    kanari::KANARI_TOKEN_TYPE,
    transaction::{SignedTransaction, Transaction},
};
use std::time::Instant;

#[derive(Parser, Debug)]
pub struct StressTest {
    /// Sender wallet address (optional). If omitted, uses selected wallet in config.
    #[arg(short, long)]
    pub from: Option<String>,

    /// Recipient address
    #[arg(short, long)]
    pub to: String,

    /// Amount in Kanari per transaction (will be converted to Mist)
    #[arg(short, long)]
    pub amount: f64,

    /// Number of transactions to send (default: 1000)
    #[arg(short, long, default_value_t = 1000)]
    pub count: u64,

    /// Wallet password
    #[arg(short, long)]
    pub password: String,

    /// Request tokens from dev faucet first (funds the sender)
    #[arg(long)]
    pub faucet: bool,

    /// Faucet amount in Kanari (default: amount * count * 2 to cover all txs + gas)
    #[arg(long)]
    pub faucet_amount: Option<f64>,

    /// Dev faucet address override
    #[arg(long)]
    pub dev_address: Option<String>,

    /// Dev faucet password override
    #[arg(long)]
    pub dev_password: Option<String>,

    /// RPC endpoint
    #[clap(long = "rpc")]
    pub rpc_endpoint: Option<String>,
}

impl StressTest {
    pub async fn execute(&self) -> Result<()> {
        let rpc = get_rpc_endpoint(self.rpc_endpoint.clone());
        let from_addr = resolve_sender(self.from.clone())?;
        let to_addr = normalize_addr(&self.to)?;
        let wallet = load_wallet_for(&from_addr, Some(self.password.clone()))?;

        const MIST_PER_KANARI: f64 = 1_000_000_000.0;
        let amount_mist = (self.amount * MIST_PER_KANARI).round() as u64;
        if amount_mist == 0 {
            anyhow::bail!("--amount is too small; it rounds to 0 Mist");
        }

        eprintln!("=== Kanari Stress Test ===");
        eprintln!("  From:     {}", from_addr);
        eprintln!("  To:       {}", to_addr);
        eprintln!("  Per-tx:   {} KANARI ({} Mist)", self.amount, amount_mist);
        eprintln!("  Count:    {} transactions", self.count);

        let client = RpcClient::new(&rpc);

        // ── Step 1: Check node connection ──
        {
            let height = client
                .get_block_height()
                .await
                .context("Cannot connect to RPC server")?;
            eprintln!("  Node height: {}", height);
        }

        // ── Step 2: Faucet (fund sender) ──
        if self.faucet {
            // Give a generous 10x cushion — 2x barely covers gas + amount
            // for a single run, and leftover is needed for subsequent runs.
            let faucet_amt = self
                .faucet_amount
                .unwrap_or((self.amount * self.count as f64) * 10.0);
            eprintln!();
            eprintln!("--- Faucet: Requesting {:.9} KANARI ---", faucet_amt);
            match kanari_faucet::request_from_dev(
                self.dev_address.as_deref(),
                self.dev_password.as_deref(),
                Some(&from_addr),
                faucet_amt,
                &rpc,
            )
            .await
            {
                Ok(_) => {
                    eprintln!("  Faucet done. Waiting 2s for node to process...");
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                Err(e) => {
                    eprintln!("  Faucet warning (continuing anyway): {}", e);
                }
            }
        }

        // ── Step 3: Get current account state ──
        let account = client
            .get_account(&from_addr)
            .await
            .context("Failed to get sender account")?;

        // Check balance before starting
        let native_balance = account
            .token_balances
            .get(KANARI_TOKEN_TYPE)
            .copied()
            .unwrap_or(0);
        let total_needed = amount_mist.saturating_mul(self.count);
        // Rough gas estimate: ~100k gas * 1000 price = 100M Mist per tx
        let gas_per_tx: u64 = 100_000 * 1000;
        let total_gas = gas_per_tx.saturating_mul(self.count);
        let total_required = total_needed.saturating_add(total_gas);

        eprintln!();
        eprintln!("--- Balance check ---");
        eprintln!(
            "  Native balance: {:.9} KANARI",
            native_balance as f64 / MIST_PER_KANARI
        );
        eprintln!(
            "  Transfer total: {:.9} KANARI",
            total_needed as f64 / MIST_PER_KANARI
        );
        eprintln!(
            "  Est. gas:       {:.9} KANARI",
            total_gas as f64 / MIST_PER_KANARI
        );
        eprintln!(
            "  Required:       {:.9} KANARI",
            total_required as f64 / MIST_PER_KANARI
        );

        if native_balance < total_required {
            anyhow::bail!(
                "Insufficient balance: have {:.9} KANARI, need {:.9} KANARI\n\
                 Use --faucet or request more tokens.",
                native_balance as f64 / MIST_PER_KANARI,
                total_required as f64 / MIST_PER_KANARI,
            );
        }

        let sender_tagged = get_sender_for_tx(&wallet, &from_addr)?;

        // ── Step 4: Send N transactions ──
        eprintln!();
        eprintln!("--- Sending {} transactions ---", self.count);

        let mut seq = account.sequence_number;
        let mut success_count = 0u64;
        let mut fail_count = 0u64;
        let start_time = Instant::now();
        let report_interval = (self.count / 20).max(1); // ~5% progress report

        for i in 0..self.count {
            let tx =
                Transaction::new_transfer(sender_tagged.clone(), to_addr.clone(), amount_mist, seq);

            let mut signed_tx = SignedTransaction::new(tx);
            signed_tx
                .sign(&wallet.private_key, wallet.curve_type)
                .context("Failed to sign transaction")?;

            let tx_data = kanari_rpc_api::SignedTransactionData {
                sender: sender_tagged.clone(),
                recipient: Some(to_addr.clone()),
                amount: Some(amount_mist),
                gas_limit: signed_tx.transaction.gas_limit(),
                gas_price: signed_tx.transaction.gas_price(),
                sequence_number: seq,
                signature: Some(signed_tx.signature.clone()),
                // false = go through mempool → DAG → P2P gossip to all nodes
                execute_immediate: Some(false),
            };

            match client.submit_transaction(tx_data).await {
                Ok(status) => {
                    success_count += 1;
                    if (i + 1) % report_interval == 0 || i + 1 == self.count {
                        let elapsed = start_time.elapsed();
                        let tps = if elapsed.as_secs_f64() > 0.0 {
                            success_count as f64 / elapsed.as_secs_f64()
                        } else {
                            0.0
                        };
                        eprintln!(
                            "  [{}/{}] hash={} seq={} tps={:.1}",
                            i + 1,
                            self.count,
                            &status.hash[..16.min(status.hash.len())],
                            seq,
                            tps,
                        );
                    }
                }
                Err(e) => {
                    fail_count += 1;
                    eprintln!("  [{}/{}] FAILED seq={}: {}", i + 1, self.count, seq, e);
                    // On first failure, try refreshing sequence number
                    if fail_count == 1 {
                        eprintln!("  Refreshing sequence number after failure...");
                        if let Ok(acct) = client.get_account(&from_addr).await {
                            if acct.sequence_number > seq {
                                seq = acct.sequence_number;
                                eprintln!("  Sequence bumped to {}", seq);
                                continue; // retry this transaction
                            }
                        }
                    }
                }
            }

            seq += 1;
        }

        let total_elapsed = start_time.elapsed();
        let total_ok = success_count;
        let total_fail = fail_count;

        eprintln!();
        eprintln!("=== Stress Test Complete ===");
        eprintln!("  Total:    {} tx", self.count);
        eprintln!("  Success:  {}", total_ok);
        eprintln!("  Failed:   {}", total_fail);
        eprintln!("  Duration: {:.2?}", total_elapsed);
        if total_elapsed.as_secs_f64() > 0.0 {
            eprintln!(
                "  TPS:      {:.1}",
                total_ok as f64 / total_elapsed.as_secs_f64()
            );
        }

        if total_fail > 0 {
            anyhow::bail!("{} of {} transactions failed", total_fail, self.count);
        }

        Ok(())
    }
}
