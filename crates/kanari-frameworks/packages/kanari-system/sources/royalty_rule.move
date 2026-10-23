// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Percentage-or-floor royalty rule for `kanari_system::transfer_policy`.
///
/// Royalty is a *rule*, not a field on the token. That is the whole point of
/// the policy design: the fee applies to every trade of the type, the type
/// owner can change it at any time, and the change takes effect immediately
/// across all instances — none of which is possible if the rate is frozen into
/// each minted object.
///
/// ## Enforcement
///
/// `pay` is what makes this safe. It takes the policy mutably, the request
/// mutably, and the buyer's payment mutably; it splits the fee, sends it to the
/// beneficiary, and only then calls `add_receipt`. A caller who tries to skip
/// `pay` cannot confirm the request, because `TransferRequest` has no `drop`
/// and `confirm_request` refuses incomplete receipts.
///
/// ## Config encoding
///
/// `transfer_policy` stores rule config as opaque BCS bytes and cannot
/// interpret them, so this module encodes and decodes its own. The layout is
/// `(u64 amount_bp, u64 min_amount, address beneficiary)` in that order, read
/// back with `peel_u64` / `peel_address`.
module kanari_system::royalty_rule {
    use std::vector;
    use kanari_system::coin::{Self, Coin};
    use kanari_system::address;
    use kanari_system::transfer;
    use kanari_system::transfer_policy::{
        Self,
        TransferPolicy,
        TransferPolicyCap,
        TransferRequest,
    };
    use kanari_system::tx_context::TxContext;

    #[test_only]
    use kanari_system::package;

    #[test_only]
    use kanari_system::tx_context;

    /// `amount_bp` was above 100%.
    const E_BPS_TOO_LARGE: u64 = 0;

    /// The buyer's payment does not cover the fee.
    const E_INSUFFICIENT: u64 = 1;

    /// The beneficiary is the reserved system address.
    const E_INVALID_BENEFICIARY: u64 = 2;

    /// The stored config was not produced by `encode_config`.
    const E_BAD_CONFIG: u64 = 3;

    /// 100% expressed in basis points.
    const MAX_BPS: u64 = 10_000;

    /// The rule witness. Its type is the registry key, so this rule can never
    /// collide with another rule module.
    struct Rule has drop {}

    /// Owner action: install the royalty rule on `T`'s policy.
    ///
    /// `amount_bp` is the percentage of the sale price (500 = 5%). `min_amount`
    /// is a floor so a very cheap sale still pays something; the higher of the
    /// two applies. Setting `amount_bp` to 0 makes it a flat commission.
    public fun add<T: key + store>(
        policy: &mut TransferPolicy<T>,
        cap: &TransferPolicyCap<T>,
        amount_bp: u64,
        min_amount: u64,
        beneficiary: address,
    ) {
        assert!(amount_bp <= MAX_BPS, E_BPS_TOO_LARGE);
        assert!(beneficiary != @0x0, E_INVALID_BENEFICIARY);
        let config = encode_config(amount_bp, min_amount, beneficiary);
        transfer_policy::add_rule<Rule, T>(Rule {}, policy, cap, config);
    }

    /// Update the rule's terms in place.
    public fun update<T: key + store>(
        policy: &mut TransferPolicy<T>,
        cap: &TransferPolicyCap<T>,
        amount_bp: u64,
        min_amount: u64,
        beneficiary: address,
    ) {
        assert!(amount_bp <= MAX_BPS, E_BPS_TOO_LARGE);
        assert!(beneficiary != @0x0, E_INVALID_BENEFICIARY);
        let config = encode_config(amount_bp, min_amount, beneficiary);
        transfer_policy::replace_rule<Rule, T>(Rule {}, policy, cap, config);
    }

    /// Remove the royalty rule entirely, making trades of `T` free again.
    public fun remove<T: key + store>(
        policy: &mut TransferPolicy<T>,
        cap: &TransferPolicyCap<T>,
    ) {
        transfer_policy::remove_rule<Rule, T>(Rule {}, policy, cap);
    }

    /// The fee owed for a sale of `paid` base units.
    ///
    /// Callable before a trade so a buyer can quote the price off-chain or in a
    /// dry run without constructing a request.
    public fun fee_amount<T: key + store>(policy: &TransferPolicy<T>, paid: u64): u64 {
        let bytes = transfer_policy::rule_config<Rule, T>(policy);
        let amount_bp = decode_bp(bytes);
        let min_amount = decode_min(bytes);
        compute(paid, amount_bp, min_amount)
    }

