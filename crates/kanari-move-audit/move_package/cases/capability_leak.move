module 0x1::M {

    public fun leak_cap(treasury: &mut TreasuryCap<Coin>): TreasuryCap<Coin> {
        *treasury
    }

}

