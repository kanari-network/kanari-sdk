// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use kanari_types::transaction::SignedTransaction;

/// Transaction scheduler used by deterministic checkpoint execution.
///
/// The scheduler only bounds speculative work. Safety is decided after execution from
/// resolver traces plus canonical effects; the engine serially retries conflicts.
pub struct TransactionScheduler;

impl TransactionScheduler {
    /// Bounds speculative memory, resolver traces, and retry amplification.
    pub const MAX_SPECULATIVE_WAVE_SIZE: usize = 64;

    pub const fn requires_serial_execution() -> bool {
        false
    }

    pub fn schedule(transactions: Vec<SignedTransaction>) -> Vec<Vec<SignedTransaction>> {
        transactions
            .chunks(Self::MAX_SPECULATIVE_WAVE_SIZE)
            .map(<[SignedTransaction]>::to_vec)
            .collect()
    }
}

#[cfg(test)]
#[path = "../tests/unit/scheduler_tests.rs"]
mod tests;
