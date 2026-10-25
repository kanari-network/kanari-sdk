// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Build handlers: assemble unsigned requests with gas and nonce selection.
//!
//! The publish, transfer, and call entry points live in the submodules; this
//! module holds the helpers they share.

use super::*;

mod call;
mod publish;
mod transfer;

pub use call::handle_build_call_function;
pub use publish::{handle_build_publish_module, handle_build_publish_package};
pub use transfer::{
    handle_build_native_coin_consolidation, handle_build_native_transfer,
    handle_build_nft_transfer, handle_build_token_transfer,
};

pub(crate) fn fresh_nonce(nonce: Option<u64>, min_nonce: Option<u64>) -> anyhow::Result<u64> {
    if let Some(nonce) = nonce {
        anyhow::ensure!(nonce != 0, "nonce must be non-zero");
        return Ok(nonce);
    }

    // Sequential counter — keeps nonces small and monotonically increasing so
    // the sender_nonce_watermark in the engine never overflows JSON safe range.
    //
    // The counter resets to 1 on every process restart while the watermark
    // persists: without the floor below, a sender with a high committed
    // watermark would see every fresh build rejected as stale until the
    // global counter caught up. `min_nonce` (the engine's expected nonce for
    // this sender) keeps generated nonces above the watermark from the first
    // build after a restart.
    static NONCE_COUNTER: AtomicU64 = AtomicU64::new(1);
    let floor = min_nonce.unwrap_or(1).max(1);
    let mut current = NONCE_COUNTER.load(Ordering::Relaxed);
    loop {
        let next = current.max(floor);
        match NONCE_COUNTER.compare_exchange_weak(
            current,
            next.saturating_add(1),
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return Ok(next),
            Err(actual) => current = actual,
        }
    }
}

/// Fail-fast gas validation shared by every build handler: a zero/invalid
/// gas price or limit can never produce an admissible transaction (the engine
/// rejects it), so tell the caller immediately instead of building a doomed
/// request. This also migrates zero-fee-era clients to paid gas with a clear
/// message. Returns `Some(error_response)` when the request must be rejected.
fn invalid_build_gas_response(
    request_id: u64,
    gas_limit: u64,
    gas_price: u64,
) -> Option<RpcResponse> {
    if gas_limit == 0 {
        return Some(invalid_params_response(
            request_id,
            "gas_limit must be non-zero",
        ));
    }
    if !gas_price_is_valid(gas_price) {
        return Some(invalid_params_response(
            request_id,
            format!(
                "gas_price {gas_price} is not valid for the active gas model ({}); minimum is 1 Mist",
                kanari_types::gas::GAS_MODEL,
            ),
        ));
    }
    None
}
