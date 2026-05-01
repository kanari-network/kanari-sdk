// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod blockchain;
pub mod consensus;

/// Calculate BFT quorum threshold using formula: 2f + 1
///
/// where f = floor((n-1)/3) is the maximum number of Byzantine (faulty) nodes
/// that the system can tolerate while maintaining safety and liveness.
///
/// # Formula Derivation
///
/// In Byzantine Fault Tolerance (BFT) systems:
/// - **n** = total number of validators/authorities
/// - **f** = maximum tolerated Byzantine faults = ⌊(n-1)/3⌋
/// - **quorum** = minimum votes needed for consensus = 2f + 1
///
/// This ensures that even if f nodes are malicious, there are still at least
/// f+1 honest nodes in any quorum, guaranteeing that two different quorums
/// must overlap by at least one honest node.
///
/// # Examples
///
/// ```
/// use centauri::calculate_quorum;
///
/// // 4 validators: can tolerate 1 faulty node
/// assert_eq!(calculate_quorum(4), 3);  // f=1, quorum=3
///
/// // 7 validators: can tolerate 2 faulty nodes
/// assert_eq!(calculate_quorum(7), 5);  // f=2, quorum=5
///
/// // 10 validators: can tolerate 3 faulty nodes
/// assert_eq!(calculate_quorum(10), 7); // f=3, quorum=7
///
/// // Edge case: 0 validators
/// assert_eq!(calculate_quorum(0), 0);
/// ```
///
/// # Safety Properties
///
/// The quorum size ensures:
/// 1. **Safety**: No two conflicting decisions can both reach quorum
/// 2. **Liveness**: Honest nodes can always make progress if ≤ f nodes are faulty
/// 3. **Optimality**: Uses minimum number of votes required (no waste)
///
/// # References
///
/// - Castro, M., & Liskov, B. (1999). Practical Byzantine Fault Tolerance.
///   OSDI '99. <https://pmg.csail.mit.edu/papers/osdi99.pdf>
/// - Yin, M., et al. (2019). HotStuff: BFT Consensus with Linearity and Responsiveness.
///   PODC '19.
///
/// # Arguments
///
/// * `total_authorities` - Total number of validators in the committee
///
/// # Returns
///
/// Minimum number of votes/signatures required to reach consensus
///
/// # Panics
///
/// Never panics. Returns 0 if `total_authorities` is 0.
pub fn calculate_quorum(total_authorities: usize) -> usize {
    if total_authorities == 0 {
        return 0;
    }
    let f = (total_authorities - 1) / 3;
    2 * f + 1
}
