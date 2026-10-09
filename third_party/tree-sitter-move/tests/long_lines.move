module test::long_lines {
    use kanari_system::balance::{Self, Balance, Supply};

    public fun create_pool_with_a_very_long_function_name<CoinTypeA, CoinTypeB>(
        fee_percent: u64,
        ctx: &mut TxContext,
    ): Pool<CoinTypeA, CoinTypeB> {
        Pool {
            reserve_a: balance::zero<CoinTypeA>(),
            reserve_b: balance::zero<CoinTypeB>(),
            lp_supply: balance::new_supply<LPToken<CoinTypeA, CoinTypeB>>(),
            fee_percent,
        }
    }

    public struct Pool<phantom CoinTypeA, phantom CoinTypeB> has key, store {
        reserve_a: Balance<CoinTypeA>,
        reserve_b: Balance<CoinTypeB>,
        lp_supply: Supply<LPToken<CoinTypeA, CoinTypeB>>,
        fee_percent: u64,
    }
}
