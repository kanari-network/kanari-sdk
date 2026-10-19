// End-to-end coverage for the Sui-parity stack, wired together the way `init`
// does it: Publisher -> Display -> TransferPolicy -> royalty rule.
//
// The point of these tests is not that each primitive works in isolation (the
// framework's own tests cover that). It is that the pieces compose, and
// specifically that the royalty gate holds across the whole trade path.
#[test_only]
module kanari_pet::pet_tests {
    use kanari_pet::pet::{Self, KanariPet, FEE};
    use kanari_system::balance;
    use kanari_system::coin::{Self, Coin};
    use kanari_system::collection;
    use kanari_system::display;
    use kanari_system::object;
    use kanari_system::package::Publisher;
    use kanari_system::royalty_rule;
    use kanari_system::transfer;
    use kanari_system::transfer_policy::{Self, TransferPolicy, TransferPolicyCap};
    use std::vector;
    use std::string::String;
    use kanari_system::tx_context::{Self, TxContext};

    #[test_only]
    /// One-time witness for this test module, named after it in caps so
    /// `package::new` accepts it. Stands in for the package`s own witness,
    /// which lives in `kanari_pet::pet` and is not reachable from a test.
    struct PET_TESTS has drop {}

    const ALICE: address = @0xA11CE;
    const CREATOR: address = @0xC8EA70;

    /// Wire up a policy with the 5% royalty exactly as `init` does. The
    /// publisher is consumed and handed back so callers can dispose of it —
    /// `Publisher` has `store` but not `drop`.
    fun setup(
        ctx: &mut TxContext,
    ): (TransferPolicy<KanariPet>, TransferPolicyCap<KanariPet>, Publisher<PET_TESTS>) {
        let publisher = kanari_system::package::new<PET_TESTS>(PET_TESTS {}, ctx);
        let (policy, cap) = transfer_policy::new<KanariPet, PET_TESTS>(&publisher, ctx);
        royalty_rule::add<KanariPet>(&mut policy, &cap, 500, 0, CREATOR);
        (policy, cap, publisher)
    }

    /// `royalty_rule::pay` needs a `&mut TransferRequest`, and this edition has
    /// no `let mut`, so the hot potato is threaded through a helper and
    /// returned. It is the same value; only moved.
    fun pay_fee(
        policy: &mut TransferPolicy<KanariPet>,
        request: transfer_policy::TransferRequest<KanariPet>,
        payment: &mut coin::Coin<FEE>,
        ctx: &mut TxContext,
    ): transfer_policy::TransferRequest<KanariPet> {
        let request_ref = &mut request;
        royalty_rule::pay<KanariPet, FEE>(policy, request_ref, payment, ctx);
        request
    }

    #[test_only]
    /// Mint `amount` units of the fee currency for a test. Uses `pet`'s own
    /// `FEE` witness so the currency is the same one `trade` settles in.
    fun mint_fees(amount: u64, ctx: &mut TxContext): Coin<FEE> {
        let (treasury, _metadata) = coin::create_currency(
            pet::fee_witness(),
            9,
            b"TESTFEE",
            b"Test Fee",
            b"Royalties settle in this for tests",
            std::option::none(),
            ctx,
        );
        let payment = coin::mint<FEE>(&mut treasury, amount, ctx);
        transfer::public_freeze_object(treasury);
        transfer::public_freeze_object(_metadata);
        payment
    }

    #[test]
    fun test_royalty_gate_holds_for_a_full_trade() {
        let ctx = tx_context::dummy();
        let (policy, cap, publisher) = setup(&mut ctx);

        // A 1000-unit sale owes 5% = 50.
        assert!(pet::quote_royalty(&policy, 1_000) == 50, 0);

        let item = object::id_from_address(@0x1);
        let from = object::id_from_address(@0x2);
        let payment = mint_fees(1_000, &mut ctx);
        let request = transfer_policy::new_request<KanariPet>(&policy, item, 1_000, from);

        let request = pay_fee(&mut policy, request, &mut payment, &mut ctx);
        let (_item, paid, _from) =
            transfer_policy::confirm_request<KanariPet>(&policy, request);
        assert!(paid == 1_000, 1);
        // `pay` split the 5% fee off and forwarded it to the beneficiary, so
        // 950 must remain with the buyer.
        assert!(coin::value(&payment) == 950, 2);

        // `Coin` has no `drop` — a payment cannot be silently discarded.
        let balance = coin::into_balance<FEE>(payment);
        let _ = balance::destroy(balance);

        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(publisher);
    }

