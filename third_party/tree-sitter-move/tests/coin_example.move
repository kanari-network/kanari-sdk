module test::coin_example {
    use kanari_system::coin::{Self, Coin, TreasuryCap};
    use kanari_system::tx_context::TxContext;

    struct EURO has drop {}

    fun init(witness: EURO, ctx: &mut TxContext) {
        let (treasury, metadata) = coin::create_currency<EURO>(
            witness,
            6,
            b"EURO",
            b"EURO Token",
            b"",
            option::none(),
            ctx,
        );
        transfer::public_transfer(treasury, tx_context::sender(ctx));
        transfer::public_transfer(metadata, tx_context::sender(ctx));
    }

    public entry fun mint(treasury_cap: &mut TreasuryCap<EURO>, amount: u64, ctx: &mut TxContext) {
        let minted = coin::mint(treasury_cap, amount, ctx);
        transfer::public_transfer(minted, tx_context::sender(ctx));
    }
}
