// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! RSASSA-PKCS1-V1_5 with SHA-256 (RS256) native functions

use move_vm_runtime::native_charge_gas_early_exit;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::{
    pop_arg,
    values::{Value, VectorRef},
};
use sha2::Digest;
use smallvec::smallvec;

use rsa::BigUint;
use rsa::RsaPublicKey;
use rsa::pkcs1v15::{Signature, VerifyingKey};

use sha2::Sha256;

use move_core_types::gas_algebra::InternalGas;

use crate::crypto::make_native;

// Error codes for RS256 native functions
pub const E_INVALID_SIGNATURE: u64 = 1;
pub const E_INVALID_PUBKEY: u64 = 2;
pub const E_INVALID_HASH_TYPE: u64 = 3;
pub const E_INVALID_MESSAGE_LENGTH: u64 = 4;

// Constants from Move module
const RSASSA_PKCS1_V1_5_MINIMUM_MODULUS_LENGTH: u64 = 2048; // bits
const RSASSA_PKCS1_V1_5_MINIMUM_EXPONENT_LENGTH: u64 = 1; // bytes
const RSASSA_PKCS1_V1_5_MAXIMUM_EXPONENT_LENGTH: u64 = 512; // bytes
const SHA256_MESSAGE_LENGTH: u64 = 32; // bytes
const SHA256_HASH_TYPE: u8 = 0;

// Maximum message length accepted by natives (prevent large-memory DoS)
const MAX_MSG_BYTES: usize = 1_000_000; // 1 MB

/// Verifies RS256 signature using RSA public key components (n, e)
fn verify_rs256(n_bytes: &[u8], e_bytes: &[u8], msg_hash: &[u8], sig_bytes: &[u8]) -> bool {
    // Construct RSA public key from modulus (n) and exponent (e)
    let pubkey = match RsaPublicKey::new(
        BigUint::from_bytes_be(n_bytes),
        BigUint::from_bytes_be(e_bytes),
    ) {
        Ok(key) => key,
        Err(_) => return false,
    };

    // Create verifying key with SHA256 digest
    // Note: We use the sha2 from rsa's re-export to avoid version conflicts
    let verifying_key = VerifyingKey::<rsa::sha2::Sha256>::new(pubkey);

    // Parse signature
    let signature = match Signature::try_from(sig_bytes) {
        Ok(sig) => sig,
        Err(_) => return false,
    };

    // Verify the signature against the pre-hashed message
    use rsa::signature::hazmat::PrehashVerifier;
    verifying_key.verify_prehash(msg_hash, &signature).is_ok()
}

/// Native function for RS256 signature verification (with internal hashing)
pub fn make_verify_native(gas_cost: InternalGas) -> NativeFunction {
    make_native(move |context, _ty_args, mut args| {
        native_charge_gas_early_exit!(context, gas_cost);

        let msg: VectorRef = pop_arg!(args, VectorRef);
        let e: VectorRef = pop_arg!(args, VectorRef);
        let n: VectorRef = pop_arg!(args, VectorRef);
        let signature: VectorRef = pop_arg!(args, VectorRef);

        let msg_bytes = msg.as_bytes_ref().to_vec();
        let e_bytes = e.as_bytes_ref().to_vec();
        let n_bytes = n.as_bytes_ref().to_vec();
        let sig_bytes = signature.as_bytes_ref().to_vec();

        // Validate inputs
        if msg_bytes.len() > MAX_MSG_BYTES {
            return Ok(NativeResult::err(
                context.gas_used(),
                E_INVALID_MESSAGE_LENGTH,
            ));
        }

        // Check modulus length (must be >= 2048 bits = 256 bytes)
        if n_bytes.len() < (RSASSA_PKCS1_V1_5_MINIMUM_MODULUS_LENGTH / 8) as usize {
            return Ok(NativeResult::err(context.gas_used(), E_INVALID_PUBKEY));
        }

        // Check exponent length
        if e_bytes.len() < RSASSA_PKCS1_V1_5_MINIMUM_EXPONENT_LENGTH as usize
            || e_bytes.len() > RSASSA_PKCS1_V1_5_MAXIMUM_EXPONENT_LENGTH as usize
        {
            return Ok(NativeResult::err(context.gas_used(), E_INVALID_PUBKEY));
        }

        // Signature length should match modulus length
        if sig_bytes.len() != n_bytes.len() {
            return Ok(NativeResult::err(context.gas_used(), E_INVALID_SIGNATURE));
        }

        // Hash the message with SHA256
        let hashed_msg = Sha256::digest(&msg_bytes);

        // Verify RS256 signature
        let result = verify_rs256(&n_bytes, &e_bytes, &hashed_msg, &sig_bytes);

        Ok(NativeResult::ok(
            context.gas_used(),
            smallvec![Value::bool(result)],
        ))
    })
}

/// Native function for RS256 signature verification (pre-hashed message)
pub fn make_verify_prehash_native(gas_cost: InternalGas) -> NativeFunction {
    make_native(move |context, _ty_args, mut args| {
        native_charge_gas_early_exit!(context, gas_cost);

        let hash_type: u8 = pop_arg!(args, u8);
        let msg: VectorRef = pop_arg!(args, VectorRef);
        let e: VectorRef = pop_arg!(args, VectorRef);
        let n: VectorRef = pop_arg!(args, VectorRef);
        let signature: VectorRef = pop_arg!(args, VectorRef);

        let msg_bytes = msg.as_bytes_ref().to_vec();
        let e_bytes = e.as_bytes_ref().to_vec();
        let n_bytes = n.as_bytes_ref().to_vec();
        let sig_bytes = signature.as_bytes_ref().to_vec();

        // Validate hash type (only SHA256 supported)
        if hash_type != SHA256_HASH_TYPE {
            return Ok(NativeResult::err(context.gas_used(), E_INVALID_HASH_TYPE));
        }

        // Validate message length (must be 32 bytes for SHA256)
        if msg_bytes.len() != SHA256_MESSAGE_LENGTH as usize {
            return Ok(NativeResult::err(
                context.gas_used(),
                E_INVALID_MESSAGE_LENGTH,
            ));
        }

        // Check modulus length (must be >= 2048 bits = 256 bytes)
        if n_bytes.len() < (RSASSA_PKCS1_V1_5_MINIMUM_MODULUS_LENGTH / 8) as usize {
            return Ok(NativeResult::err(context.gas_used(), E_INVALID_PUBKEY));
        }

        // Check exponent length
        if e_bytes.len() < RSASSA_PKCS1_V1_5_MINIMUM_EXPONENT_LENGTH as usize
            || e_bytes.len() > RSASSA_PKCS1_V1_5_MAXIMUM_EXPONENT_LENGTH as usize
        {
            return Ok(NativeResult::err(context.gas_used(), E_INVALID_PUBKEY));
        }

        // Signature length should match modulus length
        if sig_bytes.len() != n_bytes.len() {
            return Ok(NativeResult::err(context.gas_used(), E_INVALID_SIGNATURE));
        }

        // Verify signature with pre-hashed message
        let result = verify_rs256(&n_bytes, &e_bytes, &msg_bytes, &sig_bytes);

        Ok(NativeResult::ok(
            context.gas_used(),
            smallvec![Value::bool(result)],
        ))
    })
}

/// Creates the rs256 native functions iterator
pub fn make_rs256_natives(gas_cost: InternalGas) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = vec![
        ("native_verify".to_string(), make_verify_native(gas_cost)),
        (
            "native_verify_prehash".to_string(),
            make_verify_prehash_native(gas_cost),
        ),
    ];

    crate::helpers::make_module_natives(natives)
}
