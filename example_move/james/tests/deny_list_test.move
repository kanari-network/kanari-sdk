// Tests for the regulated JAMES / EURO / THB coins and the KariKid NFT:
// deny-list add/remove/contains, global pause, and checked transfers.
#[test_only]
module james::deny_list_tests {
    use james::euro;
    use james::james;
    use james::kdeny;
    use james::nft;
    use james::thb;
    use kanari_system::coin;
    use kanari_system::collection;
    use kanari_system::deny_list;
    use kanari_system::transfer;
    use kanari_system::tx_context;

    const ALICE: address = @0x100;
    const BOB: address = @0x200;

    // ================= JAMES =================
    #[test]
    fun test_james_deny_add_contains_remove() {
        let ctx = &mut tx_context::dummy();
        let (treasury, cap, metadata) = james::create_regulated_for_test(ctx);
        let deny_list = deny_list::new_for_testing(ctx);

        assert!(!james::is_denied(&deny_list, ALICE), 0);
        james::deny_add(&mut deny_list, &cap, ALICE, ctx);
        assert!(james::is_denied(&deny_list, ALICE), 1);
        assert!(!james::is_denied(&deny_list, BOB), 2);
        james::deny_remove(&mut deny_list, &cap, ALICE, ctx);
        assert!(!james::is_denied(&deny_list, ALICE), 3);

        transfer::public_freeze_object(treasury);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(metadata);
        transfer::public_freeze_object(deny_list);
    }

    #[test]
    fun test_james_legacy_deny_cap() {
        let ctx = &mut tx_context::dummy();
        let cap = deny_list::new_denycap_for_testing<james::JAMES>(ctx);
        let deny_list = deny_list::new_for_testing(ctx);

        james::deny_add(&mut deny_list, &cap, ALICE, ctx);
        assert!(james::is_denied(&deny_list, ALICE), 0);
        james::deny_remove(&mut deny_list, &cap, ALICE, ctx);
        assert!(!james::is_denied(&deny_list, ALICE), 1);

        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(deny_list);
    }

    #[test]
    fun test_james_checked_transfer_ok() {
        let ctx = &mut tx_context::dummy();
        let (treasury, cap, metadata) = james::create_regulated_for_test(ctx);
        let deny_list = deny_list::new_for_testing(ctx);

        let c = coin::mint(&mut treasury, 1000, ctx);
        james::transfer_amount_with_list(&mut c, 400, ALICE, &deny_list, ctx);
        assert!(coin::value(&c) == 600, 0);

        transfer::public_transfer(c, @0x1);
        transfer::public_freeze_object(treasury);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(metadata);
        transfer::public_freeze_object(deny_list);
    }

    #[test]
    #[expected_failure(location = kanari_system::pay, abort_code = 1)]
    fun test_james_checked_transfer_denied_aborts() {
        let ctx = &mut tx_context::dummy();
        let (treasury, cap, metadata) = james::create_regulated_for_test(ctx);
        let deny_list = deny_list::new_for_testing(ctx);
        james::deny_add(&mut deny_list, &cap, ALICE, ctx);

        let c = coin::mint(&mut treasury, 1000, ctx);
        james::transfer_amount_with_list(&mut c, 400, ALICE, &deny_list, ctx);

        transfer::public_transfer(c, @0x1);
        transfer::public_freeze_object(treasury);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(metadata);
        transfer::public_freeze_object(deny_list);
    }

    #[test]
    fun test_james_pause_unpause() {
        let ctx = &mut tx_context::dummy();
        let (treasury, cap, metadata) = james::create_regulated_for_test(ctx);
        let deny_list = deny_list::new_for_testing(ctx);

        assert!(!james::is_paused(&deny_list), 0);
        james::deny_pause(&mut deny_list, &cap, ctx);
        assert!(james::is_paused(&deny_list), 1);
        james::deny_unpause(&mut deny_list, &cap, ctx);
        assert!(!james::is_paused(&deny_list), 2);

        transfer::public_freeze_object(treasury);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(metadata);
        transfer::public_freeze_object(deny_list);
    }

    #[test]
    #[expected_failure(location = kanari_system::pay, abort_code = 1)]
    fun test_james_checked_transfer_paused_aborts() {
        let ctx = &mut tx_context::dummy();
        let (treasury, cap, metadata) = james::create_regulated_for_test(ctx);
        let deny_list = deny_list::new_for_testing(ctx);
        james::deny_pause(&mut deny_list, &cap, ctx);

        // BOB is not denied, but the paused coin still blocks the transfer.
        let c = coin::mint(&mut treasury, 1000, ctx);
        james::transfer_amount_with_list(&mut c, 400, BOB, &deny_list, ctx);

        transfer::public_transfer(c, @0x1);
        transfer::public_freeze_object(treasury);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(metadata);
        transfer::public_freeze_object(deny_list);
    }

    // ================= EURO =================
    #[test]
    fun test_euro_deny_flow_and_transfer_ok() {
        let ctx = &mut tx_context::dummy();
        let (treasury, cap, metadata) = euro::create_regulated_for_test(ctx);
        let deny_list = deny_list::new_for_testing(ctx);

        euro::deny_add(&mut deny_list, &cap, BOB, ctx);
        assert!(euro::is_denied(&deny_list, BOB), 0);
        assert!(!euro::is_denied(&deny_list, ALICE), 1);

        let c = coin::mint(&mut treasury, 500, ctx);
        euro::transfer_amount_with_list(&mut c, 200, ALICE, &deny_list, ctx);
        assert!(coin::value(&c) == 300, 2);

        transfer::public_transfer(c, @0x1);
        transfer::public_freeze_object(treasury);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(metadata);
        transfer::public_freeze_object(deny_list);
    }

