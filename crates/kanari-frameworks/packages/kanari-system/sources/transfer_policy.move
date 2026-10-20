// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Creator-controlled transfer rules.
///
/// This is `sui::transfer_policy` and `sui::policy` **merged into one module**.
/// Sui can split them because Move lets a module read another's fields; this
/// runtime's legacy edition does not, so `friend` access to private fields plus
/// a two-way `use` would be a dependency cycle. Fields are private to the
/// defining module in every edition, so keeping the rule table here and
/// exposing it through witness-gated functions is the only way to get the same
/// API surface without the cycle. `use kanari_system::policy::*` still resolves
/// for rule modules.
///
/// ## Enforcement
///
/// The safety mechanism is the shape of `TransferRequest`, not a runtime check.
/// It is declared with **no abilities at all** — no `store`, no `drop`, no
/// `copy`. A transaction cannot put one away and cannot abandon it: the only
/// way out is `confirm_request`, which refuses unless every registered rule
/// has submitted its receipt. A transaction that creates a request and forgets
/// it fails to execute.
///
/// Without this, "the buyer must pay a royalty" would be a convention. Here it
/// is a type-level constraint: there is no `drop` to fall back on.
///
/// ## Rules
///
/// A rule is a zero-sized witness type in the rule's own module
/// (`struct Rule has drop {}`), which namespaces its state and means adding a
/// rule never changes `TransferPolicy`'s layout. Only a function taking that
/// witness can write that rule's receipt, so a rule's module is the sole
/// authority on whether it has been satisfied.
///
/// Collected value is tracked as plain `u64` amounts per rule plus a running
/// `Balance<T>`; `AnyRule` must stay type-erased to live in the rule table.
module kanari_system::transfer_policy {
    use std::bcs;
    use std::vector;
    use kanari_system::object::{Self, ID, UID};
    use kanari_system::package::{Self, Publisher};
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;
    use kanari_system::vec_map::{Self, VecMap};
    use kanari_system::vec_set::{Self, VecSet};

    // === Policy errors ===
    /// Not every registered rule reported before confirmation.
    const E_INCOMPLETE_RECEIPTS: u64 = 0;

    // === Rule registry errors ===
    /// The rule named by `R` is not registered on this policy.
    const E_NO_RULE: u64 = 10;

    /// A rule exists that this call did not register.
    const E_RULE_ALREADY_PRESENT: u64 = 11;

    /// A receipt or collection was submitted for an unregistered rule.
    const E_NO_SUCH_RULE: u64 = 12;

    /// Type-erased state for one rule. `config` holds BCS bytes the rule module
    /// encoded; this module cannot interpret them, which is the point.
    struct AnyRule has drop, store {
        /// Receipt names collected so far.
        receipts: VecSet<vector<u8>>,
        /// BCS-encoded rule config, written by the rule's own module.
        config: vector<u8>,
    }

    /// Registered rules, keyed by rule type name. Type-erased so adding a rule
    /// never changes this struct's layout.
    struct TransferPolicy<phantom T: key + store> has key, store {
        id: UID,
        rules: VecMap<vector<u8>, AnyRule>,
        /// Per-rule collected totals, keyed the same way as `rules`. Plain
        /// amounts so the table above stays type-erased.
        amounts: VecMap<vector<u8>, u64>,
        /// Total the owner is owed across all rules, in base units.
        ///
        /// Tracked as a `u64` rather than a `Balance<T>`: `Balance<T>` is
        /// neither `copy` nor `drop`, so it cannot live in a struct that also
        /// needs to be shareable, destructible, and reassignable. Rules move
        /// the actual `Coin<T>` to the beneficiary directly and record the
        /// amount here for the owner's accounting, so no custody is needed and
        /// nothing can be stranded inside the policy.
        owed: u64,
    }

