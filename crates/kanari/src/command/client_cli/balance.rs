// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::command::common::{get_rpc_endpoint, normalize_addr};
use anyhow::{Context, Result};
use clap::*;
use kanari_rpc_api::{GetAllBalancesRequest, RpcRequest, RpcResponse, methods};
use kanari_rpc_client::RpcClient;

/// Show token balances for an address
#[derive(Parser, Debug)]
#[clap(name = "balances")]
pub struct Balance {
    /// Address to query
    #[clap(long = "address")]
    pub address: String,

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
        let address_normalized = normalize_addr(&self.address)?;

        eprintln!("Querying token balances...");
        eprintln!("   Address: {}", address_normalized);
        eprintln!("   RPC: {}\n", rpc);

        let _client = RpcClient::new(&rpc);

        let request = GetAllBalancesRequest {
            address: address_normalized.clone(),
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

                    let symbol = balance
                        .get("symbol")
                        .and_then(|s| s.as_str())
                        .unwrap_or(token_type);

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
                        eprintln!("Token: {}", symbol);
                        eprintln!(
                            "  Balance: {}.{:0width$} {}",
                            whole,
                            fraction,
                            symbol,
                            width = decimals as usize
                        );
                        eprintln!("  Type: {}", token_type);
                        eprintln!("  Raw Amount: {}", amount);
                        eprintln!("------------------------------");
                    } else {
                        eprintln!(
                            "  {} {}.{:0width$}",
                            symbol,
                            whole,
                            fraction,
                            width = decimals as usize
                        );
                    }
                }
                eprintln!("\nTotal tokens: {}", balances.len());

                // If only the native KANARI balance is present, attempt a best-effort
                // fallback: inspect account `owned_objects` for token metadata (e.g., TreasuryCap/CoinMetadata)
                if balances.len() == 1
                    && let Some(first) = balances.first()
                    && first.get("token_type").and_then(|t| t.as_str()) == Some("KANARI")
                {
                    // Fetch full account info and look for token objects
                    let acct_req = RpcRequest {
                        jsonrpc: "2.0".to_string(),
                        method: methods::GET_ACCOUNT.to_string(),
                        params: serde_json::json!({ "address": address_normalized }),
                        id: 1,
                    };

                    if let Ok(resp) = http_client.post(&rpc).json(&acct_req).send().await
                        && let Ok(val) = resp.json::<RpcResponse>().await
                        && let Some(result_acc) = val.result
                        && let Some(owned) =
                            result_acc.get("owned_objects").and_then(|v| v.as_array())
                    {
                        eprintln!("\nDetected owned objects (possible tokens):");
                        for obj in owned.iter() {
                            let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                            let ty = obj.get("type_").and_then(|v| v.as_str()).unwrap_or("?");

                            // If this looks like a coin object, try fetching the object
                            // and parsing the last 8 bytes as a little-endian u64 amount.
                            if ty.contains("::coin::Coin<") {
                                let obj_req = RpcRequest {
                                    jsonrpc: "2.0".to_string(),
                                    method: methods::GET_OBJECT.to_string(),
                                    params: serde_json::json!({ "object_id": id }),
                                    id: 1,
                                };

                                if let Ok(resp) = http_client.post(&rpc).json(&obj_req).send().await
                                    && let Ok(val) = resp.json::<RpcResponse>().await
                                    && let Some(res) = val.result
                                    && let Some(data_arr) =
                                        res.get("data").and_then(|d| d.as_array())
                                    && data_arr.len() >= 8
                                {
                                    let n = data_arr.len();
                                    let mut bytes = [0u8; 8];
                                    let mut ok = true;
                                    for i in 0..8 {
                                        match data_arr[n - 8 + i].as_u64() {
                                            Some(b) if b <= 0xff => bytes[i] = b as u8,
                                            _ => {
                                                ok = false;
                                                break;
                                            }
                                        }
                                    }
                                    if ok {
                                        let amount = u64::from_le_bytes(bytes);
                                        // Display using same formatting as other balances (9 decimals default)
                                        let decimals = 9u32;
                                        let divisor = 10u64.pow(decimals);
                                        let whole = amount / divisor;
                                        let fraction = amount % divisor;
                                        eprintln!(
                                            "  {}  ({}) -> TOKEN {}.{:0width$}",
                                            id,
                                            ty,
                                            whole,
                                            fraction,
                                            width = decimals as usize
                                        );
                                        continue;
                                    }
                                }
                            }

                            // Fallback: just print id and type
                            eprintln!("  {}  ({})", id, ty);
                        }
                        eprintln!(
                            "\nTip: use `kanari client account get --address <addr>` to inspect object data and determine token types."
                        );
                    }
                }
            } else {
                eprintln!("No balances found");
            }
        } else {
            eprintln!("Invalid response format");
        }

        Ok(())
    }
}
