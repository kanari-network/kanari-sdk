// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Read-only Move entry handling.
//!
//! Object ids supplied in `args` are resolved only at object-parameter
//! positions obtained from the module bytecode. This gives `client view` the
//! same ergonomic Object-ID interface as transaction builders without ever
//! mistaking a pure `address` value for an object.

use super::*;

/// Handle a read-only Move function request.
pub async fn handle_view_function(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let first_param = match first_array_param(request.id, &request.params) {
        Ok(param) => param,
        Err(response) => return *response,
    };
    let view_data: ViewFunctionRequest =
        match parse_labeled_params(request.id, first_param, "view function data") {
            Ok(data) => data,
            Err(response) => return *response,
        };

    if let Err(response) = parse_hex_address(request.id, &view_data.package, "package address") {
        return *response;
    }

    let object_inputs = if let Some(inputs) = view_data.object_inputs.clone() {
        inputs
    } else {
        let object_param_indices = function_object_param_indices(
            state,
            &view_data.package,
            &view_data.module,
            &view_data.function,
        );
        match infer_view_object_inputs(state, &view_data.args, object_param_indices.as_ref()) {
            Ok(inputs) => inputs,
            Err(error) => return invalid_params_response(request.id, error.to_string()),
        }
    };
    if let Err(response) = validate_object_inputs_and_gas(request.id, &object_inputs, None) {
        return *response;
    }
    if let Err(response) = validate_object_inputs_match_state(state, request.id, &object_inputs) {
        return *response;
    }

    info!(
        "Executing view function: {}::{}::{}",
        view_data.package, view_data.module, view_data.function
    );
    match state.engine.execute_view_function(
        &view_data.package,
        &view_data.module,
        &view_data.function,
        &view_data.type_args,
        &view_data.args,
        &object_inputs,
    ) {
        Ok(result) => respond_with_serialize(
            request.id,
            serde_json::json!({
                "status": "success",
                "action": "view",
                "result": result,
            }),
        ),
        Err(error) => {
            error!("View function execution failed: {error}");
            internal_error_response(
                request.id,
                format!("View function execution failed: {error}"),
            )
        }
    }
}
