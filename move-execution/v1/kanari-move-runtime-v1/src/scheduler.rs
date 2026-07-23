// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use kanari_types::transaction::SignedTransaction;
use std::collections::BTreeSet;

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
        let mut scheduled: Vec<Vec<SignedTransaction>> = Vec::new();
        let mut current_wave = Vec::new();
        let mut current_reserved_keys = BTreeSet::new();

        for signed_tx in transactions {
            let conflict_keys = signed_tx
                .transaction
                .get_conflict_keys()
                .into_iter()
                .collect::<BTreeSet<_>>();

            let wave_full = current_wave.len() >= Self::MAX_SPECULATIVE_WAVE_SIZE;
            let conflicts_with_current = conflict_keys
                .iter()
                .any(|conflict_key| current_reserved_keys.contains(conflict_key));

            if !current_wave.is_empty() && (wave_full || conflicts_with_current) {
                scheduled.push(std::mem::take(&mut current_wave));
                current_reserved_keys.clear();
            }

            current_reserved_keys.extend(conflict_keys);
            current_wave.push(signed_tx);
        }

        if !current_wave.is_empty() {
            scheduled.push(current_wave);
        }

        scheduled
    }
}

#[cfg(test)]
#[path = "../tests/unit/scheduler_tests.rs"]
mod tests;
