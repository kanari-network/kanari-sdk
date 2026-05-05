// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Base64 encoding/decoding native functions

use move_vm_runtime::native_charge_gas_early_exit;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::{
    pop_arg,
    values::{Value, VectorRef},
};
use smallvec::smallvec;

use move_core_types::gas_algebra::InternalGas;

use crate::crypto::make_native;

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub decode: InternalGas,
    pub encode: InternalGas,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            decode: InternalGas::new(0),
            encode: InternalGas::new(0),
        }
    }
}

/// Native function for base64 decoding
pub fn make_decode_native(gas_cost: InternalGas) -> NativeFunction {
    make_native(move |context, _ty_args, mut args| {
        native_charge_gas_early_exit!(context, gas_cost);

        let input: VectorRef = pop_arg!(args, VectorRef);
        let input_bytes = input.as_bytes_ref().to_vec();

        // Decode base64/base64url
        let result = match decode_base64(&input_bytes) {
            Ok(decoded) => decoded,
            Err(_) => return Ok(NativeResult::err(context.gas_used(), 1)),
        };

        Ok(NativeResult::ok(
            context.gas_used(),
            smallvec![Value::vector_u8(result)],
        ))
    })
}

/// Native function for base64 encoding
pub fn make_encode_native(gas_cost: InternalGas) -> NativeFunction {
    make_native(move |context, _ty_args, mut args| {
        native_charge_gas_early_exit!(context, gas_cost);

        let input: VectorRef = pop_arg!(args, VectorRef);
        let input_bytes = input.as_bytes_ref().to_vec();

        // Encode to base64
        let encoded = encode_base64(&input_bytes);

        Ok(NativeResult::ok(
            context.gas_used(),
            smallvec![Value::vector_u8(encoded)],
        ))
    })
}

/// Decodes base64 or base64url encoded data
fn decode_base64(input: &[u8]) -> Result<Vec<u8>, ()> {
    use base64::{Engine as _, engine::general_purpose};

    // Try base64url first (URL-safe, no padding)
    if let Ok(decoded) = general_purpose::URL_SAFE_NO_PAD.decode(input) {
        return Ok(decoded);
    }

    // Try standard base64 with padding
    if let Ok(decoded) = general_purpose::STANDARD.decode(input) {
        return Ok(decoded);
    }

    // Try base64url with padding
    if let Ok(decoded) = general_purpose::URL_SAFE.decode(input) {
        return Ok(decoded);
    }

    // Try standard base64 without padding
    if let Ok(decoded) = general_purpose::STANDARD_NO_PAD.decode(input) {
        return Ok(decoded);
    }

    Err(())
}

/// Encodes data to base64 (standard encoding with padding)
fn encode_base64(input: &[u8]) -> Vec<u8> {
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD.encode(input).into_bytes()
}

/// Creates the base64 native functions iterator
pub fn make_base64_natives(
    gas_params: GasParameters,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = vec![
        (
            "native_decode".to_string(),
            make_decode_native(gas_params.decode),
        ),
        (
            "native_encode".to_string(),
            make_encode_native(gas_params.encode),
        ),
    ];

    crate::helpers::make_module_natives(natives)
}
