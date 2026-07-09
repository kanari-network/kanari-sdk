// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::command::common::{
    check_node_connection, consolidate_coin_objects, get_rpc_endpoint, get_sender_for_tx,
    load_wallet_for, normalize_addr, object_call_context, resolve_sender, resolve_transaction_gas,
    select_native_coin_object, sign_and_call_function, spendable_coin_objects,
};
use anyhow::{Context, Result};
use clap::Parser;
use kanari_rpc_api::CallFunctionRequest;
use kanari_rpc_client::RpcClient;
use kanari_types::kanari::KANARI_TOKEN_TYPE;

#[derive(Parser, Debug)]
pub struct Transfer {
    /// Sender wallet address (optional). If omitted, uses selected wallet in config.
    #[arg(short, long)]
    pub from: Option<String>,
    /// Recipient address
    #[arg(short, long)]
    pub to: String,
    /// Amount in Kanari (will be converted to Mist)
    #[arg(short, long)]
    pub amount: f64,
    /// Wallet password
    #[arg(short, long)]
    pub password: String,

    /// RPC endpoint
    #[clap(long = "rpc")]
    pub rpc_endpoint: Option<String>,
}

impl Transfer {
    pub async fn execute(&self) -> Result<()> {
        let rpc = get_rpc_endpoint(self.rpc_endpoint.clone());
        let from_addr = resolve_sender(self.from.clone())?;
        let to_addr = normalize_addr(&self.to)?;
        let wallet = load_wallet_for(&from_addr, Some(self.password.clone()))?;
        let (gas_limit, gas_price) = resolve_transaction_gas(None, None);

        eprintln!("Transferring Kanari tokens...");
        eprintln!("  From: {}", from_addr);
        eprintln!("  To: {}", to_addr);
        eprintln!("  Amount: {} KANARI", self.amount);

        // Convert Kanari to Mist (1 KANARI = 10^9 Mist)
        const MIST_PER_KANARI: f64 = 1_000_000_000.0;
        let amount_mist = (self.amount * MIST_PER_KANARI).round() as u64;
        eprintln!("  Amount (Mist): {}", amount_mist);

        // Connect to RPC server
        let client = RpcClient::new(&rpc);
        check_node_connection(&client, &rpc).await?;

        // Get owner state to determine the next sequence number and available objects.
        let owner = client
            .get_owner(&from_addr)
            .await
            .context("Failed to get sender owner state")?;

        let sender_tagged = get_sender_for_tx(&wallet, &from_addr)?;

        let owned_objects = owner
            .owned_objects
            .as_ref()
            .context("Sender owner state has no owned object list from RPC")?;

        let required_balance = amount_mist.saturating_add(gas_limit.saturating_mul(gas_price));
        let mut selected_coin =
            select_native_coin_object(owned_objects, required_balance).with_context(|| {
            format!(
                "No spendable Coin<{}> object found for {}",
                KANARI_TOKEN_TYPE, from_addr
            )
        })?;

        let mut next_sequence = owner.sequence_number;
        if selected_coin.selected_balance < required_balance {
            if selected_coin.total_balance < required_balance {
                anyhow::bail!(
                    "Insufficient Coin<{}> balance for {}.\n  - required (amount + max gas): {} Mist\n  - best coin object: {} Mist\n  - total spendable across coin objects: {} Mist",
                    KANARI_TOKEN_TYPE,
                    from_addr,
                    required_balance,
                    selected_coin.selected_balance,
                    selected_coin.total_balance
                );
            }

            eprintln!("  Consolidating coin objects before transfer...");
            let consolidation = consolidate_coin_objects(
                &client,
                &wallet,
                &sender_tagged,
                KANARI_TOKEN_TYPE,
                &spendable_coin_objects(owned_objects, KANARI_TOKEN_TYPE),
                required_balance,
                owner.sequence_number,
                gas_limit,
                gas_price,
            )
            .await?;
            selected_coin = consolidation.0;
            next_sequence = consolidation.1;
        }

        eprintln!("  Using coin object: {}", selected_coin.coin_object_id);
        eprintln!("  Selected Coin Balance (Mist): {}", selected_coin.selected_balance);
        eprintln!(
            "  Total Spendable Coin Balance (Mist): {}",
            selected_coin.total_balance
        );

        let (object_inputs, gas_payment) = object_call_context(
            &sender_tagged,
            selected_coin.coin_object_ref.clone(),
            gas_limit,
            gas_price,
        );

        let call_req = CallFunctionRequest {
            sender: sender_tagged.clone(),
            package: "0x2".to_string(),
            module: "kanari".to_string(),
            function: "transfer".to_string(),
            type_args: vec![],
            args: vec![
                move_core_types::account_address::AccountAddress::from_hex_literal(
                    &selected_coin.coin_object_id,
                )
                    .context("Invalid coin object ID")?
                    .to_vec(),
                bcs::to_bytes(&amount_mist).context("Failed to serialize amount")?,
                bcs::to_bytes(
                    &move_core_types::account_address::AccountAddress::from_hex_literal(&to_addr)?,
                )
                .context("Failed to serialize recipient address")?,
            ],
            object_inputs: Some(object_inputs),
            gas_limit,
            gas_price,
            sequence_number: next_sequence,
            gas_payment: Some(gas_payment),
            signature: None,
            execute_immediate: Some(true),
        };
        eprintln!("  Gas Limit: {}", gas_limit);
        eprintln!("  Gas Price: {} Mist/gas", gas_price);
        eprintln!("  Submitting transaction to node...");

        let status = sign_and_call_function(&client, &wallet, call_req).await?;

        eprintln!("  Transaction submitted successfully");
        eprintln!("  Transaction hash: {}", status.hash);
        eprintln!("  Status: {}", status.status);

        Ok(())
    }
}
