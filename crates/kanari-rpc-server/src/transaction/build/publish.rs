// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Build handlers for module and package publish/upgrade transactions.

use super::*;

fn select_build_gas_and_nonce(
    state: &RpcServerState,
    request_id: u64,
    sender: &str,
    gas_limit: u64,
    gas_price: u64,
    nonce: Option<u64>,
) -> Result<(GasPayment, u64), Box<RpcResponse>> {
    parse_hex_address(request_id, sender, "sender address")?;

    if let Some(response) = invalid_build_gas_response(request_id, gas_limit, gas_price) {
        return Err(Box::new(response));
    }

    let owner_info = state
        .engine
        .get_owner_info(sender)
        .ok_or_else(|| Box::new(internal_error_response(request_id, "Owner not found")))?;
    let owned_objects = owner_info.owned_objects.ok_or_else(|| {
        Box::new(internal_error_response(
            request_id,
            "Owner has no owned object list",
        ))
    })?;
    let pending_access_keys = state.engine.pending_access_keys_snapshot();
    let gas_payment = select_native_gas_payment(
        &owned_objects,
        sender,
        gas_limit.saturating_mul(effective_gas_price(gas_price)),
        gas_limit,
        gas_price,
        &[],
        &pending_access_keys,
    )
    .map_err(|error| {
        Box::new(RpcResponse {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(transaction_error_with_reason(error.to_string())),
            id: request_id,
        })
    })?;
    let nonce = fresh_nonce(nonce, owner_info.nonce)
        .map_err(|error| Box::new(internal_error_response(request_id, error.to_string())))?;

    Ok((gas_payment, nonce))
}

pub async fn handle_build_publish_module(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let build_data: BuildPublishModuleRequest =
        match parse_labeled_params(request.id, &request.params, "build publish data") {
            Ok(data) => data,
            Err(response) => return *response,
        };

    let (gas_payment, nonce) = match select_build_gas_and_nonce(
        state,
        request.id,
        &build_data.sender,
        build_data.gas_limit,
        build_data.gas_price,
        build_data.nonce,
    ) {
        Ok(prepared) => prepared,
        Err(response) => return *response,
    };

    respond_with_serialize(
        request.id,
        PublishModuleRequest {
            sender: build_data.sender,
            module_bytes: build_data.module_bytes,
            module_name: build_data.module_name,
            gas_limit: build_data.gas_limit,
            gas_price: build_data.gas_price,
            nonce: Some(nonce),
            gas_payment: Some(gas_payment),
            signature: None,
            execute_immediate: build_data.execute_immediate,
        },
    )
}

pub async fn handle_build_publish_package(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let build_data: BuildPublishPackageRequest =
        match parse_labeled_params(request.id, &request.params, "build publish package data") {
            Ok(data) => data,
            Err(response) => return *response,
        };

    if build_data.modules.is_empty() {
        return invalid_params_response(request.id, "Package publish requires at least one module");
    }

    let (gas_payment, nonce) = match select_build_gas_and_nonce(
        state,
        request.id,
        &build_data.sender,
        build_data.gas_limit,
        build_data.gas_price,
        build_data.nonce,
    ) {
        Ok(prepared) => prepared,
        Err(response) => return *response,
    };

    respond_with_serialize(
        request.id,
        PublishPackageRequest {
            sender: build_data.sender,
            modules: build_data.modules,
            gas_limit: build_data.gas_limit,
            gas_price: build_data.gas_price,
            nonce: Some(nonce),
            gas_payment: Some(gas_payment),
            signature: None,
            execute_immediate: build_data.execute_immediate,
        },
    )
}