    /// Owner authority over `T`'s policy. Whoever holds this can change the
    /// rules, so it is the object that must never leak.
    struct TransferPolicyCap<phantom T: key + store> has key, store {
        id: UID,
    }

    /// A pending transfer awaiting rule resolution. No abilities, on purpose.
    struct TransferRequest<phantom T: key> {
        /// The item being traded.
        item: ID,
        /// Amount the buyer paid, for percentage-based rules.
        paid: u64,
        /// Where the item came from (e.g. the kiosk id).
        from: ID,
        /// Receipt names collected so far.
        receipts: VecSet<vector<u8>>,
    }

    // ==================== Policy lifecycle ====================

    /// Create a policy and its owner cap for `T`.
    ///
    /// Sharing the policy is what makes `T` tradable in a kiosk. A type whose
    /// policy is never shared simply cannot be traded — trading is opt-in, and
    /// the creator revokes it by destroying the policy.
    ///
    /// The publisher must be from `T`'s defining package
    /// (`package::assert_governs_type`): a policy minted from a foreign
    /// publisher would let a package inject royalty rules into trades it does
    /// not own.
    public fun new<T: key + store, W: drop>(
        pub: &Publisher<W>,
        ctx: &mut TxContext,
    ): (TransferPolicy<T>, TransferPolicyCap<T>) {
        package::assert_governs_type<T, W>(pub);
        let policy = TransferPolicy<T> {
            id: object::new(ctx),
            rules: vec_map::empty<vector<u8>, AnyRule>(),
            amounts: vec_map::empty<vector<u8>, u64>(),
            owed: 0,
        };
        let cap = TransferPolicyCap<T> { id: object::new(ctx) };
        (policy, cap)
    }

    /// Share the policy, making `T` tradable.
    public fun share<T: key + store>(policy: TransferPolicy<T>) {
        transfer::share_object(policy);
    }

    /// Destroy the policy, making `T` permanently non-tradable. Any pending
    /// request for `T` can no longer confirm.
    ///
    /// No value is stranded: rules pay beneficiaries directly, so the only thing
    /// lost is the `owed` accounting figure.
    public fun destroy<T: key + store>(policy: TransferPolicy<T>) {
        let TransferPolicy { id, rules, amounts, owed } = policy;
        let _ = rules;
        let _ = amounts;
        let _ = owed;
        object::delete(id);
    }

    /// Transfer the owner cap, handing over full control of `T`'s rules.
    public fun transfer_cap<T: key + store>(cap: TransferPolicyCap<T>, recipient: address) {
        transfer::public_transfer(cap, recipient);
    }

    /// Freeze the owner cap so rules can never be changed again.
    public fun freeze_cap<T: key + store>(cap: TransferPolicyCap<T>) {
        transfer::public_freeze_object(cap);
    }

    /// The policy's id.
    public fun id<T: key + store>(policy: &TransferPolicy<T>): ID {
        object::uid_to_inner(&policy.id)
    }

    /// The owner cap's id.
    public fun cap_id<T: key + store>(cap: &TransferPolicyCap<T>): ID {
        object::uid_to_inner(&cap.id)
    }

    // ==================== Requests (hot potato) ====================

    /// Open a request to transfer `item` for `paid`. The returned value has no
    /// abilities and must be resolved by `confirm_request` before the
    /// transaction can end.
    public fun new_request<T: key + store>(
        _policy: &TransferPolicy<T>,
        item: ID,
        paid: u64,
        from: ID,
    ): TransferRequest<T> {
        TransferRequest<T> {
            item,
            paid,
            from,
            receipts: vec_set::empty(),
        }
    }

    /// Resolve a request, consuming the hot potato.
    ///
    /// Aborts unless every registered rule reported. This is the one point
    /// where "the buyer paid" becomes "the rules are satisfied", and it is
    /// reachable only while holding a request.
    public fun confirm_request<T: key + store>(
        policy: &TransferPolicy<T>,
        request: TransferRequest<T>,
    ): (ID, u64, ID) {
        assert!(are_receipts_complete(policy, &request), E_INCOMPLETE_RECEIPTS);
        let TransferRequest { item, paid, from, receipts } = request;
        let _ = receipts;
        (item, paid, from)
    }

