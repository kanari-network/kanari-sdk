// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::command::common::get_rpc_endpoint;
use crate::command::tx_output::print_json_value;
use anyhow::{Context, Result};
use clap::Parser;
use kanari_rpc_client::RpcClient;

/// Show transaction details by hash
#[derive(Parser, Debug)]
#[clap(name = "tx")]
pub struct Tx {
    /// Transaction hash to query
    #[clap(long = "hash")]
    pub hash: String,

    /// RPC endpoint URL
    #[clap(long = "rpc")]
    pub rpc_endpoint: Option<String>,

    /// Print the full RPC response as JSON
    #[clap(long = "json")]
    pub json: bool,
}

impl Tx {
    pub async fn execute(&self) -> Result<()> {
        let rpc = get_rpc_endpoint(self.rpc_endpoint.clone());

        eprintln!("Querying transaction...");
        eprintln!("   Hash: {}", self.hash);
        eprintln!("   RPC: {}\n", rpc);

        let client = RpcClient::new(&rpc);
        let details = client
            .get_transaction(&self.hash)
            .await
            .with_context(|| format!("Failed to fetch transaction {}", self.hash))?;

        if self.json {
            let value = serde_json::to_value(&details)
                .context("Failed to serialize transaction details")?;
            print_json_value("", "Transaction details", &value);
            return Ok(());
        }

        eprintln!("TRANSACTION");
        eprintln!("------------------------------");
        eprintln!("Hash: {}", details.hash);
        eprintln!("Status: {}", details.status);
        eprintln!(
            "Outcome: success={} previewed={} submitted={} committed={}",
            details.success, details.previewed, details.submitted, details.committed
        );

        if let Some(block_height) = details.block_height {
            eprintln!("Block Height: {}", block_height);
        } else {
            eprintln!("Block Height: pending");
        }

        eprintln!("Type: {}", details.tx_type);
        eprintln!("Sender: {}", details.sender);

        if let Some(sender_address) = details.sender_address.as_deref() {
            eprintln!("Sender Address: {}", sender_address);
        }

        if let Some(nonce) = details.nonce {
            eprintln!("Nonce: {}", nonce);
        }

        eprintln!("Gas Limit: {}", details.gas_limit);
        eprintln!("Gas Price: {}", details.gas_price);

        if let Some(gas_used) = details.gas_used {
            eprintln!("Gas Used: {}", gas_used);
        } else {
            eprintln!("Gas Used: pending");
        }

        if let Some(module) = details.module.as_deref() {
            eprintln!("Module: {}", module);
        }

        if let Some(function) = details.function.as_deref() {
            eprintln!("Function: {}", function);
        }

        if let Some(module_functions) = details.module_functions.as_ref()
            && !module_functions.is_empty()
        {
            eprintln!("Module Functions: {}", module_functions.join(", "));
        }

        if let Some(object_inputs) = details.object_inputs.as_ref() {
            eprintln!("Object Inputs: {}", object_inputs.len());
        }

        if let Some(gas_payment) = details.gas_payment.as_ref() {
            eprintln!("Gas Payment Objects: {}", gas_payment.payment_objects.len());
            if let Some(first_payment) = gas_payment.payment_objects.first() {
                eprintln!("Gas Payment First Object: {}", first_payment.object_id);
            }
        }

        if let Some(effects) = details.effects.as_ref() {
            eprintln!("Effect Status: {}", effects.status);
            eprintln!("Effect Gas Used: {}", effects.gas_used);
            eprintln!("Input Objects: {}", effects.input_objects.len());
            eprintln!("Shared Inputs: {}", effects.shared_inputs.len());
            eprintln!("Gas Objects: {}", effects.gas_object_refs.len());
            eprintln!("Object Changes: {}", effects.object_changes.len());
            eprintln!("Created Objects: {}", effects.created.len());
            eprintln!("Mutated Objects: {}", effects.mutated.len());
            eprintln!("Deleted Objects: {}", effects.deleted.len());
            eprintln!("Transferred Objects: {}", effects.transferred.len());
            eprintln!("Causal Edges: {}", effects.causal_edges.len());
        }

        Ok(())
    }
}
