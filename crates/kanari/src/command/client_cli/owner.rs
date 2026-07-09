// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::command::common::{get_rpc_endpoint, resolve_sender};
use anyhow::{Context, Result};
use clap::*;
use kanari_rpc_client::RpcClient;

/// Owner subcommands
#[derive(Subcommand, Debug)]
pub enum OwnerCommand {
    /// Get owner info (owner, sequence, modules, token balances, owned objects)
    Get {
        /// Owner address to query
        #[clap(long = "owner")]
        owner: Option<String>,

        /// RPC endpoint URL
        #[clap(long = "rpc")]
        rpc_endpoint: Option<String>,
    },
}

impl OwnerCommand {
    pub async fn execute(&self) -> Result<()> {
        match self {
            OwnerCommand::Get {
                owner,
                rpc_endpoint,
            } => {
                let rpc = get_rpc_endpoint(rpc_endpoint.clone());
                let owner_normalized = resolve_sender(owner.clone())?;

                let client = RpcClient::new(&rpc);
                let owner_info = client.get_owner(&owner_normalized).await.with_context(|| {
                    format!("Failed to get owner info for {}", owner_normalized)
                })?;

                eprintln!("Owner info for {}:\n", owner_normalized);
                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(&owner_info)
                        .unwrap_or_else(|_| "<invalid result>".to_string())
                );

                Ok(())
            }
        }
    }
}
