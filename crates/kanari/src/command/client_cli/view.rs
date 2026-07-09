// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::command::common::get_rpc_endpoint;
use anyhow::{Context, Result};
use clap::Parser;
use kanari_rpc_api::{RpcRequest, RpcResponse, methods};
use kanari_types::transaction::{ObjectInput, ObjectOwnerKind, ObjectRef};

/// Call a view function (read-only, no transaction submission)
///
/// This command allows you to call Move module functions that don't modify state.
/// View functions are executed locally without submitting a transaction to the blockchain,
/// making them free and instant - perfect for querying contract state.
///
/// Examples:
///   # Get Escrow deal state
///   kanari client view --package 0x123... --module escrow --function get_state \
///     --type-arg "0x1::aptos_coin::AptosCoin" --arg 0xDEAL_ID
///
///   # Get proof count
///   kanari client view --package 0x123... --module escrow --function get_proof_count \
///     --arg 0xPROOF_ID
///
///   # Show raw JSON output
///   kanari client view --package 0x123... --module escrow --function get_state \
///     --type-arg "0x1::aptos_coin::AptosCoin" --arg 0xDEAL_ID --raw
#[derive(Parser, Debug)]
pub struct View {
    /// Package address containing the module (e.g., 0x123abc...)
    #[clap(long = "package")]
    pub package: String,

    /// Module name within the package (e.g., escrow, coin, nft)
    #[clap(long = "module")]
    pub module: String,

    /// Function name to call (must be a public/friend view function)
    #[clap(long = "function")]
    pub function: String,

    /// Type arguments for generic functions (e.g., 0x1::aptos_coin::AptosCoin).
    /// Can be specified multiple times for multiple type parameters.
    #[clap(long = "type-arg", short = 't')]
    pub type_args: Vec<String>,

    /// Function arguments in hex format (BCS serialized bytes).
    /// Can be specified multiple times for multiple arguments.
    /// Example: --arg 0xDEADBEEF or -a 0x1234
    #[clap(long = "arg", short = 'a')]
    pub args: Vec<String>,

    /// Explicit owned object input in the form OWNER,OBJECT_ID,VERSION,DIGEST
    #[clap(long = "owned-input")]
    pub owned_inputs: Vec<String>,

    /// Explicit shared object input in the form OBJECT_ID,VERSION,DIGEST
    #[clap(long = "shared-input")]
    pub shared_inputs: Vec<String>,

    /// Explicit immutable object input in the form OBJECT_ID,VERSION,DIGEST
    #[clap(long = "immutable-input")]
    pub immutable_inputs: Vec<String>,

    /// RPC endpoint URL (overrides default from config)
    #[clap(long = "rpc")]
    pub rpc_endpoint: Option<String>,

    /// Display raw JSON response instead of formatted output
    #[clap(long = "raw")]
    pub raw: bool,
}

impl View {
    pub async fn execute(&self) -> Result<()> {
        let rpc = get_rpc_endpoint(self.rpc_endpoint.clone());

        eprintln!("Calling view function...");
        eprintln!("   Package: {}", self.package);
        eprintln!("   Module: {}", self.module);
        eprintln!("   Function: {}", self.function);

        if !self.type_args.is_empty() {
            eprintln!("   Type Args: {:?}", self.type_args);
        }

        if !self.args.is_empty() {
            eprintln!("   Args: {} argument(s)", self.args.len());
        }
        let object_inputs = self.parse_object_inputs()?;
        if !object_inputs.is_empty() {
            eprintln!("   Object Inputs: {} explicit object ref(s)", object_inputs.len());
        }

        eprintln!("   RPC: {}\n", rpc);

        // Convert hex string args to bytes
        let args_bytes: Vec<Vec<u8>> = self
            .args
            .iter()
            .map(|arg| {
                // Remove 0x prefix if present
                let hex_str = arg.trim_start_matches("0x");
                hex::decode(hex_str).unwrap_or_else(|_| {
                    eprintln!("Warning: Failed to decode hex argument: {}", arg);
                    vec![]
                })
            })
            .collect();

        // Convert bytes to hex strings for RPC (more compact than array of numbers)
        let args_hex: Vec<String> = args_bytes
            .iter()
            .map(|bytes| format!("0x{}", hex::encode(bytes)))
            .collect();

        let request_data = serde_json::json!({
            "package": self.package,
            "module": self.module,
            "function": self.function,
            "type_args": self.type_args,
            "args": args_hex,
            "object_inputs": object_inputs
        });

        let rpc_request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: methods::VIEW_FUNCTION.to_string(),
            params: serde_json::json!([request_data]),
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
            eprintln!("❌ Error: {} (code: {})", error.message, error.code);
            return Ok(());
        }

