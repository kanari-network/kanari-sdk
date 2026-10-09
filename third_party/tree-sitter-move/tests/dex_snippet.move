module test::dex_snippet {
    use kanari_system::balance::{Self, Balance, Supply};
    use kanari_system::coin::{Self, Coin};

    public struct Pool<phantom CoinTypeA, phantom CoinTypeB> has key, store {
        reserve_a: Balance<CoinTypeA>,
        reserve_b: Balance<CoinTypeB>,
        lp_supply: Supply<LPToken<CoinTypeA, CoinTypeB>>,
        fee_percent: u64,
    }

    public fun get_amount_out(amount_in: u64, reserve_in: u64, reserve_out: u64): u64 {
        assert!(reserve_in > 0 && reserve_out > 0, 0);
        let numerator = amount_in * reserve_out;
        let denominator = reserve_in + amount_in;
        numerator / denominator
    }
}