    /// True iff every registered rule has reported on `request`.
    public fun are_receipts_complete<T: key + store>(
        policy: &TransferPolicy<T>,
        request: &TransferRequest<T>,
    ): bool {
        let keys = vec_map::keys(&policy.rules);
        let i = 0;
        while (i < vector::length(&keys)) {
            let k = *vector::borrow(&keys, i);
            if (!vec_set::contains(&request.receipts, &k)) {
                return false
            };
            i = i + 1;
        };
        true
    }

    /// The item being traded.
    public fun item<T: key + store>(request: &TransferRequest<T>): ID {
        request.item
    }

    /// The amount paid, for rules that charge a percentage.
    public fun paid<T: key + store>(request: &TransferRequest<T>): u64 {
        request.paid
    }

    /// The source object (e.g. a kiosk).
    public fun from<T: key + store>(request: &TransferRequest<T>): ID {
        request.from
    }

    // ==================== Rule registry ====================

    /// Register `R` on `policy` with an encoded `config`.
    ///
    /// Requires the type owner's `TransferPolicyCap<T>`, and `R`'s witness.
    public fun add_rule<Rule: drop, T: key + store>(
        _rule: Rule,
        policy: &mut TransferPolicy<T>,
        _cap: &TransferPolicyCap<T>,
        config: vector<u8>,
    ) {
        let key = rule_key<Rule>();
        assert!(!vec_map::contains(&policy.rules, &key), E_RULE_ALREADY_PRESENT);
        vec_map::insert(&mut policy.rules, key, new_rule_config(config));
    }

    /// Replace the config of an already-registered rule.
    ///
    /// Removes first: `vec_map::insert` aborts on a duplicate key.
    public fun replace_rule<Rule: drop, T: key + store>(
        _rule: Rule,
        policy: &mut TransferPolicy<T>,
        _cap: &TransferPolicyCap<T>,
        config: vector<u8>,
    ) {
        let key = rule_key<Rule>();
        assert!(vec_map::contains(&policy.rules, &key), E_NO_RULE);
        let (removed_key, removed) = vec_map::remove(&mut policy.rules, &key);
        let _ = removed_key;
        let _ = removed;
        vec_map::insert(&mut policy.rules, key, new_rule_config(config));
    }

    /// Remove `R`. Receipts on past requests are unaffected because those were
    /// already resolved.
    public fun remove_rule<Rule: drop, T: key + store>(
        _rule: Rule,
        policy: &mut TransferPolicy<T>,
        _cap: &TransferPolicyCap<T>,
    ) {
        let key = rule_key<Rule>();
        assert!(vec_map::contains(&policy.rules, &key), E_NO_RULE);
        let (removed_key, removed) = vec_map::remove(&mut policy.rules, &key);
        let _ = removed_key;
        let _ = removed;
    }

    /// True iff `R` is registered.
    public fun has_rule<Rule: drop, T: key + store>(policy: &TransferPolicy<T>): bool {
        vec_map::contains(&policy.rules, &rule_key<Rule>())
    }

    /// Count of registered rules.
    public fun rule_count<T: key + store>(policy: &TransferPolicy<T>): u64 {
        vec_map::size(&policy.rules)
    }

    /// The encoded config bytes for `R`, for the rule module to decode.
    public fun rule_config<Rule: drop, T: key + store>(policy: &TransferPolicy<T>): vector<u8> {
        let key = rule_key<Rule>();
        assert!(vec_map::contains(&policy.rules, &key), E_NO_RULE);
        let config = vec_map::get(&policy.rules, &key).config;
        config
    }

