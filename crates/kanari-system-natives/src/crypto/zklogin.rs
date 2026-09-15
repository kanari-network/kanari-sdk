// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! zkLogin native functions (Phase 3b + 3c).
//!
//! Two verification paths share the ephemeral Ed25519 session key:
//! - Phase 3b (linkable): JWT RS256 verification against an approved JWK +
//!   claim binding (iss/aud/exp/nonce). `sub` and `salt` are visible inputs.
//! - Phase 3c (private): BN254 Groth16 proof against a caller-supplied
//!   verifying key + public inputs. Private witnesses (`sub`/`salt`) never
//!   reach the chain when the circuit keeps them private.

use kanari_crypto::cryptos::verify_ed25519_native;
use kanari_crypto::signatures::zklogin::{
    JwksDocument, compute_nonce, decode_jwt_claims, derive_zklogin_address, verify_claims_timing,
    verify_jwt_with_jwks,
};
use kanari_crypto::signatures::zklogin_proof::{
    MAX_PROOF_BYTES, MAX_VK_BYTES, verify_groth16_proof,
};

use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::native_charge_gas_early_exit;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::natives::function::{NativeResult, PartialVMResult};
use move_vm_types::{
    pop_arg,
    values::{Value, VectorRef},
};
use smallvec::smallvec;

use crate::crypto::make_native;
use crate::helpers::expect_native_signature;

// Error codes for zkLogin natives (must match zklogin.move)
pub const E_INVALID_JWT: u64 = 1;
pub const E_INVALID_JWK: u64 = 2;
#[allow(dead_code)]
pub const E_INVALID_EPHEMERAL_SIG: u64 = 3;
pub const E_CLAIM_MISMATCH: u64 = 4;
pub const E_EXPIRED: u64 = 5;
/// Groth16 verifying key / proof malformed (must match zklogin.move).
pub const E_INVALID_PROOF: u64 = 6;

// Bounds (DoS caps, aligned with other crypto natives)
pub const MAX_JWT_BYTES: usize = 16 * 1024; // 16 KiB
pub const MAX_JWKS_BYTES: usize = 64 * 1024; // 64 KiB
const MAX_MSG_BYTES: usize = 1_000_000; // 1 MB

fn err(code: u64, context: &move_vm_runtime::native_functions::NativeContext) -> NativeResult {
    NativeResult::err(context.gas_used(), code)
}

/// `zklogin::verify(jwt, jwks_json, iss, aud, now_secs) -> bool`
/// Returns true iff RS256 verifies AND iss/aud/exp all check out.
fn make_verify_native(gas_cost: InternalGas) -> NativeFunction {
    make_native(
        move |context, ty_args, mut arguments| -> PartialVMResult<NativeResult> {
            use move_vm_types::natives::function::NativeResult as NR;
            native_charge_gas_early_exit!(context, gas_cost);
            expect_native_signature(arguments.len(), 5, ty_args.len(), 0)?;

            let now_ref: u64 = pop_arg!(arguments, u64);
            let aud_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let iss_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let jwks_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let jwt_ref: VectorRef = pop_arg!(arguments, VectorRef);

            let jwt = jwt_ref.as_bytes_ref().to_vec();
            let jwks_bytes = jwks_ref.as_bytes_ref().to_vec();
            let iss = iss_ref.as_bytes_ref().to_vec();
            let aud = aud_ref.as_bytes_ref().to_vec();

            if jwt.len() > MAX_JWT_BYTES || jwks_bytes.len() > MAX_JWKS_BYTES {
                return Ok(err(E_INVALID_JWT, context));
            }
            let (Ok(jwt_str), Ok(iss_str), Ok(aud_str)) = (
                std::str::from_utf8(&jwt),
                std::str::from_utf8(&iss),
                std::str::from_utf8(&aud),
            ) else {
                return Ok(err(E_INVALID_JWT, context));
            };
            let jwks: JwksDocument = match serde_json::from_slice(&jwks_bytes) {
                Ok(j) => j,
                Err(_) => return Ok(err(E_INVALID_JWK, context)),
            };
            // Panic catcher: jsonwebtoken contains no panic paths we know of,
            // but a native must never unwind into the VM.
            let verified = std::panic::catch_unwind(|| {
                verify_jwt_with_jwks(jwt_str, &jwks, iss_str, aud_str, now_ref)
            });
            match verified {
                Ok(Ok(_)) => Ok(NR::ok(context.gas_used(), smallvec![Value::bool(true)])),
                Ok(Err(e)) => {
                    use kanari_crypto::signatures::SignatureError;
                    let code = match e {
                        SignatureError::InvalidFormat(_) => E_INVALID_JWT,
                        SignatureError::InvalidPublicKey(_) => E_CLAIM_MISMATCH,
                        SignatureError::VerificationFailed => E_EXPIRED,
                        _ => E_INVALID_JWT,
                    };
                    Ok(err(code, context))
                }
                Err(_) => Ok(err(E_INVALID_JWT, context)),
            }
        },
    )
}

