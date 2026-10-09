module test::generics {
    use std::option::{Self, Option};

    public struct Pool<phantom CoinTypeA, phantom CoinTypeB> has key, store {
        reserve_a: Balance<CoinTypeA>,
        reserve_b: Balance<CoinTypeB>,
        lp_supply: Supply<LPToken<CoinTypeA, CoinTypeB>>,
        fee_percent: u64,
    }

    public fun create_pool<CoinTypeA, CoinTypeB>(
        fee_percent: u64,
        ctx: &mut TxContext,
    ): Pool<CoinTypeA, CoinTypeB> {
        Pool {
            reserve_a: balance::zero(),
            reserve_b: balance::zero(),
            lp_supply: balance::new_supply(),
            fee_percent,
        }
    }

    fun add_liquidity_internal<CoinTypeA, CoinTypeB>(
        pool: &mut Pool<CoinTypeA, CoinTypeB>,
        amount_a: u64,
        amount_b: u64,
    ): u64 {
        let reserve_a = balance::value(&pool.reserve_a);
        let reserve_b = balance::value(&pool.reserve_b);
        assert!(amount_a > 0 && amount_b > 0, 0);
        reserve_a + reserve_b
    }
}