    /// Build a fresh `AnyRule` with no receipts.
    public fun new_rule_config(config: vector<u8>): AnyRule {
        AnyRule {
            receipts: vec_set::empty(),
            config,
        }
    }

    /// Encode `config` as BCS bytes for `add_rule`.
    public fun encode_config<T: drop>(config: &T): vector<u8> {
        bcs::to_bytes(config)
    }

    // ==================== Rule execution ====================

    /// Submit the receipt for `R`, unblocking `confirm_request` for it.
    ///
    /// The only path that can mark a rule satisfied, and it requires `R`'s
    /// witness, so a rule's module is the only thing that can declare it paid.
    public fun add_receipt<Rule: drop, T: key + store>(
        _rule: Rule,
        policy: &mut TransferPolicy<T>,
        request: &mut TransferRequest<T>,
    ) {
        let key = rule_key<Rule>();
        assert!(vec_map::contains(&policy.rules, &key), E_NO_SUCH_RULE);
        vec_set::insert(&mut vec_map::get_mut(&mut policy.rules, &key).receipts, key);
        vec_set::insert(&mut request.receipts, key);
    }

    /// Number of rules that have reported on `request`.
    public fun receipts_count<T: key + store>(
        policy: &TransferPolicy<T>,
        request: &TransferRequest<T>,
    ): u64 {
        let keys = vec_map::keys(&policy.rules);
        let i = 0;
        let done = 0;
        while (i < vector::length(&keys)) {
            let k = *vector::borrow(&keys, i);
            if (vec_set::contains(&request.receipts, &k)) {
                done = done + 1;
            };
            i = i + 1;
        };
        done
    }

    /// True iff every registered rule has reported.
    public fun are_receipts_done<T: key + store>(
        policy: &TransferPolicy<T>,
        request: &TransferRequest<T>,
    ): bool {
        receipts_count(policy, request) == vec_map::size(&policy.rules)
    }

    /// Amount `R` has collected so far.
    public fun collected<Rule: drop, T: key + store>(policy: &TransferPolicy<T>): u64 {
        let key = rule_key<Rule>();
        assert!(vec_map::contains(&policy.rules, &key), E_NO_RULE);
        amount_of(policy, &key)
    }

    /// Record that `R` collected `amount`. Rule modules call this after moving
    /// the value to the beneficiary.
    ///
    /// Accumulates rather than overwrites, and goes through remove-then-insert
    /// because `vec_map::insert` aborts on an existing key.
    public fun record_collection<Rule: drop, T: key + store>(
        _rule: Rule,
        policy: &mut TransferPolicy<T>,
        amount: u64,
    ) {
        let key = rule_key<Rule>();
        assert!(vec_map::contains(&policy.rules, &key), E_NO_SUCH_RULE);
        let already = amount_of(policy, &key);
        set_amount(policy, &key, already + amount);
    }

    /// Take back what `R` collected. Owner-only via `TransferPolicyCap<T>`.
    /// The caller pays the returned amount out.
    public fun take_collection<Rule: drop, T: key + store>(
        _rule: Rule,
        policy: &mut TransferPolicy<T>,
        _cap: &TransferPolicyCap<T>,
    ): u64 {
        let key = rule_key<Rule>();
        assert!(vec_map::contains(&policy.rules, &key), E_NO_SUCH_RULE);
        let amount = amount_of(policy, &key);
        set_amount(policy, &key, 0);
        amount
    }

    /// Record that the owner has been paid `amount`, keeping the running total
    /// a rule author can query for accounting.
    ///
    /// Deliberately not custody: rule modules move the `Coin<T>` to the
    /// beneficiary themselves and call this for the ledger. Holding real value
    /// inside the policy would need a `Balance<T>`, which has neither `copy`
    /// nor `drop` and so cannot live in a shareable struct.
    public fun record_payout<T: key + store>(
        policy: &mut TransferPolicy<T>,
        amount: u64,
    ) {
        policy.owed = policy.owed + amount;
    }

