// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use clap::*;
use kanari_crypto::wallet::load_wallet;
use kanari_types::address::Address;
use move_core_types::{account_address::AccountAddress, language_storage::TypeTag, parser};

/// Call a Move function on the blockchain
#[derive(Parser)]
#[clap(name = "call")]
pub struct Call {
    /// Package address (hex)
    /// Example: 0x840512ff... (use `--module <NAME>` separately)
    #[clap(long = "package")]
    pub package: String,

    /// Module name (required). Use `--module <NAME>` with `--package <ADDRESS>`.
    #[clap(long = "module", value_name = "MODULE")]
    pub module: String,

    /// Function name in module
    #[clap(long = "function")]
    pub function: String,

    /// Type arguments to the generic function being called.
    /// All must be specified, or the call will fail.
    /// Example: 0x1::coin::KANARI
    #[clap(long = "type-args")]
    pub type_args: Vec<String>,

    /// Simplified ordered args like in the function syntax.
    /// ObjectIDs, Addresses must be hex strings.
    /// Example: 0x123 1000 true
    #[clap(long = "args", num_args = 0..)]
    pub args: Vec<String>,

    /// Sender/Caller address (from wallet)
    #[clap(long = "sender")]
    pub sender: String,

    /// Wallet password (required for signing)
    #[clap(long = "password")]
    pub password: Option<String>,

    /// Gas limit for the transaction
    #[clap(long = "gas-limit", default_value = "200000")]
    pub gas_limit: u64,

    /// Gas price in Mist
    #[clap(long = "gas-price", default_value = "1000")]
    pub gas_price: u64,

    /// RPC endpoint
    #[clap(long = "rpc", default_value = "http://localhost:3000")]
    pub rpc_endpoint: String,

    /// Execute immediately and return changeset (shows created objects)
    #[clap(long = "immediate")]
    pub immediate: bool,

    /// Dry run (estimate gas without executing)
    #[clap(long = "dry-run")]
    pub dry_run: bool,
}

