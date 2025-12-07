use anyhow::{Context, Result};
use clap::*;
use reqwest::blocking::Client;
use serde_json::Value;

/// Show token balances for an address
#[derive(Parser)]
#[clap(name = "balances")]
pub struct Balances {
    /// Address to query
    #[clap(long = "address")]
    pub address: String,

    /// RPC endpoint URL
    #[clap(long = "rpc", default_value = "http://localhost:3000")]
    pub rpc_endpoint: String,

    /// Show detailed information
    #[clap(long = "detailed", short = 'd')]
    pub detailed: bool,
}

impl Balances {
    pub fn execute(&self) -> Result<()> {
        println!("Querying token balances...");
        println!("   Address: {}", self.address);
        println!("   RPC: {}\n", self.rpc_endpoint);

        let client = Client::new();
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "kanari_getAllBalances",
            "params": {
                "address": self.address
            },
            "id": 1
        });

        let response = client
            .post(&self.rpc_endpoint)
            .json(&request)
            .send()
            .context("Failed to send RPC request")?;

        let rpc_response: Value = response.json().context("Failed to parse RPC response")?;

        if let Some(error) = rpc_response.get("error") {
            println!("❌ Error: {}", error.get("message").unwrap_or(&Value::Null));
            return Ok(());
        }

        if let Some(result) = rpc_response.get("result") {
            if let Some(balances) = result.get("balances").and_then(|b| b.as_array()) {
                println!("TOKEN BALANCES");
                println!("------------------------------");

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
                        println!("Token: {}", symbol);
                        println!(
                            "  Balance: {}.{:0width$} {}",
                            whole,
                            fraction,
                            symbol,
                            width = decimals as usize
                        );
                        println!("  Type: {}", token_type);
                        println!("  Raw Amount: {}", amount);
                        println!("------------------------------");
                    } else {
                        println!("  {} {}.{:0<9}", symbol, whole, fraction);
                    }
                }
                println!("\nTotal tokens: {}", balances.len());
            } else {
                println!("No balances found");
            }
        } else {
            println!("Invalid response format");
        }

        Ok(())
    }
}