    /// Total the owner has been paid across the policy's lifetime.
    public fun owed<T: key + store>(policy: &TransferPolicy<T>): u64 {
        policy.owed
    }

    /// Canonical registry key for rule type `R`: its type-name bytes.
    fun rule_key<Rule: drop>(): vector<u8> {
        let name = std::type_name::get<Rule>();
        *std::ascii::as_bytes(std::type_name::borrow_string(&name))
    }

    /// Read a ledger entry without requiring the rule to be registered.
    fun amount_of<T: key + store>(policy: &TransferPolicy<T>, key: &vector<u8>): u64 {
        if (vec_map::contains(&policy.amounts, key)) {
            *vec_map::get(&policy.amounts, key)
        } else {
            0
        }
    }

    /// Write a ledger entry. `vec_map::insert` aborts on a duplicate key, so an
    /// existing entry is removed first.
    fun set_amount<T: key + store>(policy: &mut TransferPolicy<T>, key: &vector<u8>, value: u64) {
        if (vec_map::contains(&policy.amounts, key)) {
            let (removed_key, removed) = vec_map::remove(&mut policy.amounts, key);
            let _ = removed_key;
            let _ = removed;
        };
        vec_map::insert(&mut policy.amounts, *key, value);
    }

    // ==================== Tests ====================
    #[test_only]
    use kanari_system::tx_context;

    #[test_only]
    /// One-time witness named after this module, so `package::new` accepts it.
    /// `key + store` because it stands in for the item type being traded.
    struct TRANSFER_POLICY has key, store, drop {}

    #[test]
    fun test_policy_and_request_lifecycle() {
        let ctx = tx_context::dummy();
        let pub = package::new<TRANSFER_POLICY>(TRANSFER_POLICY {}, &mut ctx);
        let (policy, cap) = new<TRANSFER_POLICY, TRANSFER_POLICY>(&pub, &mut ctx);

        let item = object::id_from_address(@0x1);
        let from = object::id_from_address(@0x2);
        let request = new_request<TRANSFER_POLICY>(&policy, item, 500, from);
        assert!(paid(&request) == 500, 0);
        // No rules, so nothing is outstanding.
        assert!(are_receipts_complete(&policy, &request), 1);
        let (_item, paid_out, _from) = confirm_request(&policy, request);
        assert!(paid_out == 500, 2);

        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(pub);
    }

    #[test]
    #[expected_failure(abort_code = E_INCOMPLETE_RECEIPTS)]
    fun test_confirm_requires_every_receipt() {
        let ctx = tx_context::dummy();
        let pub = package::new<TRANSFER_POLICY>(TRANSFER_POLICY {}, &mut ctx);
        let (policy, cap) = new<TRANSFER_POLICY, TRANSFER_POLICY>(&pub, &mut ctx);
        add_rule<TRANSFER_POLICY, TRANSFER_POLICY>(
            TRANSFER_POLICY {},
            &mut policy,
            &cap,
            b"config",
        );
        let item = object::id_from_address(@0x1);
        let from = object::id_from_address(@0x2);
        // A rule is registered but never reported, so this must abort rather
        // than let the trade through for free.
        let request = new_request<TRANSFER_POLICY>(&policy, item, 500, from);
        confirm_request(&policy, request);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(pub);
    }

