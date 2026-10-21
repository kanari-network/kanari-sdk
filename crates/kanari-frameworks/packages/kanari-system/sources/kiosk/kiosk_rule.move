// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// The kiosk rule: an item whose policy registers this rule can only be
/// traded through `kanari_system::kiosk`.
///
/// Mirrors `sui::kiosk_rule`. The point is to turn a documented convention —
/// "policy sales should go through the seller's kiosk" — into a mechanism: if
/// the rule is registered, `transfer_policy::confirm_request` aborts unless
/// the rule's receipt is on the request, and the receipt can only ever be
/// minted inside `kiosk::settle`, which is `friend`-gated to the kiosk module.
/// A trade that bypasses the kiosk cannot produce the receipt, so it cannot
/// confirm.
///
/// ## Why the kiosk can be trusted here
///
/// A kiosk rule receipt is not issued on a credential the caller might
/// forge. `kiosk::purchase` and `kiosk::purchase_with_cap` take the real
/// `Kiosk` object as a `&mut` argument — live object identity enforced by the
/// runtime — and `settle` marks the request only in that context. Nothing
/// else can reach this module's receipt function: it is `public(friend)` and
/// the module's only friend is the kiosk itself.
///
/// ## Deliberate scope
///
/// The rule carries no config: it is either registered or not. Availability
/// (`is_enabled`) is exposed so a seller UI can show where the item can be
/// sold before the rules are applied.
module kanari_system::kiosk_rule {
    use kanari_system::transfer_policy::{
        Self, TransferPolicy, TransferPolicyCap, TransferRequest,
    };

    // The only path that can mint kiosk-rule receipts. Reachable only from
    // the kiosk module, which calls it inside `settle` — after the sale was
    // validated against the live kiosk object.
    friend kanari_system::kiosk;

    /// The rule witness. Zero-sized and `drop`-able like every transfer-policy
    /// rule; authority lives in the friend gate, not in the value.
    struct Rule has drop {}

    public fun add_rule<T: key + store>(
        policy: &mut TransferPolicy<T>,
        cap: &TransferPolicyCap<T>,
    ) {
        transfer_policy::add_rule<Rule, T>(Rule {}, policy, cap, b"");
    }

    public fun remove_rule<T: key + store>(
        policy: &mut TransferPolicy<T>,
        cap: &TransferPolicyCap<T>,
    ) {
        transfer_policy::remove_rule<Rule, T>(Rule {}, policy, cap);
    }

    /// True iff `T`'s policy requires kiosk-only trading.
    public fun is_enabled<T: key + store>(policy: &TransferPolicy<T>): bool {
        transfer_policy::has_rule<Rule, T>(policy)
    }

    /// Mint the kiosk-rule receipt for `request`.
    ///
    /// `public(friend)`: reachable only from the kiosk module, which calls it
    /// from inside `settle` — i.e. after the sale was validated against the
    /// live kiosk object.
    public(friend) fun kiosk_receipt<T: key + store>(
        policy: &mut TransferPolicy<T>,
        request: &mut TransferRequest<T>,
    ) {
        transfer_policy::add_receipt<Rule, T>(Rule {}, policy, request);
    }

    // ==================== Tests ====================
    #[test_only]
    use kanari_system::object;

    #[test_only]
    use kanari_system::package;

    #[test_only]
    use kanari_system::tx_context;

    #[test_only]
    use kanari_system::transfer;

    #[test_only]
    /// A second item type serving as the one-time publisher witness (it needs
    /// `drop` for `package::new`, and `key + store` to stand in for the traded
    /// type bound into the policy).
    struct KIOSK_RULE has key, store, drop {
        id: kanari_system::object::UID,
    }

    #[test]
    fun add_and_remove_rule() {
        let ctx = tx_context::dummy();
        let pub = package::new<KIOSK_RULE>(KIOSK_RULE { id: object::new(&mut ctx) }, &mut ctx);
        let (policy, cap) = transfer_policy::new<KIOSK_RULE, KIOSK_RULE>(&pub, &mut ctx);
        assert!(!is_enabled(&policy), 0);
        add_rule(&mut policy, &cap);
        assert!(is_enabled(&policy), 1);
        remove_rule(&mut policy, &cap);
        assert!(!is_enabled(&policy), 2);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(pub);
    }

    #[test]
    #[expected_failure(abort_code = 0, location = kanari_system::transfer_policy)]
    /// With the rule registered, a request minted outside the kiosk cannot be
    /// confirmed: `confirm_request` needs the rule's receipt and no path can
    /// mint it from here.
    fun bypassing_the_kiosk_cannot_confirm() {
        let ctx = tx_context::dummy();
        let pub = package::new<KIOSK_RULE>(KIOSK_RULE { id: object::new(&mut ctx) }, &mut ctx);
        let (policy, cap) = transfer_policy::new<KIOSK_RULE, KIOSK_RULE>(&pub, &mut ctx);
        add_rule(&mut policy, &cap);

        let item = object::id_from_address(@0x1);
        let from = object::id_from_address(@0x2);
        let request = transfer_policy::new_request<KIOSK_RULE>(&policy, item, 100, from);
        let (_i, _p, _f) = transfer_policy::confirm_request(&policy, request);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(pub);
    }
}
