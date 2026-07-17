// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod scheduler;
pub use scheduler::TransactionScheduler;

pub mod changeset;
mod common;
mod genesis;
mod kanari_gas_meter;
pub mod move_runtime;

pub mod state;
pub mod storage;

pub use changeset::ChangeSet;

/// Immutable view of uncommitted canonical state used by speculative execution.
pub type StateOverlay = std::sync::Arc<std::collections::BTreeMap<Vec<u8>, Option<Vec<u8>>>>;
pub type StateOverlayView<'a> = Option<&'a StateOverlay>;

/// Grouped runtime exports. Prefer these paths for new code.
pub mod runtime {
    pub use crate::move_runtime::MoveRuntime;
}
