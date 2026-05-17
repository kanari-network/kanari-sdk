// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! ECDSA secp256k1 (K1) native functions

use move_vm_runtime::native_charge_gas_early_exit;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::natives::function::{NativeResult, PartialVMResult};
use move_vm_types::{
    pop_arg,
    values::{Value, VectorRef},
};
use smallvec::smallvec;

use k256::PublicKey as K256PublicKey;
use k256::ecdsa::{
    Signature as K256Signature, VerifyingKey as K256VerifyingKey,
    signature::hazmat::PrehashVerifier as K256PrehashVerifier,
};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use secp256k1::{
    Message as SecpMessage, Secp256k1, XOnlyPublicKey,
    ecdsa::RecoverableSignature as SecpRecoverableSignature, ecdsa::RecoveryId as SecpRecoveryId,
    schnorr::Signature as SchnorrSig,
};
use sha2::{Digest, Sha256};
use sha3::Keccak256;

use std::convert::TryInto;

use move_core_types::gas_algebra::InternalGas;

use crate::crypto::make_native;
use crate::helpers::expect_native_signature;

// Error codes for ECDSA K1 native functions
pub const E_INVALID_RECOVERY: u64 = 1;
pub const E_INVALID_SIGNATURE: u64 = 2;
pub const E_INVALID_PUBKEY: u64 = 3;
pub const E_INVALID_XONLY_PUBKEY: u64 = 5;
pub const E_INVALID_MESSAGE: u64 = 6;
pub const E_INVALID_SCHNORR_SIGNATURE: u64 = 7;

// Maximum message length accepted by natives (prevent large-memory DoS)
pub const MAX_MSG_BYTES: usize = 1_000_000; // 1 MB

/// Creates the ecdsa_k1::ecrecover native function
pub fn make_ecrecover_native(gas_cost: InternalGas) -> NativeFunction {
    make_native(
        move |context, ty_args, mut arguments| -> PartialVMResult<NativeResult> {
            use move_vm_types::natives::function::NativeResult as NR;

            native_charge_gas_early_exit!(context, gas_cost);
            expect_native_signature(arguments.len(), 3, ty_args.len(), 0)?;

            // pop in reverse order: hash, msg, signature
            let hash_type: u8 = pop_arg!(arguments, u8);
            let msg_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let signature_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let msg: Vec<u8> = msg_ref.as_bytes_ref().to_vec();
            let signature: Vec<u8> = signature_ref.as_bytes_ref().to_vec();

            // Validate signature length
            if signature.len() != 65 {
                return Ok(NR::err(context.gas_used(), E_INVALID_SIGNATURE));
            }

            // Prevent overly large messages
            if msg.len() > MAX_MSG_BYTES {
                return Ok(NR::err(context.gas_used(), E_INVALID_MESSAGE));
            }

            // Hash the message
            let msg_hash = match compute_message_hash(&msg, hash_type) {
                Ok(hash) => hash,
                Err(error_code) => return Ok(NR::err(context.gas_used(), error_code)),
            };

            // Recover public key from signature
            match recover_public_key(&signature, &msg_hash) {
                Ok(pubkey_bytes) => Ok(NR::ok(
                    context.gas_used(),
                    smallvec![Value::vector_u8(pubkey_bytes)],
                )),
                Err(error_code) => Ok(NR::err(context.gas_used(), error_code)),
            }
        },
    )
}

/// Computes the hash of a message based on the hash type
fn compute_message_hash(msg: &[u8], hash_type: u8) -> Result<Vec<u8>, u64> {
    if hash_type == 0u8 {
        // keccak256
        use sha3::Digest;
        Ok(Keccak256::digest(msg).to_vec())
    } else {
        // SHA256
        use sha2::Digest;
        Ok(Sha256::digest(msg).to_vec())
    }
}

