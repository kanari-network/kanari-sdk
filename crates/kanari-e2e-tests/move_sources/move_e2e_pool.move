module kanari_e2e_tests::move_e2e_pool {
    use kanari_system::balance::{Self, Balance};
    use kanari_system::coin::{Self, Coin};
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    struct Pool<phantom A, phantom B> has key {
        id: UID,
        balance_a: Balance<A>,
        balance_b: Balance<B>,
        total_shares: u64,
        owner: address
    }

    struct LPToken<phantom A, phantom B> has key, store {
        id: UID,
        amount: u64
    }

    public fun create_pool<A, B>(
        coin_a: Coin<A>, coin_b: Coin<B>, ctx: &mut TxContext
    ): address {
        let amount_a = coin::value(&coin_a);
        let amount_b = coin::value(&coin_b);
        let pool = Pool<A, B> {
            id: object::new(ctx),
            balance_a: coin::into_balance(coin_a),
            balance_b: coin::into_balance(coin_b),
            total_shares: amount_a + amount_b,
            owner: tx_context::sender(ctx)
        };
        let pool_addr = object::uid_to_address(&pool.id);
        transfer::share_object(pool);
        pool_addr
    }

    public fun add_liquidity<A, B>(
        pool: &mut Pool<A, B>, coin_a: Coin<A>, coin_b: Coin<B>, ctx: &mut TxContext
    ): LPToken<A, B> {
        let amount_a = coin::value(&coin_a);
        let amount_b = coin::value(&coin_b);
        balance::join(&mut pool.balance_a, coin::into_balance(coin_a));
        balance::join(&mut pool.balance_b, coin::into_balance(coin_b));
        let shares = amount_a + amount_b;
        pool.total_shares = pool.total_shares + shares;
        LPToken<A, B> { id: object::new(ctx), amount: shares }
    }

    public fun remove_liquidity<A, B>(
        pool: &mut Pool<A, B>, lp: LPToken<A, B>, ctx: &mut TxContext
    ): (Coin<A>, Coin<B>) {
        let LPToken { id, amount } = lp;
        object::delete(id);
        assert!(amount <= pool.total_shares, 600);
        pool.total_shares = pool.total_shares - amount;
        let amt_a = (balance::value(&pool.balance_a) * amount) / pool.total_shares;
        let amt_b = (balance::value(&pool.balance_b) * amount) / pool.total_shares;
        let coin_a = coin::take(&mut pool.balance_a, amt_a, ctx);
        let coin_b = coin::take(&mut pool.balance_b, amt_b, ctx);
        (coin_a, coin_b)
    }

    public fun get_reserves<A, B>(pool: &Pool<A, B>): (u64, u64) {
        (balance::value(&pool.balance_a), balance::value(&pool.balance_b))
    }

    public fun get_total_shares<A, B>(pool: &Pool<A, B>): u64 {
        pool.total_shares
    }
}

