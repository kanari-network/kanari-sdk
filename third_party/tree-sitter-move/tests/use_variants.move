module test::use_variants {
    use std::option;
    use std::vector as v;
    use kanari_system::coin::{Self, Coin, TreasuryCap};
    use kanari_system::tx_context::{Self, TxContext};

    public fun demo(ctx: &mut TxContext): Coin<USDC> {
        let c = coin::zero<USDC>(ctx);
        c
    }
}
