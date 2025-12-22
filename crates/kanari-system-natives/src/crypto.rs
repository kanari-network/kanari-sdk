// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use move_core_types::account_address::AccountAddress;
use move_vm_runtime::native_charge_gas_early_exit;
use move_vm_runtime::native_functions::make_table_from_iter;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::natives::function::PartialVMResult;
use move_vm_types::{
    pop_arg,
    values::{Value, VectorRef},
};
use smallvec::smallvec;

use k256::PublicKey as K256PublicKey;
use k256::ecdsa::{
    Signature as K256Signature, VerifyingKey as K256VerifyingKey,
    signature::Verifier as K256Verifier,
};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use p256::ecdsa::{Signature as P256Signature, VerifyingKey as P256VerifyingKey};
use secp256k1::{
    Message as SecpMessage, PublicKey as SecpPublicKey, Secp256k1,
    ecdsa::RecoverableSignature as SecpRecoverableSignature, ecdsa::RecoveryId as SecpRecoveryId,
};
use sha2::Sha256;
use sha3::{Digest, Keccak256};

use ed25519_dalek::{Signature as EdSignature, VerifyingKey as EdPublicKey};
use move_core_types::gas_algebra::InternalGas;
use std::convert::TryInto;

use crate::make_native;

// Error codes for native crypto functions
const E_INVALID_RECOVERY: u64 = 1;
const E_INVALID_SIGNATURE: u64 = 2;
const E_INVALID_PUBKEY: u64 = 3;
const E_UNSUPPORTED_HASH_FOR_P256: u64 = 4;
const E_INVALID_XONLY_PUBKEY: u64 = 5;
const E_INVALID_MESSAGE: u64 = 6;
const E_INVALID_SCHNORR_SIGNATURE: u64 = 7;

// Maximum message length accepted by natives (prevent large-memory DoS)
const MAX_MSG_BYTES: usize = 1_000_000; // 1 MB

