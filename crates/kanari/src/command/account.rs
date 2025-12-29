// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use clap::*;
use reqwest::blocking::Client;
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
        #[clap(long = "rpc", default_value = "http://localhost:19001")]
        rpc_endpoint: String,
    },
}

impl AccountCommand {
    pub fn execute(&self) -> Result<()> {
        match self {
            AccountCommand::Get {
                address,
                rpc_endpoint,
            } => {
                let client = Client::new();
                let request = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "kanari_getAccount",
                    // RPC expects params as a string containing the address
                    "params": address,
                    "id": 1
                });

                let response = client
                    .post(rpc_endpoint)
                    .json(&request)
                    .send()
                    .context("Failed to send RPC request")?;

                let rpc_response: Value =
                    response.json().context("Failed to parse RPC response")?;

                if let Some(error) = rpc_response.get("error") {
                    println!("❌ Error: {}", error.get("message").unwrap_or(&Value::Null));
                    return Ok(());
                }

                if let Some(result) = rpc_response.get("result") {
                    println!("Account info for {}:\n", address);
                    println!(
                        "{}",
                        serde_json::to_string_pretty(result)
                            .unwrap_or_else(|_| "<invalid result>".to_string())
                    );
                } else {
                    println!("Invalid response format");
                }

                Ok(())
            }
        }
    }
}
