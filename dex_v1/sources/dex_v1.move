module dex_v1::dex_v1 {
    use kanari_system::object::{Self, UID, ID};
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
    // 1. Create Pool
    // =================================================================
    public fun create_pool<CoinTypeA, CoinTypeB>(
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
        // เพื่อล้างค่า (Consume) ออกจากฟังก์ชันตามกฎของ Move
        transfer::public_transfer(pool, tx_context::sender(ctx));
    }

    // =================================================================
    // 2. Add Liquidity
    // =================================================================
    public fun add_liquidity<CoinTypeA, CoinTypeB>(
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
        
        // ✅ แก้เป็น supply_total ตาม balance.move
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

        // ✅ แก้เป็น merge แทน join ตาม balance.move
        balance::merge(&mut pool.reserve_a, coin::into_balance(coin_a));
        balance::merge(&mut pool.reserve_b, coin::into_balance(coin_b));

        let lp_balance = balance::increase_supply(&mut pool.lp_supply, liquidity);
        coin::from_balance(lp_balance, ctx)
    }

    // =================================================================
    // 3. Remove Liquidity
    // =================================================================
    public fun remove_liquidity<CoinTypeA, CoinTypeB>(
        pool: &mut Pool<CoinTypeA, CoinTypeB>,
        lp_coin: Coin<LP_TOKEN<CoinTypeA, CoinTypeB>>,
        ctx: &mut TxContext
    ): (Coin<CoinTypeA>, Coin<CoinTypeB>) {
        let liquidity = coin::value(&lp_coin);
        assert!(liquidity > 0, E_INSUFFICIENT_LIQUIDITY_BURNED);

        let reserve_a = balance::value(&pool.reserve_a);
        let reserve_b = balance::value(&pool.reserve_b);
        
        // ✅ แก้เป็น supply_total
        let total_lp_supply = balance::supply_total(&pool.lp_supply);

        let amount_a = math::mul_div_u64(liquidity, reserve_a, total_lp_supply);
        let amount_b = math::mul_div_u64(liquidity, reserve_b, total_lp_supply);

        assert!(amount_a > 0 && amount_b > 0, E_INSUFFICIENT_AMOUNT);

        // ✅ แก้ไขวิธี Burn LP ให้ตรงกับ decrease_supply ที่รับค่าเป็น u64
        let lp_balance = coin::into_balance(lp_coin);
        let burned_amount = balance::destroy(lp_balance); // ดึงค่ายอดเงินออกมาแล้วทำลายกล่อง
        balance::decrease_supply(&mut pool.lp_supply, burned_amount);

        let balance_a_out = balance::split(&mut pool.reserve_a, amount_a);
        let balance_b_out = balance::split(&mut pool.reserve_b, amount_b);

        (
            coin::from_balance(balance_a_out, ctx),
            coin::from_balance(balance_b_out, ctx)
        )
    }

    // =================================================================
    // 4. Swap A for B
    // =================================================================
    public fun swap_a_for_b<CoinTypeA, CoinTypeB>(
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

        // ✅ แก้เป็น merge แทน join
        balance::merge(&mut pool.reserve_a, coin::into_balance(coin_in));

        let balance_out = balance::split(&mut pool.reserve_b, amount_out);

        coin::from_balance(balance_out, ctx)
    }

    // =================================================================
    // 5. Swap B for A
    // =================================================================
    public fun swap_b_for_a<CoinTypeA, CoinTypeB>(
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

        // ✅ แก้เป็น merge แทน join
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
}