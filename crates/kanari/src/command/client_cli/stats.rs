// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use clap::Parser;
use kanari_rpc_client::RpcClient;
use log::error;

#[derive(Parser, Debug)]
pub struct Stats {
    /// RPC endpoint
    #[clap(long = "rpc", default_value = "http://127.0.0.1:19001")]
    pub rpc_endpoint: String,
}

impl Stats {
    pub async fn execute(&self) -> Result<()> {
        let client = RpcClient::new(&self.rpc_endpoint);

        match client.get_stats().await {
            Ok(stats) => {
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
                eprintln!("─────────────────────────────────");
            }
            Err(_) => {
                error!("  Cannot connect to RPC server at {}", self.rpc_endpoint);
                error!("  Please start the node first: cargo run --bin kanari-node");
                return Err(anyhow::anyhow!("RPC server not available"));
            }
        }

        Ok(())
    }
}
