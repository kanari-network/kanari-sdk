// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result, bail};
use kanari_crypto::wallet::Wallet;
use kanari_rpc_api::{
    CallFunctionRequest, OwnerInfo, RpcRequest, RpcResponse, TransactionStatus, methods,
};
use kanari_rpc_client::RpcClient;
use kanari_types::transaction::{SignedTransaction, Transaction};
use reqwest::blocking::Client;

use crate::command::tx_output::print_rpc_error;

pub fn sign_call_function_request(
    mut request: CallFunctionRequest,
    wallet: &Wallet,
) -> Result<CallFunctionRequest> {
    let transaction = Transaction::ExecuteFunction {
        sender: request.sender.clone(),
        module: format!("{}::{}", request.package, request.module),
        function: request.function.clone(),
        type_args: request.type_args.clone(),
        args: request.args.clone(),
        object_inputs: request.object_inputs.clone().unwrap_or_default(),
        gas_payment: request.gas_payment.clone(),
        gas_limit: request.gas_limit,
        gas_price: request.gas_price,
        sequence_number: request.sequence_number,
    };

    let mut signed_tx = SignedTransaction::new(transaction);
    signed_tx
        .sign(&wallet.private_key, wallet.curve_type)
        .context("Failed to sign transaction")?;
    request.signature = Some(signed_tx.signature);
    Ok(request)
}

pub async fn sign_and_call_function(
    client: &RpcClient,
    wallet: &Wallet,
    request: CallFunctionRequest,
) -> Result<TransactionStatus> {
    let signed_request = sign_call_function_request(request, wallet)?;
    let status = client
        .call_function(signed_request)
        .await
        .context("Failed to submit transaction")?;

    if !status.success || (!status.submitted && !status.committed) {
        bail!(
            "Transaction was not successful (status: {}, submitted: {}, committed: {}, previewed: {}). Tx hash: {}",
            status.status,
            status.submitted,
            status.committed,
            status.previewed,
            status.hash
        );
    }

    Ok(status)
}

pub fn get_owner_info(
    client: &Client,
    rpc_endpoint: &str,
    sender_normalized: &str,
) -> Result<OwnerInfo> {
    let acct_req = RpcRequest {
        jsonrpc: "2.0".to_string(),
        method: methods::GET_OWNER.to_string(),
        params: serde_json::to_value(sender_normalized)
            .context("Failed to serialize sender for RPC")?,
        id: 1,
    };

    let resp = client
        .post(rpc_endpoint)
        .json(&acct_req)
        .send()
        .context("Failed to query owner info from RPC")?;

    let rpc_resp: RpcResponse = resp.json().context("Failed to parse owner RPC response")?;

    if let Some(error) = rpc_resp.error {
        print_rpc_error("", &error);
        bail!("RPC did not return owner info for sender");
    }

    let result = rpc_resp
        .result
        .context("RPC did not return owner info for sender")?;
    serde_json::from_value(result).context("Failed to decode owner info from RPC")
}

pub fn get_owner_sequence(
    client: &Client,
    rpc_endpoint: &str,
    sender_normalized: &str,
) -> Result<u64> {
    Ok(get_owner_info(client, rpc_endpoint, sender_normalized)?.sequence_number)
}
