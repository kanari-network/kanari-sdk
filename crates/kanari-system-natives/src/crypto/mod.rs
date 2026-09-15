// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

mod ecdsa_k1;
mod ecdsa_r1;
mod ed25519;
mod pqc;
mod rs256;
mod zklogin;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod benches;

use std::{collections::VecDeque, sync::Arc};

use move_core_types::gas_algebra::InternalGas;

// Re-export error codes and constants from submodules for backward compatibility
pub use ecdsa_k1::{
    E_INVALID_MESSAGE, E_INVALID_PUBKEY, E_INVALID_RECOVERY, E_INVALID_SCHNORR_SIGNATURE,
    E_INVALID_SIGNATURE, E_INVALID_XONLY_PUBKEY, MAX_MSG_BYTES,
};
pub use ecdsa_r1::E_UNSUPPORTED_HASH_FOR_P256;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::{NativeResult, PartialVMResult},
    values::Value,
};
pub use rs256::{E_INVALID_HASH_TYPE as RS256_E_INVALID_HASH_TYPE, E_INVALID_MESSAGE_LENGTH};

/// Helper function to create native functions
pub fn make_native<F>(f: F) -> NativeFunction
where
    F: Fn(&mut NativeContext, Vec<Type>, VecDeque<Value>) -> PartialVMResult<NativeResult>
        + Send
        + Sync
        + 'static,
{
    Arc::new(f)
}

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub ecrecover: InternalGas,
    pub decompress_pubkey: InternalGas,
    pub verify_k1: InternalGas,
    pub verify_r1: InternalGas,
    pub ed25519_verify: InternalGas,
    pub dilithium2_verify: InternalGas,
    pub dilithium3_verify: InternalGas,
    pub dilithium5_verify: InternalGas,
    pub sphincs_plus_sha256_robust_verify: InternalGas,
    pub falcon512_verify: InternalGas,
    pub falcon1024_verify: InternalGas,
    pub ed25519_dilithium3_verify: InternalGas,
    pub k256_dilithium3_verify: InternalGas,
    pub rs256_verify: InternalGas,
    pub zklogin_verify: InternalGas,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            ecrecover: 0.into(),
            decompress_pubkey: 0.into(),
            verify_k1: 0.into(),
            verify_r1: 0.into(),
            ed25519_verify: 0.into(),
            dilithium2_verify: 0.into(),
            dilithium3_verify: 0.into(),
            dilithium5_verify: 0.into(),
            sphincs_plus_sha256_robust_verify: 0.into(),
            falcon512_verify: 0.into(),
            falcon1024_verify: 0.into(),
            ed25519_dilithium3_verify: 0.into(),
            k256_dilithium3_verify: 0.into(),
            rs256_verify: 0.into(),
            zklogin_verify: 0.into(),
        }
    }

    /// Sui-style production schedule: `zeros()` is for tests/dev only;
    /// node software injects `production()` at the `all_natives` call site.
    ///
    /// Calibrated by release-build micro-benchmarks (x86_64,
    /// `crypto_calibrate --release`):
    ///
    /// ```text
    /// k1_decompress ~7.7us  | ed25519_verify ~46us   | k1_ecrecover ~56us
    /// falcon512 ~71us       | k1_verify ~75us        | falcon1024 ~108us
    /// dilithium2 ~145us     | r1_verify ~193us       | dilithium3 ~233us
    /// hybrid_ed+dil3 ~299us | hybrid_k256+dil3 ~312us| dilithium5 ~341us
    /// rs256_rfc ~363us      | sphincs ~949us
    /// sha256_1KB ~0.7us     | sha256_1MB ~647us      | dilithium3_1MB ~3.2ms
    /// ```
    ///
    /// Anchor: same as `math_calculate` (cheapest math ops = 100 units
    /// ~= 2ns, i.e. ~50 units/ns). Values below round the measured
    /// ns/op up to a conservative flat fee for small messages.
    ///
    /// NOTE: verify natives accept up to 1MB messages (hashed or signed
    /// directly). Large-message cost grows separately (`sha256_1MB`,
    /// `dilithium3_1MB`); a future `per_byte` slope should be added if
    /// large messages become a DoS concern. For now the flat fee covers
    /// the typical small-message path.
    pub fn production() -> Self {
        // Affordability schedule: flat fees in the low thousands so a
        // single verify fits comfortably in `max_gas_per_tx = 100_000`.
        // Relative order follows measured cost (decompress < ed25519 <
        // k1 < falcon < dilithium < hybrid < rs256 < sphincs), but the
        // absolute scale is deliberately cheap — DoS protection for
        // large messages relies on the 1MB cap, not on pricing the
        // worst case.
        Self {
            ecrecover: 1_800.into(),
            decompress_pubkey: 500.into(),
            verify_k1: 2_400.into(),
            verify_r1: 6_000.into(),
            ed25519_verify: 1_500.into(),
            dilithium2_verify: 4_600.into(),
            dilithium3_verify: 7_200.into(),
            dilithium5_verify: 9_500.into(),
            sphincs_plus_sha256_robust_verify: 9_900.into(),
            falcon512_verify: 2_200.into(),
            falcon1024_verify: 3_400.into(),
            ed25519_dilithium3_verify: 8_500.into(),
            k256_dilithium3_verify: 8_800.into(),
            rs256_verify: 9_800.into(),
            // RSA-2048 verify class (jsonwebtoken RS256 + JSON parse).
            zklogin_verify: 9_800.into(),
        }
    }
}

