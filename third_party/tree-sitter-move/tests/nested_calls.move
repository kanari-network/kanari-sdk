module test::nested_calls {
    use kanari_system::balance;
    use kanari_system::coin;

    public fun complex(pool_id: address, amount: u64, ctx: &mut TxContext): u64 {
        let doubled = add(mul(amount, 2), div(amount, 2));
        let congestion = if (doubled > 1000) {
            balance::value(&get_balance(pool_id)) + 1
        } else {
            0
        };
        congestion
    }

    fun add(a: u64, b: u64): u64 {
        a + b
    }

    fun mul(a: u64, b: u64): u64 {
        a * b
    }

    fun div(a: u64, b: u64): u64 {
        a / b
    }

    fun get_balance(_id: address): Balance {
        abort 0
    }

    struct Balance has drop {
        value: u64,
    }
}