    /// The fee formula: percentage of `paid`, floored at `min_amount`.
    public fun compute(paid: u64, amount_bp: u64, min_amount: u64): u64 {
        let share = (paid as u128) * (amount_bp as u128) / (MAX_BPS as u128);
        let share = (share as u64);
        if (share < min_amount) { min_amount } else { share }
    }

    /// Buyer action: pay the royalty and unblock the request.
    ///
    /// The fee is split off `payment` and sent straight to the beneficiary, so
    /// the policy never holds value — nothing can be stranded inside it. The
    /// receipt is added last: if the split or the transfer aborted, the receipt
    /// is never written and the request stays unconfirmable.
    ///
    /// `Asset` is the currency the fee settles in and `T` is the traded type.
    /// They are separate because a royalty is almost never denominated in the
    /// asset being traded — charging a NFT in itself would be circular. The
    /// canonical shape is an NFT type `T` with a stablecoin `Asset`, which is
    /// why this takes two type parameters rather than reusing one.
    public fun pay<T: key + store, Asset: key + store>(
        policy: &mut TransferPolicy<T>,
        request: &mut TransferRequest<T>,
        payment: &mut Coin<Asset>,
        ctx: &mut TxContext,
    ) {
        let bytes = transfer_policy::rule_config<Rule, T>(policy);
        let amount_bp = decode_bp(bytes);
        let min_amount = decode_min(bytes);
        let beneficiary = decode_beneficiary(bytes);

        let paid = transfer_policy::paid(request);
        let fee = compute(paid, amount_bp, min_amount);
        // A zero-price trade with no floor owes nothing, but the receipt is
        // still required so the trade can complete.
        if (fee > 0) {
            assert!(coin::value(payment) >= fee, E_INSUFFICIENT);
            let royalty = coin::split(payment, fee, ctx);
            transfer::public_transfer(royalty, beneficiary);
        };

        transfer_policy::record_payout<T>(policy, fee);
        transfer_policy::add_receipt<Rule, T>(Rule {}, policy, request);
    }

    /// Encoded config length: 8 (`amount_bp`) + 8 (`min_amount`) + 32
    /// (`beneficiary`).
    const CONFIG_LEN: u64 = 48;

    /// Pack the config for storage in the policy's opaque config slot.
    ///
    /// Hand-packed rather than `bcs::to_bytes` on a struct: the native
    /// serializer's behaviour on a plain (non-`#[bcs]`) struct is not something
    /// to depend on for a value that has to round-trip between modules. The
    /// layout is fixed-width and explicit, so decoding cannot drift.
    public fun encode_config(amount_bp: u64, min_amount: u64, beneficiary: address): vector<u8> {
        let bytes = vector::empty<u8>();
        vector::append(&mut bytes, u64_le(amount_bp));
        vector::append(&mut bytes, u64_le(min_amount));
        vector::append(&mut bytes, address::to_bytes(beneficiary));
        bytes
    }

    /// Little-endian u64, least-significant byte first.
    fun u64_le(value: u64): vector<u8> {
        let bytes = vector::empty<u8>();
        let i = 0;
        while (i < 8) {
            let shift = (i as u8) * 8;
            vector::push_back(&mut bytes, (((value >> shift) & 0xFF) as u8));
            i = i + 1;
        };
        bytes
    }

    /// Read a little-endian u64 out of `bytes` starting at `offset`.
    fun read_u64(bytes: &vector<u8>, offset: u64): u64 {
        let value = 0u64;
        let i = 0;
        while (i < 8) {
            let byte = (*vector::borrow(bytes, offset + i) as u64);
            value = value | (byte << ((i as u8) * 8));
            i = i + 1;
        };
        value
    }

    fun decode_bp(bytes: vector<u8>): u64 {
        read_u64(&bytes, 0)
    }

    fun decode_min(bytes: vector<u8>): u64 {
        read_u64(&bytes, 8)
    }

    fun decode_beneficiary(bytes: vector<u8>): address {
        assert!(vector::length(&bytes) >= CONFIG_LEN, E_BAD_CONFIG);
        let raw = vector::empty<u8>();
        let i = 16;
        while (i < CONFIG_LEN) {
            vector::push_back(&mut raw, *vector::borrow(&bytes, i));
            i = i + 1;
        };
        address::from_bytes(raw)
    }

    #[test]
    fun test_fee_formula() {
        // 5% of 10_000 = 500.
        assert!(compute(10_000, 500, 0) == 500, 0);
        // 2.5% of 1_001 = 25.025, truncated to 25.
        assert!(compute(1_001, 250, 0) == 25, 1);
        // The floor wins when the percentage is smaller.
        assert!(compute(100, 500, 10) == 10, 2);
        // A percentage above the floor wins.
        assert!(compute(10_000, 500, 10) == 500, 3);
        // A free trade with no floor owes nothing.
        assert!(compute(0, 500, 0) == 0, 4);
        // A free trade with a floor still owes the floor.
        assert!(compute(0, 0, 7) == 7, 5);
        // 100% is the cap.
        assert!(compute(1_000, 10_000, 0) == 1_000, 6);
        // Truncation never rounds up into overcharging: 1 * 5000bp = 0.5 -> 0.
        assert!(compute(1, 5_000, 0) == 0, 7);
    }

