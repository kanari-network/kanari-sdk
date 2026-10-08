#[allow(unused_field)]
module tester::escrow_like_object_input {
    use std::string::String;
    use kanari_system::coin::{Self, Coin};
    use kanari_system::object::{Self, UID};
    use kanari_system::tx_context::{Self, TxContext};

    const E_NOT_ENOUGH_BALANCE: u64 = 7;

    struct Marker has key, store {
        id: UID
    }

    public entry fun create_deal<CoinType>(
        deal_id: String,
        seller: address,
        amount: u64,
        description: String,
        buyer_coin_id: address,
        ctx: &mut TxContext
    ) {
        let _buyer = tx_context::sender(ctx);
        let _deal_id = deal_id;
        let _seller = seller;
        let _description = description;
        let buyer_coin: &mut Coin<CoinType> =
            object::borrow_global_mut<Coin<CoinType>>(buyer_coin_id);
        assert!(coin::value(buyer_coin) >= amount, E_NOT_ENOUGH_BALANCE);
    }

    public entry fun touch_marker(marker_id: address, ctx: &mut TxContext) {
        let _sender = tx_context::sender(ctx);
        let _marker: &mut Marker = object::borrow_global_mut<Marker>(marker_id);
    }
}

