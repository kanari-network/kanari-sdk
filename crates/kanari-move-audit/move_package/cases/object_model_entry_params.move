module 0x1::M {

    entry fun cancel_order<Base, Quote>(
        book: &mut OrderBook<Base, Quote>,
        order_id: u64,
        ctx: &mut TxContext
    ) {}

}