        if let Some(result) = rpc_response.result {
            if self.raw {
                // Print raw JSON
                eprintln!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                // Pretty print the result
                if let Some(status) = result.get("status").and_then(|s| s.as_str()) {
                    eprintln!("✅ Status: {}", status);
                }

                if let Some(action) = result.get("action").and_then(|a| a.as_str()) {
                    eprintln!("📋 Action: {}", action);
                }

                if let Some(view_result) = result.get("result") {
                    eprintln!("\n📊 Return Value:");

                    // Try to format based on type
                    match view_result {
                        serde_json::Value::Number(n) => {
                            eprintln!("   {}", n);
                        }
                        serde_json::Value::Bool(b) => {
                            eprintln!("   {}", b);
                        }
                        serde_json::Value::String(s) => {
                            eprintln!("   \"{}\"", s);
                        }
                        serde_json::Value::Array(arr) => {
                            eprintln!("   [");
                            for (i, item) in arr.iter().enumerate() {
                                eprintln!("     {}: {}", i, item);
                            }
                            eprintln!("   ]");
                        }
                        _ => {
                            eprintln!("{}", serde_json::to_string_pretty(view_result)?);
                        }
                    }
                }
            }
        } else {
            eprintln!("No result returned");
        }

        Ok(())
    }

    fn parse_owned_input(&self, raw: &str) -> Result<ObjectInput> {
        let parts: Vec<_> = raw.split(',').map(str::trim).collect();
        if parts.len() != 4 {
            anyhow::bail!("Invalid --owned-input. Expected OWNER,OBJECT_ID,VERSION,DIGEST");
        }

        Ok(ObjectInput {
            object_ref: ObjectRef::new(
                parts[1].to_string(),
                Some(parts[2].parse().context("Invalid owned input version")?),
                Some(parts[3].to_string()),
            ),
            owner: Some(ObjectOwnerKind::AddressOwner(parts[0].to_string())),
            mutable: false,
        })
    }

    fn parse_unowned_input(&self, raw: &str, owner: ObjectOwnerKind) -> Result<ObjectInput> {
        let parts: Vec<_> = raw.split(',').map(str::trim).collect();
        if parts.len() != 3 {
            anyhow::bail!("Invalid object input. Expected OBJECT_ID,VERSION,DIGEST");
        }

        Ok(ObjectInput {
            object_ref: ObjectRef::new(
                parts[0].to_string(),
                Some(parts[1].parse().context("Invalid object input version")?),
                Some(parts[2].to_string()),
            ),
            owner: Some(owner),
            mutable: false,
        })
    }

    fn parse_object_inputs(&self) -> Result<Vec<ObjectInput>> {
        let mut inputs = Vec::new();
        for input in &self.owned_inputs {
            inputs.push(self.parse_owned_input(input)?);
        }
        for input in &self.shared_inputs {
            inputs.push(self.parse_unowned_input(input, ObjectOwnerKind::Shared)?);
        }
        for input in &self.immutable_inputs {
            inputs.push(self.parse_unowned_input(input, ObjectOwnerKind::Immutable)?);
        }
        Ok(inputs)
    }
}
