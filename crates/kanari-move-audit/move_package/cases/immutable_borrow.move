module 0x1::M {

    entry fun foo(signer: signer, book: &mut OrderBook<Base, Quote>) {
        let _b = vector::borrow(&book.bids, 0);
        let _t = table::borrow(&registry.admins, addr);
    }

}

