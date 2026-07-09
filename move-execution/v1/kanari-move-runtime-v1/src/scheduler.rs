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
        let mut waves: Vec<(std::collections::BTreeSet<String>, Vec<SignedTransaction>)> =
            Vec::new();

        for tx in transactions {
            let access_keys = tx
                .transaction
                .object_access_keys()
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>();

            if access_keys.is_empty() {
                waves.push((std::collections::BTreeSet::new(), vec![tx]));
                continue;
            }

            let mut placed = false;
            for (wave_keys, wave_txs) in &mut waves {
                if wave_keys.is_disjoint(&access_keys) {
                    wave_keys.extend(access_keys.iter().cloned());
                    wave_txs.push(tx.clone());
                    placed = true;
                    break;
                }
            }

            if !placed {
                waves.push((access_keys, vec![tx]));
            }
        }

        waves.into_iter().map(|(_, txs)| txs).collect()
    }
}

#[cfg(test)]
#[path = "../tests/unit/scheduler_tests.rs"]
mod tests;
