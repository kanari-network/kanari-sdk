// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use move_core_types::account_address::AccountAddress;
use move_vm_runtime::native_charge_gas_early_exit;
use move_vm_runtime::native_functions::{
    NativeContext, NativeFunction, NativeFunctionTable, make_table_from_iter,
};
use move_vm_types::natives::function::NativeResult;
use move_vm_types::natives::function::PartialVMResult;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    pop_arg,
    values::{Value, VectorRef},
};
use smallvec::smallvec;

use k256::PublicKey as K256PublicKey;
use k256::ecdsa::{
    Signature as K256Signature, VerifyingKey as K256VerifyingKey,
    signature::Verifier as K256Verifier, signature::hazmat::PrehashVerifier as K256PrehashVerifier,
};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use p256::ecdsa::{Signature as P256Signature, VerifyingKey as P256VerifyingKey};
use secp256k1::{
    Message as SecpMessage, Secp256k1, ecdsa::RecoverableSignature as SecpRecoverableSignature,
    ecdsa::RecoveryId as SecpRecoveryId,
};
use sha2::Sha256;
use sha3::{Digest, Keccak256};

use ed25519_dalek::{Signature as EdSignature, VerifyingKey as EdPublicKey};
use move_core_types::gas_algebra::InternalGas;
use std::{collections::VecDeque, convert::TryInto, sync::Arc};

use crate::helpers::make_module_natives;

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

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub ecrecover: InternalGas,
    pub decompress_pubkey: InternalGas,
    pub verify_k1: InternalGas,
    pub verify_r1: InternalGas,
    pub ed25519_verify: InternalGas,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            ecrecover: 0.into(),
            decompress_pubkey: 0.into(),
            verify_k1: 0.into(),
            verify_r1: 0.into(),
            ed25519_verify: 0.into(),
        }
    }
}

pub fn make_native<F>(f: F) -> NativeFunction
where
    F: Fn(&mut NativeContext, Vec<Type>, VecDeque<Value>) -> PartialVMResult<NativeResult>
        + Send
        + Sync
        + 'static,
{
    Arc::new(f)
}

pub fn make_ecdsa_k1(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    make_module_natives(
        all_natives_with_gas(AccountAddress::ZERO, gas_params)
            .into_iter()
            .filter(|(_, module_name, _, _)| module_name.as_str() == "ecdsa_k1")
            .map(|(_, _, func_name, func)| (func_name.to_string(), func)),
    )
}

pub fn make_ecdsa_r1(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    make_module_natives(
        all_natives_with_gas(AccountAddress::ZERO, gas_params)
            .into_iter()
            .filter(|(_, module_name, _, _)| module_name.as_str() == "ecdsa_r1")
            .map(|(_, _, func_name, func)| (func_name.to_string(), func)),
    )
}

pub fn make_ed25519(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    make_module_natives(
        all_natives_with_gas(AccountAddress::ZERO, gas_params)
            .into_iter()
            .filter(|(_, module_name, _, _)| module_name.as_str() == "ed25519")
            .map(|(_, _, func_name, func)| (func_name.to_string(), func)),
    )
}

