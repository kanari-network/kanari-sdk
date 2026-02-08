// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use clap::*;
use reqwest::Client;
use serde_json::Value;

/// Account subcommands
#[derive(Subcommand, Debug)]
pub enum AccountCommand {
    /// Get account info (address, balance, modules, token balances)
    Get {
        /// Address to query
        #[clap(long = "address")]
        address: String,

        /// RPC endpoint URL
        #[clap(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
}

impl AccountCommand {
    pub async fn execute(&self) -> Result<()> {
        match self {
            AccountCommand::Get {
                address,
                rpc_endpoint,
            } => {
                let rpc = rpc_endpoint
                    .clone()
                    .or_else(kanari_common::get_active_rpc)
                    .unwrap_or_else(|| "http://127.0.0.1:19001".to_string());

                let client = Client::new();
                let request = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "kanari_getAccount",
                    // RPC expects params as a string containing the address
                    "params": address,
                    "id": 1
                });

                let response = client
                    .post(&rpc)
                    .json(&request)
                    .send()
                    .await
                    .context("Failed to send RPC request")?;

                let rpc_response: Value = response
                    .json()
                    .await
                    .context("Failed to parse RPC response")?;

                if let Some(error) = rpc_response.get("error") {
                    eprintln!(
                        " Error: {}",
                        error.get("message").unwrap_or(&serde_json::Value::Null)
                    );
                    return Ok(());
                }

                if let Some(result) = rpc_response.get("result") {
                    eprintln!("Account info for {}:\n", address);
                    eprintln!(
                        "{}",
                        serde_json::to_string_pretty(result)
                            .unwrap_or_else(|_| "<invalid result>".to_string())
                    );
                } else {
                    eprintln!("Invalid response format");
                }

                Ok(())
            }
        }
    }
}
