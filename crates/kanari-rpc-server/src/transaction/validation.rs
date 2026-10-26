// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! RPC boundary validation for object references and gas payments.

use super::*;

pub(crate) fn validate_object_ref_completeness(
    id: u64,
    field: &str,
    object_ref: &ObjectRef,
) -> Result<(), Box<RpcResponse>> {
    if object_ref.version.is_some() ^ object_ref.digest.is_some() {
        return Err(Box::new(invalid_params_response(
            id,
            format!("{field} must include both version and digest when either is provided"),
        )));
    }
    Ok(())
}

fn canonical_rpc_object_id(
    id: u64,
    field: &str,
    object_id: &str,
) -> Result<String, Box<RpcResponse>> {
    AccountAddress::from_hex_literal(object_id)
        .map(|address| object_id_for_address(&address))
        .map_err(|err| {
            Box::new(invalid_params_response(
                id,
                format!("{field} must be a valid object id: {err}"),
            ))
        })
}

pub(crate) fn validate_object_inputs_and_gas(
    id: u64,
    object_inputs: &[kanari_types::transaction::ObjectInput],
    gas_payment: Option<&kanari_types::transaction::GasPayment>,
) -> Result<(), Box<RpcResponse>> {
    let mut mutable_object_ids = HashSet::new();
    for (index, input) in object_inputs.iter().enumerate() {
        validate_object_ref_completeness(
            id,
            &format!("object_inputs[{index}].object_ref"),
            &input.object_ref,
        )?;
        let canonical_object_id = canonical_rpc_object_id(
            id,
            &format!("object_inputs[{index}].object_ref.object_id"),
            &input.object_ref.object_id,
        )?;
        if input.mutable && !mutable_object_ids.insert(canonical_object_id) {
            return Err(Box::new(invalid_params_response(
                id,
                format!(
                    "Duplicate mutable object input {} is not allowed",
                    input.object_ref.object_id
                ),
            )));
        }
    }

    if let Some(gas_payment) = gas_payment {
        let mut gas_object_ids = HashSet::new();
        for (index, payment) in gas_payment.payment_objects.iter().enumerate() {
            validate_object_ref_completeness(
                id,
                &format!("gas_payment.payment_objects[{index}]"),
                payment,
            )?;
            let canonical_payment_id = canonical_rpc_object_id(
                id,
                &format!("gas_payment.payment_objects[{index}].object_id"),
                &payment.object_id,
            )?;
            if !gas_object_ids.insert(canonical_payment_id.clone()) {
                return Err(Box::new(invalid_params_response(
                    id,
                    format!(
                        "Duplicate gas payment object {} is not allowed",
                        payment.object_id
                    ),
                )));
            }
            if mutable_object_ids.contains(&canonical_payment_id) {
                return Err(Box::new(invalid_params_response(
                    id,
                    format!(
                        "Gas payment object {} cannot overlap with a mutable object input",
                        payment.object_id
                    ),
                )));
            }
        }
    }

    Ok(())
}

pub(crate) fn validate_object_inputs_match_state(
    state: &RpcServerState,
    id: u64,
    object_inputs: &[kanari_types::transaction::ObjectInput],
) -> Result<(), Box<RpcResponse>> {
    for input in object_inputs {
        match state.engine.get_object_by_ref(&input.object_ref) {
            Ok(Some(_)) => {}
            Ok(None) => {
                return Err(Box::new(invalid_params_response(
                    id,
                    format!(
                        "Object input {} does not match current state ref",
                        input.object_ref.object_id
                    ),
                )));
            }
            Err(err) => {
                return Err(Box::new(internal_error_response(
                    id,
                    format!(
                        "Failed to validate object input {}: {}",
                        input.object_ref.object_id, err
                    ),
                )));
            }
        }
    }

    Ok(())
}
