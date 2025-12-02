// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use super::reroot_path;
use anyhow::{Context, Result, bail};
use clap::*;
use kanari_crypto::wallet::load_wallet;
use kanari_types::address::Address;
use move_package::BuildConfig;
use std::path::PathBuf;

/// Publish the Move module to the blockchain
#[derive(Parser)]
#[clap(name = "publish")]
pub struct Publish {
    /// Path to the Move package (defaults to current directory)
    #[clap(long = "package-path")]
    pub package_path: Option<PathBuf>,

    /// Gas limit for the transaction
    #[clap(long = "gas-limit", default_value = "1000000")]
    pub gas_limit: u64,

    /// Gas price in Mist
    #[clap(long = "gas-price", default_value = "1000")]
    pub gas_price: u64,

    /// Account address publishing the module (from wallet)
    #[clap(long = "sender")]
    pub sender: String,

    /// Wallet password (required for signing)
    #[clap(long = "password")]
    pub password: Option<String>,


    /// RPC endpoint
    #[clap(long = "rpc", default_value = "http://127.0.0.1:3000")]
    pub rpc_endpoint: String,
}

impl Publish {
    pub fn execute(self, path: Option<PathBuf>, config: BuildConfig) -> Result<()> {
        let rerooted_path = reroot_path(path.or(self.package_path.clone()))?;

        // Validate sender address: normalize to 0x-prefixed format
        let sender_normalized = {
            let s = self.sender.trim();
            let hex = if s.starts_with("0x") || s.starts_with("0X") {
                &s[2..]
            } else {
                s
            };
            if hex.len() > 64 {
                bail!("Sender address too long: {}", self.sender);
            }
            format!("0x{:0>64}", hex)
        };

        let _sender_addr = Address::from_hex_literal(&sender_normalized)
            .with_context(|| format!("Invalid sender address: {}", self.sender))?;

        println!("Building Move package...");

        // Build the package
        let compiled_package = config.compile_package(&rerooted_path, &mut std::io::stderr())?;

        println!("Package compiled successfully!");
        println!("   Modules: {}", compiled_package.all_modules().count());

        // Get compiled modules
        let modules: Vec<_> = compiled_package.all_modules().collect();

        if modules.is_empty() {
            bail!("No modules found in package");
        }

        // Load wallet (signing is required)
        let wallet = {
            let password = self
                .password
                .as_ref()
                .context("Password required for signing (use --password)")?;

            let w = load_wallet(&self.sender, password).context(
                "Failed to load wallet. Make sure the wallet exists and password is correct",
            )?;

            println!("Wallet loaded: {} (curve: {})", self.sender, w.curve_type);
            w
        };

        println!("\nPublishing modules to blockchain...");
        println!("   RPC: {}", self.rpc_endpoint);
        println!("   Sender: {}", sender_normalized);

        let mut published_count = 0;
        let mut skipped_count = 0;

        for module_unit in &modules {
            let module = &module_unit.unit.module;
            let module_name = module.self_id().name().to_string();
            let module_address = module.self_id().address().to_string();

            // Normalize module address for comparison
            let module_addr_normalized = {
                let hex = if module_address.starts_with("0x") || module_address.starts_with("0X") {
                    &module_address[2..]
                } else {
                    &module_address
                };
                format!("0x{:0>64}", hex)
            };

            // Only publish modules where the module address matches the sender
            if module_addr_normalized.to_lowercase() != sender_normalized.to_lowercase() {
                println!(
                    "\n   ⏭️  Skipping Module: {} (address {} doesn't match sender)",
                    module_name, module_address
                );
                skipped_count += 1;
                continue;
            }

            let module_bytecode = {
                let mut bytes = vec![];
                module.serialize(&mut bytes)?;
                bytes
            };

            println!("\n   Module: {}", module_name);
            println!("     Size: {} bytes", module_bytecode.len());
            println!("     Address: {}", module.self_id().address());
            println!("     Functions: {}", module.function_defs.len());

            // Estimate gas
            let estimated_gas = 60_000 + (module_bytecode.len() as u64 * 10);
            let estimated_cost = estimated_gas * self.gas_price;
            println!("     Estimated Gas: {} units", estimated_gas);
            println!(
                "     Estimated Cost: {} Mist ({:.6} KANARI)",
                estimated_cost,
                estimated_cost as f64 / 1_000_000_000.0
            );

            if estimated_gas > self.gas_limit {
                eprintln!(
                    "     Warning: Estimated gas ({}) exceeds limit ({})",
                    estimated_gas, self.gas_limit
                );
            }

            // Create PublishModuleRequest and submit to RPC endpoint
            use kanari_rpc_api::{PublishModuleRequest, RpcRequest, RpcResponse, methods};
            use reqwest::blocking::Client;

            // Get current sequence number for sender from RPC (so signature includes it)
            let mut seq_num: u64 = 0;
            {
                use kanari_rpc_api::{RpcRequest, RpcResponse, methods};
                let acct_req = RpcRequest {
                    jsonrpc: "2.0".to_string(),
                    method: methods::GET_ACCOUNT.to_string(),
                    params: serde_json::to_value(sender_normalized.clone())
                        .unwrap_or(serde_json::json!(null)),
                    id: 1,
                };

                let client = Client::new();
                match client.post(&self.rpc_endpoint).json(&acct_req).send() {
                    Ok(resp) => match resp.json::<RpcResponse>() {
                        Ok(rpc_resp) => {
                            if let Some(result) = rpc_resp.result {
                                if let Ok(account_value) =
                                    serde_json::from_value::<serde_json::Value>(result)
                                {
                                    if let Some(sn) = account_value
                                        .get("sequence_number")
                                        .and_then(|v| v.as_u64())
                                    {
                                        seq_num = sn;
                                    }
                                }
                            }
                        }
                        Err(e) => eprintln!("     Failed to parse account RPC response: {}", e),
                    },
                    Err(e) => eprintln!("     Failed to query account sequence: {}", e),
                }
            }

            // Sign transaction using the loaded wallet
            let signature = {
                use kanari_move_runtime::Transaction;
                let transaction = Transaction::PublishModule {
                    sender: sender_normalized.clone(),
                    module_bytes: module_bytecode.clone(),
                    module_name: module_name.clone(),
                    gas_limit: self.gas_limit,
                    gas_price: self.gas_price,
                    sequence_number: seq_num,
                };

                // Get transaction hash (same way server does it)
                let tx_hash = transaction.hash();

                // Sign with wallet
                match kanari_crypto::sign_message(&wallet.private_key, &tx_hash, wallet.curve_type) {
                    Ok(sig) => {
                        println!("     🔐 Transaction signed with {} key", wallet.curve_type);
                        Some(sig)
                    }
                    Err(e) => {
                        eprintln!("     ⚠️  Failed to sign transaction: {}", e);
                        None
                    }
                }
            };

            let pub_req = PublishModuleRequest {
                sender: sender_normalized.clone(),
                module_bytes: module_bytecode.clone(),
                module_name: module_name.clone(),
                gas_limit: self.gas_limit,
                gas_price: self.gas_price,
                sequence_number: seq_num,
                signature,
            };

            let rpc_request = RpcRequest {
                jsonrpc: "2.0".to_string(),
                method: methods::PUBLISH_MODULE.to_string(),
                params: serde_json::to_value(pub_req).unwrap_or(serde_json::json!(null)),
                id: 1,
            };

            println!("     🔁 Sending publish RPC to {} ...", self.rpc_endpoint);
            let client = Client::new();
            match client.post(&self.rpc_endpoint).json(&rpc_request).send() {
                Ok(resp) => match resp.json::<RpcResponse>() {
                    Ok(rpc_resp) => {
                        if let Some(err) = rpc_resp.error {
                            eprintln!("     RPC error: {} (code {})", err.message, err.code);
                        } else if let Some(result) = rpc_resp.result {
                            // Try to parse as TransactionResult
                            if let Ok(tx_result) = serde_json::from_value::<kanari_rpc_api::TransactionResult>(result.clone()) {
                                println!("     ✅ Transaction: {}", tx_result.hash);
                                println!("     📊 Status: {}", tx_result.status);
                                println!("     ⛽ Gas used: {} Mist", tx_result.gas_used);
                                
                                // Show error message if transaction failed
                                if let Some(ref error_msg) = tx_result.error_message {
                                    eprintln!("\n     ❌ Transaction failed: {}", error_msg);
                                }
                                
                                if !tx_result.created_objects.is_empty() {
                                    println!("\n     📦 Created Objects:");
                                    for obj in &tx_result.created_objects {
                                        println!("        🆔 Object ID: {}", obj.id);
                                        println!("           Owner: {}", obj.owner);
                                        println!("           Type: {}", obj.type_);
                                        
                                        // Highlight TreasuryCap objects
                                        if obj.type_.contains("TreasuryCap") {
                                            println!("           💰 This is a TreasuryCap - use for minting!");
                                            println!("\n           📋 To mint tokens, use:");
                                            println!("           kanari move call \\");
                                            println!("             --sender {} \\", sender_normalized);
                                            println!("             --password <PASSWORD> \\");
                                            println!("             --module \"{}::{}\" \\", module_addr_normalized, module_name);
                                            println!("             --function mint \\");
                                            println!("             --args {} <AMOUNT> <RECIPIENT>", obj.id);
                                        }
                                        println!();
                                    }
                                }
                            } else {
                                // Fallback to old format
                                println!("     RPC result: {}", result);
                            }
                        } else {
                            println!("     RPC response has no result and no error");
                        }
                    }
                    Err(e) => eprintln!("     Failed to parse RPC response: {}", e),
                },
                Err(e) => eprintln!("     Failed to send RPC request: {}", e),
            }

            published_count += 1;
        }

        println!("\n✅ Package build and validation complete!");
        println!("   Published: {} modules", published_count);
        println!("   Skipped: {} dependency modules", skipped_count);

        Ok(())
    }
}