    #[test_only]
    /// Named after this module so `package::new` accepts it.
    struct ROYALTY_RULE has key, store, drop {}

    #[test]
    fun test_rule_roundtrip() {
        let ctx = tx_context::dummy();
        let pub = package::new<ROYALTY_RULE>(ROYALTY_RULE {}, &mut ctx);
        let (policy, cap) = transfer_policy::new<ROYALTY_RULE, ROYALTY_RULE>(&pub, &mut ctx);

        // Not installed yet.
        assert!(!transfer_policy::has_rule<Rule, ROYALTY_RULE>(&policy), 0);

        add<ROYALTY_RULE>(&mut policy, &cap, 500, 10, @0xB0B);
        assert!(transfer_policy::has_rule<Rule, ROYALTY_RULE>(&policy), 1);
        assert!(transfer_policy::rule_count(&policy) == 1, 2);
        assert!(fee_amount<ROYALTY_RULE>(&policy, 10_000) == 500, 3);
        assert!(fee_amount<ROYALTY_RULE>(&policy, 100) == 10, 4);

        // Update the terms: 250 bp = 2.5%, and the min floor drops to 0 so the
        // percentage applies even on a small sale.
        update<ROYALTY_RULE>(&mut policy, &cap, 250, 0, @0xC0C);
        assert!(fee_amount<ROYALTY_RULE>(&policy, 10_000) == 250, 5);
        // 2.5% of 100 = 2.5, truncated to 2. The old floor of 10 is gone.
        assert!(fee_amount<ROYALTY_RULE>(&policy, 100) == 2, 6);

        // Remove it.
        remove<ROYALTY_RULE>(&mut policy, &cap);
        assert!(!transfer_policy::has_rule<Rule, ROYALTY_RULE>(&policy), 7);

        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(pub);
    }

    #[test]
    #[expected_failure(abort_code = E_BPS_TOO_LARGE)]
    fun test_rejects_over_100_percent() {
        let ctx = tx_context::dummy();
        let pub = package::new<ROYALTY_RULE>(ROYALTY_RULE {}, &mut ctx);
        let (policy, cap) = transfer_policy::new<ROYALTY_RULE, ROYALTY_RULE>(&pub, &mut ctx);
        add<ROYALTY_RULE>(&mut policy, &cap, 10_001, 0, @0xB0B);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(pub);
    }

    #[test_only]
    use kanari_system::object;

    #[test_only]
    use std::option;

    #[test_only]
    /// The asset the royalty settles in (never instantiated as a value; it
    /// only exists because `pay` requires `Asset: key + store` and a mintable
    /// coin needs a witness).
    struct ASSET has key, store, drop {
        id: object::UID,
    }

    // vec_set::insert aborts its duplicate with code 0 (`EKeyAlreadyExists`
    // is not importable across modules), which is exactly what a second
    // `pay` on the same request trips. The literal `0` below cannot be a
    // named const: the docs generator treats every const (test-only or not)
    // as an abort-code description, and the real module already maps
    // `E_BPS_TOO_LARGE = 0`, so a second zero-code const is a hard error.
    #[test]
    fun test_pay_splits_the_fee_and_unblocks_confirmation() {
        let ctx = tx_context::dummy();
        let pub = package::new<ROYALTY_RULE>(ROYALTY_RULE {}, &mut ctx);
        let (policy, cap) = transfer_policy::new<ROYALTY_RULE, ROYALTY_RULE>(&pub, &mut ctx);
        add<ROYALTY_RULE>(&mut policy, &cap, 500, 0, @0xB0B);

        let (treasury, denycap, metadata) = coin::create_regulated_currency<ASSET>(
            ASSET { id: object::new(&mut ctx) },
            2, b"USD", b"Test Asset", b"", option::none(), &mut ctx,
        );
        let donation = coin::mint(&mut treasury, 1_000, &mut ctx);
        let request = transfer_policy::new_request<ROYALTY_RULE>(
            &policy,
            object::id_from_address(@0xC0DE),
            1_000,
            object::id_from_address(@0xF00D),
        );
        pay<ROYALTY_RULE, ASSET>(&mut policy, &mut request, &mut donation, &mut ctx);
        // 5% of 1_000 = 500bp * 1000 / 10000 = 50 left the payment.
        assert!(coin::value(&donation) == 950, 0);
        // The payout was recorded on the policy, and the hot potato is
        // confirmable only after `pay` wrote the receipt.
        assert!(transfer_policy::owed(&policy) == 50, 1);
        let (item, paid, from) = transfer_policy::confirm_request(&policy, request);
        assert!(object::id_to_address(&item) == @0xC0DE, 2);
        assert!(paid == 1_000, 3);
        assert!(object::id_to_address(&from) == @0xF00D, 4);
        // Cleanup: burning the leftover keeps the TreasuryCap arithmetic
        // honest (supply drops from 1_000 to 50).
        coin::burn(&mut treasury, donation);
        transfer::public_freeze_object(treasury);
        transfer::public_freeze_object(denycap);
        transfer::public_freeze_object(metadata);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(pub);
    }

