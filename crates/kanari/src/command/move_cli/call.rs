// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::common::{build_blocking_client, get_account_sequence, load_wallet_for, normalize_addr};
use anyhow::{Context, Result};
use clap::*;
use kanari_core::Transaction;
use kanari_move_runtime::gas::{GasEstimate, GasOperation};
use move_core_types::{account_address::AccountAddress, language_storage::TypeTag, parser};

/// Call a Move function on the blockchain
#[derive(Parser)]
#[clap(
    name = "call",
    about = "Call a Move function on the blockchain",
    after_help = "Examples:\n  kanari call --package 0x1 --module coin --function transfer --args \"address:0x2\" \"u64:1000\" --sender 0x1\n  kanari call --package 0x1 --module game --function start --args \"[\\\"0x1\\\",\\\"0x2\\\"]\" --sender 0x1 --password mypass\n"
)]
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
    #[clap(long = "gas-limit", default_value = "100000")]
    pub gas_limit: u64,

    /// Gas price in Mist
    #[clap(long = "gas-price", default_value = "100")]
    pub gas_price: u64,

    /// RPC endpoint
    #[clap(long = "rpc", default_value = "http://localhost:3000")]
    pub rpc_endpoint: String,
    // `immediate` execution option removed — always submit normally.
}

impl Call {
    pub fn execute(self) -> Result<()> {
        println!("Preparing function call...");

        // Normalize and validate addresses

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

        println!("Call Details:");
        println!("   Package: {}::{}", package_normalized, module_name);
        println!("   Function: {}", self.function);
        println!("   Sender: {}", sender_normalized);
        println!("   Gas Limit: {}", self.gas_limit);
        println!("   Gas Price: {}", self.gas_price);

        // Load wallet (signing is required). If password not provided via CLI, prompt.
        let wallet = {
            let w = load_wallet_for(&sender_normalized, self.password.clone())?;
            println!(
                "   Wallet loaded: {} (curve: {})",
                sender_normalized, w.curve_type
            );
            w
        };

        // Parse and validate type arguments (keep original string values for RPC but validate them)
        let parsed_type_args: Vec<String> = if !self.type_args.is_empty() {
            let mut validated = Vec::new();
            for type_arg in &self.type_args {
                // validate by attempting to parse to a TypeTag
                let _ = self
                    .parse_type_arg(type_arg)
                    .with_context(|| format!("Invalid type argument: {}", type_arg))?;
                validated.push(type_arg.clone());
            }
            println!("   Type Args: {}", self.type_args.join(", "));
            validated
        } else {
            vec![]
        };

        // Parse arguments (support typed args and JSON arrays).
        // Heuristic: support untyped mint form `0x<OBJID> <amount> <0x<recipient>>` by
        // serializing the first arg (32-byte hex object id) as TreasuryCap-like bytes
        // (32 raw bytes + zero u64) while leaving amount/address parsing unchanged.
        let parsed_args: Vec<Vec<u8>> = if !self.args.is_empty() {
            let mut parsed = Vec::new();
            let mut i = 0usize;
            while i < self.args.len() {
                // If we can look ahead for the common pattern (objid, amount, address),
                // and the current token looks like a 32-byte hex, produce cap bytes.
                if i + 2 < self.args.len() {
                    let maybe_obj = self.args[i].trim();
                    let maybe_amount = self.args[i + 1].trim();
                    let maybe_addr = self.args[i + 2].trim();

                    let is_32b_hex = maybe_obj.starts_with("0x")
                        && maybe_obj[2..].len() == 64
                        && maybe_obj[2..].chars().all(|c| c.is_ascii_hexdigit());
                    let is_u64 = maybe_amount.parse::<u64>().is_ok();
                    let is_addr_like = maybe_addr.starts_with("0x")
                        && maybe_addr[2..].chars().all(|c| c.is_ascii_hexdigit());

                    if is_32b_hex && is_u64 && is_addr_like {
                        // Build TreasuryCap-like raw bytes: 32 raw id bytes + u64(0) little-endian
                        let hex_part = &maybe_obj[2..];
                        let mut bytes = hex::decode(hex_part).with_context(|| {
                            format!("Failed to parse hex object id: {}", maybe_obj)
                        })?;
                        bytes.extend(&0u64.to_le_bytes());
                        parsed.push(bytes);
                        // advance by one; the next tokens (amount/address) will be parsed normally
                        i += 1;
                        continue;
                    }
                }

                let bytes = self.parse_arg_flexible(&self.args[i])?;
                parsed.push(bytes);
                i += 1;
            }

            println!("   Arguments: {} args provided", parsed.len());
            parsed
        } else {
            vec![]
        };

        // Estimate gas using runtime `ContractCall` which accounts for function name length
        let operation = GasOperation::ContractCall {
            function_name_len: self.function.len(),
        };
        let estimate = GasEstimate::from_operation(operation, self.gas_price);
        println!("Gas estimation:");
        println!("   Estimated: {} units", estimate.gas_units);
        println!("   Limit: {} units", self.gas_limit);
        println!(
            "   Total Cost: {} Mist ({:.9} KANARI)",
            estimate.total_cost_mist, estimate.total_cost_kanari
        );

        // Create transaction
        println!("Creating transaction...");

        // Query account sequence number so signature and RPC include it (fail-fast)
        let client = build_blocking_client(30)?;
        let seq_num: u64 = get_account_sequence(&client, &self.rpc_endpoint, &sender_normalized)?;

        // Sign transaction using the loaded wallet
        let signature = {
            // Format module as "address::module_name" for runtime compatibility
            let module_full = format!("{}::{}", package_normalized, module_name);

            // Create proper Transaction to match server's expectation (use parsed args/type_args)
            let transaction = Transaction::ExecuteFunction {
                sender: sender_normalized.clone(),
                module: module_full.clone(),
                function: self.function.clone(),
                type_args: parsed_type_args.clone(),
                args: parsed_args.clone(),
                gas_limit: self.gas_limit,
                gas_price: self.gas_price,
                sequence_number: seq_num,
            };

            // Get transaction hash (same way server does it)
            let tx_hash = transaction.hash();

            // Sign with wallet; signing failure is fatal
            let sig = kanari_crypto::sign_message(&wallet.private_key, &tx_hash, wallet.curve_type)
                .map_err(|e| anyhow::anyhow!("Failed to sign transaction: {}", e))?;
            println!("   Transaction signed (curve: {})", wallet.curve_type);
            Some(sig)
        };

        // Build CallFunctionRequest and wrap into RpcRequest
        use kanari_rpc_api::{CallFunctionRequest, RpcRequest, RpcResponse, methods};

        // Format module as "address::module_name" for runtime compatibility
        let module_full = format!("{}::{}", package_normalized, module_name);

        let call_req = CallFunctionRequest {
            sender: sender_normalized.clone(),
            package: package_normalized.clone(),
            module: module_full,
            function: self.function.clone(),
            type_args: parsed_type_args.clone(),
            args: parsed_args.clone(),
            gas_limit: self.gas_limit,
            gas_price: self.gas_price,
            sequence_number: seq_num,
            signature,
            execute_immediate: Some(true),
        };

        let rpc_request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: methods::CALL_FUNCTION.to_string(),
            params: serde_json::to_value(call_req).unwrap_or(serde_json::json!(null)),
            id: 1,
        };

