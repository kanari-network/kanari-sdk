module 0x1::M {

    public entry fun place_bid<Base, Quote>(signer: signer, book: &mut OrderBook<Base, Quote>) {}
    entry fun good_entry(signer: signer) {}
    public entry fun no_type_params(signer: signer) {}

}