impl Call {
    pub fn execute(self) -> Result<()> {
        println!("Preparing function call...");

        // Normalize and validate addresses
        let normalize_addr = |a: &str| -> Result<String> {
            let s = a.trim();
            let hex = if s.starts_with("0x") || s.starts_with("0X") {
                &s[2..]
            } else {
                s
            };
            if hex.len() > 64 {
                anyhow::bail!("Address too long: {}", a);
            }
            Ok(format!("0x{:0>64}", hex))
        };

        // Expect separate flags: `--package <ADDRESS>` and `--module <MODULE>`.
        if self.package.contains("::") {
            anyhow::bail!(
                "Combined form `address::module` is not supported. Use `--package <ADDRESS> --module <MODULE>` instead."
            );
        }

        let package_str = &self.package;
        let module_name = &self.module;

        let sender_normalized = normalize_addr(&self.sender)
            .with_context(|| format!("Invalid sender address: {}", self.sender))?;
        let package_normalized = normalize_addr(package_str)
            .with_context(|| format!("Invalid package address: {}", package_str))?;

        let _sender_addr = Address::from_hex_literal(&sender_normalized)
            .with_context(|| format!("Invalid sender address: {}", self.sender))?;
        let _package_addr = Address::from_hex_literal(&package_normalized)
            .with_context(|| format!("Invalid package address: {}", package_str))?;

        println!("Call Details:");
        println!("   Package: {}::{}", package_normalized, module_name);
        println!("   Function: {}", self.function);
        println!("   Sender: {}", sender_normalized);
        println!("   Gas Limit: {}", self.gas_limit);
        println!("   Gas Price: {}", self.gas_price);

        // Load wallet (signing is required)
        let wallet = {
            let password = self
                .password
                .as_ref()
                .context("Password required for signing (use --password)")?;

            let w = load_wallet(&sender_normalized, password).context(
                "Failed to load wallet. Make sure the wallet exists and password is correct",
            )?;

            println!(
                "   Wallet loaded: {} (curve: {})",
                sender_normalized, w.curve_type
            );
            w
        };

        // Parse type arguments
        let _type_args = if !self.type_args.is_empty() {
            let mut parsed = Vec::new();
            for type_arg in &self.type_args {
                let type_tag = self.parse_type_arg(type_arg)?;
                parsed.push(type_tag);
            }
            println!("   Type Args: {}", self.type_args.join(", "));
            parsed
        } else {
            vec![]
        };

        // Parse arguments
        let _args = if !self.args.is_empty() {
            let parsed = self.parse_args_vec(&self.args)?;
            println!("   Arguments: {} args provided", parsed.len());
            for (i, arg) in self.args.iter().enumerate() {
                println!("     [{}]: {}", i, arg);
            }
            parsed
        } else {
            vec![]
        };

        // Estimate gas
        let estimated_gas = 35_000 + (self.function.len() as u64 * 100);
        println!("Gas estimation:");
        println!("   Estimated: {} units", estimated_gas);
        println!("   Limit: {} units", self.gas_limit);
        println!("   Total Cost: {} Mist", estimated_gas * self.gas_price);

        if self.dry_run {
            println!("Dry run mode - not executing");
            return Ok(());
        }

        // Create transaction
        println!("Creating transaction...");

        // Query account sequence number so signature and RPC include it
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
                    Err(e) => eprintln!("   Failed to parse account RPC response: {}", e),
                },
                Err(e) => eprintln!("   Failed to query account sequence: {}", e),
            }
        }

        // Sign transaction using the loaded wallet
        let signature = {
            // Format module as "address::module_name" for runtime compatibility
            let module_full = format!("{}::{}", package_normalized, module_name);

            // Create proper Transaction to match server's expectation
            use kanari_move_runtime::Transaction;
            let transaction = Transaction::ExecuteFunction {
                sender: sender_normalized.clone(),
                module: module_full.clone(),
                function: self.function.clone(),
                type_args: self.type_args.clone(),
                args: _args.clone(),
                gas_limit: self.gas_limit,
                gas_price: self.gas_price,
                sequence_number: seq_num,
            };

            // Get transaction hash (same way server does it)
            let tx_hash = transaction.hash();

            // Sign with wallet
            match kanari_crypto::sign_message(&wallet.private_key, &tx_hash, wallet.curve_type) {
                Ok(sig) => {
                    println!("   Transaction signed (curve: {})", wallet.curve_type);
                    Some(sig)
                }
                Err(e) => {
                    eprintln!("   Failed to sign transaction: {}", e);
                    None
                }
            }
        };

        // Build CallFunctionRequest and wrap into RpcRequest
        use kanari_rpc_api::{CallFunctionRequest, RpcRequest, RpcResponse, methods};
        use reqwest::blocking::Client;

        // Format module as "address::module_name" for runtime compatibility
        let module_full = format!("{}::{}", package_normalized, module_name);

        let call_req = CallFunctionRequest {
            sender: sender_normalized.clone(),
            package: package_normalized.clone(),
            module: module_full,
            function: self.function.clone(),
            type_args: self.type_args.clone(),
            args: _args.clone(),
            gas_limit: self.gas_limit,
            gas_price: self.gas_price,
            sequence_number: seq_num,
            signature,
            execute_immediate: if self.immediate { Some(true) } else { None },
        };

        let rpc_request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: methods::CALL_FUNCTION.to_string(),
            params: serde_json::to_value(call_req).unwrap_or(serde_json::json!(null)),
            id: 1,
        };

        println!("Sending RPC request to {}...", self.rpc_endpoint);

        let client = Client::new();
        match client.post(&self.rpc_endpoint).json(&rpc_request).send() {
            Ok(resp) => match resp.json::<RpcResponse>() {
                Ok(rpc_resp) => {
                    if let Some(err) = rpc_resp.error {
                        eprintln!("RPC error: {} (code {})", err.message, err.code);
                    } else if let Some(result) = rpc_resp.result {
                        // Try to parse as TransactionResult
                        if let Ok(tx_result) = serde_json::from_value::<
                            kanari_rpc_api::TransactionResult,
                        >(result.clone())
                        {
                            println!("Transaction: {}", tx_result.hash);
                            println!("Status: {}", tx_result.status);
                            println!("Gas used: {} Mist", tx_result.gas_used);

                            // Show error message if transaction failed
                            if let Some(ref error_msg) = tx_result.error_message {
                                eprintln!("Transaction failed: {}", error_msg);
                            }
                        } else if self.immediate {
                            // Try to parse immediate execution response with changeset
                            if let Some(changeset_obj) = result.get("changeset") {
                                println!(
                                    "Transaction: {}",
                                    result
                                        .get("hash")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown")
                                );
                                println!(
                                    "Status: {}",
                                    result
                                        .get("status")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown")
                                );

                                // Show created objects
                                if let Some(created_objs) = changeset_obj
                                    .get("created_objects")
                                    .and_then(|v| v.as_array())
                                {
                                    if !created_objs.is_empty() {
                                        println!("\nCreated Objects:");
                                        for obj in created_objs {
                                            if let (Some(id), Some(obj_type), Some(owner)) = (
                                                obj.get("id").and_then(|v| v.as_str()),
                                                obj.get("type").and_then(|v| v.as_str()),
                                                obj.get("owner").and_then(|v| v.as_str()),
                                            ) {
                                                println!("  📦 Object ID: {}", id);
                                                println!("     Type: {}", obj_type);
                                                println!("     Owner: {}", owner);
                                            }
                                        }
                                    } else {
                                        println!("\nNo objects created");
                                    }
                                }

                                // Show gas used
                                if let Some(gas_used) =
                                    changeset_obj.get("gas_used").and_then(|v| v.as_u64())
                                {
                                    println!("\nGas used: {} Mist", gas_used);
                                }
                            } else {
                                println!("RPC result: {}", result);
                            }
                        } else {
                            // Fallback to plain JSON display
                            println!("RPC result: {}", result);
                        }
                    } else {
                        println!("RPC response has no result and no error");
                    }
                }
                Err(e) => eprintln!("Failed to parse RPC response: {}", e),
            },
            Err(e) => eprintln!("Failed to send RPC request: {}", e),
        }

        println!("Function call prepared and RPC sent.");
        println!("Next steps:");
        println!(" - Check transaction status");
        println!(" - View execution results on explorer");

        Ok(())
    }

    /// Parse a single type argument
    fn parse_type_arg(&self, type_arg: &str) -> Result<TypeTag> {
        let type_arg = type_arg.trim();

        // Parse type tag
        let type_tag = parser::parse_type_tag(type_arg)
            .with_context(|| format!("Failed to parse type argument: {}", type_arg))?;

        Ok(type_tag)
    }

    /// Parse function arguments from Vec<String>
    fn parse_args_vec(&self, args_vec: &[String]) -> Result<Vec<Vec<u8>>> {
        let mut result = Vec::new();

        for arg in args_vec {
            let arg = arg.trim();

            // Try to parse as different types
            let bytes = if arg.starts_with(r"0x") {
                // Hex address or bytes
                let hex_str = &arg[2..];

                // Check if it looks like an address (1-64 hex chars)
                if hex_str.len() <= 64 && hex_str.chars().all(|c| c.is_ascii_hexdigit()) {
                    // Pad to 32 bytes for addresses
                    let padded = format!("{:0>64}", hex_str);
                    let addr = AccountAddress::from_hex_literal(&format!(r"0x{}", padded))
                        .with_context(|| format!("Failed to parse address: {}", arg))?;
                    bcs::to_bytes(&addr)?
                } else {
                    // Raw hex bytes
                    hex::decode(hex_str).with_context(|| format!("Failed to parse hex: {}", arg))?
                }
            } else if let Ok(num) = arg.parse::<u64>() {
                // u64 number
                bcs::to_bytes(&num)?
            } else if let Ok(num) = arg.parse::<u128>() {
                // u128 number
                bcs::to_bytes(&num)?
            } else if arg == "true" || arg == "false" {
                // Boolean
                let b = arg == "true";
                bcs::to_bytes(&b)?
            } else {
                // String
                bcs::to_bytes(arg)?
            };

            result.push(bytes);
        }

        Ok(result)
    }
}