    #[test]
    fun test_receipt_unblocks_confirmation() {
        let ctx = tx_context::dummy();
        let pub = package::new<TRANSFER_POLICY>(TRANSFER_POLICY {}, &mut ctx);
        let (policy, cap) = new<TRANSFER_POLICY, TRANSFER_POLICY>(&pub, &mut ctx);
        add_rule<TRANSFER_POLICY, TRANSFER_POLICY>(
            TRANSFER_POLICY {},
            &mut policy,
            &cap,
            b"config",
        );
        assert!(rule_count(&policy) == 1, 0);
        assert!(has_rule<TRANSFER_POLICY, TRANSFER_POLICY>(&policy), 1);

        let item = object::id_from_address(@0x1);
        let from = object::id_from_address(@0x2);
        let request = new_request<TRANSFER_POLICY>(&policy, item, 500, from);
        assert!(!are_receipts_complete(&policy, &request), 2);
        add_receipt<TRANSFER_POLICY, TRANSFER_POLICY>(
            TRANSFER_POLICY {},
            &mut policy,
            &mut request,
        );
        assert!(receipts_count(&policy, &request) == 1, 3);
        assert!(are_receipts_complete(&policy, &request), 4);

        // Collection accounting.
        record_collection<TRANSFER_POLICY, TRANSFER_POLICY>(
            TRANSFER_POLICY {},
            &mut policy,
            50,
        );
        assert!(collected<TRANSFER_POLICY, TRANSFER_POLICY>(&policy) == 50, 5);
        assert!(
            take_collection<TRANSFER_POLICY, TRANSFER_POLICY>(
                TRANSFER_POLICY {},
                &mut policy,
                &cap,
            ) == 50,
            6
        );
        assert!(collected<TRANSFER_POLICY, TRANSFER_POLICY>(&policy) == 0, 7);

        confirm_request(&policy, request);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(pub);
    }

    #[test]
    #[expected_failure(abort_code = E_RULE_ALREADY_PRESENT)]
    fun test_cannot_register_same_rule_twice() {
        let ctx = tx_context::dummy();
        let pub = package::new<TRANSFER_POLICY>(TRANSFER_POLICY {}, &mut ctx);
        let (policy, cap) = new<TRANSFER_POLICY, TRANSFER_POLICY>(&pub, &mut ctx);
        add_rule<TRANSFER_POLICY, TRANSFER_POLICY>(
            TRANSFER_POLICY {},
            &mut policy,
            &cap,
            b"v1",
        );
        add_rule<TRANSFER_POLICY, TRANSFER_POLICY>(
            TRANSFER_POLICY {},
            &mut policy,
            &cap,
            b"v2",
        );
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(pub);
    }

    #[test]
    #[expected_failure(abort_code = E_NO_SUCH_RULE)]
    fun test_receipt_requires_registered_rule() {
        let ctx = tx_context::dummy();
        let pub = package::new<TRANSFER_POLICY>(TRANSFER_POLICY {}, &mut ctx);
        let (policy, cap) = new<TRANSFER_POLICY, TRANSFER_POLICY>(&pub, &mut ctx);
        let item = object::id_from_address(@0x1);
        let from = object::id_from_address(@0x2);
        let request = new_request<TRANSFER_POLICY>(&policy, item, 100, from);
        // No rule registered, so this aborts. The `confirm_request` below is
        // unreachable; it exists because `TransferRequest` has no `drop`, so the
        // compiler requires every path to consume it even when an earlier line
        // aborts. That requirement is the hot potato working as intended.
        add_receipt<TRANSFER_POLICY, TRANSFER_POLICY>(
            TRANSFER_POLICY {},
            &mut policy,
            &mut request,
        );
        let (_i, _p, _f) = confirm_request(&policy, request);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(pub);
    }

    #[test]
    #[expected_failure(abort_code = package::E_WRONG_PACKAGE)]
    /// A policy minted from a foreign publisher would let a package inject
    /// royalty rules into trades it does not own. The forged publisher stands
    /// in for a claim made by another package.
    fun new_refuses_foreign_publisher() {
        let ctx = tx_context::dummy();
        let fake = package::forged_publisher_for_test<TRANSFER_POLICY>(
            b"0x44::other::OTHER", &mut ctx,
        );
        let (policy, cap) = new<TRANSFER_POLICY, TRANSFER_POLICY>(&fake, &mut ctx);
        share(policy);
        freeze_cap(cap);
        transfer::public_freeze_object(fake);
    }
}
