#[test_only]
module dex_v1::dex_v1_tests {
    use dex_v1::dex_v1;
    use kanari_system::tx_context;
    // Test token types
    struct TEST_COIN_A has drop {}
    struct TEST_COIN_B has drop {}

    #[test]
    fun test_create_pool() {
        let ctx = &mut tx_context::dummy();
        
        // Create a pool for testing (doesn't use transfer)
        let pool = dex_v1::create_pool_for_testing<TEST_COIN_A, TEST_COIN_B>(30, ctx);
        
        // Verify pool was created with correct initial state
        assert!(dex_v1::get_fee_percent<TEST_COIN_A, TEST_COIN_B>(&pool) == 30, 0);
        assert!(dex_v1::get_reserve_a<TEST_COIN_A, TEST_COIN_B>(&pool) == 0, 1);
        assert!(dex_v1::get_reserve_b<TEST_COIN_A, TEST_COIN_B>(&pool) == 0, 2);
        assert!(dex_v1::get_lp_supply<TEST_COIN_A, TEST_COIN_B>(&pool) == 0, 3);
        
        // Cleanup using helper function
        dex_v1::destroy_pool_for_testing<TEST_COIN_A, TEST_COIN_B>(pool);
    }

    #[test]
    fun test_view_functions() {
        let ctx = &mut tx_context::dummy();
        
        // Create pool with different fee
        let pool = dex_v1::create_pool_for_testing<TEST_COIN_A, TEST_COIN_B>(25, ctx);
        
        // Test all view functions
        let pool_id = dex_v1::get_pool_id<TEST_COIN_A, TEST_COIN_B>(&pool);
        assert!(pool_id != @0x0, 0); // Just verify it's not zero address
        
        let (reserve_a, reserve_b, lp_supply, fee_percent) = 
            dex_v1::get_pool_info<TEST_COIN_A, TEST_COIN_B>(&pool);
        
        assert!(reserve_a == 0, 1);
        assert!(reserve_b == 0, 2);
        assert!(lp_supply == 0, 3);
        assert!(fee_percent == 25, 4);
        
        // Test swap output calculation on empty pool
        let output = dex_v1::get_swap_a_for_b_output<TEST_COIN_A, TEST_COIN_B>(&pool, 10000);
        assert!(output == 0, 5);
        
        // Cleanup
        dex_v1::destroy_pool_for_testing<TEST_COIN_A, TEST_COIN_B>(pool);
    }
}