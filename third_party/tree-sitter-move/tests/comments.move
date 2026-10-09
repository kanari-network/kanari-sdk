// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Module documentation block.
/// Spans multiple lines.
module test::comments {
    use test::other;

    // Section: constants

    /// Maximum value.
    const MAX: u64 = 100; // trailing note

    // A standalone note.

    /// Documents the struct.
    struct Documented has drop {
        /// Documents the field.
        value: u64, // trailing field note
    }

    /// Documents the function.
    public fun documented(): u64 {
        1
    }
}
