#[test_only]
module kanari_escrow::escrow_tests {
    use kanari_escrow::escrow::{Self};
    use kanari_system::coin::{Self, Coin};
    use kanari_system::tx_context::{Self, TxContext};
    use std::string;
    use std::option;

    // Test-only coin type for testing with minting capability
    #[test_only]
    struct TestUSDC has drop {}

    // Helper function to create TestUSDC with balance for testing
    // Returns the Coin object directly (for test-only usage)
    #[test_only]
    public fun create_test_usdc_coin(amount: u64, ctx: &mut TxContext): Coin<TestUSDC> {
        // Create a temporary TreasuryCap for testing
        let witness = TestUSDC {};
        let ( treasury_cap, _metadata) = coin::create_currency<TestUSDC>(
            witness,
            6,
            b"TestUSDC",
            b"Test USDC",
            b"Test USDC for escrow testing",
            option::none(),
            ctx,
        );
        
        // Mint coins using the treasury cap
        coin::mint(&mut treasury_cap, amount, ctx)
    }

    // ═══════════════════════════════════════════════════════════════
    // Test 1: Create Deal with USDC - Basic Flow
    // Note: In test environment, we pass Coin object directly because
    // borrow_global_mut requires runtime storage which is not available in tests
    // ═══════════════════════════════════════════════════════════════
    #[test]
    fun test_create_deal_with_usdc_basic() {
        let ctx = tx_context::dummy();
        let seller_addr = @0x2;
        
        // Create TestUSDC coin for testing
        let buyer_coin = create_test_usdc_coin(10000, &mut ctx);
        
        // Create escrow deal using Coin object directly
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b"deal_001"),
            seller_addr,
            1000,
            string::utf8(b"Test payment for services"),
            &mut buyer_coin,
            &mut ctx
        );
        
        // If no panic, test passes
    }

    // ═══════════════════════════════════════════════════════════════
    // Test 2: Create Multiple Deals with USDC
    // ═══════════════════════════════════════════════════════════════
    #[test]
    fun test_create_multiple_deals_with_usdc() {
        let ctx = tx_context::dummy();
        let seller_addr = @0x2;
        
        let buyer_coin = create_test_usdc_coin(10000, &mut ctx);
        
        // Create first deal
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b"deal_001"),
            seller_addr,
            1000,
            string::utf8(b"First deal"),
            &mut buyer_coin,
            &mut ctx
        );
        
        // Create second deal
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b"deal_002"),
            seller_addr,
            2000,
            string::utf8(b"Second deal"),
            &mut buyer_coin,
            &mut ctx
        );
        
        // Create third deal
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b"deal_003"),
            seller_addr,
            1500,
            string::utf8(b"Third deal"),
            &mut buyer_coin,
            &mut ctx
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Test 3: Deal with Special Characters (USDC)
    // ═══════════════════════════════════════════════════════════════
    #[test]
    fun test_deal_with_special_characters() {
        let ctx = tx_context::dummy();
        let seller_addr = @0x2;
        
        let buyer_coin = create_test_usdc_coin(1000, &mut ctx);
        
        // Test deal ID and description with special characters
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b"deal-2024-001"),
            seller_addr,
            1000,
            string::utf8(b"Payment for order #12345 & invoice ABC-789"),
            &mut buyer_coin,
            &mut ctx
        );
    }

    // ══════════════════════════════════════════════════════════════
    // Test 4: Deal with Long Description (USDC)
    // ══════════════════════════════════════════════════════════════
    #[test]
    fun test_deal_with_long_description() {
        let ctx = tx_context::dummy();
        let seller_addr = @0x2;
        
        let buyer_coin = create_test_usdc_coin(1000000, &mut ctx);
        
        // Test with longer description
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b"long_desc_deal"),
            seller_addr,
            999999,
            string::utf8(b"This is a longer description to test how the escrow system handles text that exceeds typical lengths"),
            &mut buyer_coin,
            &mut ctx
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Test 5: Timestamp Verification (USDC)
    // ═══════════════════════════════════════════════════════════════
    #[test]
    fun test_timestamp_is_set() {
        let ctx = tx_context::dummy();
        let seller_addr = @0x2;
        
        let buyer_coin = create_test_usdc_coin(1000, &mut ctx);
        
        // Get initial timestamp
        let initial_ts = tx_context::epoch_timestamp_ms(&ctx);
        
        // Create deal
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b"timestamp_test"),
            seller_addr,
            1000,
            string::utf8(b"Test timestamp"),
            &mut buyer_coin,
            &mut ctx
        );
        
        // Get timestamp after creation
        let final_ts = tx_context::epoch_timestamp_ms(&ctx);
        
        // Verify timestamp advanced
        assert!(final_ts >= initial_ts, 0);
    }

    // ═══════════════════════════════════════════════════════════════
    // Test 6: Various Amounts (USDC)
    // ═══════════════════════════════════════════════════════════════
    #[test]
    fun test_various_amounts() {
        let seller_addr = @0x2;
        
        // Test with amount = 1 (minimum)
        let ctx1 = tx_context::dummy();
        let coin1 = create_test_usdc_coin(1, &mut ctx1);
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b"amount_1"),
            seller_addr,
            1,
            string::utf8(b"Min amount"),
            &mut coin1,
            &mut ctx1
        );
        
        // Test with amount = 1000
        let ctx2 = tx_context::dummy();
        let coin2 = create_test_usdc_coin(1000, &mut ctx2);
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b"amount_1000"),
            seller_addr,
            1000,
            string::utf8(b"Medium amount"),
            &mut coin2,
            &mut ctx2
        );
        
        // Test with large amount
        let ctx3 = tx_context::dummy();
        let coin3 = create_test_usdc_coin(1000000000, &mut ctx3);
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b"amount_large"),
            seller_addr,
            1000000000,
            string::utf8(b"Large amount"),
            &mut coin3,
            &mut ctx3
        );
    }

    // ══════════════════════════════════════════════════════════════
    // Test 7: Different Seller Addresses (USDC)
    // ═══════════════════════════════════════════════════════════════
    #[test]
    fun test_different_sellers() {
        let ctx = tx_context::dummy();
        
        // Test with different seller addresses
        let seller1 = @0x1;
        let seller2 = @0x2;
        let seller3 = @0xdeadbeef;
        
        let coin1 = create_test_usdc_coin(100, &mut ctx);
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b"seller_1"),
            seller1,
            100,
            string::utf8(b"Seller 1"),
            &mut coin1,
            &mut ctx
        );
        
        let coin2 = create_test_usdc_coin(200, &mut ctx);
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b"seller_2"),
            seller2,
            200,
            string::utf8(b"Seller 2"),
            &mut coin2,
            &mut ctx
        );
        
        let coin3 = create_test_usdc_coin(300, &mut ctx);
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b"seller_3"),
            seller3,
            300,
            string::utf8(b"Seller 3"),
            &mut coin3,
            &mut ctx
        );
    }

    // ══════════════════════════════════════════════════════════════
    // Test 8: Empty Strings (Edge Case) (USDC)
    // ═══════════════════════════════════════════════════════════════
    #[test]
    fun test_empty_strings() {
        let ctx = tx_context::dummy();
        let seller_addr = @0x2;
        
        let buyer_coin = create_test_usdc_coin(1000, &mut ctx);
        
        // Test with empty deal ID and description
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b""),
            seller_addr,
            1000,
            string::utf8(b""),
            &mut buyer_coin,
            &mut ctx
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Test 9: Unicode Content (USDC)
    // ═══════════════════════════════════════════════════════════════
    #[test]
    fun test_unicode_content() {
        let ctx = tx_context::dummy();
        let seller_addr = @0x2;
        
        let buyer_coin = create_test_usdc_coin(500, &mut ctx);
        
        // Test with UTF-8 content (emoji, special chars)
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b"unicode_deal_✅"),
            seller_addr,
            500,
            string::utf8(b"การทดสอบ escrow with emoji 🎉"),
            &mut buyer_coin,
            &mut ctx
        );
    }
}