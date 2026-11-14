module 0x1::M {
    public fun give() {
        transfer::public_transfer(make_coin(), @0x1);
    }
}