/// Recovers the public key from a signature and message hash
fn recover_public_key(signature: &[u8], msg_hash: &[u8]) -> Result<Vec<u8>, u64> {
    // Validate message hash length (must be 32 bytes)
    if msg_hash.len() != 32 {
        return Err(E_INVALID_MESSAGE);
    }

    // Extract recovery ID and signature components
    let mut sig64 = [0u8; 64];
    sig64.copy_from_slice(&signature[0..64]);
    let v = signature[64];

    // RecoveryId: accept 0..=3 or legacy 27/28 values; reject others
    let rec_id = if v <= 3 {
        SecpRecoveryId::try_from(v as i32)
    } else if v == 27 || v == 28 {
        SecpRecoveryId::try_from((v - 27) as i32)
    } else {
        Err(secp256k1::Error::InvalidSignature)
    };

    let rec_id = rec_id.map_err(|_| E_INVALID_RECOVERY)?;
    let secp_sig =
        SecpRecoverableSignature::from_compact(&sig64, rec_id).map_err(|_| E_INVALID_RECOVERY)?;

    // Create message from digest
    let msg32: [u8; 32] = msg_hash.try_into().map_err(|_| E_INVALID_MESSAGE)?;
    let message = SecpMessage::from_digest(msg32);

    // Recover public key
    let secp = Secp256k1::new();
    let pubkey = secp
        .recover_ecdsa(message, &secp_sig)
        .map_err(|_| E_INVALID_RECOVERY)?;

    // Convert to compressed bytes (33 bytes)
    Ok(pubkey.serialize().to_vec())
}

/// Creates the ecdsa_k1::decompress_pubkey native function
pub fn make_decompress_pubkey_native(gas_cost: InternalGas) -> NativeFunction {
    make_native(
        move |context, ty_args, mut arguments| -> PartialVMResult<NativeResult> {
            use move_vm_types::natives::function::NativeResult as NR;
            native_charge_gas_early_exit!(context, gas_cost);
            expect_native_signature(arguments.len(), 1, ty_args.len(), 0)?;

            let pubkey_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let mut pubkey: Vec<u8> = pubkey_ref.as_bytes_ref().to_vec();

            // Accept 64-byte uncompressed X||Y (missing 0x04 prefix) and normalize to SEC1.
            if pubkey.len() == 64 {
                let mut prefixed = Vec::with_capacity(65);
                prefixed.push(0x04);
                prefixed.extend_from_slice(&pubkey);
                pubkey = prefixed;
            }

            // Parse and decompress the public key
            let pk = match K256PublicKey::from_sec1_bytes(&pubkey) {
                Ok(p) => p,
                Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_PUBKEY)),
            };

            // Convert to uncompressed format (65 bytes)
            let ep = pk.to_encoded_point(false);
            let out = ep.as_bytes().to_vec();

            Ok(NR::ok(context.gas_used(), smallvec![Value::vector_u8(out)]))
        },
    )
}

/// Creates the ecdsa_k1::verify native function
pub fn make_verify_k1_native(gas_cost: InternalGas) -> NativeFunction {
    make_native(
        move |context, ty_args, mut arguments| -> PartialVMResult<NativeResult> {
            use move_vm_types::natives::function::NativeResult as NR;
            native_charge_gas_early_exit!(context, gas_cost);
            expect_native_signature(arguments.len(), 4, ty_args.len(), 0)?;

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

            // Try Schnorr verification first (64-byte signature with 32-byte x-only pubkey)
            if signature.len() == 64 && public_key.len() == 32 {
                return verify_schnorr_signature(context, &msg, &public_key, &signature);
            }

            // Handle invalid x-only pubkey scenarios
            if signature.len() == 64 && public_key.len() < 33 {
                return Ok(NR::err(context.gas_used(), E_INVALID_XONLY_PUBKEY));
            }

            if signature.len() != 64 && public_key.len() == 32 {
                return Ok(NR::err(context.gas_used(), E_INVALID_SCHNORR_SIGNATURE));
            }

            // Normalize public key encoding
            normalize_secp256k1_pubkey(&mut public_key);

            // Perform ECDSA verification with proper error handling
            match verify_ecdsa_signature(&public_key, &signature, &msg, hash_type) {
                Ok(verified) => Ok(NR::ok(context.gas_used(), smallvec![Value::bool(verified)])),
                Err(error_code) => Ok(NR::err(context.gas_used(), error_code)),
            }
        },
    )
}

