/// Thin entry/view wrappers over `kanari_system::types` so the runtime can
/// drive the one-time-witness convention check and the witness-claim registry
/// through entry/view execution. `claim_witness` has host-side effects (the
/// runtime persists claims and aborts double claims even across
/// transactions), which only a runtime-level test can observe.
module tester::types_e2e {
    use kanari_system::types;

    /// Follows the one-time-witness convention: named after this module in
    /// all caps.
    struct TYPES_E2E has drop {}

    /// Same shape, wrong name: not a witness for this module.
    struct NOT_WITNESS has drop {}

    /// A second, unrelated claimable type: claiming it must not disturb the
    /// `TYPES_E2E` claim and vice versa.
    struct OTHER has drop {}

    public entry fun claim_otw() {
        let w = TYPES_E2E {};
        types::claim_witness(&w);
    }

    /// Claims twice inside one transaction: the second claim must abort via
    /// the transaction-local pending set, before durable state is consulted.
    public entry fun claim_twice() {
        let w = TYPES_E2E {};
        types::claim_witness(&w);
        types::claim_witness(&w);
    }

    public entry fun claim_other() {
        let w = OTHER {};
        types::claim_witness(&w);
    }

    public fun is_otw(): bool {
        let w = TYPES_E2E {};
        types::is_one_time_witness(&w)
    }

    public fun is_not_otw(): bool {
        let w = NOT_WITNESS {};
        types::is_one_time_witness(&w)
    }
}
