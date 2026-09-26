// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Blockchain transaction types, split by concern:
// - `signed`: [`SignedTransaction`] creation, signing and verification.
// - `native`: [`NativeCall`] fast-path for native KANARI transfers/burns.
// - `objects`: object/effect types (`ObjectRef`, `ObjectChange`, ...).
// - `kind`: the [`Transaction`] enum and its behavior.
//
// Every name previously available as `kanari_types::transaction::X` is
// re-exported here, so downstream imports keep working unchanged.
mod kind;
mod native;
mod objects;
mod signed;

pub use kind::Transaction;
pub use native::NativeCall;
pub use objects::{
    GasPayment, ObjectChange, ObjectChangeKind, ObjectGraphEdge, ObjectGraphEdgeKind, ObjectInput,
    ObjectOwnerKind, ObjectRef, PublishedModule, TransactionEffects,
};
pub use signed::{SignedTransaction, VerifiedSignedTransaction};

#[cfg(test)]
mod tests;
