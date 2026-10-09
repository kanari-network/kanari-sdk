module test::entry_points {
    use kanari_system::tx_context::TxContext;

    public entry fun no_args() {}

    public entry fun with_args(amount: u64, recipient: address, ctx: &mut TxContext) {}

    entry fun bare_entry(flag: bool) {}

    public entry fun long_signature(
        pool_id: address,
        coin_in_id: address,
        amount_in: u64,
        ctx: &mut TxContext,
    ) {}
}
