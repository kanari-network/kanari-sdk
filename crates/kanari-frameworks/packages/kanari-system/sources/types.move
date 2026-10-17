// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Type-convention checks mirroring `sui::types`.
module kanari_system::types {
    /// True iff `T` follows the one-time-witness convention: a `drop` struct
    /// named after its defining module in all caps (e.g. `my_coin::MY_COIN`).
    ///
    /// Convention check only — it does not consult on-chain state. Combine
    /// with module-local construction (only the defining module can pack the
    /// witness value) to bound authority, as `coin` and `balance` do.
    public native fun is_one_time_witness<T: drop>(witness: &T): bool;

    /// Claim `T` for currency creation. Aborts when `T` was already claimed,
    /// giving each coin type a single canonical currency. The host persists
    /// claims; a second `create_currency`/`create_regulated_currency` for
    /// the same `T` aborts even across transactions.
    public native fun claim_witness<T: drop>(witness: &T);

    #[test_only]
    struct TYPES has drop {}

    #[test_only]
    struct NotAWitness has drop {}

    #[test]
    fun test_otw_convention_positive() {
        let w = TYPES {};
        assert!(is_one_time_witness(&w), 0);
    }

    #[test]
    fun test_otw_convention_negative() {
        let w = NotAWitness {};
        assert!(!is_one_time_witness(&w), 0);
        assert!(!is_one_time_witness<u64>(&42), 1);
    }
}
