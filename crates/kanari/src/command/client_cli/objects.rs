// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::command::common::{get_rpc_endpoint, resolve_sender};
use anyhow::{Context, Result};
use clap::Parser;
use kanari_rpc_api::{GetOwnedObjectsRequest, RpcRequest, RpcResponse, methods};

/// List owned objects
#[derive(Parser, Debug)]
pub struct Objects {
    /// Owner address to query (optional). If omitted, uses selected wallet in config.
    #[clap(long = "owner")]
    pub owner: Option<String>,

    /// Object type to filter by (optional)
    #[clap(long = "type")]
    pub object_type: Option<String>,

    /// RPC endpoint URL
    #[clap(long = "rpc")]
    pub rpc_endpoint: Option<String>,

    /// Show detailed information
    #[clap(long = "detailed", short = 'd')]
    pub detailed: bool,
}

impl Objects {
    pub async fn execute(&self) -> Result<()> {
        let rpc = get_rpc_endpoint(self.rpc_endpoint.clone());
        let owner_address = resolve_sender(self.owner.clone())?;

        eprintln!("Querying owned objects...");
        eprintln!("   Owner: {}", owner_address);
        if let Some(ref obj_type) = self.object_type {
            eprintln!("   Type filter: {}", obj_type);
        }
        eprintln!("   RPC: {}\n", rpc);

        let request = GetOwnedObjectsRequest {
            owner: owner_address.clone(),
            object_type: self.object_type.clone(),
        };

        let rpc_request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: methods::GET_OWNED_OBJECTS.to_string(),
            params: serde_json::to_value(request).unwrap_or(serde_json::json!(null)),
            id: 1,
        };

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
            if let Some(objects) = result.get("objects").and_then(|o| o.as_array()) {
                if objects.is_empty() {
                    eprintln!("No objects found.");
                    return Ok(());
                }

                eprintln!("OWNED OBJECTS");
                eprintln!("------------------------------");

                for (i, obj) in objects.iter().enumerate() {
                    let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    let owner = obj.get("owner").and_then(|v| v.as_str()).unwrap_or("?");
                    let type_ = obj.get("type_").and_then(|v| v.as_str()).unwrap_or("?");
                    let version = obj.get("version").and_then(|v| v.as_u64()).unwrap_or(0);

                    eprintln!("Object #{}:", i + 1);
                    eprintln!("  ID: {}", id);
                    eprintln!("  Owner: {}", owner);
                    eprintln!("  Type: {}", type_);
                    eprintln!("  Version: {}", version);

                    if self.detailed
                        && let Some(data) = obj.get("data").and_then(|d| d.as_array())
                    {
                        eprintln!("  Data ({} bytes):", data.len());
                        // Display first 32 bytes and last 32 bytes if data is long
                        if data.len() <= 64 {
                            let hex_data: Vec<String> = data
                                .iter()
                                .map(|b| format!("{:02x}", b.as_u64().unwrap_or(0)))
                                .collect();
                            eprintln!("    {}", hex_data.join(" "));
                        } else {
                            let first_32: Vec<String> = data
                                .iter()
                                .take(32)
                                .map(|b| format!("{:02x}", b.as_u64().unwrap_or(0)))
                                .collect();
                            let last_32: Vec<String> = data
                                .iter()
                                .rev()
                                .take(32)
                                .rev()
                                .map(|b| format!("{:02x}", b.as_u64().unwrap_or(0)))
                                .collect();
                            eprintln!("    {} ... {}", first_32.join(" "), last_32.join(" "));
                            eprintln!("    ({} bytes total)", data.len());
                        }
                    }
                    eprintln!();
                }

                eprintln!("Total: {} object(s)", objects.len());
            } else {
                eprintln!("Unexpected response format.");
            }
        } else {
            eprintln!("No result returned from RPC.");
        }

        Ok(())
    }
}