        println!("Sending RPC request to {}...", self.rpc_endpoint);

        let client = build_blocking_client(30)?;
        match client.post(&self.rpc_endpoint).json(&rpc_request).send() {
            Ok(resp) => match resp.json::<RpcResponse>() {
                Ok(rpc_resp) => {
                    if let Some(err) = rpc_resp.error {
                        eprintln!("RPC error: {} (code {})", err.message, err.code);
                    } else if let Some(result) = rpc_resp.result {
                        // Parse result as `TransactionResult` if possible, otherwise print raw JSON
                        if let Ok(tx_result) = serde_json::from_value::<
                            kanari_rpc_api::TransactionResult,
                        >(result.clone())
                        {
                            println!("Transaction: {}", tx_result.hash);
                            println!("Status: {}", tx_result.status);
                            println!("Gas used: {} Mist", tx_result.gas_used);

                            if let Some(ref error_msg) = tx_result.error_message {
                                eprintln!("Transaction failed: {}", error_msg);
                            }
                        } else {
                            match serde_json::to_string_pretty(&result) {
                                Ok(s) => println!("RPC result:\n{}", s),
                                Err(_) => println!("RPC result: {}", result),
                            }
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

    /// Flexible parser for a single argument supporting typed syntax and JSON arrays
    fn parse_arg_flexible(&self, arg: &str) -> Result<Vec<u8>> {
        let s = arg.trim();

        // If JSON array, try to parse homogeneous elements and BCS-encode a Vec<T>
        if s.starts_with('[') && s.ends_with(']') {
            let v: serde_json::Value = serde_json::from_str(s)
                .with_context(|| format!("Failed to parse JSON array arg: {}", s))?;
            if let serde_json::Value::Array(arr) = v {
                if arr.is_empty() {
                    return Ok(bcs::to_bytes::<Vec<u8>>(&vec![])?);
                }

                // All numbers -> Vec<u64>
                if arr.iter().all(|e| e.is_u64()) {
                    let mut vec_n = Vec::new();
                    for e in arr {
                        vec_n.push(e.as_u64().unwrap());
                    }
                    return Ok(bcs::to_bytes(&vec_n)?);
                }

                // All bools -> Vec<bool>
                if arr.iter().all(|e| e.is_boolean()) {
                    let mut vec_b = Vec::new();
                    for e in arr {
                        vec_b.push(e.as_bool().unwrap());
                    }
                    return Ok(bcs::to_bytes(&vec_b)?);
                }

                // All hex addresses (strings starting with 0x) -> Vec<AccountAddress>
                if arr
                    .iter()
                    .all(|e| e.is_string() && e.as_str().unwrap().starts_with("0x"))
                {
                    let mut vec_addr = Vec::new();
                    for e in arr {
                        let saddr = e.as_str().unwrap();
                        let hex = if saddr.starts_with("0x") || saddr.starts_with("0X") {
                            &saddr[2..]
                        } else {
                            saddr
                        };
                        let padded = format!("{:0>64}", hex);
                        let a = AccountAddress::from_hex_literal(&format!("0x{}", padded))
                            .with_context(|| {
                                format!("Failed to parse address in array: {}", saddr)
                            })?;
                        vec_addr.push(a);
                    }
                    return Ok(bcs::to_bytes(&vec_addr)?);
                }

                // All strings -> Vec<String>
                if arr.iter().all(|e| e.is_string()) {
                    let mut vec_s = Vec::new();
                    for e in arr {
                        vec_s.push(e.as_str().unwrap().to_string());
                    }
                    return Ok(bcs::to_bytes(&vec_s)?);
                }

                // Unsupported or mixed-type arrays are rejected
                anyhow::bail!(
                    "Unsupported or mixed-type array: all elements must be u64, bool, hex addresses (\"0x...\"), or plain strings. Got: {:?}",
                    arr
                );
            }
        }

        // Typed form: "type:value"
        if let Some(pos) = s.find(':') {
            let (ty, val) = s.split_at(pos);
            let val = &val[1..];
            match ty {
                "u64" => {
                    let n: u64 = val
                        .parse()
                        .with_context(|| format!("Invalid u64: {}", val))?;
                    return Ok(bcs::to_bytes(&n)?);
                }
                "u128" => {
                    let n: u128 = val
                        .parse()
                        .with_context(|| format!("Invalid u128: {}", val))?;
                    return Ok(bcs::to_bytes(&n)?);
                }
                "bool" => {
                    let b = match val {
                        "true" => true,
                        "false" => false,
                        _ => return Err(anyhow::anyhow!("Invalid bool: {}", val)),
                    };
                    return Ok(bcs::to_bytes(&b)?);
                }
                "address" => {
                    let hex = if val.starts_with("0x") || val.starts_with("0X") {
                        &val[2..]
                    } else {
                        val
                    };
                    let padded = format!("{:0>64}", hex);
                    let addr = AccountAddress::from_hex_literal(&format!("0x{}", padded))
                        .with_context(|| format!("Failed to parse address: {}", val))?;
                    return Ok(bcs::to_bytes(&addr)?);
                }
                "hex" => {
                    let hv = if val.starts_with("0x") || val.starts_with("0X") {
                        &val[2..]
                    } else {
                        val
                    };
                    return Ok(
                        hex::decode(hv).with_context(|| format!("Failed to parse hex: {}", val))?
                    );
                }
                _ => {
                    // fallthrough to auto-detect
                }
            }
        }

        // Auto-detect (legacy): hex addresses/bytes, numbers, bools, string
        let arg = s;
        if arg.starts_with("0x") {
            let hex_str = &arg[2..];
            if hex_str.len() <= 64 && hex_str.chars().all(|c| c.is_ascii_hexdigit()) {
                // Treat as address (pad to 32 bytes)
                let padded = format!("{:0>64}", hex_str);
                let addr = AccountAddress::from_hex_literal(&format!(r"0x{}", padded))
                    .with_context(|| format!("Failed to parse address: {}", arg))?;
                return Ok(bcs::to_bytes(&addr)?);
            }
            // raw hex bytes
            return Ok(
                hex::decode(hex_str).with_context(|| format!("Failed to parse hex: {}", arg))?
            );
        }

        if let Ok(num) = arg.parse::<u64>() {
            return Ok(bcs::to_bytes(&num)?);
        }

        if let Ok(num) = arg.parse::<u128>() {
            return Ok(bcs::to_bytes(&num)?);
        }

        if arg == "true" || arg == "false" {
            let b = arg == "true";
            return Ok(bcs::to_bytes(&b)?);
        }

        Ok(bcs::to_bytes(arg)?)
    }
}
