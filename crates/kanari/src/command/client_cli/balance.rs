// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::command::common::{get_rpc_endpoint, resolve_sender};
use anyhow::{Context, Result};
use clap::*;
use kanari_rpc_client::RpcClient;

/// Show token balances for an owner address
#[derive(Parser, Debug)]
#[clap(name = "balances")]
pub struct Balance {
    /// Owner address to query
    #[clap(long = "owner")]
    pub owner: Option<String>,

    /// RPC endpoint URL
    #[clap(long = "rpc")]
    pub rpc_endpoint: Option<String>,

    /// Show detailed information
    #[clap(long = "detailed", short = 'd')]
    pub detailed: bool,
}

impl Balance {
    pub async fn execute(&self) -> Result<()> {
        let rpc = get_rpc_endpoint(self.rpc_endpoint.clone());
        let owner = resolve_sender(self.owner.clone())?;

        eprintln!("Querying owner token balances...");
        eprintln!("   Owner: {}", owner);
        eprintln!("   RPC: {}\n", rpc);

        let client = RpcClient::new(&rpc);
        let result = client
            .get_owner_balances(&owner)
            .await
            .context("Failed to fetch owner balances")?;

        {
            if let Some(balances) = result.get("balances").and_then(|b| b.as_array()) {
                eprintln!("TOKEN BALANCES");
                eprintln!("------------------------------");

                for balance in balances {
                    let token_type = balance
                        .get("token_type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("UNKNOWN");

                    let amount = balance.get("balance").and_then(|a| a.as_u64()).unwrap_or(0);

                    // No fallback: decimals must come from API metadata.
                    let decimals_opt = balance.get("decimals").and_then(|d| d.as_u64());

                    if self.detailed {
                        eprintln!("Token Type: {}", token_type);
                        match decimals_opt {
                            Some(decimals) => {
                                let divisor = 10u64.pow(decimals as u32);
                                let whole = amount / divisor;
                                let fraction = amount % divisor;
                                eprintln!(
                                    "  Balance: {}.{:0width$} ({})",
                                    whole,
                                    fraction,
                                    token_type,
                                    width = decimals as usize
                                );
                            }
                            None => {
                                eprintln!("  Balance: {} (raw, decimals unknown)", amount);
                            }
                        }
                        eprintln!("  Raw Amount: {}", amount);

                        // Display metadata if available
                        if let Some(name) = balance.get("name").and_then(|n| n.as_str()) {
                            eprintln!("  Name: {}", name);
                        }
                        if let Some(symbol) = balance.get("symbol").and_then(|s| s.as_str()) {
                            eprintln!("  Symbol: {}", symbol);
                        }
                        if let Some(description) =
                            balance.get("description").and_then(|d| d.as_str())
                        {
                            eprintln!("  Description: {}", description);
                        }
                        if let Some(icon_url) = balance.get("icon_url").and_then(|i| i.as_str()) {
                            eprintln!("  Icon URL: {}", icon_url);
                        }

                        eprintln!("------------------------------");
                    } else {
                        match decimals_opt {
                            Some(decimals) => {
                                let divisor = 10u64.pow(decimals as u32);
                                let whole = amount / divisor;
                                let fraction = amount % divisor;
                                eprintln!(
                                    "  {} {}.{:0width$}",
                                    token_type,
                                    whole,
                                    fraction,
                                    width = decimals as usize
                                );
                            }
                            None => {
                                eprintln!("  {} {} (raw, decimals unknown)", token_type, amount);
                            }
                        }
                    }
                }
                eprintln!("\nTotal tokens: {}", balances.len());
            } else {
                eprintln!("No balances found");
            }
        }

        Ok(())
    }
}