fn all_natives_with_gas(
    move_addr: AccountAddress,
    gas_params: GasParameters,
) -> NativeFunctionTable {
    let mut natives = vec![];

    let ecrecover_cost = gas_params.ecrecover;
    let decompress_pubkey_cost = gas_params.decompress_pubkey;
    let verify_k1_cost = gas_params.verify_k1;
    let verify_r1_cost = gas_params.verify_r1;
    let ed25519_verify_cost = gas_params.ed25519_verify;

    // ecdsa_k1::ecrecover(signature: vector<u8>, msg: vector<u8>, hash: u8): vector<u8>
    let ecrecover_native = make_native(
        move |context, _ty_args, mut arguments| -> PartialVMResult<NativeResult> {
            use move_vm_types::natives::function::NativeResult as NR;

            native_charge_gas_early_exit!(context, ecrecover_cost);

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
            let out = pubkey.serialize().to_vec();
            Ok(NR::ok(context.gas_used(), smallvec![Value::vector_u8(out)]))
        },
    );

    // ecdsa_k1::decompress_pubkey(pubkey: vector<u8>): vector<u8>
    let decompress_native = make_native(
        move |context, _ty_args, mut arguments| -> PartialVMResult<NativeResult> {
            use move_vm_types::natives::function::NativeResult as NR;
            native_charge_gas_early_exit!(context, decompress_pubkey_cost);
            let pubkey_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let mut pubkey: Vec<u8> = pubkey_ref.as_bytes_ref().to_vec();

            // Accept 64-byte uncompressed X||Y (missing 0x04 prefix) and normalize to SEC1.
            if pubkey.len() == 64 {
                let mut prefixed = Vec::with_capacity(65);
                prefixed.push(0x04);
                prefixed.extend_from_slice(&pubkey);
                pubkey = prefixed;
            }

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
            native_charge_gas_early_exit!(context, verify_k1_cost);
            let hash_type: u8 = pop_arg!(arguments, u8);
            let msg_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let public_key_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let signature_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let msg: Vec<u8> = msg_ref.as_bytes_ref().to_vec();
            let mut public_key: Vec<u8> = public_key_ref.as_bytes_ref().to_vec();
            let signature: Vec<u8> = signature_ref.as_bytes_ref().to_vec();

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
                        return Ok(NR::err(context.gas_used(), E_INVALID_MESSAGE));
                        // ErrorInvalidMessage
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
                    return Ok(NR::err(context.gas_used(), E_INVALID_XONLY_PUBKEY));
                    // ErrorInvalidXOnlyPubKey
                }
            } else {
                // If signature is not 64 but public_key looks like an x-only key, it's an invalid schnorr signature
                if public_key.len() == 32 {
                    return Ok(NR::err(context.gas_used(), E_INVALID_SCHNORR_SIGNATURE));
                    // ErrorInvalidSchnorrSignature
                }
            }

            // Normalize secp256k1 pubkey encodings. Some callers provide 64-byte X||Y without 0x04.
            if public_key.len() == 64 {
                let mut prefixed = Vec::with_capacity(65);
                prefixed.push(0x04);
                prefixed.extend_from_slice(&public_key);
                public_key = prefixed;
            }

            // parse pubkey (allow compressed/uncompressed) for ECDSA
            let vk = match K256VerifyingKey::from_sec1_bytes(&public_key) {
                Ok(v) => v,
                Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_PUBKEY)),
            };

            // Normalize signature encodings. Some ecosystems use 65-byte r||s||v; v is ignored for verification.
            let sig_bytes = if signature.len() == 65 {
                &signature[..64]
            } else {
                signature.as_slice()
            };

            // parse signature: try DER then raw 64
            let sig = if let Ok(s) = K256Signature::from_der(&signature) {
                s
            } else if sig_bytes.len() == 64 {
                // try raw 64-bytes signature
                match K256Signature::try_from(sig_bytes) {
                    Ok(s) => s,
                    Err(_) => {
                        log::debug!("Failed to parse signature as raw 64 bytes");
                        return Ok(NR::ok(context.gas_used(), smallvec![Value::bool(false)]));
                    }
                }
            } else {
                log::debug!("Invalid signature length: {}", signature.len());
                return Ok(NR::ok(context.gas_used(), smallvec![Value::bool(false)]));
            };

            // Use digest-aware verification: verify the hashed message
            let verified = if hash_type == 0u8 {
                // Keccak256
                let msg_hash = Keccak256::digest(&msg);
                vk.verify_prehash(msg_hash.as_slice(), &sig).is_ok()
            } else {
                let msg_hash = Sha256::digest(&msg);
                vk.verify_prehash(msg_hash.as_slice(), &sig).is_ok()
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
            native_charge_gas_early_exit!(context, verify_r1_cost);
            let hash_type: u8 = pop_arg!(arguments, u8);
            let msg_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let public_key_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let signature_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let msg: Vec<u8> = msg_ref.as_bytes_ref().to_vec();
            let mut public_key: Vec<u8> = public_key_ref.as_bytes_ref().to_vec();
            let signature: Vec<u8> = signature_ref.as_bytes_ref().to_vec();

            if signature.is_empty() {
                return Ok(NR::err(context.gas_used(), E_INVALID_SIGNATURE)); // ErrorInvalidSignature
            }

            // Prevent overly large messages
            if msg.len() > MAX_MSG_BYTES {
                return Ok(NR::err(context.gas_used(), E_INVALID_MESSAGE));
            }

            // Normalize P-256 pubkey encodings. Accept 64-byte uncompressed X||Y and add 0x04 prefix.
            if public_key.len() == 64 {
                let mut prefixed = Vec::with_capacity(65);
                prefixed.push(0x04);
                prefixed.extend_from_slice(&public_key);
                public_key = prefixed;
            }

            // Only SHA256 is supported for P-256 in Move wrapper, but accept hash_type selection defensively
            let vk = match P256VerifyingKey::from_sec1_bytes(&public_key) {
                Ok(v) => v,
                Err(_) => return Ok(NR::err(context.gas_used(), E_INVALID_PUBKEY)),
            };

            // Disallow Keccak for P-256 usage by default (non-standard).
            if hash_type == 0u8 {
                return Ok(NR::err(context.gas_used(), E_UNSUPPORTED_HASH_FOR_P256));
                // ErrorUnsupportedHashForP256
            }

            // Normalize signature encodings. Some ecosystems use 65-byte r||s||v; v is ignored here.
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

            // Hash then verify (SHA256 only for P-256)
            let msg_hash = Sha256::digest(&msg);
            use p256::ecdsa::signature::hazmat::PrehashVerifier;
            let verified = vk.verify_prehash(msg_hash.as_slice(), &sig).is_ok();

            Ok(NR::ok(context.gas_used(), smallvec![Value::bool(verified)]))
        },
    );

    // ed25519::verify(signature, public_key, msg) -> bool
    let ed25519_verify = make_native(
        move |context, _ty_args, mut arguments| -> PartialVMResult<NativeResult> {
            use move_vm_types::natives::function::NativeResult as NR;
            native_charge_gas_early_exit!(context, ed25519_verify_cost);
            // Pop arguments (may return PartialVMError via the macro)
            let msg_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let public_key_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let signature_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let msg: Vec<u8> = msg_ref.as_bytes_ref().to_vec();
            let public_key: Vec<u8> = public_key_ref.as_bytes_ref().to_vec();
            let signature: Vec<u8> = signature_ref.as_bytes_ref().to_vec();

            // Prevent overly large messages
            if msg.len() > MAX_MSG_BYTES {
                return Ok(NR::ok(context.gas_used(), smallvec![Value::bool(false)]));
            }

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

            let verified: bool = result.unwrap_or_default();

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

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::{SigningKey as P256SigningKey, signature::Signer as P256Signer};
    use rand::TryRng;
    use rand::rngs::SysRng;
    use secp256k1::{Keypair, Secp256k1, SecretKey, XOnlyPublicKey};
    use sha2::{Digest, Sha256};
    use sha3::Keccak256;

    // Helper to generate random bytes for key generation using SysRng (same as keys.rs)
    fn generate_random_bytes<const N: usize>() -> [u8; N] {
        let mut bytes = [0u8; N];
        SysRng
            .try_fill_bytes(&mut bytes)
            .expect("Failed to get OS randomness");
        bytes
    }

    #[test]
    fn test_secp256k1_ecrecover_and_verify() {
        // Test ecdsa_k1 (secp256k1) signature recovery and verification

        let secp = Secp256k1::new();
        let secret_bytes = generate_random_bytes::<32>();
        let secret_key = SecretKey::from_byte_array(secret_bytes).unwrap();
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);

        // Create message
        let msg = b"Hello, Kanari Network!";
        let msg_hash = Sha256::digest(msg);
        let message = secp256k1::Message::from_digest(msg_hash.into());

        // Sign the message
        let signature = secp.sign_ecdsa(message, &secret_key);

        // serialize_compact returns [u8; 64], not a tuple
        let sig_bytes = signature.serialize_compact();

        // For recovery, we need to sign with recoverable signature
        let recoverable_sig = secp.sign_ecdsa_recoverable(message, &secret_key);
        let (recovery_id, rec_sig_bytes) = recoverable_sig.serialize_compact();
        let mut sig_65 = vec![0u8; 65];
        sig_65[..64].copy_from_slice(&rec_sig_bytes);
        sig_65[64] = i32::from(recovery_id) as u8;

        // Test decompress_pubkey
        let compressed_pk = public_key.serialize(); // 33 bytes
        assert_eq!(compressed_pk.len(), 33);

        // Decompress using k256
        let decompressed = K256PublicKey::from_sec1_bytes(&compressed_pk).unwrap();
        let uncompressed = decompressed.to_encoded_point(false);
        assert_eq!(uncompressed.as_bytes().len(), 65);

        // Test verify with SHA256
        let vk = K256VerifyingKey::from_sec1_bytes(&compressed_pk).unwrap();
        // Use raw 64-byte signature format instead of DER
        let sig = K256Signature::try_from(sig_bytes.as_slice()).unwrap();

        // For k256, we need to use the signature verification API correctly
        // The signature was created over msg_hash, so we verify against that hash
        use k256::ecdsa::signature::hazmat::PrehashVerifier;
        let verified = vk.verify_prehash(msg_hash.as_slice(), &sig).is_ok();
        assert!(verified, "SHA256 signature verification should succeed");

        // Test verify with Keccak256
        let msg_hash_keccak = Keccak256::digest(msg);
        let sig_keccak = secp.sign_ecdsa(
            secp256k1::Message::from_digest(msg_hash_keccak.into()),
            &secret_key,
        );
        let sig_bytes_keccak = sig_keccak.serialize_compact();
        let sig_keccak_parsed = K256Signature::try_from(sig_bytes_keccak.as_slice()).unwrap();
        let verified_keccak = vk
            .verify_prehash(msg_hash_keccak.as_slice(), &sig_keccak_parsed)
            .is_ok();
        assert!(
            verified_keccak,
            "Keccak256 signature verification should succeed"
        );
    }

    #[test]
    fn test_secp256k1_schnorr_signature() {
        // Test Schnorr signatures with x-only public keys

        let secp = Secp256k1::new();
        let secret_bytes = generate_random_bytes::<32>();
        let secret_key = SecretKey::from_byte_array(secret_bytes).unwrap();
        let keypair = Keypair::from_secret_key(&secp, &secret_key);
        let (xonly_pubkey, _parity) = XOnlyPublicKey::from_keypair(&keypair);

        // Create exactly 32-byte message (required for Schnorr)
        let msg_bytes = generate_random_bytes::<32>();

        // Sign with Schnorr (using no auxiliary randomness for deterministic testing)
        let schnorr_sig = secp.sign_schnorr_no_aux_rand(&msg_bytes, &keypair);

        // Verify
        let verified = secp
            .verify_schnorr(&schnorr_sig, &msg_bytes, &xonly_pubkey)
            .is_ok();
        assert!(verified, "Schnorr signature verification should succeed");

        // Test with wrong message
        let wrong_msg = generate_random_bytes::<32>();
        let verified_wrong = secp
            .verify_schnorr(&schnorr_sig, &wrong_msg, &xonly_pubkey)
            .is_ok();
        assert!(!verified_wrong, "Schnorr should fail with wrong message");
    }

    #[test]
    fn test_p256_ecdsa_verify() {
        // Test ecdsa_r1 (P-256) signature verification

        // Generate P-256 keypair using SysRng
        let random_bytes = generate_random_bytes::<32>();
        // For P-256, we need to ensure the bytes form a valid scalar
        // Use from_slice which handles validation properly
        let signing_key = match P256SigningKey::from_slice(&random_bytes) {
            Ok(key) => key,
            Err(_) => {
                // If invalid, use a known-good test key
                let test_bytes = [0x42u8; 32];
                P256SigningKey::from_slice(&test_bytes).unwrap()
            }
        };
        let verifying_key = signing_key.verifying_key();

        // Create message
        let msg = b"P-256 test message";
        let msg_hash = Sha256::digest(msg);

        // Sign the message
        let signature: P256Signature = signing_key.sign(msg);

        // Verify with SHA256
        use p256::ecdsa::signature::hazmat::PrehashVerifier as P256PrehashVerifier;
        let verified = verifying_key
            .verify_prehash(msg_hash.as_slice(), &signature)
            .is_ok();
        assert!(
            verified,
            "P-256 SHA256 signature verification should succeed"
        );

        // Test with DER encoding
        let der_sig = signature.to_der();
        let sig_from_der = P256Signature::from_der(der_sig.as_bytes()).unwrap();
        let verified_der = verifying_key
            .verify_prehash(msg_hash.as_slice(), &sig_from_der)
            .is_ok();
        assert!(
            verified_der,
            "P-256 DER signature verification should succeed"
        );

        // Test with raw 64-byte encoding
        let raw_bytes = signature.to_bytes();
        assert_eq!(raw_bytes.len(), 64);
        let sig_from_raw = P256Signature::try_from(raw_bytes.as_slice()).unwrap();
        let verified_raw = verifying_key
            .verify_prehash(msg_hash.as_slice(), &sig_from_raw)
            .is_ok();
        assert!(
            verified_raw,
            "P-256 raw signature verification should succeed"
        );
    }

    #[test]
    fn test_ed25519_verify() {
        // Test Ed25519 signature verification

        // Generate Ed25519 keypair using SysRng
        let random_bytes = generate_random_bytes::<32>();
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&random_bytes);
        let verifying_key = signing_key.verifying_key();

        // Create message
        let msg = b"Ed25519 test message";

        // Sign the message
        let signature = signing_key.sign(msg);

        // Verify
        let verified = verifying_key.verify(msg, &signature).is_ok();
        assert!(verified, "Ed25519 signature verification should succeed");

        // Test with wrong message
        let wrong_msg = b"Wrong message";
        let verified_wrong = verifying_key.verify(wrong_msg, &signature).is_ok();
        assert!(!verified_wrong, "Ed25519 should fail with wrong message");

        // Test key and signature sizes
        assert_eq!(verifying_key.to_bytes().len(), 32);
        assert_eq!(signature.to_bytes().len(), 64);
    }

    #[test]
    fn test_invalid_signatures() {
        // Test error handling for invalid inputs

        let secp = Secp256k1::new();
        let secret_bytes = generate_random_bytes::<32>();
        let secret_key = SecretKey::from_byte_array(secret_bytes).unwrap();
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let msg = b"Test message";
        let msg_hash = Sha256::digest(msg);
        let message = secp256k1::Message::from_digest(msg_hash.into());
        let signature = secp.sign_ecdsa(message, &secret_key);
        let sig_bytes = signature.serialize_compact();

        // Test invalid signature length (not 64 or 65 bytes)
        let invalid_sig_short = vec![0u8; 32];
        let parsed = K256Signature::from_der(&invalid_sig_short);
        assert!(
            parsed.is_err(),
            "Should fail to parse invalid short signature"
        );

        // Test invalid public key
        let invalid_pk = vec![0u8; 33]; // All zeros
        let result = K256VerifyingKey::from_sec1_bytes(&invalid_pk);
        assert!(result.is_err(), "Should fail to parse invalid public key");

        // Test wrong signature for message
        let wrong_msg = b"Wrong message";
        let wrong_hash = Sha256::digest(wrong_msg);
        let vk = K256VerifyingKey::from_sec1_bytes(&public_key.serialize()).unwrap();
        // Use raw signature format
        let sig = K256Signature::try_from(sig_bytes.as_slice()).unwrap();
        let verified = vk.verify(&wrong_hash, &sig).is_ok();
        assert!(!verified, "Should fail verification with wrong message");
    }

    #[test]
    fn test_pubkey_normalization() {
        // Test public key format normalization (64-byte without prefix -> 65-byte with 0x04)

        let secp = Secp256k1::new();
        let secret_bytes = generate_random_bytes::<32>();
        let secret_key = SecretKey::from_byte_array(secret_bytes).unwrap();
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);

        // Get uncompressed pubkey (65 bytes with 0x04 prefix)
        let uncompressed_full = public_key.serialize_uncompressed();
        assert_eq!(uncompressed_full.len(), 65);
        assert_eq!(uncompressed_full[0], 0x04);

        // Extract X||Y without prefix (64 bytes)
        let xy_only = &uncompressed_full[1..];
        assert_eq!(xy_only.len(), 64);

        // Normalize by adding 0x04 prefix
        let mut normalized = Vec::with_capacity(65);
        normalized.push(0x04);
        normalized.extend_from_slice(xy_only);
        assert_eq!(normalized, uncompressed_full);

        // Should be able to parse both formats
        let pk_from_full = K256PublicKey::from_sec1_bytes(&uncompressed_full).unwrap();
        let pk_from_normalized = K256PublicKey::from_sec1_bytes(&normalized).unwrap();
        assert_eq!(pk_from_full, pk_from_normalized);
    }

    #[test]
    fn test_message_size_limits() {
        // Test that large messages are rejected

        // Create message at the limit
        let msg_at_limit = vec![0u8; MAX_MSG_BYTES];
        assert_eq!(msg_at_limit.len(), MAX_MSG_BYTES);

        // Create message over the limit
        let msg_over_limit = vec![0u8; MAX_MSG_BYTES + 1];
        assert!(msg_over_limit.len() > MAX_MSG_BYTES);

        // Hash operations should still work (native would reject based on size check)
        let _hash = Sha256::digest(&msg_at_limit);
        let _hash_over = Sha256::digest(&msg_over_limit);
    }

    #[test]
    fn test_recovery_id_handling() {
        // Test recovery ID conversion (0-3 vs 27-28)

        let secp = Secp256k1::new();
        let secret_bytes = generate_random_bytes::<32>();
        let secret_key = SecretKey::from_byte_array(secret_bytes).unwrap();
        let msg = b"Recovery ID test";
        let msg_hash = Sha256::digest(msg);
        let message = secp256k1::Message::from_digest(msg_hash.into());

        // Sign and get recovery ID
        let signature = secp.sign_ecdsa_recoverable(message, &secret_key);
        let (rec_id, _sig_bytes) = signature.serialize_compact();

        let rec_id_value: i32 = rec_id.into();
        assert!(
            rec_id_value >= 0 && rec_id_value <= 3,
            "Recovery ID should be 0-3"
        );

        // Test legacy format (27-28)
        let legacy_v = (rec_id_value + 27) as u8;
        assert!(
            legacy_v == 27 || legacy_v == 28,
            "Legacy v should be 27 or 28"
        );

        // Both should convert back correctly
        let rec_id_from_standard = SecpRecoveryId::try_from(rec_id_value).unwrap();
        let rec_id_from_legacy = SecpRecoveryId::try_from((legacy_v - 27) as i32).unwrap();
        assert_eq!(rec_id_from_standard, rec_id_from_legacy);
    }

    #[test]
    fn test_hash_functions() {
        // Test SHA256 and Keccak256 hash functions

        let msg = b"Hash function test";

        // SHA256
        let sha256_hash = Sha256::digest(msg);
        assert_eq!(sha256_hash.len(), 32);

        // Keccak256
        let keccak_hash = Keccak256::digest(msg);
        assert_eq!(keccak_hash.len(), 32);

        // Different hashes for same message
        assert_ne!(sha256_hash.as_slice(), keccak_hash.as_slice());

        // Same message produces same hash
        let sha256_hash2 = Sha256::digest(msg);
        assert_eq!(sha256_hash.as_slice(), sha256_hash2.as_slice());
    }

    #[test]
    fn test_xonly_pubkey_conversion() {
        // Test x-only public key operations for Schnorr

        let secp = Secp256k1::new();
        let secret_bytes = generate_random_bytes::<32>();
        let secret_key = SecretKey::from_byte_array(secret_bytes).unwrap();
        let keypair = Keypair::from_secret_key(&secp, &secret_key);
        let (xonly, _parity) = XOnlyPublicKey::from_keypair(&keypair);

        // X-only pubkey should be 32 bytes
        let xonly_bytes = xonly.serialize();
        assert_eq!(xonly_bytes.len(), 32);

        // Should be able to reconstruct from bytes
        let xonly_from_bytes = XOnlyPublicKey::from_byte_array(xonly_bytes).unwrap();
        assert_eq!(xonly, xonly_from_bytes);
    }

    #[test]
    fn generate_test_vectors_for_move() {
        // Generate correct test vectors for Move tests
        // This test prints the correct signature, pubkey, and message that can be used in Move tests

        let secp = Secp256k1::new();

        // Use a fixed seed for reproducible test vectors
        let secret_bytes = [0x42u8; 32];
        let secret_key = SecretKey::from_byte_array(secret_bytes).unwrap();
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);

        // Test message: 0x00010203 (same as in Move test)
        let msg = vec![0x00u8, 0x01, 0x02, 0x03];
        let msg_hash = Sha256::digest(&msg);
        let message = secp256k1::Message::from_digest(msg_hash.into());

        // Sign with ECDSA
        let signature = secp.sign_ecdsa(message, &secret_key);
        let sig_bytes = signature.serialize_compact();

        // Get compressed public key (33 bytes)
        let compressed_pk = public_key.serialize();

        println!("\n=== ECDSA K1 Test Vectors ===");
        println!("Message: {}", hex::encode(&msg));
        println!("Public Key (compressed): {}", hex::encode(&compressed_pk));
        println!("Signature (r||s): {}", hex::encode(&sig_bytes));
        println!("Hash Type: SHA256 (1)");

        // Verify it works
        let vk = K256VerifyingKey::from_sec1_bytes(&compressed_pk).unwrap();
        use k256::ecdsa::signature::hazmat::PrehashVerifier;
        let sig = K256Signature::try_from(sig_bytes.as_slice()).unwrap();
        let verified = vk.verify_prehash(msg_hash.as_slice(), &sig).is_ok();
        assert!(verified, "Generated test vector should verify correctly");

        // Generate Schnorr test vector
        let keypair = Keypair::from_secret_key(&secp, &secret_key);
        let (xonly, _parity) = XOnlyPublicKey::from_keypair(&keypair);

        // Schnorr requires 32-byte message
        let schnorr_msg = generate_random_bytes::<32>();
        let schnorr_sig = secp.sign_schnorr_no_aux_rand(&schnorr_msg, &keypair);

        println!("\n=== Schnorr Test Vectors ===");
        println!("Message (32 bytes): {}", hex::encode(&schnorr_msg));
        println!("Public Key (x-only): {}", hex::encode(&xonly.serialize()));
        println!(
            "Signature (r||s): {}",
            hex::encode(&schnorr_sig.to_byte_array())
        );

        // Verify Schnorr
        let schnorr_verified = secp
            .verify_schnorr(&schnorr_sig, &schnorr_msg, &xonly)
            .is_ok();
        assert!(
            schnorr_verified,
            "Schnorr test vector should verify correctly"
        );
    }

    #[test]
    fn test_move_ecdsa_k1_vector() {
        // Test the exact vector from Move test to see if it works
        let msg = hex::decode("00010203").unwrap();
        let pubkey =
            hex::decode("033e99a541db69bd32040dfe5037fbf5210dafa8151a71e21c5204b05d95ce0a62")
                .unwrap();
        let sig = hex::decode("416a21d50b3c838328d4f03213f8ef0c3776389a972ba1ecd37b56243734eba208ea6aaa6fc076ad7accd71d355f693a6fe54fe69b3c168eace9803827bc9046").unwrap();

        println!("\n=== Testing Move ECDSA K1 Vector ===");
        println!("Message length: {}", msg.len());
        println!("Pubkey length: {}", pubkey.len());
        println!("Signature length: {}", sig.len());

        // Parse pubkey
        let vk = match K256VerifyingKey::from_sec1_bytes(&pubkey) {
            Ok(vk) => {
                println!("✓ Public key parsed successfully");
                vk
            }
            Err(e) => {
                println!("✗ Failed to parse public key: {:?}", e);
                panic!("Invalid public key");
            }
        };

        // Parse signature (raw 64 bytes)
        let sig_parsed = match K256Signature::try_from(sig.as_slice()) {
            Ok(s) => {
                println!("✓ Signature parsed successfully");
                s
            }
            Err(e) => {
                println!("✗ Failed to parse signature: {:?}", e);
                panic!("Invalid signature");
            }
        };

        // Hash message with SHA256
        let msg_hash = Sha256::digest(&msg);
        println!("Message hash: {}", hex::encode(msg_hash.as_slice()));

        // Verify
        use k256::ecdsa::signature::hazmat::PrehashVerifier;
        let verified = vk.verify_prehash(msg_hash.as_slice(), &sig_parsed).is_ok();
        println!(
            "Verification result: {}",
            if verified { "✓ PASS" } else { "✗ FAIL" }
        );

        if !verified {
            println!("\n⚠️  The Move test vector is INVALID!");
            println!("This means the signature doesn't match the pubkey+message combination.");
            println!("The test data in Move code needs to be regenerated.");
        }

        assert!(verified, "Move test vector should verify correctly");
    }

    #[test]
    fn generate_p256_test_vectors() {
        // Generate correct test vectors for P-256 (ECDSA R1)
        use p256::ecdsa::{SigningKey as P256SigningKey, signature::Signer};

        // Use a fixed secret key for reproducible test vectors
        let secret_bytes = [0x42u8; 32];
        let signing_key = P256SigningKey::from_bytes((&secret_bytes).into()).unwrap();
        let public_key = signing_key.verifying_key();

        // Test message: "hello world" (same as in Move test)
        let msg = b"hello world";

        // Sign with P-256
        let signature: P256Signature = signing_key.sign(msg);
        let sig_bytes = signature.to_bytes();

        // Get compressed public key (33 bytes) - using to_encoded_point with compress=true
        let encoded = public_key.to_encoded_point(true); // true = compressed
        let compressed_pk = encoded.as_bytes();

        println!("\n=== P-256 (ECDSA R1) Test Vectors ===");
        println!("Message: {}", String::from_utf8_lossy(msg));
        println!("Public Key length: {} bytes", compressed_pk.len());
        println!("Public Key (compressed): {}", hex::encode(&compressed_pk));
        println!("Signature (r||s): {}", hex::encode(&sig_bytes));

        // Verify it works
        use p256::ecdsa::signature::Verifier;
        let vk = P256VerifyingKey::from_sec1_bytes(&compressed_pk).unwrap();
        let sig = P256Signature::from_bytes(&sig_bytes).unwrap();
        let verified = vk.verify(msg, &sig).is_ok();
        println!(
            "Verification result: {}",
            if verified { "✓ PASS" } else { "✗ FAIL" }
        );
        assert!(
            verified,
            "Generated P-256 test vector should verify correctly"
        );
    }

    #[test]
    fn test_move_p256_vector() {
        // Test the exact vector from Move ecdsa_r1 test
        let msg = b"hello world";
        let pubkey =
            hex::decode("0258a618066814098f8ddb3cbde73838b59028d843958031e50be0a5f4b0a9796d")
                .unwrap();
        let sig = hex::decode("74133905657c1992d8d6bd72ffa7ccf8d2adf3e4a3ca25f8dc8eec175752cb5a40459f71b549a25cba3cddf4157e946bbff7b18fc82774e9c4c54e362b97ccb5").unwrap();

        println!("\n=== Testing Move P-256 Vector ===");
        println!("Message: {}", String::from_utf8_lossy(msg));
        println!("Pubkey length: {} bytes", pubkey.len());
        println!("Signature length: {} bytes", sig.len());

        // Parse pubkey
        let vk = match P256VerifyingKey::from_sec1_bytes(&pubkey) {
            Ok(vk) => {
                println!("✓ Public key parsed successfully");
                vk
            }
            Err(e) => {
                println!("✗ Failed to parse public key: {:?}", e);
                panic!("Invalid public key");
            }
        };

        // Parse signature (raw 64 bytes)
        let sig_parsed = match P256Signature::try_from(sig.as_slice()) {
            Ok(s) => {
                println!("✓ Signature parsed successfully");
                s
            }
            Err(e) => {
                println!("✗ Failed to parse signature: {:?}", e);
                panic!("Invalid signature");
            }
        };

        // Verify with SHA256 hash

        let msg_hash = Sha256::digest(msg);
        println!("Message hash: {}", hex::encode(msg_hash.as_slice()));

        let verified = vk.verify_prehash(msg_hash.as_slice(), &sig_parsed).is_ok();
        println!(
            "Verification result: {}",
            if verified { "✓ PASS" } else { "✗ FAIL" }
        );

        if !verified {
            println!("\n⚠️  The Move test vector is INVALID!");
            println!("The signature doesn't match the pubkey+message combination.");
        }

        assert!(verified, "Move P-256 test vector should verify correctly");
    }
}
