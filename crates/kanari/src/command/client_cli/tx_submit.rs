// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Shared transaction submission helpers for client CLI modules that wrap
//! Move entry calls (governance, multisig, ...). Centralises the duplicated
//! `ResolvedTx` / `EntryObject` / `resolve_tx` / `submit_entry` / `bcs_*`
//! boilerplate so each module only keeps its command-specific logic.

use crate::command::common::{
    check_node_connection, get_rpc_endpoint, get_sender_for_tx, load_wallet_for, normalize_addr,
    resolve_sender, resolve_transaction_gas,
};
use crate::command::rpc_helpers::{object_input_from_object_id, sign_and_call_function};
use crate::command::tx_output::print_transaction_status;
use anyhow::{Context, Result};
use kanari_rpc_api::BuildCallFunctionRequest;
use kanari_rpc_client::RpcClient;

pub struct EntryObject {
    pub id: String,
    pub mutable: bool,
    /// When true, the object must be owned by the transaction sender (used for
    /// capability objects such as `GovernanceCap`); otherwise ownership is not
    /// enforced at the RPC layer (cross-owner objects such as shared proposals).
    pub owner_check: bool,
}

pub struct ResolvedTx {
    pub sender_normalized: String,
    pub sender_tagged: String,
    pub wallet: kanari_crypto::wallet::Wallet,
    pub gas_limit: u64,
    pub gas_price: u64,
}

pub async fn resolve_tx(
    from: Option<String>,
    password: Option<String>,
    rpc_endpoint: Option<String>,
    gas_limit: Option<u64>,
    gas_price: Option<u64>,
) -> Result<(ResolvedTx, RpcClient)> {
    let rpc = get_rpc_endpoint(rpc_endpoint);
    let sender_normalized = resolve_sender(from)?;
    let wallet = load_wallet_for(&sender_normalized, password)?;
    let (gas_limit, gas_price) = resolve_transaction_gas(gas_limit, gas_price);
    let client = RpcClient::new(&rpc);
    check_node_connection(&client, &rpc).await?;
    let sender_tagged = get_sender_for_tx(&wallet, &sender_normalized)?;
    Ok((ResolvedTx { sender_normalized, sender_tagged, wallet, gas_limit, gas_price }, client))
}

#[allow(clippy::too_many_arguments)]
pub async fn submit_entry(
    tx: &ResolvedTx,
    client: &RpcClient,
    package: &str,
    module: &str,
    function: &str,
    type_args: Vec<String>,
    args: Vec<Vec<u8>>,
    objects: Vec<EntryObject>,
) -> Result<()> {
    let mut object_inputs = Vec::with_capacity(objects.len());
    for entry in &objects {
        let normalized = normalize_addr(&entry.id)?;
        let expected = if entry.owner_check { Some(tx.sender_normalized.as_str()) } else { None };
        let mut input = object_input_from_object_id(client, &normalized, expected).await?;
        input.mutable = entry.mutable;
        object_inputs.push(input);
    }

    let prepared = client
        .build_call_function(BuildCallFunctionRequest {
            sender: tx.sender_tagged.clone(),
            package: package.to_string(),
            module: module.to_string(),
            function: function.to_string(),
            type_args,
            args,
            object_inputs: Some(object_inputs),
            gas_limit: tx.gas_limit,
            gas_price: tx.gas_price,
            nonce: None,
            execute_immediate: Some(true),
        })
        .await
        .context("Failed to build call function transaction")?;

    if let Some(inputs) = &prepared.object_inputs {
        for input in inputs {
            eprintln!("  Object input: {}", input.object_ref.object_id);
        }
    }
    eprintln!("  Gas Limit: {}", prepared.gas_limit);
    eprintln!("  Gas Price: {} Mist/gas", prepared.gas_price);

    let status = sign_and_call_function(client, &tx.wallet, prepared)
        .await
        .context("Failed to submit transaction")?;
    print_transaction_status("  ", &status);
    if !status.success {
        anyhow::bail!(
            "Transaction failed (status: {}, hash: {}). Preview did not commit; retry with --gas-limit higher or check node logs for the Move abort code.",
            status.status,
            status.hash
        );
    }
    Ok(())
}

pub fn bcs_addr(addr_normalized: &str) -> Result<Vec<u8>> {
    bcs::to_bytes(
        &move_core_types::account_address::AccountAddress::from_hex_literal(addr_normalized)
            .map_err(|e| anyhow::anyhow!("Invalid address {}: {:?}", addr_normalized, e))?,
    )
    .context("Failed to BCS-encode address")
}

pub fn bcs_u64(v: u64) -> Result<Vec<u8>> {
    bcs::to_bytes(&v).context("Failed to BCS-encode u64")
}

pub fn bcs_bool(v: bool) -> Result<Vec<u8>> {
    bcs::to_bytes(&v).context("Failed to BCS-encode bool")
}

pub fn bcs_string(s: &str) -> Result<Vec<u8>> {
    bcs::to_bytes(&s.as_bytes().to_vec()).context("Failed to BCS-encode string")
}
