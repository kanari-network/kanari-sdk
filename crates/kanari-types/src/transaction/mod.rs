// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Blockchain transaction types, split by concern:
// - `signed`: [`SignedTransaction`] creation, signing and verification.
// - `native`: [`NativeCall`] fast-path for native KANARI transfers/burns.
// - `objects`: object/effect types (`ObjectRef`, `ObjectChange`, ...).
// - `kind`: the [`Transaction`] enum and its behavior.
// - `access_keys`: shared conflict/access-key resolution used by `kind`.
//
// Every name previously available as `kanari_types::transaction::X` is
// re-exported here, so downstream imports keep working unchanged.
mod access_keys;
mod kind;
mod native;
mod objects;
mod signed;

/// Largest per-sender nonce watermark that is safe as a JSON integer.
///
/// Legacy random nonces above this bound are discarded so they cannot lock a
/// sender out permanently.
pub const MAX_JSON_SAFE_NONCE_WATERMARK: u64 = 1u64 << 53;

/// Returns `value` when it is below the JSON-safe watermark, else `None`.
#[must_use]
pub fn sanitize_nonce_watermark(value: u64) -> Option<u64> {
    (value < MAX_JSON_SAFE_NONCE_WATERMARK).then_some(value)
}

pub use kind::Transaction;
pub use native::NativeCall;
pub use objects::{
    GasPayment, ObjectChange, ObjectChangeKind, ObjectGraphEdge, ObjectGraphEdgeKind, ObjectInput,
    ObjectOwnerKind, ObjectRef, PublishedModule, TransactionEffects,
};
pub use signed::{SignedTransaction, VerifiedSignedTransaction};

#[cfg(test)]
mod tests;
