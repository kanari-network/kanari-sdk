// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::command::common::get_rpc_endpoint;
use anyhow::{Context, Result};
use clap::Parser;
use kanari_rpc_client::RpcClient;

#[derive(Parser, Debug)]
pub struct Stats {
    /// RPC endpoint
    #[clap(long = "rpc")]
    pub rpc_endpoint: Option<String>,
}

impl Stats {
    pub async fn execute(&self) -> Result<()> {
        let rpc = get_rpc_endpoint(self.rpc_endpoint.clone());

        let client = RpcClient::new(&rpc);

        let stats = client
            .get_stats()
            .await
            .with_context(|| format!("Failed to connect to RPC server at {}", rpc))?;

        const MIST_PER_KANARI: f64 = 1_000_000_000.0;
        let total_supply_kanari = stats.total_supply as f64 / MIST_PER_KANARI;

        eprintln!("Kanari Blockchain Statistics");
        eprintln!("------------------------------");
        eprintln!("  Block Height: {}", stats.height);
        eprintln!("  Total Blocks: {}", stats.total_blocks);
        eprintln!("  Total Transactions: {}", stats.total_transactions);
        eprintln!("  Pending Transactions: {}", stats.pending_transactions);
        eprintln!("  Total Accounts: {}", stats.total_accounts);
        eprintln!("  Total Supply: {:.0} KANARI", total_supply_kanari);
        eprintln!("----------------------------------------");

        Ok(())
    }
}