    #[test]
    #[expected_failure]
    fun test_trade_without_paying_royalty_aborts() {
        let ctx = tx_context::dummy();
        let (policy, cap, publisher) = setup(&mut ctx);

        let item = object::id_from_address(@0x1);
        let from = object::id_from_address(@0x2);
        let request = transfer_policy::new_request<KanariPet>(&policy, item, 1_000, from);

        // Deliberately skip `royalty_rule::pay`. No receipt is ever added, so
        // `confirm_request` aborts on the incomplete-receipt check. This is the
        // enforcement the hot potato buys: the buyer cannot opt out of the fee.
        let (_item, _paid, _from) =
            transfer_policy::confirm_request<KanariPet>(&policy, request);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(publisher);
    }

    #[test]
    fun test_display_is_type_keyed_and_readable() {
        let ctx = tx_context::dummy();
        let publisher = kanari_system::package::new<PET_TESTS>(PET_TESTS {}, &mut ctx);

        let keys = vector::empty<vector<u8>>();
        let values = vector::empty<vector<u8>>();
        vector::push_back(&mut keys, b"name");
        vector::push_back(&mut values, b"Kanari Pet");
        let d = display::create_with_fields<KanariPet, PET_TESTS>(
            &publisher,
            keys,
            values,
            &mut ctx,
        );

        // The type name is always a field, so a client can identify the display
        // from its object alone.
        assert!(display::field_count<KanariPet>(&d) == 2, 0);
        let name = display::get<KanariPet>(&d, b"name");
        assert!(*std::option::borrow(&name) == std::string::utf8(b"Kanari Pet"), 1);

        transfer::public_freeze_object(d);
        transfer::public_freeze_object(publisher);
    }

    #[test]
    fun test_supply_cap_is_enforced_through_the_showcase_mint() {
        let ctx = tx_context::dummy();
        let (coll, cap) = collection::create_collection(
            &mut ctx,
            b"Coll",
            b"D",
            b"https://b",
            b"https://w",
            1,
        );

        // One mint is allowed.
        pet::mint(&mut cap, &coll, ALICE, &mut ctx);
        assert!(collection::remaining(&cap) == 0, 0);
        // A second cannot: `consume_for_mint` bounds the monotonic `issued`
        // counter against `max_supply`, so faking burns cannot reopen supply.
        assert!(collection::issued(&cap) == 1, 1);

        // Neither `Collection` nor `NftCap` has `drop` — the latter lost it
        // during the supply-cap hardening so a cap cannot be silently
        // discarded and strand the remaining supply. Both must be disposed of.
        transfer::public_freeze_object(coll);
        transfer::public_freeze_object(cap);
    }

    #[test]
    #[expected_failure(location = kanari_system::collection, abort_code = 1)]
    fun test_second_mint_beyond_supply_aborts() {
        let ctx = tx_context::dummy();
        let (coll, cap) = collection::create_collection(
            &mut ctx,
            b"Coll",
            b"D",
            b"https://b",
            b"https://w",
            1,
        );
        pet::mint(&mut cap, &coll, ALICE, &mut ctx);
        // The second mint aborts. The freezes below are unreachable; they exist
        // because `Collection` and `NftCap` have no `drop`, so the compiler
        // requires every path to dispose of them even past an abort.
        pet::mint(&mut cap, &coll, ALICE, &mut ctx);
        transfer::public_freeze_object(coll);
        transfer::public_freeze_object(cap);
    }

    #[test_only]
    /// Mint a pet into the caller's hands so the test can read and mutate it —
    /// `mint` transfers to the tx sender, which unit tests cannot retrieve
    /// objects from.
    fun mint_here(
        cap: &mut collection::NftCap,
        coll: &collection::Collection,
        ctx: &mut TxContext,
    ): KanariPet {
        pet::mint_for_test(cap, coll, ctx)
    }

