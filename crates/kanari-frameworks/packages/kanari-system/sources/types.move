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
    /// claims; a second `create_currency`/`create_regulated_currency` for the
    /// same `T` aborts even across transactions.
    public native fun claim_witness<T: drop>(witness: &T);

    /// Claim `T` for exactly one `Display<T>`, aborting if it was already claimed.
    /// Persisted by the host across transactions.
    ///
    /// This is what enforces one authority per type per kind. Without it the
    /// consumer signatures take an *unbound* `&Publisher<W>`, so on its own a
    /// publisher proves only that the caller owns some type in its own package —
    /// enough to mint a second `Display<T>` for a type you do not own and have
    /// wallets render two different names for the same token.
    ///
    /// Separate from `claim_witness` (currency) and `claim_transfer_policy`, so
    /// one type can legitimately hold one of each.
    public native fun claim_display<T: key>();

    /// Claim `T` for exactly one `TransferPolicy<T>`, aborting if already
    /// claimed. See `claim_display` for why this is needed.
    ///
    /// Consequence: `transfer_policy::destroy` is terminal. A type whose policy
    /// is destroyed can never be made tradable again.
    public native fun claim_transfer_policy<T: key>();

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