/// `zklogin::verify_ephemeral(ephemeral_pubkey, msg, sig) -> bool`
/// Ed25519 check for the session key (non-aborting: false on bad input).
fn make_verify_ephemeral_native(gas_cost: InternalGas) -> NativeFunction {
    make_native(
        move |context, ty_args, mut arguments| -> PartialVMResult<NativeResult> {
            use move_vm_types::natives::function::NativeResult as NR;
            native_charge_gas_early_exit!(context, gas_cost);
            expect_native_signature(arguments.len(), 3, ty_args.len(), 0)?;

            let msg_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let pk_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let sig_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let msg = msg_ref.as_bytes_ref().to_vec();
            let pk = pk_ref.as_bytes_ref().to_vec();
            let sig = sig_ref.as_bytes_ref().to_vec();
            if msg.len() > MAX_MSG_BYTES {
                return Ok(NR::ok(context.gas_used(), smallvec![Value::bool(false)]));
            }
            let ok = std::panic::catch_unwind(|| verify_ed25519_native(&pk, &sig, &msg))
                .unwrap_or_default();
            Ok(NR::ok(context.gas_used(), smallvec![Value::bool(ok)]))
        },
    )
}

/// `zklogin::derive_address(iss, aud, sub, salt) -> vector<u8>`
/// Returns the 32-byte address (aborts E_CLAIM_MISMATCH on bad input).
fn make_derive_address_native(gas_cost: InternalGas) -> NativeFunction {
    make_native(
        move |context, ty_args, mut arguments| -> PartialVMResult<NativeResult> {
            use move_vm_types::natives::function::NativeResult as NR;
            native_charge_gas_early_exit!(context, gas_cost);
            expect_native_signature(arguments.len(), 4, ty_args.len(), 0)?;

            let salt_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let sub_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let aud_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let iss_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let iss_bytes = iss_ref.as_bytes_ref().to_vec();
            let aud_bytes = aud_ref.as_bytes_ref().to_vec();
            let sub_bytes = sub_ref.as_bytes_ref().to_vec();
            let salt_bytes = salt_ref.as_bytes_ref().to_vec();
            let (Ok(iss), Ok(aud), Ok(sub)) = (
                std::str::from_utf8(&iss_bytes),
                std::str::from_utf8(&aud_bytes),
                std::str::from_utf8(&sub_bytes),
            ) else {
                return Ok(err(E_CLAIM_MISMATCH, context));
            };
            match derive_zklogin_address(iss, aud, sub, &salt_bytes) {
                Ok(hex_addr) => {
                    let bytes = hex::decode(hex_addr.trim_start_matches("0x")).unwrap_or_default();
                    Ok(NR::ok(
                        context.gas_used(),
                        smallvec![Value::vector_u8(bytes)],
                    ))
                }
                Err(_) => Ok(err(E_CLAIM_MISMATCH, context)),
            }
        },
    )
}

/// `zklogin::check_nonce(jwt, ephemeral_pubkey, max_epoch, randomness) -> bool`
/// True iff the JWT nonce claim equals `compute_nonce(...)`. Non-aborting.
fn make_check_nonce_native(gas_cost: InternalGas) -> NativeFunction {
    make_native(
        move |context, ty_args, mut arguments| -> PartialVMResult<NativeResult> {
            use move_vm_types::natives::function::NativeResult as NR;
            native_charge_gas_early_exit!(context, gas_cost);
            expect_native_signature(arguments.len(), 4, ty_args.len(), 0)?;

            let rand_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let epoch: u64 = pop_arg!(arguments, u64);
            let pk_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let jwt_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let jwt_bytes = jwt_ref.as_bytes_ref().to_vec();
            let pk_bytes = pk_ref.as_bytes_ref().to_vec();
            let rand_bytes = rand_ref.as_bytes_ref().to_vec();
            let Ok(jwt_str) = std::str::from_utf8(&jwt_bytes) else {
                return Ok(NR::ok(context.gas_used(), smallvec![Value::bool(false)]));
            };
            let Ok(claims) = decode_jwt_claims(jwt_str) else {
                return Ok(NR::ok(context.gas_used(), smallvec![Value::bool(false)]));
            };
            let expected = compute_nonce(&pk_bytes, epoch, &rand_bytes);
            let ok = claims.nonce.as_deref() == Some(expected.as_str());
            // exp is checked by `verify`, not here — nonce binding only.
            let _ = verify_claims_timing;
            Ok(NR::ok(context.gas_used(), smallvec![Value::bool(ok)]))
        },
    )
}

