// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Build handlers for native, coin-consolidation, token, and NFT transfers.

use super::*;

fn serialize_move_arg<T: serde::Serialize>(
    id: u64,
    value: &T,
    field: &str,
) -> Result<Vec<u8>, Box<RpcResponse>> {
    bcs::to_bytes(value).map_err(|error| {
        Box::new(internal_error_response(
            id,
            format!("Failed to serialize {field}: {error}"),
        ))
    })
}

pub async fn handle_build_native_transfer(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let build_data: BuildNativeTransferRequest =
        match parse_labeled_params(request.id, &request.params, "build native transfer data") {
            Ok(data) => data,
            Err(response) => return *response,
        };

    if let Err(response) = parse_hex_address(request.id, &build_data.sender, "sender address") {
        return *response;
    }
    if let Err(response) = parse_hex_address(request.id, &build_data.recipient, "recipient") {
        return *response;
    }
    if build_data.amount == 0 {
        return invalid_params_response(request.id, "amount must be non-zero");
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

    let pending_access_keys = state.engine.pending_access_keys_snapshot();
    let (coin_object_ref, gas_payment) = match select_native_transfer_and_gas_payment(
        &owned_objects,
        &build_data.sender,
        build_data.amount,
        build_data.gas_limit,
        build_data.gas_price,
        &pending_access_keys,
        &build_data
            .excluded_object_ids
            .iter()
            .cloned()
            .collect::<HashSet<_>>(),
    ) {
        Ok(payment) => payment,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(transaction_error_with_reason(format!(
                    "{}. Native KANARI transfer needs one coin to cover transfer plus gas, or separate coins for each.",
                    e
                ))),
                id: request.id,
            };
        }
    };
    let nonce = match fresh_nonce(build_data.nonce, owner_info.nonce) {
        Ok(nonce) => nonce,
        Err(e) => return internal_error_response(request.id, e.to_string()),
    };
    // Normalize id spellings so the hash the client signs is computed from
    // the same strings the client submits back (see `canonical_object_id`).
    // Owned-object ids are valid hex by construction (storage index).
    let mut coin_object_ref = coin_object_ref;
    coin_object_ref.object_id = canonical_object_id(&coin_object_ref.object_id)
        .expect("owned object ids are valid hex object addresses");
    let mut gas_payment = gas_payment;
    for payment in &mut gas_payment.payment_objects {
        payment.object_id =
            canonical_object_id(&payment.object_id).expect("owned object ids are valid hex");
    }
    let mut canonical_tx = Transaction::new_transfer_with_object_ref_and_gas(
        build_data.sender.clone(),
        coin_object_ref.clone(),
        build_data.recipient.clone(),
        build_data.amount,
        nonce,
        build_data.gas_limit,
        build_data.gas_price,
    );
    if let Transaction::ExecuteFunction {
        gas_payment: canonical_gas_payment,
        ..
    } = &mut canonical_tx
    {
        *canonical_gas_payment = Some(gas_payment.clone());
    }

    respond_with_serialize(
        request.id,
        ObjectTransferData {
            sender: build_data.sender,
            coin_object_id: coin_object_ref.object_id.clone(),
            coin_object_ref: Some(coin_object_ref),
            recipient: build_data.recipient,
            amount: build_data.amount,
            gas_limit: build_data.gas_limit,
            gas_price: build_data.gas_price,
            nonce: Some(nonce),
            gas_payment: Some(gas_payment),
            signature: None,
            transaction_hash: Some(canonical_tx.hash()),
            execute_immediate: build_data.execute_immediate,
        },
    )
}