pub fn make_ecdsa_k1(
    gas_params: GasParameters,
) -> impl Iterator<Item = (String, move_vm_runtime::native_functions::NativeFunction)> {
    let natives = vec![
        (
            "ecrecover".to_string(),
            ecdsa_k1::make_ecrecover_native(gas_params.ecrecover),
        ),
        (
            "decompress_pubkey".to_string(),
            ecdsa_k1::make_decompress_pubkey_native(gas_params.decompress_pubkey),
        ),
        (
            "verify".to_string(),
            ecdsa_k1::make_verify_k1_native(gas_params.verify_k1),
        ),
    ];

    crate::helpers::make_module_natives(natives)
}

pub fn make_ecdsa_r1(
    gas_params: GasParameters,
) -> impl Iterator<Item = (String, move_vm_runtime::native_functions::NativeFunction)> {
    ecdsa_r1::make_ecdsa_r1_natives(gas_params.verify_r1)
}

pub fn make_ed25519(
    gas_params: GasParameters,
) -> impl Iterator<Item = (String, move_vm_runtime::native_functions::NativeFunction)> {
    ed25519::make_ed25519_natives(gas_params.ed25519_verify)
}

pub fn make_dilithium2(
    gas_params: GasParameters,
) -> impl Iterator<Item = (String, NativeFunction)> {
    pqc::make_dilithium2(gas_params.dilithium2_verify)
}

pub fn make_dilithium3(
    gas_params: GasParameters,
) -> impl Iterator<Item = (String, NativeFunction)> {
    pqc::make_dilithium3(gas_params.dilithium3_verify)
}

pub fn make_dilithium5(
    gas_params: GasParameters,
) -> impl Iterator<Item = (String, NativeFunction)> {
    pqc::make_dilithium5(gas_params.dilithium5_verify)
}

pub fn make_sphincs_plus_sha256_robust(
    gas_params: GasParameters,
) -> impl Iterator<Item = (String, NativeFunction)> {
    pqc::make_sphincs_plus_sha256_robust(gas_params.sphincs_plus_sha256_robust_verify)
}

pub fn make_falcon512(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    pqc::make_falcon512(gas_params.falcon512_verify)
}

pub fn make_falcon1024(
    gas_params: GasParameters,
) -> impl Iterator<Item = (String, NativeFunction)> {
    pqc::make_falcon1024(gas_params.falcon1024_verify)
}

pub fn make_ed25519_dilithium3(
    gas_params: GasParameters,
) -> impl Iterator<Item = (String, NativeFunction)> {
    pqc::make_ed25519_dilithium3(gas_params.ed25519_dilithium3_verify)
}

pub fn make_k256_dilithium3(
    gas_params: GasParameters,
) -> impl Iterator<Item = (String, NativeFunction)> {
    pqc::make_k256_dilithium3(gas_params.k256_dilithium3_verify)
}

pub fn make_rs256(
    gas_params: GasParameters,
) -> impl Iterator<Item = (String, move_vm_runtime::native_functions::NativeFunction)> {
    rs256::make_rs256_natives(gas_params.rs256_verify)
}

pub fn make_zklogin(
    gas_params: GasParameters,
) -> impl Iterator<Item = (String, move_vm_runtime::native_functions::NativeFunction)> {
    zklogin::make_zklogin_natives(gas_params.zklogin_verify)
}
