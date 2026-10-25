// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Build handler for generic entry-function calls.

use super::*;

pub async fn handle_build_call_function(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let build_data: BuildCallFunctionRequest =
        match parse_labeled_params(request.id, &request.params, "build call data") {
            Ok(data) => data,
            Err(response) => return *response,
        };

    if let Err(response) = parse_hex_address(request.id, &build_data.sender, "sender address") {
        return *response;
    }
    if let Err(response) = parse_hex_address(request.id, &build_data.package, "package address") {
        return *response;
    }
    if let Some(response) =
        invalid_build_gas_response(request.id, build_data.gas_limit, build_data.gas_price)
    {
        return response;
    }

    let Some(owner_info) = state.engine.get_owner_info(&build_data.sender) else {
        return internal_error_response(request.id, "Owner not found");
    };
    let Some(owned_objects) = owner_info.owned_objects else {
        return internal_error_response(request.id, "Owner has no owned object list");
    };

    let object_ref_arg_indices = function_object_param_indices(
        state,
        &build_data.package,
        &build_data.module,
        &build_data.function,
    );
    let object_inputs = if let Some(inputs) = build_data.object_inputs.clone() {
        if let Err(response) = validate_object_inputs_and_gas(request.id, &inputs, None) {
            return *response;
        }
        if let Err(response) = validate_object_inputs_match_state(state, request.id, &inputs) {
            return *response;
        }
        inputs
    } else {
        match infer_object_inputs(
            state,
            &build_data.sender,
            &build_data.args,
            object_ref_arg_indices.as_ref(),
        ) {
            Ok(inputs) => inputs,
            Err(e) => {
                return RpcResponse {
                    jsonrpc: "2.0".into(),
                    result: None,
                    error: Some(transaction_error_with_reason(e.to_string())),
                    id: request.id,
                };
            }
        }
    };
    let exclude_ids = object_inputs
        .iter()
        .filter(|input| input.mutable)
        .map(|input| input.object_ref.object_id.clone())
        .collect::<Vec<_>>();
    let pending_access_keys = state.engine.pending_access_keys_snapshot();
    let burn_amount = build_call_native_burn_amount(&build_data);
    // A native burn consumes from the KANARI balance alongside gas; other
    // coins only ever pay gas, so scope the extra need to KANARI.
    let mut transfer_needs = std::collections::BTreeMap::new();
    if let Some(amount) = burn_amount {
        transfer_needs.insert(GAS_COIN.to_string(), amount);
    }
    let (gas_payment, effective_gas_price_value) = match select_gas_payment_auto(
        &owned_objects,
        &build_data.sender,
        build_data.gas_limit,
        build_data.gas_price,
        &exclude_ids,
        &pending_access_keys,
        &transfer_needs,
    ) {
        Ok(selected) => selected,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(transaction_error_with_reason(e.to_string())),
                id: request.id,
            };
        }
    };
    let nonce = match fresh_nonce(build_data.nonce, owner_info.nonce) {
        Ok(nonce) => nonce,
        Err(e) => return internal_error_response(request.id, e.to_string()),
    };

    respond_with_serialize(
        request.id,
        CallFunctionRequest {
            sender: build_data.sender,
            package: build_data.package,
            module: build_data.module,
            function: build_data.function,
            type_args: build_data.type_args,
            args: build_data.args,
            object_inputs: if object_inputs.is_empty() {
                None
            } else {
                Some(object_inputs)
            },
            gas_limit: build_data.gas_limit,
            gas_price: effective_gas_price_value,
            nonce: Some(nonce),
            gas_payment: Some(gas_payment),
            signature: None,
            execute_immediate: build_data.execute_immediate,
        },
    )
}
