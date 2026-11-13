module 0x1::M {

    entry fun foo(signer: signer, book: &mut OrderBook<Base, Quote>) {
        let _n = vector::length(&book.bids);
        let _a = table::contains(&registry.admins, addr);
        let _v = coin::value(&coin);
        let _s = signer::address_of(&signer);
    }

}