/// `zklogin::verify_proof(vk_bytes, public_inputs_bytes, proof_bytes) -> bool`
/// BN254 Groth16 check (Phase 3c). Non-aborting: `false` for a well-formed
/// but unsatisfying proof; aborts `E_INVALID_PROOF` on malformed input.
fn make_verify_proof_native(gas_cost: InternalGas) -> NativeFunction {
    make_native(
        move |context, ty_args, mut arguments| -> PartialVMResult<NativeResult> {
            use move_vm_types::natives::function::NativeResult as NR;
            native_charge_gas_early_exit!(context, gas_cost);
            expect_native_signature(arguments.len(), 3, ty_args.len(), 0)?;

            let proof_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let inputs_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let vk_ref: VectorRef = pop_arg!(arguments, VectorRef);
            let vk_bytes = vk_ref.as_bytes_ref().to_vec();
            let inputs_bytes = inputs_ref.as_bytes_ref().to_vec();
            let proof_bytes = proof_ref.as_bytes_ref().to_vec();
            if vk_bytes.len() > MAX_VK_BYTES || proof_bytes.len() > MAX_PROOF_BYTES {
                return Ok(err(E_INVALID_PROOF, context));
            }
            // Panic catcher: arkworks deserialization is fallible, but a
            // native must never unwind into the VM.
            let verified = std::panic::catch_unwind(|| {
                verify_groth16_proof(&vk_bytes, &inputs_bytes, &proof_bytes)
            });
            match verified {
                Ok(Ok(valid)) => Ok(NR::ok(context.gas_used(), smallvec![Value::bool(valid)])),
                Ok(Err(e)) => {
                    use kanari_crypto::signatures::SignatureError;
                    match e {
                        SignatureError::InvalidFormat(_) => Ok(err(E_INVALID_PROOF, context)),
                        // Well-formed inputs that fail inside the SNARK: not
                        // attributable to the caller, report `false`.
                        _ => Ok(NR::ok(context.gas_used(), smallvec![Value::bool(false)])),
                    }
                }
                Err(_) => Ok(err(E_INVALID_PROOF, context)),
            }
        },
    )
}

/// Creates the zklogin native functions iterator.
pub fn make_zklogin_natives(
    gas_cost: InternalGas,
    proof_gas_cost: InternalGas,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = vec![
        ("native_verify".to_string(), make_verify_native(gas_cost)),
        (
            "native_verify_ephemeral".to_string(),
            make_verify_ephemeral_native(gas_cost),
        ),
        (
            "native_derive_address".to_string(),
            make_derive_address_native(gas_cost),
        ),
        (
            "native_check_nonce".to_string(),
            make_check_nonce_native(gas_cost),
        ),
        (
            "native_verify_proof".to_string(),
            make_verify_proof_native(proof_gas_cost),
        ),
    ];

    crate::helpers::make_module_natives(natives)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_match_move() {
        assert_eq!(E_INVALID_JWT, 1);
        assert_eq!(E_INVALID_JWK, 2);
        assert_eq!(super::E_INVALID_EPHEMERAL_SIG, 3);
        assert_eq!(E_CLAIM_MISMATCH, 4);
        assert_eq!(E_EXPIRED, 5);
        assert_eq!(E_INVALID_PROOF, 6);
    }

    #[test]
    fn bounds_are_sane() {
        assert_eq!(MAX_JWT_BYTES, 16 * 1024);
        assert_eq!(MAX_JWKS_BYTES, 64 * 1024);
        assert_eq!(MAX_VK_BYTES, 32 * 1024);
        assert_eq!(MAX_PROOF_BYTES, 1024);
    }
}