pub async fn handle_build_native_coin_consolidation(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let build_data: BuildNativeCoinConsolidationRequest = match parse_labeled_params(
        request.id,
        &request.params,
        "build native coin consolidation data",
    ) {
        Ok(data) => data,
        Err(response) => return *response,
    };

    if let Err(response) = parse_hex_address(request.id, &build_data.sender, "sender address") {
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

    let (primary_object, merge_object, gas_payment) = match select_native_coin_consolidation_step(
        &owned_objects,
        &build_data.sender,
        build_data.required_amount,
        build_data.gas_limit,
        build_data.gas_price,
    ) {
        Ok(selection) => selection,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(transaction_error_with_reason(e.to_string())),
                id: request.id,
            };
        }
    };

    let primary_input = match build_object_input(&primary_object, &build_data.sender) {
        Ok(input) => input,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(transaction_error_with_reason(e.to_string())),
                id: request.id,
            };
        }
    };
    let merge_input = match build_object_input(&merge_object, &build_data.sender) {
        Ok(input) => input,
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
    let primary_id = match parse_hex_address(request.id, &primary_object.id, "primary object id") {
        Ok(address) => address.to_vec(),
        Err(response) => return *response,
    };
    let merge_id = match parse_hex_address(request.id, &merge_object.id, "merge object id") {
        Ok(address) => address.to_vec(),
        Err(response) => return *response,
    };

    respond_with_serialize(
        request.id,
        CallFunctionRequest {
            sender: build_data.sender,
            package: Address::KANARI_SYSTEM_ADDRESS.to_string(),
            module: CoinModule::COIN_MODULE.to_string(),
            function: CoinModule::function_names().join_entry.to_string(),
            type_args: vec![GAS_COIN.to_string()],
            args: vec![primary_id, merge_id],
            object_inputs: Some(vec![primary_input, merge_input]),
            gas_limit: build_data.gas_limit,
            gas_price: build_data.gas_price,
            nonce: Some(nonce),
            gas_payment: Some(gas_payment),
            signature: None,
            execute_immediate: build_data.execute_immediate,
        },
    )
}

