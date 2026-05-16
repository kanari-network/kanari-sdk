// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::command::common::{get_rpc_endpoint, resolve_sender};
use anyhow::{Context, Result};
use clap::*;
use kanari_rpc_client::RpcClient;

/// Account subcommands
#[derive(Subcommand, Debug)]
pub enum AccountCommand {
    /// Get account info (address, sequence, modules, token balances, owned objects)
    Get {
        /// Address to query
        #[clap(long = "address")]
        address: Option<String>,

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
                let rpc = get_rpc_endpoint(rpc_endpoint.clone());
                let address_normalized = resolve_sender(address.clone())?;

                let client = RpcClient::new(&rpc);
                let account_info =
                    client
                        .get_account(&address_normalized)
                        .await
                        .with_context(|| {
                            format!("Failed to get account info for {}", address_normalized)
                        })?;

                eprintln!("Account info for {}:\n", address_normalized);
                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(&account_info)
                        .unwrap_or_else(|_| "<invalid result>".to_string())
                );

                Ok(())
            }
        }
    }
}
