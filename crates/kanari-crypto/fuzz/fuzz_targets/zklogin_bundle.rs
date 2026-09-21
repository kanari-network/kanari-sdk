// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#![no_main]
// Disabled on Windows MSVC (requires LLVM/ASAN which is not available).
// Linux CI runs this via cargo-fuzz; the same coverage runs everywhere as
// `cargo test -p kanari-crypto --test fuzz_tests prop_fuzz_zklogin`.

use kanari_crypto::signatures::groth16::public_inputs_from_be_bytes;
use kanari_crypto::signatures::zk_authenticator::verify_zklogin_tx_signature;
use kanari_crypto::signatures::zklogin::{decode_jwt_claims, derive_zklogin_address_v2};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Split input into (address-ish, message, signature). Attackers control
    // the signature bytes on the mempool path — nothing here may panic.
    let addr_len = data.first().map(|b| (*b as usize).min(256)).unwrap_or(0);
    let (addr, rest) = data.split_at(addr_len.min(data.len()));
    let (msg, sig) = rest.split_at(rest.len().min(1024));
    let addr_str = String::from_utf8_lossy(addr);
    let _ = verify_zklogin_tx_signature(&addr_str, msg, sig);

    // Claims parsing + address derivation + input decoding: Ok or Err,
    // never a panic.
    if let Ok(jwt) = std::str::from_utf8(&data[..data.len().min(512)]) {
        let _ = decode_jwt_claims(jwt);
    }
    let mut salt = [0u8; 32];
    let n = data.len().min(32);
    salt[..n].copy_from_slice(&data[..n]);
    let _ = derive_zklogin_address_v2("https://accounts.google.com", "cid", "u", &salt);
    let _ = public_inputs_from_be_bytes(&data[..data.len().min(65 * 32)]);
});
