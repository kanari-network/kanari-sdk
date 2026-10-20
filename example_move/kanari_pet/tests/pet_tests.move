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
        vector::push_back(&mut keys, std::string::utf8(b"mood"));
        vector::push_back(&mut values, std::string::utf8(b"calm"));

        let pet = pet::mint_with_attributes_for_test(
            &mut cap,
            &coll,
            keys,
            values,
            &mut ctx,
        );

        assert!(pet::attribute_count(&pet) == 2, 0);
        assert!(
            std::option::is_some(&pet::attribute(&pet, &std::string::utf8(b"breed"))),
            1,
        );
        // An absent key is `none`, not a shifted neighbour.
        assert!(
            std::option::is_none(&pet::attribute(&pet, &std::string::utf8(b"nickname"))),
            2,
        );

        // Upsert does not add a second entry.
        pet::set_string_attribute(&mut pet, std::string::utf8(b"breed"), std::string::utf8(b"akita"));
        assert!(pet::attribute_count(&pet) == 2, 3);
        assert!(
            *std::option::borrow(&pet::attribute(&pet, &std::string::utf8(b"breed")))
                == std::string::utf8(b"akita"),
            4,
        );

        // Wholesale replacement swaps the free-form set.
        let keys = vector::empty<String>();
        let values = vector::empty<String>();
        vector::push_back(&mut keys, std::string::utf8(b"nickname"));
        vector::push_back(&mut values, std::string::utf8(b"happy"));
        pet::set_string_attributes(&mut pet, keys, values);
        assert!(!pet::has_attribute(&pet, &std::string::utf8(b"breed")), 5);
        assert!(!pet::has_attribute(&pet, &std::string::utf8(b"mood")), 6);
        assert!(pet::has_attribute(&pet, &std::string::utf8(b"nickname")), 7);

        transfer::public_freeze_object(pet);
        transfer::public_freeze_object(coll);
        transfer::public_freeze_object(cap);
    }

    // ==================== Progression ====================
    #[test_only]
    /// Stats with round numbers: $1.00 base, 10 power per level, cap 50.
    fun test_stats(ctx: &mut TxContext): pet::Stats {
        pet::new_stats(CREATOR, 1_000_000, 10, 1_000, ctx)
    }

    #[test_only]
    /// `amount` micro-USD (USD has 6 decimals, so 1_000_000 == $1.00). The
    /// `USD` witness is module-private, so the framework's own `#[test_only]`
    /// helper does the currency creation.
    fun usd(amount: u64, ctx: &mut TxContext): Coin<kanari_system::usd::USD> {
        kanari_system::usd::mint_for_test(amount, ctx)
    }

    #[test_only]
    /// The stand-up every progression test needs, in one call: a collection
    /// with its cap, the shared `Stats`, and a pet already minted at `level`.
    /// Keeps the `create_collection` argument noise out of the test bodies.
    fun stage(
        level: u64,
        ctx: &mut TxContext,
    ): (
        KanariPet,
        collection::Collection,
        collection::NftCap,
        pet::Stats,
    ) {
        let (coll, cap) = collection::create_collection(
            ctx, b"Coll", b"D", b"https://b", b"https://w", 10,
        );
        let stats = test_stats(ctx);
        let pet = pet::mint_with_level_for_test(&mut cap, &coll, &stats, level, ctx);
        (pet, coll, cap, stats)
    }

    #[test_only]
    /// Freeze the four objects every progression test ends with. The bodies
    /// end on one line instead of four boilerplate calls.
    fun seal(
        pet: KanariPet,
        coll: collection::Collection,
        cap: collection::NftCap,
        stats: pet::Stats,
    ) {
        transfer::public_freeze_object(pet);
        transfer::public_freeze_object(coll);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(stats);
    }

    #[test]
    /// The cost is flat: every level costs exactly `base_cost_usd` ($1.00),
    /// whatever the current level. Pinned at the ceiling too, so nobody can
    /// reintroduce escalation in one branch and break client quotes.
    fun test_upgrade_cost_is_flat_per_level() {
        let ctx = tx_context::dummy();
        let stats = test_stats(&mut ctx);
        assert!(pet::upgrade_cost(&stats, 0) == 1_000_000, 0);
        assert!(pet::upgrade_cost(&stats, 1) == 1_000_000, 1);
        assert!(pet::upgrade_cost(&stats, 9) == 1_000_000, 2);
        assert!(pet::upgrade_cost(&stats, 999) == 1_000_000, 3);

        transfer::public_freeze_object(stats);
    }

    #[test]
    /// A paid upgrade raises level, recomputes power, moves the rarity band,
    /// and debits exactly the quoted cost.
    fun test_upgrade_spends_usd_and_advances_the_pet() {
        let ctx = tx_context::dummy();
        let (pet, coll, cap, stats) = stage(0, &mut ctx);

        // Level 0: nothing yet.
        assert!(pet::level(&pet) == 0, 0);
        assert!(pet::power(&pet) == 0, 1);
        assert!(pet::rarity(&pet) == std::string::utf8(b"Common"), 2);

        let payment = usd(1_000_000, &mut ctx);
        pet::upgrade(&mut pet, &stats, &mut payment, &mut ctx);

        assert!(pet::level(&pet) == 1, 3);
        // Power is derived from level, not caller-supplied.
        assert!(pet::power(&pet) == 10, 4);
        // Level 1 is still Common.
        assert!(pet::rarity(&pet) == std::string::utf8(b"Common"), 5);
        // The exact quoted cost left the payer.
        assert!(coin::value(&payment) == 0, 6);

        let balance = coin::into_balance(payment);
        let _ = balance::destroy(balance);
        seal(pet, coll, cap, stats);
    }

    #[test]
    /// Crossing a band boundary changes rarity. Pinned at the exact thresholds
    /// so an off-by-one in the ladder cannot ship silently.
    fun test_rarity_bands_follow_the_level() {
        let ctx = tx_context::dummy();
        let stats = test_stats(&mut ctx);
        assert!(pet::rarity_for_level(0) == std::string::utf8(b"Common"), 0);
        assert!(pet::rarity_for_level(4) == std::string::utf8(b"Common"), 1);
        assert!(pet::rarity_for_level(5) == std::string::utf8(b"Uncommon"), 2);
        assert!(pet::rarity_for_level(9) == std::string::utf8(b"Uncommon"), 3);
        assert!(pet::rarity_for_level(10) == std::string::utf8(b"Rare"), 4);
        assert!(pet::rarity_for_level(24) == std::string::utf8(b"Rare"), 5);
        assert!(pet::rarity_for_level(25) == std::string::utf8(b"Epic"), 6);
        assert!(pet::rarity_for_level(39) == std::string::utf8(b"Epic"), 7);
        assert!(pet::rarity_for_level(40) == std::string::utf8(b"Legendary"), 8);
        assert!(pet::rarity_for_level(50) == std::string::utf8(b"Legendary"), 9);
        // The cap is now 1_000; the ladder must not restart past 40.
        assert!(pet::rarity_for_level(1_000) == std::string::utf8(b"Legendary"), 10);

        transfer::public_freeze_object(stats);
    }

    #[test]
    /// A pet can climb far enough to reach Legendary, paying the flat $1.00
    /// cost at each step. This is the end-to-end proof that upgrades compose.
    fun test_climbing_to_legendary() {
        let ctx = tx_context::dummy();
        let (pet, coll, cap, stats) = stage(0, &mut ctx);

        // 40 flat steps at $1.00 = $40.00, plus a little slack.
        let payment = usd(100_000_000, &mut ctx);
        let i = 0;
        while (i < 40) {
            pet::upgrade(&mut pet, &stats, &mut payment, &mut ctx);
            i = i + 1;
        };

        assert!(pet::level(&pet) == 40, 0);
        assert!(pet::power(&pet) == 400, 1);
        assert!(pet::rarity(&pet) == std::string::utf8(b"Legendary"), 2);

        let balance = coin::into_balance(payment);
        let _ = balance::destroy(balance);
        seal(pet, coll, cap, stats);
    }

    #[test]
    #[expected_failure(location = kanari_pet::pet, abort_code = 2)]
    /// Underpayment must abort. Without this the sink is optional and every pet
    /// could be maxed for free.
    fun test_upgrade_aborts_on_insufficient_usd() {
        let ctx = tx_context::dummy();
        let (pet, coll, cap, stats) = stage(1, &mut ctx);

        // Every level costs a flat $1.00; a cent short aborts.
        let payment = usd(999_999, &mut ctx);
        pet::upgrade(&mut pet, &stats, &mut payment, &mut ctx);

        let balance = coin::into_balance(payment);
        let _ = balance::destroy(balance);
        seal(pet, coll, cap, stats);
    }

    #[test]
    #[expected_failure(location = kanari_pet::pet, abort_code = 0)]
    /// The economy-defining negative case: the free setter must refuse a
    /// reserved key. If this passed, anyone could write `power = 999999` for
    /// nothing and the whole paid-progression system would be decorative.
    fun test_free_setter_cannot_write_level() {
        let ctx = tx_context::dummy();
        let (pet, coll, cap, stats) = stage(0, &mut ctx);

        pet::set_string_attribute(&mut pet, std::string::utf8(b"level"), std::string::utf8(b"999"));

        seal(pet, coll, cap, stats);
    }

    #[test]
    #[expected_failure(location = kanari_pet::pet, abort_code = 0)]
    /// Same for `power` — the value a buyer would care about most.
    fun test_free_setter_cannot_write_power() {
        let ctx = tx_context::dummy();
        let (pet, coll, cap, stats) = stage(0, &mut ctx);

        pet::set_string_attribute(&mut pet, std::string::utf8(b"power"), std::string::utf8(b"999999"));

        seal(pet, coll, cap, stats);
    }

    #[test]
    #[expected_failure(location = kanari_pet::pet, abort_code = 0)]
    /// And `rarity`, which is otherwise a claim the chain agrees with.
    fun test_free_setter_cannot_write_rarity() {
        let ctx = tx_context::dummy();
        let (pet, coll, cap, stats) = stage(0, &mut ctx);

        pet::set_string_attribute(&mut pet, std::string::utf8(b"rarity"), std::string::utf8(b"Legendary"));

        seal(pet, coll, cap, stats);
    }

    #[test]
    #[expected_failure(location = kanari_pet::pet, abort_code = 0)]
    /// The wholesale setter must refuse reserved keys too, or a caller could
    /// batch-write a fake `level` alongside legitimate-looking traits.
    fun test_wholesale_setter_cannot_write_reserved_keys() {
        let ctx = tx_context::dummy();
        let (pet, coll, cap, stats) = stage(0, &mut ctx);

        let keys = vector::empty<String>();
        let values = vector::empty<String>();
        vector::push_back(&mut keys, std::string::utf8(b"breed"));
        vector::push_back(&mut values, std::string::utf8(b"shiba"));
        vector::push_back(&mut keys, std::string::utf8(b"power"));
        vector::push_back(&mut values, std::string::utf8(b"9999"));
        pet::set_string_attributes(&mut pet, keys, values);

        seal(pet, coll, cap, stats);
    }

    #[test]
    #[expected_failure(location = kanari_pet::pet, abort_code = 0)]
    /// Removing a reserved key is refused as well: `upgrade` reads the current
    /// level back out of the map, so erasing it would wedge the pet.
    fun test_cannot_remove_a_reserved_key() {
        let ctx = tx_context::dummy();
        let (pet, coll, cap, stats) = stage(5, &mut ctx);

        let _ = pet::remove_attribute(&mut pet, &std::string::utf8(b"level"));

        seal(pet, coll, cap, stats);
    }

    #[test]
    /// A wholesale attribute replacement must not silently erase progression.
    /// The reserved keys are preserved even though the caller's list omits
    /// them — otherwise "replace my traits" would zero a paid level.
    fun test_wholesale_replace_preserves_progression() {
        let ctx = tx_context::dummy();
        let (pet, coll, cap, stats) = stage(7, &mut ctx);
        assert!(pet::level(&pet) == 7, 0);

        let keys = vector::empty<String>();
        let values = vector::empty<String>();
        vector::push_back(&mut keys, std::string::utf8(b"breed"));
        vector::push_back(&mut values, std::string::utf8(b"shiba"));
        pet::set_string_attributes(&mut pet, keys, values);

        // Level survived the wholesale replace.
        assert!(pet::level(&pet) == 7, 1);
        assert!(pet::power(&pet) == 70, 2);
        // And the caller's trait landed alongside it.
        assert!(pet::has_attribute(&pet, &std::string::utf8(b"breed")), 3);

        seal(pet, coll, cap, stats);
    }

    #[test]
    #[expected_failure(location = kanari_pet::pet, abort_code = 1)]
    /// A pet at the level ceiling cannot be upgraded further. Without this the
    /// counter would run past `max_level` and `power_per_level * level` would
    /// eventually overflow.
    fun test_upgrade_aborts_at_max_level() {
        let ctx = tx_context::dummy();
        let (pet, coll, cap, stats) = stage(1_000, &mut ctx);
        assert!(pet::level(&pet) == 1_000, 0);

        let payment = usd(100_000_000, &mut ctx);
        pet::upgrade(&mut pet, &stats, &mut payment, &mut ctx);

        let balance = coin::into_balance(payment);
        let _ = balance::destroy(balance);
        seal(pet, coll, cap, stats);
    }

    #[test]
    #[expected_failure(location = kanari_pet::pet, abort_code = 3)]
    /// Minting above the ceiling is refused, so no pet starts in a state
    /// `upgrade` cannot reason about.
    fun test_mint_above_max_level_aborts() {
        let ctx = tx_context::dummy();
        let (over, coll, cap, stats) = stage(1_001, &mut ctx);
        transfer::public_freeze_object(over);

        transfer::public_freeze_object(coll);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(stats);
    }

    #[test]
    /// Progression keys are ordinary attributes, so a wallet renders them with
    /// no special-casing while they stay write-protected through the setters.
    fun test_progression_is_readable_as_attributes() {
        let ctx = tx_context::dummy();
        let (pet, coll, cap, stats) = stage(12, &mut ctx);

        assert!(pet::has_attribute(&pet, &std::string::utf8(b"level")), 0);
        assert!(pet::has_attribute(&pet, &std::string::utf8(b"power")), 1);
        assert!(pet::has_attribute(&pet, &std::string::utf8(b"rarity")), 2);
        // Level 12 lands in Rare.
        assert!(
            *std::option::borrow(&pet::attribute(&pet, &std::string::utf8(b"power")))
                == std::string::utf8(b"120"),
            3,
        );
        assert!(
            *std::option::borrow(&pet::attribute(&pet, &std::string::utf8(b"rarity")))
                == std::string::utf8(b"Rare"),
            4,
        );

        seal(pet, coll, cap, stats);
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

    #[test]
    /// Regression note for a confirmed exploit, kept because the guard is
    /// invisible from inside this package.
    ///
    /// `upgrade` takes a caller-supplied `&Stats`, so while `new_stats` was
    /// `public` a caller built `Stats { base_cost_usd: 1, power_per_level:
    /// 1_000_000, max_level: 9_999 }` and bought a million power for one
    /// micro-USD.
    ///
    /// `#[test_only]` still exposes `new_stats` *here*, so this suite cannot
    /// observe the fix. What proves it is the negative-compile fixture
    /// `pet_stats_forgery`, which asserts `new_stats` is unbound from another
    /// package. Asserting anything about the attack above would be misleading:
    /// the vulnerability lived in the release build, not in these tests.
    fun test_shared_stats_carry_the_intended_economics() {
        let ctx = tx_context::dummy();
        let (coll, cap) = collection::create_collection(
            &mut ctx, b"Coll", b"D", b"https://b", b"https://w", 10,
        );
        let honest = test_stats(&mut ctx);

        // The values `init` actually shares: power is 10x level, ceiling 1_000,
        // every step costs $1.00, and the treasury is the publisher.
        assert!(pet::power_for_level(&honest, 1) == 10, 0);
        assert!(pet::max_level(&honest) == 1_000, 1);
        assert!(pet::upgrade_cost(&honest, 0) == 1_000_000, 2);
        assert!(pet::treasury(&honest) == CREATOR, 3);

        let pet = pet::mint_with_level_for_test(&mut cap, &coll, &honest, 3, &mut ctx);
        assert!(pet::level(&pet) == 3, 4);
        assert!(pet::power(&pet) == 30, 5);

        transfer::public_freeze_object(pet);
        transfer::public_freeze_object(honest);
        transfer::public_freeze_object(coll);
        transfer::public_freeze_object(cap);
    }
}