/// Verifies a Schnorr signature
fn verify_schnorr_signature(
    context: &mut NativeContext,
    msg: &[u8],
    public_key: &[u8],
    signature: &[u8],
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    // Schnorr requires exactly 32-byte message
    if msg.len() != 32 {
        return Ok(NR::err(context.gas_used(), E_INVALID_MESSAGE));
    }

    let msg32: [u8; 32] = match msg.try_into() {
        Ok(a) => a,
        Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_MESSAGE)),
    };

    // Parse x-only public key
    let pub_array: [u8; 32] = match public_key.try_into() {
        Ok(arr) => arr,
        Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_XONLY_PUBKEY)),
    };

    let xpk = match XOnlyPublicKey::from_byte_array(pub_array) {
        Ok(x) => x,
        Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_XONLY_PUBKEY)),
    };

    // Parse Schnorr signature
    let sig_array: [u8; 64] = match signature.try_into() {
        Ok(arr) => arr,
        Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_SCHNORR_SIGNATURE)),
    };

    let sch_sig = SchnorrSig::from_byte_array(sig_array);
    let secp = Secp256k1::new();
    let verified = secp.verify_schnorr(&sch_sig, &msg32, &xpk).is_ok();

    NR::map_partial_vm_result_one(context.gas_used(), Ok(Value::bool(verified)))
}

/// Normalizes secp256k1 public key encoding
fn normalize_secp256k1_pubkey(public_key: &mut Vec<u8>) {
    // If 64 bytes (X||Y without 0x04 prefix), add the prefix
    if public_key.len() == 64 {
        let mut prefixed = Vec::with_capacity(65);
        prefixed.push(0x04);
        prefixed.extend_from_slice(public_key);
        *public_key = prefixed;
    }
}

/// Verifies an ECDSA signature and returns error code on failure
fn verify_ecdsa_signature(
    public_key: &[u8],
    signature: &[u8],
    msg: &[u8],
    hash_type: u8,
) -> Result<bool, u64> {
    // Parse public key - return ErrorInvalidPubKey if invalid
    let vk = match K256VerifyingKey::from_sec1_bytes(public_key) {
        Ok(v) => v,
        Err(_) => return Err(E_INVALID_PUBKEY),
    };

    // Normalize signature (handle 65-byte r||s||v format)
    let sig_bytes = if signature.len() == 65 {
        &signature[..64]
    } else {
        signature
    };

    // Parse signature: try DER then raw 64 bytes
    let sig = if let Ok(s) = K256Signature::from_der(signature) {
        s
    } else if sig_bytes.len() == 64 {
        match K256Signature::try_from(sig_bytes) {
            Ok(s) => s,
            Err(_) => {
                log::debug!("Failed to parse signature as raw 64 bytes");
                return Err(E_INVALID_SIGNATURE);
            }
        }
    } else {
        log::debug!("Invalid signature length: {}", signature.len());
        return Err(E_INVALID_SIGNATURE);
    };

    // Hash and verify
    let result = if hash_type == 0u8 {
        // Keccak256
        let msg_hash = Keccak256::digest(msg);
        vk.verify_prehash(msg_hash.as_slice(), &sig).is_ok()
    } else {
        // SHA256
        let msg_hash = Sha256::digest(msg);
        vk.verify_prehash(msg_hash.as_slice(), &sig).is_ok()
    };

    Ok(result)
}