    #[test]
    #[expected_failure(location = kanari_system::pay, abort_code = 1)]
    fun test_euro_checked_transfer_denied_aborts() {
        let ctx = &mut tx_context::dummy();
        let (treasury, cap, metadata) = euro::create_regulated_for_test(ctx);
        let deny_list = deny_list::new_for_testing(ctx);
        euro::deny_add(&mut deny_list, &cap, BOB, ctx);

        let c = coin::mint(&mut treasury, 500, ctx);
        euro::transfer_amount_with_list(&mut c, 200, BOB, &deny_list, ctx);

        transfer::public_transfer(c, @0x1);
        transfer::public_freeze_object(treasury);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(metadata);
        transfer::public_freeze_object(deny_list);
    }

    // ================= THB (fee + deny) =================
    #[test]
    fun test_thb_checked_transfer_with_fee_ok() {
        let ctx = &mut tx_context::dummy();
        let (treasury, cap, metadata) = thb::create_regulated_for_test(ctx);
        let deny_list = deny_list::new_for_testing(ctx);

        // amount 1000 -> 0.1% fee = 1; 2000 - 1001 leaves 999 with the sender.
        let c = coin::mint(&mut treasury, 2000, ctx);
        thb::transfer_amount_with_list(&mut c, 1000, ALICE, &deny_list, ctx);
        assert!(coin::value(&c) == 999, 0);

        transfer::public_transfer(c, @0x1);
        transfer::public_freeze_object(treasury);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(metadata);
        transfer::public_freeze_object(deny_list);
    }

    #[test]
    #[expected_failure(location = james::thb, abort_code = 1)]
    fun test_thb_checked_transfer_denied_aborts() {
        let ctx = &mut tx_context::dummy();
        let (treasury, cap, metadata) = thb::create_regulated_for_test(ctx);
        let deny_list = deny_list::new_for_testing(ctx);
        thb::deny_add(&mut deny_list, &cap, ALICE, ctx);

        let c = coin::mint(&mut treasury, 2000, ctx);
        thb::transfer_amount_with_list(&mut c, 1000, ALICE, &deny_list, ctx);

        transfer::public_transfer(c, @0x1);
        transfer::public_freeze_object(treasury);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(metadata);
        transfer::public_freeze_object(deny_list);
    }

    // ================= KariKid NFT =================
    fun setup_collection(
        ctx: &mut tx_context::TxContext,
    ): (collection::Collection, collection::NftCap) {
        collection::create_collection(
            ctx,
            b"KariKid",
            b"test collection",
            b"https://james-project.com/banner.png",
            b"https://james-project.com",
            10,
        )
    }

    #[test]
    fun test_nft_deny_add_contains_remove() {
        let ctx = &mut tx_context::dummy();
        let (coll, cap) = setup_collection(ctx);
        let deny_list = kdeny::new_for_testing(ctx);

        assert!(!nft::is_denied(&deny_list, ALICE), 0);
        nft::deny_add(&mut deny_list, &cap, ALICE, ctx);
        assert!(nft::is_denied(&deny_list, ALICE), 1);
        assert!(!nft::is_denied(&deny_list, BOB), 2);
        nft::deny_remove(&mut deny_list, &cap, ALICE, ctx);
        assert!(!nft::is_denied(&deny_list, ALICE), 3);

        transfer::public_freeze_object(coll);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(deny_list);
    }

    #[test]
    fun test_nft_transfer_ok_when_clean() {
        let ctx = &mut tx_context::dummy();
        let (coll, cap) = setup_collection(ctx);
        let deny_list = kdeny::new_for_testing(ctx);

        let nft = nft::mint_for_test(&mut cap, ctx);
        nft::transfer_with_list(nft, BOB, &deny_list, ctx);

        transfer::public_freeze_object(coll);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(deny_list);
    }

    #[test]
    #[expected_failure(location = james::nft, abort_code = 1)]
    fun test_nft_transfer_denied_aborts() {
        let ctx = &mut tx_context::dummy();
        let (coll, cap) = setup_collection(ctx);
        let deny_list = kdeny::new_for_testing(ctx);
        nft::deny_add(&mut deny_list, &cap, ALICE, ctx);

        let nft = nft::mint_for_test(&mut cap, ctx);
        nft::transfer_with_list(nft, ALICE, &deny_list, ctx);

        transfer::public_freeze_object(coll);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(deny_list);
    }

    #[test]
    fun test_nft_pause_unpause() {
        let ctx = &mut tx_context::dummy();
        let (coll, cap) = setup_collection(ctx);
        let deny_list = kdeny::new_for_testing(ctx);

        assert!(!nft::is_paused(&deny_list), 0);
        nft::deny_pause(&mut deny_list, &cap, ctx);
        assert!(nft::is_paused(&deny_list), 1);
        nft::deny_unpause(&mut deny_list, &cap, ctx);
        assert!(!nft::is_paused(&deny_list), 2);

        transfer::public_freeze_object(coll);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(deny_list);
    }

    #[test]
    #[expected_failure(location = james::nft, abort_code = 1)]
    fun test_nft_transfer_paused_aborts() {
        let ctx = &mut tx_context::dummy();
        let (coll, cap) = setup_collection(ctx);
        let deny_list = kdeny::new_for_testing(ctx);
        nft::deny_pause(&mut deny_list, &cap, ctx);

        // BOB is not denied, but the pause still blocks the transfer.
        let nft = nft::mint_for_test(&mut cap, ctx);
        nft::transfer_with_list(nft, BOB, &deny_list, ctx);

        transfer::public_freeze_object(coll);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(deny_list);
    }
}