pub async fn handle_build_token_transfer(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let build_data: BuildTokenTransferRequest =
        match parse_labeled_params(request.id, &request.params, "build token transfer data") {
            Ok(data) => data,
            Err(response) => return *response,
        };

    if let Err(response) = parse_hex_address(request.id, &build_data.sender, "sender address") {
        return *response;
    }
    let recipient = match parse_hex_address(request.id, &build_data.recipient, "recipient") {
        Ok(address) => address,
        Err(response) => return *response,
    };
    if build_data.amount == 0 {
        return invalid_params_response(request.id, "amount must be non-zero");
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

    let coin_object_ref = match select_coin_object_for_token(
        &owned_objects,
        &build_data.token_type,
        build_data.amount,
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
    let object = match state.engine.get_object_by_ref(&coin_object_ref) {
        Ok(Some(object)) => object,
        Ok(None) => return internal_error_response(request.id, "Selected token object not found"),
        Err(e) => return internal_error_response(request.id, e.to_string()),
    };
    let object_input = match build_object_input(&object, &build_data.sender) {
        Ok(input) => input,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(transaction_error_with_reason(e.to_string())),
                id: request.id,
            };
        }
    };
    // The transferred tokens come from the selected coin object; any gas coin
    // the wallet can afford may pay for execution.
    let mut transfer_needs = std::collections::BTreeMap::new();
    transfer_needs.insert(
        CoinModule::normalize_token_type(&build_data.token_type),
        build_data.amount,
    );
    let (gas_payment, effective_gas_price_value) = match select_gas_payment_auto(
        &owned_objects,
        &build_data.sender,
        build_data.gas_limit,
        build_data.gas_price,
        std::slice::from_ref(&coin_object_ref.object_id),
        &state.engine.pending_access_keys_snapshot(),
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

    let module_parts: Vec<&str> = build_data.token_type.split("::").collect();
    if module_parts.len() < 3 {
        return invalid_params_response(
            request.id,
            "Invalid token type format. Expected address::module::struct",
        );
    }
    let nonce = match fresh_nonce(build_data.nonce, owner_info.nonce) {
        Ok(nonce) => nonce,
        Err(e) => return internal_error_response(request.id, e.to_string()),
    };
    let coin_object_id =
        match parse_hex_address(request.id, &coin_object_ref.object_id, "coin object id") {
            Ok(address) => address.to_vec(),
            Err(response) => return *response,
        };
    let amount = match serialize_move_arg(request.id, &build_data.amount, "transfer amount") {
        Ok(amount) => amount,
        Err(response) => return *response,
    };
    let recipient = match serialize_move_arg(
        request.id,
        &recipient.to_account_address(),
        "recipient address",
    ) {
        Ok(recipient) => recipient,
        Err(response) => return *response,
    };

    respond_with_serialize(
        request.id,
        CallFunctionRequest {
            sender: build_data.sender,
            package: module_parts[0].to_string(),
            module: module_parts[1].to_string(),
            // Token modules expose the amount-based transfer entry point.
            // Keep the coin selection and gas policy in the RPC layer while
            // passing only the canonical Move call to the client.
            function: UsdModule::function_names().transfer_amount.to_string(),
            type_args: vec![],
            args: vec![coin_object_id, amount, recipient],
            object_inputs: Some(vec![object_input]),
            gas_limit: build_data.gas_limit,
            gas_price: effective_gas_price_value,
            nonce: Some(nonce),
            gas_payment: Some(gas_payment),
            signature: None,
            execute_immediate: build_data.execute_immediate,
        },
    )
}

/// Handle build NFT transfer request using the framework transfer entry for all
/// key+store NFT object types.
pub async fn handle_build_nft_transfer(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let build_data: BuildNftTransferRequest =
        match parse_labeled_params(request.id, &request.params, "build NFT transfer data") {
            Ok(data) => data,
            Err(response) => return *response,
        };

    if let Err(response) = parse_hex_address(request.id, &build_data.sender, "sender address") {
        return *response;
    }
    let recipient = match parse_hex_address(request.id, &build_data.recipient, "recipient") {
        Ok(address) => address,
        Err(response) => return *response,
    };
    if build_data.object_id.trim().is_empty() {
        return invalid_params_response(request.id, "object_id is required");
    }
    if normalize_addr(&build_data.object_id) == normalize_addr(&build_data.recipient) {
        return invalid_params_response(request.id, "recipient must differ from the NFT object id");
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

    // The NFT must already be owned by the sender; look it up in the owner's
    // object list so version/digest come from the same snapshot as gas.
    let owned_nft = owned_objects
        .iter()
        .find(|object| normalize_addr(&object.id) == normalize_addr(&build_data.object_id))
        .cloned();
    let Some(owned_nft) = owned_nft else {
        return invalid_params_response(
            request.id,
            format!(
                "NFT object {} not found in sender's owned object list",
                build_data.object_id
            ),
        );
    };
    let nft_input = match build_object_input(&owned_nft, &build_data.sender) {
        Ok(input) => input,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(transaction_error_with_reason(e.to_string())),
                id: request.id,
            };
        }
    };

    let package = Address::KANARI_SYSTEM_ADDRESS.to_string();
    let module = TransferModule::TRANSFER_MODULE.to_string();
    let function = TransferModule::ENTRY_TRANSFER.to_string();
    let type_args = vec![owned_nft.type_.clone()];
    let mut args = vec![vec![]];
    let object_inputs = vec![nft_input];

    // Any affordable wallet coin can pay gas; the NFT stays excluded.
    let (gas_payment, effective_gas_price_value) = match select_gas_payment_auto(
        &owned_objects,
        &build_data.sender,
        build_data.gas_limit,
        build_data.gas_price,
        std::slice::from_ref(&owned_nft.id),
        &state.engine.pending_access_keys_snapshot(),
        &std::collections::BTreeMap::new(),
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
    let recipient = match serialize_move_arg(
        request.id,
        &recipient.to_account_address(),
        "recipient address",
    ) {
        Ok(recipient) => recipient,
        Err(response) => return *response,
    };
    // Object args use empty BCS slots; object refs are supplied separately.
    args.push(recipient);

    respond_with_serialize(
        request.id,
        CallFunctionRequest {
            sender: build_data.sender,
            // The canonical transfer lives in the framework package; the
            // entry wrapper `transfer<T: key + store>(obj, recipient)` is the
            // callable form — transactions only execute entry functions, and
            // the plain `public_transfer` fun aborts with
            // EXECUTE_ENTRY_FUNCTION_CALLED_ON_NON_ENTRY_FUNCTION. It takes no
            // context and binds `T` as the single type argument.
            package,
            module,
            function,
            type_args,
            args,
            object_inputs: Some(object_inputs),
            gas_limit: build_data.gas_limit,
            gas_price: effective_gas_price_value,
            nonce: Some(nonce),
            gas_payment: Some(gas_payment),
            signature: None,
            execute_immediate: build_data.execute_immediate,
        },
    )
}
