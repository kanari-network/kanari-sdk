// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::command::common::{
    check_node_connection, get_rpc_endpoint, get_sender_for_tx, load_wallet_for, resolve_sender,
    resolve_transaction_gas, sign_call_function_request,
};
use anyhow::{Context, Result, ensure};
use clap::Parser;
use kanari_rpc_api::CallFunctionRequest;
use kanari_rpc_client::RpcClient;

#[derive(Parser, Debug)]
pub struct Burn {
    /// Wallet address to burn from (optional). If omitted, uses selected wallet in config.
    #[arg(short, long)]
    pub from: Option<String>,
    /// Amount in Kanari to burn
    #[arg(short, long)]
    pub amount: f64,
    /// Wallet password
    #[arg(short, long)]
    pub password: String,

    /// RPC endpoint
    #[clap(long = "rpc")]
    pub rpc_endpoint: Option<String>,
}

impl Burn {
    pub async fn execute(&self) -> Result<()> {
        let rpc = get_rpc_endpoint(self.rpc_endpoint.clone());
        let from_addr = resolve_sender(self.from.clone())?;
        let wallet = load_wallet_for(&from_addr, Some(self.password.clone()))?;
        let (gas_limit, gas_price) = resolve_transaction_gas(None, None);

        eprintln!("Burning Kanari tokens...");
        eprintln!("  From: {}", from_addr);
        eprintln!("  Amount: {} KANARI", self.amount);

        // Convert Kanari to Mist (1 KANARI = 10^9 Mist)
        const MIST_PER_KANARI: f64 = 1_000_000_000.0;
        let amount_mist = (self.amount * MIST_PER_KANARI).round() as u64;
        ensure!(
            amount_mist > 0,
            "--amount is too small; it rounds to 0 Mist"
        );
        eprintln!("  Amount (Mist): {}", amount_mist);

        // Connect to RPC server
        let client = RpcClient::new(&rpc);
        check_node_connection(&client, &rpc).await?;

        // Get owner state to get sequence number
        let owner = client
            .get_owner(&from_addr)
            .await
            .context("Failed to get sender owner state")?;

        let sender_for_tx = get_sender_for_tx(&wallet, &from_addr)?;

        let call_req = CallFunctionRequest {
            sender: sender_for_tx.clone(),
            package: "0x2".to_string(),
            module: "kanari".to_string(),
            function: "burn_amount".to_string(),
            type_args: vec![],
            args: vec![bcs::to_bytes(&amount_mist).context("Failed to serialize burn amount")?],
            gas_limit,
            gas_price,
            sequence_number: owner.sequence_number,
            signature: None,
            execute_immediate: Some(true),
        };
        let final_call_req = sign_call_function_request(call_req, &wallet)?;
        let status = client
            .call_function(final_call_req)
            .await
            .context("Failed to submit burn transaction")?;

        eprintln!("  Transaction hash: {}", status.hash);
        eprintln!("  Status: {}", status.status);

        Ok(())
    }
}
