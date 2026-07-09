// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::command::common::{get_rpc_endpoint, resolve_sender};
use anyhow::{Context, Result};
use clap::*;
use kanari_rpc_api::{GetAllBalancesRequest, RpcRequest, RpcResponse, methods};
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

        let _client = RpcClient::new(&rpc);

        let request = GetAllBalancesRequest {
            owner: owner.clone(),
        };

        let rpc_request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: methods::GET_ALL_BALANCES.to_string(),
            params: serde_json::to_value(request).unwrap_or(serde_json::json!(null)),
            id: 1,
        };

        // Use the underlying reqwest client from RpcClient if needed,
        // or just use RpcClient's request method if it was public (it's not).
        // For now, let's just use a standard reqwest call like before but cleaner.
        let http_client = reqwest::Client::new();
        let response = http_client
            .post(&rpc)
            .json(&rpc_request)
            .send()
            .await
            .context("Failed to send RPC request")?;

        let rpc_response: RpcResponse = response
            .json()
            .await
            .context("Failed to parse RPC response")?;

        if let Some(error) = rpc_response.error {
            eprintln!("Error: {} (code: {})", error.message, error.code);
            return Ok(());
        }

        if let Some(result) = rpc_response.result {
            if let Some(balances) = result.get("balances").and_then(|b| b.as_array()) {
                eprintln!("TOKEN BALANCES");
                eprintln!("------------------------------");

                for balance in balances {
                    let token_type = balance
                        .get("token_type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("UNKNOWN");

                    let amount = balance.get("balance").and_then(|a| a.as_u64()).unwrap_or(0);

                    let decimals = balance
                        .get("decimals")
                        .and_then(|d| d.as_u64())
                        .unwrap_or(9);

                    // Convert to human readable format
                    let divisor = 10u64.pow(decimals as u32);
                    let whole = amount / divisor;
                    let fraction = amount % divisor;

                    if self.detailed {
                        eprintln!("Token Type: {}", token_type);
                        eprintln!(
                            "  Balance: {}.{:0width$} ({})",
                            whole,
                            fraction,
                            token_type,
                            width = decimals as usize
                        );
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
                        eprintln!(
                            "  {} {}.{:0width$}",
                            token_type,
                            whole,
                            fraction,
                            width = decimals as usize
                        );
                    }
                }
                eprintln!("\nTotal tokens: {}", balances.len());
            } else {
                eprintln!("No balances found");
            }
        } else {
            eprintln!("Invalid response format");
        }

        Ok(())
    }
}