    #[test]
    #[expected_failure(abort_code = E_INSUFFICIENT)]
    fun test_pay_aborts_when_payment_is_short() {
        let ctx = tx_context::dummy();
        let pub = package::new<ROYALTY_RULE>(ROYALTY_RULE {}, &mut ctx);
        let (policy, cap) = transfer_policy::new<ROYALTY_RULE, ROYALTY_RULE>(&pub, &mut ctx);
        add<ROYALTY_RULE>(&mut policy, &cap, 500, 0, @0xB0B);

        let (treasury, denycap, metadata) = coin::create_regulated_currency<ASSET>(
            ASSET { id: object::new(&mut ctx) },
            2, b"USD", b"Test Asset", b"", option::none(), &mut ctx,
        );
        // The trade is stated at 1_000, so 500bp owes a fee of 50, but the
        // buyer's payment carries only 49: `pay` must abort instead of
        // splitting what is not there.
        let payment = coin::mint(&mut treasury, 49, &mut ctx);
        let request = transfer_policy::new_request<ROYALTY_RULE>(
            &policy,
            object::id_from_address(@0xC0DE),
            1_000,
            object::id_from_address(@0xF00D),
        );
        pay<ROYALTY_RULE, ASSET>(&mut policy, &mut request, &mut payment, &mut ctx);
        // Everything below is unreachable at runtime (the `pay` above aborts
        // with E_INSUFFICIENT); it exists because the compiler requires every
        // path to consume the no-`drop` locals before returning. That is the
        // hot potato working as intended.
        coin::burn(&mut treasury, payment);
        let (_item, _paid, _from) = transfer_policy::confirm_request(&policy, request);
        transfer::public_freeze_object(treasury);
        transfer::public_freeze_object(denycap);
        transfer::public_freeze_object(metadata);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(pub);
    }

    #[test]
    #[expected_failure(abort_code = 0, location = kanari_system::vec_set)]
    fun test_pay_cannot_run_twice_on_one_request() {
        // A zero-price, floor-0 rule owes nothing, so the second pay is
        // purely the receipt write: it must abort on the duplicate instead of
        // letting one request be charged or confirmed twice.
        let ctx = tx_context::dummy();
        let pub = package::new<ROYALTY_RULE>(ROYALTY_RULE {}, &mut ctx);
        let (policy, cap) = transfer_policy::new<ROYALTY_RULE, ROYALTY_RULE>(&pub, &mut ctx);
        add<ROYALTY_RULE>(&mut policy, &cap, 0, 0, @0xB0B);

        let request_id = object::id_from_address(@0xC0DE);
        let from_id = object::id_from_address(@0xF00D);
        let request = transfer_policy::new_request<ROYALTY_RULE>(
            &policy, request_id, 0, from_id,
        );
        let payment = coin::zero<ASSET>(&mut ctx);
        pay<ROYALTY_RULE, ASSET>(&mut policy, &mut request, &mut payment, &mut ctx);
        // Receipt is in place and authoritative; nothing was owed so no coin
        // moved. A second `pay` must abort rather than re-add the receipt.
        assert!(transfer_policy::are_receipts_complete(&policy, &request), 0);
        pay<ROYALTY_RULE, ASSET>(&mut policy, &mut request, &mut payment, &mut ctx);
        // Runtime never reaches this point (the second `pay` aborts with the
        // duplicate-receipt abort of `vec_set::insert`, code 0); the rest of
        // the body consumes the no-`drop` locals as the compiler requires on
        // every path. The example is the same pattern used by the negative
        // fixtures in `transfer_policy`.
        let (_item, _paid, _from) = transfer_policy::confirm_request(&policy, request);
        let _burned = coin::into_balance(payment);
        let _ = kanari_system::balance::destroy_for_testing(_burned);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(pub);
    }
}
