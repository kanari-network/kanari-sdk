module dex_v1::dex_v1 {
    use kanari_system::object::{Self, UID};
    use kanari_system::balance::{Self, Balance, Supply};
    use kanari_system::coin::{Self, Coin};
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::math;
    use kanari_system::transfer;

    // =================================================================
    // Error Codes
    // =================================================================
    const E_INSUFFICIENT_LIQUIDITY: u64 = 1;
    const E_INSUFFICIENT_AMOUNT: u64 = 2;
    const E_INSUFFICIENT_LIQUIDITY_MINTED: u64 = 3;
    const E_INSUFFICIENT_LIQUIDITY_BURNED: u64 = 4;
    const E_INSUFFICIENT_OUTPUT_AMOUNT: u64 = 5;

    // =================================================================
    // Structs
    // =================================================================
    struct LP_TOKEN<phantom CoinTypeA, phantom CoinTypeB> has drop {}

    struct Pool<phantom CoinTypeA, phantom CoinTypeB> has key, store {
        id: UID,
        reserve_a: Balance<CoinTypeA>,
        reserve_b: Balance<CoinTypeB>,
        lp_supply: Supply<LP_TOKEN<CoinTypeA, CoinTypeB>>, 
        fee_percent: u64, 
    }

    // =================================================================
    // 1. Create Pool (Entry Function)
    // =================================================================
    public entry fun create_pool<CoinTypeA, CoinTypeB>(
        fee_percent: u64,
        ctx: &mut TxContext
    ) {
        let uid = object::new(ctx);

        let lp_supply = balance::new_supply<LP_TOKEN<CoinTypeA, CoinTypeB>>();

        let pool = Pool<CoinTypeA, CoinTypeB> {
            id: uid,
            reserve_a: balance::zero<CoinTypeA>(),
            reserve_b: balance::zero<CoinTypeB>(),
            lp_supply,
            fee_percent,
        };

        // ส่งตัวอ้างอิง (&pool) ไปบันทึกลง Database ของคุณ
        object::save_object(&pool);

        // 🚨 ส่งตัว Pool ตัวจริง (pool) ไปให้ผู้สร้าง (Sender) ถือครองไว้
        transfer::public_transfer(pool, tx_context::sender(ctx));
    }

    #[test_only]
    public fun create_pool_for_testing<CoinTypeA, CoinTypeB>(
        fee_percent: u64,
        ctx: &mut TxContext
    ): Pool<CoinTypeA, CoinTypeB> {
        let uid = object::new(ctx);

        let lp_supply = balance::new_supply<LP_TOKEN<CoinTypeA, CoinTypeB>>();

        Pool<CoinTypeA, CoinTypeB> {
            id: uid,
            reserve_a: balance::zero<CoinTypeA>(),
            reserve_b: balance::zero<CoinTypeB>(),
            lp_supply,
            fee_percent,
        }
    }

    // =================================================================
    // 2. Add Liquidity (Entry Function with Object ID and Amounts)
    // =================================================================
    public entry fun add_liquidity<CoinTypeA, CoinTypeB>(
        pool_id: address,
        coin_a_id: address,
        coin_b_id: address,
        amount_a: u64,
        amount_b: u64,
        ctx: &mut TxContext
    ) {
        // Load Pool object from storage
        let pool: &mut Pool<CoinTypeA, CoinTypeB> = object::borrow_global_mut<Pool<CoinTypeA, CoinTypeB>>(pool_id);
        
        // Load Coin objects from storage
        let coin_a: &mut Coin<CoinTypeA> = object::borrow_global_mut<Coin<CoinTypeA>>(coin_a_id);
        let coin_b: &mut Coin<CoinTypeB> = object::borrow_global_mut<Coin<CoinTypeB>>(coin_b_id);
        
        // Split specific amounts from coins
        let coin_a_obj = coin::split(coin_a, amount_a, ctx);
        let coin_b_obj = coin::split(coin_b, amount_b, ctx);
        
        // Call internal function
        let lp_token = add_liquidity_internal(pool, coin_a_obj, coin_b_obj, ctx);
        
        // Transfer LP token to sender
        transfer::public_transfer(lp_token, tx_context::sender(ctx));
    }

    // Internal function
    fun add_liquidity_internal<CoinTypeA, CoinTypeB>(
        pool: &mut Pool<CoinTypeA, CoinTypeB>,
        coin_a: Coin<CoinTypeA>,
        coin_b: Coin<CoinTypeB>,
        ctx: &mut TxContext
    ): Coin<LP_TOKEN<CoinTypeA, CoinTypeB>> {
        let amount_a = coin::value(&coin_a);
        let amount_b = coin::value(&coin_b);
        assert!(amount_a > 0 && amount_b > 0, E_INSUFFICIENT_AMOUNT);

        let reserve_a = balance::value(&pool.reserve_a);
        let reserve_b = balance::value(&pool.reserve_b);
        
        let total_lp_supply = balance::supply_total(&pool.lp_supply);

        let liquidity: u64;

        if (total_lp_supply == 0) {
            let initial_lp = math::sqrt_u128((amount_a as u128) * (amount_b as u128));
            assert!(initial_lp > 1000, E_INSUFFICIENT_LIQUIDITY_MINTED);
            liquidity = (initial_lp as u64) - 1000;
        } else {
            let lp_a = math::mul_div_u64(amount_a, total_lp_supply, reserve_a);
            let lp_b = math::mul_div_u64(amount_b, total_lp_supply, reserve_b);
            liquidity = math::min_u64(lp_a, lp_b);
        };

        assert!(liquidity > 0, E_INSUFFICIENT_LIQUIDITY_MINTED);

        balance::merge(&mut pool.reserve_a, coin::into_balance(coin_a));
        balance::merge(&mut pool.reserve_b, coin::into_balance(coin_b));

        let lp_balance = balance::increase_supply(&mut pool.lp_supply, liquidity);
        coin::from_balance(lp_balance, ctx)
    }

    // =================================================================
    // 3. Remove Liquidity (Entry Function with Object ID)
    // =================================================================
    public entry fun remove_liquidity<CoinTypeA, CoinTypeB>(
        pool_id: address,
        lp_coin_id: address,
        ctx: &mut TxContext
    ) {
        // Load Pool object from storage
        let pool: &mut Pool<CoinTypeA, CoinTypeB> = object::borrow_global_mut<Pool<CoinTypeA, CoinTypeB>>(pool_id);
        
        // Load LP Coin object from storage
        let lp_coin: &mut Coin<LP_TOKEN<CoinTypeA, CoinTypeB>> = object::borrow_global_mut<Coin<LP_TOKEN<CoinTypeA, CoinTypeB>>>(lp_coin_id);
        
        // Get amount first before splitting
        let lp_amount = coin::value(lp_coin);
        
        // Split LP coin to get the full amount
        let lp_coin_obj = coin::split(lp_coin, lp_amount, ctx);
        
        // Call internal function
        let (coin_a, coin_b) = remove_liquidity_internal(pool, lp_coin_obj, ctx);
        
        // Transfer coins back to sender
        transfer::public_transfer(coin_a, tx_context::sender(ctx));
        transfer::public_transfer(coin_b, tx_context::sender(ctx));
    }

    // Internal function
    fun remove_liquidity_internal<CoinTypeA, CoinTypeB>(
        pool: &mut Pool<CoinTypeA, CoinTypeB>,
        lp_coin: Coin<LP_TOKEN<CoinTypeA, CoinTypeB>>,
        ctx: &mut TxContext
    ): (Coin<CoinTypeA>, Coin<CoinTypeB>) {
        let liquidity = coin::value(&lp_coin);
        assert!(liquidity > 0, E_INSUFFICIENT_LIQUIDITY_BURNED);

        let reserve_a = balance::value(&pool.reserve_a);
        let reserve_b = balance::value(&pool.reserve_b);
        
        let total_lp_supply = balance::supply_total(&pool.lp_supply);

        let amount_a = math::mul_div_u64(liquidity, reserve_a, total_lp_supply);
        let amount_b = math::mul_div_u64(liquidity, reserve_b, total_lp_supply);

        assert!(amount_a > 0 && amount_b > 0, E_INSUFFICIENT_AMOUNT);

        let lp_balance = coin::into_balance(lp_coin);
        let burned_amount = balance::destroy(lp_balance);
        balance::decrease_supply(&mut pool.lp_supply, burned_amount);

        let balance_a_out = balance::split(&mut pool.reserve_a, amount_a);
        let balance_b_out = balance::split(&mut pool.reserve_b, amount_b);

        (
            coin::from_balance(balance_a_out, ctx),
            coin::from_balance(balance_b_out, ctx)
        )
    }

    // =================================================================
    // 4. Swap A for B (Entry Function with Object ID and Amount)
    // =================================================================
    public entry fun swap_a_for_b<CoinTypeA, CoinTypeB>(
        pool_id: address,
        coin_in_id: address,
        amount_in: u64,
        ctx: &mut TxContext
    ) {
        // Load Pool object from storage
        let pool: &mut Pool<CoinTypeA, CoinTypeB> = object::borrow_global_mut<Pool<CoinTypeA, CoinTypeB>>(pool_id);
        
        // Load input Coin object from storage
        let coin_in: &mut Coin<CoinTypeA> = object::borrow_global_mut<Coin<CoinTypeA>>(coin_in_id);
        
        // Split specific amount from coin
        let coin_in_obj = coin::split(coin_in, amount_in, ctx);
        
        // Call internal function
        let coin_out = swap_a_for_b_internal(pool, coin_in_obj, ctx);
        
        // Transfer output coin to sender
        transfer::public_transfer(coin_out, tx_context::sender(ctx));
    }

    // Internal function
    fun swap_a_for_b_internal<CoinTypeA, CoinTypeB>(
        pool: &mut Pool<CoinTypeA, CoinTypeB>,
        coin_in: Coin<CoinTypeA>,
        ctx: &mut TxContext
    ): Coin<CoinTypeB> {
        let amount_in = coin::value(&coin_in);
        assert!(amount_in > 0, E_INSUFFICIENT_AMOUNT);

        let reserve_in = balance::value(&pool.reserve_a);
        let reserve_out = balance::value(&pool.reserve_b);

        let amount_out = calculate_amount_out(amount_in, reserve_in, reserve_out, pool.fee_percent);
        
        assert!(amount_out > 0, E_INSUFFICIENT_OUTPUT_AMOUNT);
        assert!(amount_out < reserve_out, E_INSUFFICIENT_LIQUIDITY);

        balance::merge(&mut pool.reserve_a, coin::into_balance(coin_in));

        let balance_out = balance::split(&mut pool.reserve_b, amount_out);

        coin::from_balance(balance_out, ctx)
    }

    // =================================================================
    // 5. Swap B for A (Entry Function with Object ID and Amount)
    // =================================================================
    public entry fun swap_b_for_a<CoinTypeA, CoinTypeB>(
        pool_id: address,
        coin_in_id: address,
        amount_in: u64,
        ctx: &mut TxContext
    ) {
        // Load Pool object from storage
        let pool: &mut Pool<CoinTypeA, CoinTypeB> = object::borrow_global_mut<Pool<CoinTypeA, CoinTypeB>>(pool_id);
        
        // Load input Coin object from storage
        let coin_in: &mut Coin<CoinTypeB> = object::borrow_global_mut<Coin<CoinTypeB>>(coin_in_id);
        
        // Split specific amount from coin
        let coin_in_obj = coin::split(coin_in, amount_in, ctx);
        
        // Call internal function
        let coin_out = swap_b_for_a_internal(pool, coin_in_obj, ctx);
        
        // Transfer output coin to sender
        transfer::public_transfer(coin_out, tx_context::sender(ctx));
    }

    // Internal function
    fun swap_b_for_a_internal<CoinTypeA, CoinTypeB>(
        pool: &mut Pool<CoinTypeA, CoinTypeB>,
        coin_in: Coin<CoinTypeB>,
        ctx: &mut TxContext
    ): Coin<CoinTypeA> {
        let amount_in = coin::value(&coin_in);
        assert!(amount_in > 0, E_INSUFFICIENT_AMOUNT);

        let reserve_in = balance::value(&pool.reserve_b);
        let reserve_out = balance::value(&pool.reserve_a);

        let amount_out = calculate_amount_out(amount_in, reserve_in, reserve_out, pool.fee_percent);
        
        assert!(amount_out > 0, E_INSUFFICIENT_OUTPUT_AMOUNT);
        assert!(amount_out < reserve_out, E_INSUFFICIENT_LIQUIDITY);

        balance::merge(&mut pool.reserve_b, coin::into_balance(coin_in));

        let balance_out = balance::split(&mut pool.reserve_a, amount_out);

        coin::from_balance(balance_out, ctx)
    }

    // =================================================================
    // Internal Math
    // =================================================================
    fun calculate_amount_out(
        amount_in: u64,
        reserve_in: u64,
        reserve_out: u64,
        fee_percent: u64
    ): u64 {
        let amount_in_128 = (amount_in as u128);
        let reserve_in_128 = (reserve_in as u128);
        let reserve_out_128 = (reserve_out as u128);
        let fee_multiplier = (10000 - fee_percent as u128);

        let amount_in_with_fee = amount_in_128 * fee_multiplier;
        let numerator = amount_in_with_fee * reserve_out_128;
        let denominator = (reserve_in_128 * 10000) + amount_in_with_fee;

        (numerator / denominator as u64)
    }

    // =================================================================
    // View Functions (Getters)
    // =================================================================
    
    /// Get Pool ID
    public fun get_pool_id<CoinTypeA, CoinTypeB>(
        pool: &Pool<CoinTypeA, CoinTypeB>
    ): address {
        object::uid_address(&pool.id)
    }

    /// Get Reserve A balance
    public fun get_reserve_a<CoinTypeA, CoinTypeB>(
        pool: &Pool<CoinTypeA, CoinTypeB>
    ): u64 {
        balance::value(&pool.reserve_a)
    }

    /// Get Reserve B balance
    public fun get_reserve_b<CoinTypeA, CoinTypeB>(
        pool: &Pool<CoinTypeA, CoinTypeB>
    ): u64 {
        balance::value(&pool.reserve_b)
    }

    /// Get Total LP Token Supply
    public fun get_lp_supply<CoinTypeA, CoinTypeB>(
        pool: &Pool<CoinTypeA, CoinTypeB>
    ): u64 {
        balance::supply_total(&pool.lp_supply)
    }

    /// Get Fee Percent
    public fun get_fee_percent<CoinTypeA, CoinTypeB>(
        pool: &Pool<CoinTypeA, CoinTypeB>
    ): u64 {
        pool.fee_percent
    }

    /// Get all pool information at once
    public fun get_pool_info<CoinTypeA, CoinTypeB>(
        pool: &Pool<CoinTypeA, CoinTypeB>
    ): (u64, u64, u64, u64) {
        (
            balance::value(&pool.reserve_a),
            balance::value(&pool.reserve_b),
            balance::supply_total(&pool.lp_supply),
            pool.fee_percent
        )
    }

    /// Calculate expected output amount for swapping A to B
    public fun get_swap_a_for_b_output<CoinTypeA, CoinTypeB>(
        pool: &Pool<CoinTypeA, CoinTypeB>,
        amount_in: u64
    ): u64 {
        let reserve_in = balance::value(&pool.reserve_a);
        let reserve_out = balance::value(&pool.reserve_b);
        calculate_amount_out(amount_in, reserve_in, reserve_out, pool.fee_percent)
    }

    /// Calculate expected output amount for swapping B to A
    public fun get_swap_b_for_a_output<CoinTypeA, CoinTypeB>(
        pool: &Pool<CoinTypeA, CoinTypeB>,
        amount_in: u64
    ): u64 {
        let reserve_in = balance::value(&pool.reserve_b);
        let reserve_out = balance::value(&pool.reserve_a);
        calculate_amount_out(amount_in, reserve_in, reserve_out, pool.fee_percent)
    }

    #[test_only]
    public fun destroy_pool_for_testing<CoinTypeA, CoinTypeB>(
        pool: Pool<CoinTypeA, CoinTypeB>
    ) {
        let Pool { id, reserve_a: ra, reserve_b: rb, lp_supply: ls, fee_percent: _ } = pool;
        let _ = balance::destroy<CoinTypeA>(ra);
        let _ = balance::destroy<CoinTypeB>(rb);
        let _ = balance::supply_total(&ls);
        object::delete(id);
    }
}