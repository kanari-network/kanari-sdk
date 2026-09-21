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

use move_core_types::gas_algebra::NumBytes;
use move_core_types::gas_algebra::{InternalGas, InternalGasPerByte};

use crate::crypto::make_native;
use crate::helpers::expect_native_signature;

#[derive(Debug, Clone)]
pub struct DecodeGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

#[derive(Debug, Clone)]
pub struct EncodeGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub decode: DecodeGasParameters,
    pub encode: EncodeGasParameters,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            decode: DecodeGasParameters {
                base: InternalGas::new(0),
                per_byte: InternalGasPerByte::new(0),
            },
            encode: EncodeGasParameters {
                base: InternalGas::new(0),
                per_byte: InternalGasPerByte::new(0),
            },
        }
    }

    pub fn production() -> Self {
        // Calibrated (release bench): base ~ dispatch + setup, per_byte from
        // measured throughput (~1ns/B at anchor 100 units ~= 2ns).
        Self {
            decode: DecodeGasParameters {
                base: InternalGas::new(500),
                per_byte: InternalGasPerByte::new(50),
            },
            encode: EncodeGasParameters {
                base: InternalGas::new(500),
                per_byte: InternalGasPerByte::new(50),
            },
        }
    }
}

/// Native function for base64 decoding
pub fn make_decode_native(gas_params: DecodeGasParameters) -> NativeFunction {
    make_native(move |context, ty_args, mut args| {
        expect_native_signature(args.len(), 1, ty_args.len(), 0)?;

        let input: VectorRef = pop_arg!(args, VectorRef);
        let input_bytes = input.as_bytes_ref().to_vec();
        // Charge by input size BEFORE decoding: decode fans out (~4/3x),
        // and charging after would let out-of-gas escape on huge inputs.
        native_charge_gas_early_exit!(
            context,
            gas_params.base + gas_params.per_byte * NumBytes::new(input_bytes.len() as u64)
        );

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
pub fn make_encode_native(gas_params: EncodeGasParameters) -> NativeFunction {
    make_native(move |context, ty_args, mut args| {
        expect_native_signature(args.len(), 1, ty_args.len(), 0)?;

        let input: VectorRef = pop_arg!(args, VectorRef);
        let input_bytes = input.as_bytes_ref().to_vec();
        native_charge_gas_early_exit!(
            context,
            gas_params.base + gas_params.per_byte * NumBytes::new(input_bytes.len() as u64)
        );

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

#[cfg(test)]
mod benches {
    //! Calibration harness (ignored by default):
    //! `cargo test -p kanari-system-natives --release base64_calibrate -- --ignored --nocapture`
    use super::*;

    #[test]
    #[ignore]
    #[allow(clippy::print_stdout)]
    fn base64_calibrate() {
        use std::hint::black_box;
        use std::time::Instant;

        fn bench(name: &str, iters: u64, mut f: impl FnMut() -> Vec<u8>) {
            for _ in 0..100 {
                black_box(f());
            }
            let t = Instant::now();
            for _ in 0..iters {
                black_box(f());
            }
            println!(
                "{name:28} {:12.1} ns/op",
                t.elapsed().as_nanos() as f64 / iters as f64
            );
        }

        println!("--- base64 calibration ---");
        let small = vec![7u8; 16];
        let medium: Vec<u8> = (0..1024).map(|i| (i % 251) as u8).collect();
        let large: Vec<u8> = (0..65536).map(|i| (i % 251) as u8).collect();
        bench("encode_16B", 50_000, || encode_base64(black_box(&small)));
        bench("encode_1KB", 5_000, || encode_base64(black_box(&medium)));
        bench("encode_64KB", 100, || encode_base64(black_box(&large)));
        let enc_small = encode_base64(&small);
        let enc_medium = encode_base64(&medium);
        let enc_large = encode_base64(&large);
        bench("decode_16B", 50_000, || {
            decode_base64(black_box(&enc_small)).unwrap()
        });
        bench("decode_1KB", 5_000, || {
            decode_base64(black_box(&enc_medium)).unwrap()
        });
        bench("decode_64KB", 100, || {
            decode_base64(black_box(&enc_large)).unwrap()
        });
        // Worst case: garbage input pays for all four engine attempts.
        let garbage = vec![b'!'; 1024];
        bench("decode_invalid_1KB", 5_000, || {
            decode_base64(black_box(&garbage)).unwrap_or_default()
        });
    }
}
