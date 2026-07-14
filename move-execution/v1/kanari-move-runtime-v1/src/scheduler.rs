// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use kanari_types::transaction::SignedTransaction;

/// Transaction scheduler used by deterministic checkpoint execution.
///
/// Security note: transaction arguments and native effects are not yet represented by
/// a complete deterministic read/write set. Until that exists, every transaction is
/// placed in its own wave. This preserves canonical transaction order and prevents two
/// transactions with an unobserved conflict from executing against the same snapshot.
pub struct TransactionScheduler;

impl TransactionScheduler {
    pub fn schedule(transactions: Vec<SignedTransaction>) -> Vec<Vec<SignedTransaction>> {
        // `Transaction::object_access_keys` is deliberately not used as a parallelism
        // proof. Its keys distinguish read, mutable and gas roles and do not include
        // every native/argument effect, so disjoint strings do not imply disjoint state.
        // Preserve canonical order and execute against a fresh state after every tx.
        transactions.into_iter().map(|tx| vec![tx]).collect()
    }
}

#[cfg(test)]
#[path = "../tests/unit/scheduler_tests.rs"]
mod tests;