    #[test]
    /// Attributes survive the mint and resolve by key, not by position.
    fun test_attributes_are_carried_and_keyed() {
        let ctx = tx_context::dummy();
        let (coll, cap) = collection::create_collection(
            &mut ctx,
            b"Coll",
            b"D",
            b"https://b",
            b"https://w",
            10,
        );

        let keys = vector::empty<String>();
        let values = vector::empty<String>();
        vector::push_back(&mut keys, std::string::utf8(b"breed"));
        vector::push_back(&mut values, std::string::utf8(b"shiba"));
        vector::push_back(&mut keys, std::string::utf8(b"level"));
        vector::push_back(&mut values, std::string::utf8(b"7"));

        let pet = pet::mint_with_attributes_for_test(
            &mut cap,
            &coll,
            keys,
            values,
            &mut ctx,
        );

        assert!(pet::attribute_count(&pet) == 2, 0);
        assert!(
            std::option::is_some(&pet::attribute(&pet, &std::string::utf8(b"level"))),
            1,
        );
        // An absent key is `none`, not a shifted neighbour.
        assert!(
            std::option::is_none(&pet::attribute(&pet, &std::string::utf8(b"mood"))),
            2,
        );

        // Upsert does not add a second entry.
        pet::set_attribute(&mut pet, std::string::utf8(b"level"), std::string::utf8(b"8"));
        assert!(pet::attribute_count(&pet) == 2, 3);

        // Wholesale replacement swaps the whole set.
        let keys = vector::empty<String>();
        let values = vector::empty<String>();
        vector::push_back(&mut keys, std::string::utf8(b"mood"));
        vector::push_back(&mut values, std::string::utf8(b"happy"));
        pet::set_attributes(&mut pet, keys, values);
        assert!(pet::attribute_count(&pet) == 1, 4);
        assert!(!pet::has_attribute(&pet, &std::string::utf8(b"breed")), 5);
        assert!(pet::has_attribute(&pet, &std::string::utf8(b"mood")), 6);

        transfer::public_freeze_object(pet);
        transfer::public_freeze_object(coll);
        transfer::public_freeze_object(cap);
    }

    #[test]
    #[expected_failure(location = kanari_system::nft, abort_code = 0)]
    /// The length guard has to hold on the showcase mint too, not just inside
    /// the framework module — `kanari_pet` calls `nft::build_attributes`, but a
    /// test that only exercised `nft::Nft` would not prove that.
    fun test_mismatched_attribute_lengths_abort_at_mint() {
        let ctx = tx_context::dummy();
        let (coll, cap) = collection::create_collection(
            &mut ctx,
            b"Coll",
            b"D",
            b"https://b",
            b"https://w",
            10,
        );

        let keys = vector::empty<String>();
        let values = vector::empty<String>();
        vector::push_back(&mut keys, std::string::utf8(b"breed"));
        vector::push_back(&mut keys, std::string::utf8(b"level"));
        // One key, one value: not a pair.
        vector::push_back(&mut values, std::string::utf8(b"shiba"));

        let pet = pet::mint_with_attributes_for_test(
            &mut cap,
            &coll,
            keys,
            values,
            &mut ctx,
        );
        transfer::public_freeze_object(pet);
        transfer::public_freeze_object(coll);
        transfer::public_freeze_object(cap);
    }

    #[test]
    /// Burning returns the supply unit, so a collection can re-mint what it
    /// destroyed. This is the whole reason `burn` takes the cap and collection
    /// rather than just the pet.
    fun test_burn_returns_supply_to_the_cap() {
        let ctx = tx_context::dummy();
        let (coll, cap) = collection::create_collection(
            &mut ctx,
            b"Coll",
            b"D",
            b"https://b",
            b"https://w",
            1,
        );
        assert!(collection::remaining(&cap) == 1, 0);

        let pet = mint_here(&mut cap, &coll, &mut ctx);
        assert!(collection::remaining(&cap) == 0, 1);

        pet::burn(&mut cap, &coll, pet, &ctx);
        // The unit came back.
        assert!(collection::remaining(&cap) == 1, 2);

        transfer::public_freeze_object(coll);
        transfer::public_freeze_object(cap);
    }

