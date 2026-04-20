// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod blockchain;
pub mod consensus;


/// BFT quorum calculation helper (consistent with centauri consensus)
pub fn calculate_quorum(total_authorities: usize) -> usize {
    if total_authorities == 0 {
        return 0;
    }
    let f = (total_authorities - 1) / 3;
    2 * f + 1
}
