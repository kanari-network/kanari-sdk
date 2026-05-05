// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! ECDSA P-256 (R1) native functions

use move_vm_runtime::native_charge_gas_early_exit;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::natives::function::{NativeResult, PartialVMResult};
use move_vm_types::{
    pop_arg,
    values::{Value, VectorRef},
};
use smallvec::smallvec;

use p256::ecdsa::{Signature as P256Signature, VerifyingKey as P256VerifyingKey};
use sha2::{Digest, Sha256};

use move_core_types::gas_algebra::InternalGas;

use crate::crypto::make_native;

// Error codes for ECDSA R1 native functions
pub const E_INVALID_SIGNATURE: u64 = 2;
pub const E_INVALID_PUBKEY: u64 = 3;
pub const E_UNSUPPORTED_HASH_FOR_P256: u64 = 4;
pub const E_INVALID_MESSAGE: u64 = 6;

// Maximum message length accepted by natives (prevent large-memory DoS)
const MAX_MSG_BYTES: usize = 1_000_000; // 1 MB

/// Creates the ecdsa_r1 native functions iterator
pub fn make_ecdsa_r1_natives(
    gas_cost: InternalGas,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = vec![("native_verify".to_string(), make_verify_r1_native(gas_cost))];

    crate::helpers::make_module_natives(natives)
}

/// Creates the ecdsa_r1::native_verify native function
fn make_verify_r1_native(gas_cost: InternalGas) -> NativeFunction {
    make_native(
        move |context, _ty_args, mut arguments| -> PartialVMResult<NativeResult> {
            use move_vm_types::natives::function::NativeResult as NR;
            native_charge_gas_early_exit!(context, gas_cost);

            let hash_type: u8 = pop_arg!(arguments, u8);
            let msg_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let public_key_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let signature_ref: VectorRef = pop_arg!(arguments, VectorRef);

            let msg: Vec<u8> = msg_ref.as_bytes_ref().to_vec();
            let mut public_key: Vec<u8> = public_key_ref.as_bytes_ref().to_vec();
            let signature: Vec<u8> = signature_ref.as_bytes_ref().to_vec();

            // Validate inputs
            if signature.is_empty() {
                return Ok(NR::err(context.gas_used(), E_INVALID_SIGNATURE));
            }

            if msg.len() > MAX_MSG_BYTES {
                return Ok(NR::err(context.gas_used(), E_INVALID_MESSAGE));
            }

            // Normalize P-256 pubkey encoding
            if public_key.len() == 64 {
                let mut prefixed = Vec::with_capacity(65);
                prefixed.push(0x04);
                prefixed.extend_from_slice(&public_key);
                public_key = prefixed;
            }

            // Disallow Keccak for P-256 (non-standard)
            if hash_type == 0u8 {
                return Ok(NR::err(context.gas_used(), E_UNSUPPORTED_HASH_FOR_P256));
            }

            // Parse public key
            let vk = match P256VerifyingKey::from_sec1_bytes(&public_key) {
                Ok(v) => v,
                Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_PUBKEY)),
            };

            // Normalize and parse signature
            let sig_bytes = if signature.len() == 65 {
                &signature[..64]
            } else {
                signature.as_slice()
            };

            let sig = if let Ok(s) = P256Signature::from_der(&signature) {
                s
            } else if sig_bytes.len() == 64 {
                match P256Signature::try_from(sig_bytes) {
                    Ok(s) => s,
                    Err(_) => return Ok(NR::ok(context.gas_used(), smallvec![Value::bool(false)])),
                }
            } else {
                return Ok(NR::ok(context.gas_used(), smallvec![Value::bool(false)]));
            };

            // Hash with SHA256 and verify
            let msg_hash = Sha256::digest(&msg);
            use p256::ecdsa::signature::hazmat::PrehashVerifier;
            let verified = vk.verify_prehash(msg_hash.as_slice(), &sig).is_ok();

            Ok(NR::ok(context.gas_used(), smallvec![Value::bool(verified)]))
        },
    )
}