    #[test]
    /// Burning returns the `remaining` unit, but it does **not** reopen minting.
    ///
    /// Worth pinning precisely because the two counters disagree:
    /// `return_from_burn` increments `remaining`, so the cap reads as having a
    /// slot free again, yet `consume_for_mint` bounds the monotonic
    /// `issued_counter` instead. A collection therefore cannot be over-minted no
    /// matter how many burn/mint cycles a holder runs — the cost is that a
    /// burned pet's slot is not genuinely reusable.
    fun test_burn_returns_remaining_without_reopening_minting() {
        let ctx = tx_context::dummy();
        let (coll, cap) = collection::create_collection(
            &mut ctx,
            b"Coll",
            b"D",
            b"https://b",
            b"https://w",
            1,
        );

        let pet = mint_here(&mut cap, &coll, &mut ctx);
        assert!(collection::issued(&cap) == 1, 0);
        assert!(collection::remaining(&cap) == 0, 1);

        pet::burn(&mut cap, &coll, pet, &ctx);
        // `remaining` went back up...
        assert!(collection::remaining(&cap) == 1, 2);
        // ...but `issued` is unchanged and still at the cap. The enforcement
        // half is `test_burn_mint_cycles_cannot_exceed_max_supply`.
        assert!(collection::issued(&cap) == 1, 3);

        transfer::public_freeze_object(coll);
        transfer::public_freeze_object(cap);
    }

    #[test]
    #[expected_failure(location = kanari_system::collection, abort_code = 1)]
    /// After a burn the cap reads as having room, but the monotonic
    /// `issued_counter` still refuses the mint. Burning increments `remaining`,
    /// so without that second bound a burn/mint cycle would manufacture supply
    /// out of nothing.
    fun test_burn_mint_cycles_cannot_exceed_max_supply() {
        let ctx = tx_context::dummy();
        let (coll, cap) = collection::create_collection(
            &mut ctx,
            b"Coll",
            b"D",
            b"https://b",
            b"https://w",
            1,
        );

        let pet = mint_here(&mut cap, &coll, &mut ctx);
        pet::burn(&mut cap, &coll, pet, &ctx);
        // `remaining` is back to 1, which is exactly what makes this look
        // mintable. It is not.
        assert!(collection::remaining(&cap) == 1, 0);

        let pet2 = mint_here(&mut cap, &coll, &mut ctx);
        transfer::public_freeze_object(pet2);

        transfer::public_freeze_object(coll);
        transfer::public_freeze_object(cap);
    }

    #[test]
    /// A pet destroyed while carrying attributes still returns its unit. The
    /// burn path destructures the token, so the map must not be dropped in a
    /// way that leaves the supply accounting half-applied.
    fun test_burn_a_pet_that_has_attributes() {
        let ctx = tx_context::dummy();
        let (coll, cap) = collection::create_collection(
            &mut ctx,
            b"Coll",
            b"D",
            b"https://b",
            b"https://w",
            2,
        );

        let keys = vector::empty<String>();
        let values = vector::empty<String>();
        vector::push_back(&mut keys, std::string::utf8(b"breed"));
        vector::push_back(&mut values, std::string::utf8(b"shiba"));
        let pet = pet::mint_with_attributes_for_test(
            &mut cap,
            &coll,
            keys,
            values,
            &mut ctx,
        );
        assert!(pet::attribute_count(&pet) == 1, 0);

        pet::burn(&mut cap, &coll, pet, &ctx);
        assert!(collection::remaining(&cap) == 2, 1);

        transfer::public_freeze_object(coll);
        transfer::public_freeze_object(cap);
    }

    #[test]
    #[expected_failure(location = kanari_system::collection, abort_code = 3)]
    /// Burning must be bound to the collection that issued the pet. A cap
    /// transferred to another collection would otherwise reclaim supply from an
    /// unrelated pool.
    fun test_burn_against_the_wrong_collection_aborts() {
        let ctx = tx_context::dummy();
        let (coll_a, cap_a) = collection::create_collection(
            &mut ctx,
            b"A",
            b"D",
            b"https://b",
            b"https://w",
            5,
        );
        let (coll_b, _cap_b) = collection::create_collection(
            &mut ctx,
            b"B",
            b"D",
            b"https://b",
            b"https://w",
            5,
        );

        let pet = mint_here(&mut cap_a, &coll_a, &mut ctx);
        // cap_a belongs to coll_a, not coll_b.
        pet::burn(&mut cap_a, &coll_b, pet, &ctx);

        transfer::public_freeze_object(coll_a);
        transfer::public_freeze_object(cap_a);
        transfer::public_freeze_object(coll_b);
        transfer::public_freeze_object(_cap_b);
    }
}