pub fn all_natives(
    move_addr: AccountAddress,
) -> move_vm_runtime::native_functions::NativeFunctionTable {
    let mut natives = vec![];

    // ecdsa_k1::ecrecover(signature: vector<u8>, msg: vector<u8>, hash: u8): vector<u8>
    let ecrecover_native = make_native(
        move |context, _ty_args, mut arguments| -> PartialVMResult<NativeResult> {
            use move_vm_types::natives::function::NativeResult as NR;

            native_charge_gas_early_exit!(context, InternalGas::new(5000));

            // pop in reverse order: hash, msg, signature
            let hash_type: u8 = pop_arg!(arguments, u8);
            let msg_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let signature_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let msg: Vec<u8> = msg_ref.as_bytes_ref().to_vec();
            let signature: Vec<u8> = signature_ref.as_bytes_ref().to_vec();

            // simple gas cost = 0
            // Validate signature length
            if signature.len() != 65 {
                return Ok(NR::err(context.gas_used(), E_INVALID_SIGNATURE));
            }

            // Prevent overly large messages
            if msg.len() > MAX_MSG_BYTES {
                return Ok(NR::err(context.gas_used(), E_INVALID_MESSAGE));
            }

            // hash
            let msg_hash = if hash_type == 0u8 {
                // keccak256
                use sha3::Digest;
                Keccak256::digest(&msg).to_vec()
            } else {
                use sha2::Digest;
                Sha256::digest(&msg).to_vec()
            };

            // Recover: use secp256k1 to recover public key from (r,s,v)
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
            let rec_id = match rec_id {
                Ok(r) => r,
                Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_RECOVERY)),
            };
            let secp_sig = match SecpRecoverableSignature::from_compact(&sig64, rec_id) {
                Ok(s) => s,
                Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_RECOVERY)),
            };
            let secp = Secp256k1::new();
            // Message expects a 32-byte hash. Enforce exact length rather than truncating/padding.
            if msg_hash.len() != 32 {
                return Ok(NR::err(context.gas_used(), E_INVALID_MESSAGE));
            }
            let msg32: [u8; 32] = match msg_hash.try_into() {
                Ok(arr) => arr,
                Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_MESSAGE)),
            };
            // Use `from_digest` to construct a Message from a 32-byte digest (avoids deprecated API)
            let message = SecpMessage::from_digest(msg32);
            let pubkey = match secp.recover_ecdsa(message, &secp_sig) {
                Ok(pk) => pk,
                Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_RECOVERY)),
            };
            // Convert secp public key to compressed bytes (33) and return
            let out = SecpPublicKey::from(pubkey).serialize().to_vec();
            Ok(NR::ok(context.gas_used(), smallvec![Value::vector_u8(out)]))
        },
    );

    // ecdsa_k1::decompress_pubkey(pubkey: vector<u8>): vector<u8>
    let decompress_native = make_native(
        move |context, _ty_args, mut arguments| -> PartialVMResult<NativeResult> {
            use move_vm_types::natives::function::NativeResult as NR;
            native_charge_gas_early_exit!(context, InternalGas::new(1000));
            let pubkey_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let pubkey: Vec<u8> = pubkey_ref.as_bytes_ref().to_vec();

            // Accept compressed (33) or uncompressed (65) and return uncompressed 65
            let pk = match K256PublicKey::from_sec1_bytes(&pubkey) {
                Ok(p) => p,
                Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_PUBKEY)),
            };
            let ep = pk.to_encoded_point(false);
            let out = ep.as_bytes().to_vec();
            Ok(NR::ok(context.gas_used(), smallvec![Value::vector_u8(out)]))
        },
    );

    // ecdsa_k1::verify(signature, public_key, msg, hash) -> bool
    let verify_k1 = make_native(
        move |context, _ty_args, mut arguments| -> PartialVMResult<NativeResult> {
            use move_vm_types::natives::function::NativeResult as NR;
            native_charge_gas_early_exit!(context, InternalGas::new(2000));
            let hash_type: u8 = pop_arg!(arguments, u8);
            let msg_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let public_key_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let signature_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let msg: Vec<u8> = msg_ref.as_bytes_ref().to_vec();
            let public_key: Vec<u8> = public_key_ref.as_bytes_ref().to_vec();
            let signature: Vec<u8> = signature_ref.as_bytes_ref().to_vec();

            // Prevent overly large messages
            if msg.len() > MAX_MSG_BYTES {
                return Ok(NR::err(context.gas_used(), E_INVALID_MESSAGE));
            }

            if signature.is_empty() {
                return Ok(NR::err(context.gas_used(), E_INVALID_SIGNATURE)); // ErrorInvalidSignature
            }

            // Prevent overly large messages
            if msg.len() > MAX_MSG_BYTES {
                return Ok(NR::err(context.gas_used(), E_INVALID_MESSAGE));
            }

            // If signature is 64 bytes it may be Schnorr (x-only public key) or non-recoverable ECDSA.
            if signature.len() == 64 {
                // If a 32-byte public key is provided, treat as Schnorr x-only key.
                if public_key.len() == 32 {
                    // Schnorr requires the message to be exactly 32 bytes in these tests.
                    if msg.len() != 32 {
                        return Ok(NR::err(context.gas_used(), E_INVALID_MESSAGE)); // ErrorInvalidMessage
                    }

                    let msg32: [u8; 32] = match msg.as_slice().try_into() {
                        Ok(a) => a,
                        Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_MESSAGE)),
                    };

                    // parse x-only pubkey and schnorr signature via secp256k1
                    use secp256k1::XOnlyPublicKey as XOnlyPub;
                    use secp256k1::schnorr::Signature as SchnorrSig;

                    // Convert to fixed-size array for from_byte_array
                    let pub_array: [u8; 32] = match public_key.try_into() {
                        Ok(arr) => arr,
                        Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_XONLY_PUBKEY)), // ErrorInvalidXOnlyPubKey (wrong size)
                    };
                    let xpk = match XOnlyPub::from_byte_array(pub_array) {
                        Ok(x) => x,
                        Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_XONLY_PUBKEY)), // ErrorInvalidXOnlyPubKey
                    };

                    // Convert to fixed-size array for from_byte_array
                    let sig_array: [u8; 64] = match signature.try_into() {
                        Ok(arr) => arr,
                        Err(_) => {
                            return Ok(NR::err(context.gas_used(), E_INVALID_SCHNORR_SIGNATURE));
                        } // ErrorInvalidSchnorrSignature (wrong size)
                    };
                    let sch_sig = SchnorrSig::from_byte_array(sig_array);

                    let secp = Secp256k1::new();
                    // secp256k1 crate's schnorr API verifies a 32-byte message
                    let verified = secp.verify_schnorr(&sch_sig, &msg32, &xpk).is_ok();
                    return move_vm_types::natives::function::NativeResult::map_partial_vm_result_one(context.gas_used(), Ok(Value::bool(verified)));
                }

                // If signature is 64 but public key is neither 32 nor a valid compressed/uncompressed length,
                // treat short public keys as invalid x-only pubkeys for schnorr-specific tests.
                if public_key.len() < 33 {
                    return Ok(NR::err(context.gas_used(), E_INVALID_XONLY_PUBKEY)); // ErrorInvalidXOnlyPubKey
                }
            } else {
                // If signature is not 64 but public_key looks like an x-only key, it's an invalid schnorr signature
                if public_key.len() == 32 {
                    return Ok(NR::err(context.gas_used(), E_INVALID_SCHNORR_SIGNATURE)); // ErrorInvalidSchnorrSignature
                }
            }

            // parse pubkey (allow compressed/uncompressed) for ECDSA
            let vk = match K256VerifyingKey::from_sec1_bytes(&public_key) {
                Ok(v) => v,
                Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_PUBKEY)),
            };

            // parse signature: try DER then raw 64
            let sig = if let Ok(s) = K256Signature::from_der(&signature) {
                s
            } else if signature.len() == 64 {
                // try raw 64-bytes signature
                match K256Signature::try_from(&signature[..]) {
                    Ok(s) => s,
                    Err(_) => return Ok(NR::ok(context.gas_used(), smallvec![Value::bool(false)])),
                }
            } else {
                return Ok(NR::ok(context.gas_used(), smallvec![Value::bool(false)]));
            };

            // Use digest-aware verification: verify the hashed message
            let verified = if hash_type == 0u8 {
                // Keccak256
                use k256::ecdsa::signature::DigestVerifier;
                let mut hasher = Keccak256::new();
                hasher.update(&msg);
                vk.verify_digest(hasher, &sig).is_ok()
            } else {
                use k256::ecdsa::signature::DigestVerifier;
                let mut hasher = Sha256::new();
                hasher.update(&msg);
                vk.verify_digest(hasher, &sig).is_ok()
            };

            move_vm_types::natives::function::NativeResult::map_partial_vm_result_one(
                context.gas_used(),
                Ok(Value::bool(verified)),
            )
        },
    );

    // ecdsa_r1 (P-256) verify(signature, public_key, msg, hash) -> bool
    let verify_r1 = make_native(
        move |context, _ty_args, mut arguments| -> PartialVMResult<NativeResult> {
            use move_vm_types::natives::function::NativeResult as NR;
            native_charge_gas_early_exit!(context, InternalGas::new(2000));
            let hash_type: u8 = pop_arg!(arguments, u8);
            let msg_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let public_key_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let signature_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let msg: Vec<u8> = msg_ref.as_bytes_ref().to_vec();
            let public_key: Vec<u8> = public_key_ref.as_bytes_ref().to_vec();
            let signature: Vec<u8> = signature_ref.as_bytes_ref().to_vec();

            if signature.is_empty() {
                return Ok(NR::err(context.gas_used(), E_INVALID_SIGNATURE)); // ErrorInvalidSignature
            }

            // Prevent overly large messages
            if msg.len() > MAX_MSG_BYTES {
                return Ok(NR::err(context.gas_used(), E_INVALID_MESSAGE));
            }

            // Only SHA256 is supported for P-256 in Move wrapper, but accept hash_type selection defensively
            let vk = match P256VerifyingKey::from_sec1_bytes(&public_key) {
                Ok(v) => v,
                Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_PUBKEY)),
            };

            // Disallow Keccak for P-256 usage by default (non-standard).
            if hash_type == 0u8 {
                return Ok(NR::err(context.gas_used(), E_UNSUPPORTED_HASH_FOR_P256)); // ErrorUnsupportedHashForP256
            }

            let sig = if let Ok(s) = P256Signature::from_der(&signature) {
                s
            } else if signature.len() == 64 {
                match P256Signature::try_from(&signature[..]) {
                    Ok(s) => s,
                    Err(_) => return Ok(NR::ok(context.gas_used(), smallvec![Value::bool(false)])),
                }
            } else {
                return Ok(NR::ok(context.gas_used(), smallvec![Value::bool(false)]));
            };

            // Hash then verify via digest-aware API (SHA256 only for P-256)
            let mut hasher = Sha256::new();
            hasher.update(&msg);
            use p256::ecdsa::signature::DigestVerifier;
            let verified = vk.verify_digest(hasher, &sig).is_ok();

            Ok(NR::ok(context.gas_used(), smallvec![Value::bool(verified)]))
        },
    );

    // ed25519::verify(signature, public_key, msg) -> bool
    let ed25519_verify = make_native(
        move |context, _ty_args, mut arguments| -> PartialVMResult<NativeResult> {
            use move_vm_types::natives::function::NativeResult as NR;
            native_charge_gas_early_exit!(context, InternalGas::new(2000));
            // Pop arguments (may return PartialVMError via the macro)
            let msg_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let public_key_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let signature_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let msg: Vec<u8> = msg_ref.as_bytes_ref().to_vec();
            let public_key: Vec<u8> = public_key_ref.as_bytes_ref().to_vec();
            let signature: Vec<u8> = signature_ref.as_bytes_ref().to_vec();

            // Wrap verification in a panic catcher to avoid propagating panics into the VM
            let result = std::panic::catch_unwind(|| {
                if public_key.len() != 32 || signature.len() != 64 {
                    return false;
                }

                let pk_arr: [u8; 32] = match public_key.as_slice().try_into() {
                    Ok(a) => a,
                    Err(_) => return false,
                };
                let pk = match EdPublicKey::from_bytes(&pk_arr) {
                    Ok(p) => p,
                    Err(_) => return false,
                };

                let sig_arr: [u8; 64] = match signature.as_slice().try_into() {
                    Ok(a) => a,
                    Err(_) => return false,
                };
                let sig = EdSignature::from_bytes(&sig_arr);

                pk.verify(&msg, &sig).is_ok()
            });

            let verified = match result {
                Ok(b) => b,
                Err(_) => false,
            };

            Ok(NR::ok(context.gas_used(), smallvec![Value::bool(verified)]))
        },
    );

    // Register functions under module names
    natives.push((
        "ecdsa_k1".to_string(),
        "ecrecover".to_string(),
        ecrecover_native,
    ));
    natives.push((
        "ecdsa_k1".to_string(),
        "decompress_pubkey".to_string(),
        decompress_native,
    ));
    natives.push(("ecdsa_k1".to_string(), "verify".to_string(), verify_k1));
    natives.push((
        "ecdsa_r1".to_string(),
        "native_verify".to_string(),
        verify_r1,
    ));
    natives.push(("ed25519".to_string(), "verify".to_string(), ed25519_verify));

    make_table_from_iter(
        move_addr,
        natives
            .into_iter()
            .map(|(m, f, func)| (m.into_boxed_str(), f.into_boxed_str(), func)),
    )
}
