// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::command::common::{
    check_node_connection, get_rpc_endpoint, get_sender_for_tx, load_wallet_for, normalize_addr,
    resolve_sender,
};
use anyhow::{Context, Result};
use clap::Parser;
use kanari_rpc_api::CallFunctionRequest;
use kanari_rpc_client::RpcClient;

#[derive(Parser, Debug)]
pub struct TokenTransfer {
    /// Sender wallet address (optional). If omitted, uses selected wallet in config.
    #[arg(short, long)]
    pub from: Option<String>,
    /// Recipient address
    #[arg(short, long)]
    pub to: String,
    /// Token type to transfer (e.g., "james::james::JAMES" or "0x...::james::JAMES")
    #[arg(long)]
    pub token: String,
    /// Amount to transfer (in base units, e.g., 100000000 for 0.1 JAMES with 9 decimals)
    #[arg(long)]
    pub amount: u64,
    /// Wallet password
    #[arg(short, long)]
    pub password: String,

    /// RPC endpoint
    #[clap(long = "rpc")]
    pub rpc_endpoint: Option<String>,
}

impl TokenTransfer {
    pub async fn execute(&self) -> Result<()> {
        let rpc = get_rpc_endpoint(self.rpc_endpoint.clone());
        let from_addr = resolve_sender(self.from.clone())?;
        let to_addr = normalize_addr(&self.to)?;
        let wallet = load_wallet_for(&from_addr, Some(self.password.clone()))?;

        eprintln!(
            "Transferring {} units of {} token...",
            self.amount, self.token
        );
        eprintln!("  From: {}", from_addr);
        eprintln!("  To: {}", to_addr);

        // Connect to RPC server
        let client = RpcClient::new(&rpc);
        check_node_connection(&client, &rpc).await?;

        // Get account to get sequence number and owned objects
        let account = client
            .get_account(&from_addr)
            .await
            .context("Failed to get sender account")?;

        // Find coin objects of the specified token type
        let mut coin_objects = Vec::new();
        if let Some(owned_objects) = &account.owned_objects {
            for obj in owned_objects {
                if obj.type_.contains(&format!("::coin::Coin<{}>", self.token)) {
                    coin_objects.push(obj.id.clone());
                }
            }
        }

        if coin_objects.is_empty() {
            anyhow::bail!(
                "No {} coin objects found for address {}. Check if you have this token.",
                self.token,
                from_addr
            );
        }

        // For now, use the first coin object found as the source
        let coin_object_id = &coin_objects[0];
        eprintln!("  Using coin object: {}", coin_object_id);
        eprintln!("  Amount: {} (base units)", self.amount);

        let sender_tagged = get_sender_for_tx(&wallet, &from_addr)?;

        // Parse token format: address::module::struct
        let module_parts: Vec<&str> = self.token.split("::").collect();
        if module_parts.len() < 3 {
            anyhow::bail!("Invalid token format. Expected format: address::module::struct");
        }

        let package_address = module_parts[0].to_string();
        let module_name = module_parts[1].to_string();

        // Create the CallFunctionRequest for transfer_amount
        let call_req = CallFunctionRequest {
            sender: sender_tagged.clone(),
            package: package_address,
            module: module_name,
            function: "transfer_amount".to_string(),
            type_args: vec![],
            args: vec![
                hex::decode(coin_object_id.strip_prefix("0x").unwrap_or(coin_object_id))
                    .context("Invalid coin object ID")?,
                bcs::to_bytes(&self.amount).context("Failed to serialize amount")?,
                bcs::to_bytes(
                    &move_core_types::account_address::AccountAddress::from_hex_literal(&to_addr)?,
                )
                .context("Failed to serialize recipient address")?,
            ],
            gas_limit: 100_000,
            gas_price: 1000,
            sequence_number: account.sequence_number,
            signature: None, // Will be set after signing
            execute_immediate: Some(true),
        };

        // Create a dummy transaction to sign - use the tagged sender address
        let dummy_tx = kanari_types::transaction::Transaction::ExecuteFunction {
            sender: sender_tagged.clone(), // Use tagged address for signing
            module: format!("{}::{}", call_req.package, call_req.module),
            function: call_req.function.clone(),
            type_args: call_req.type_args.clone(),
            args: call_req.args.clone(),
            gas_limit: call_req.gas_limit,
            gas_price: call_req.gas_price,
            sequence_number: call_req.sequence_number,
        };

        let mut signed_tx = kanari_types::transaction::SignedTransaction::new(dummy_tx);
        signed_tx
            .sign(&wallet.private_key, wallet.curve_type)
            .context("Failed to sign transaction")?;

        // Update the call request with the signature
        let mut final_call_req = call_req;
        final_call_req.signature = Some(signed_tx.signature.clone());

        eprintln!("  Gas Limit: {}", final_call_req.gas_limit);
        eprintln!("  Gas Price: {} Mist/gas", final_call_req.gas_price);
        eprintln!("  Transaction signed");

        eprintln!("  Submitting transaction to node...");

        // Use call_function instead of submit_transaction
        let status = client
            .call_function(final_call_req)
            .await
            .context("Failed to submit transaction")?;

        eprintln!("  Transaction submitted successfully");
        eprintln!("  Transaction hash: {}", status.hash);
        eprintln!("  Status: {}", status.status);

        Ok(())
    }
}
