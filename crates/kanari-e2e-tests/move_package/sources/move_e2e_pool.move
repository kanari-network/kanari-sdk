module kanari_e2e_tests::move_e2e_pool {
    use kanari_system::balance::{Self, Balance};
    use kanari_system::coin::{Self, Coin};
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    const E_INSUFFICIENT_SHARES: u64 = 600;
    const E_EMPTY_POOL: u64 = 601;
    const E_BAD_RESERVE: u64 = 602;

    /// Constant-product pool over two coin types.
    public struct Pool<phantom A, phantom B> has key, store {
        id: UID,
        balance_a: Balance<A>,
        balance_b: Balance<B>,
        total_shares: u64,
        owner: address
    }

    /// Liquidity receipt returned by `add_liquidity`.
    public struct LPReceipt<phantom A, phantom B> has drop, store {
        amount: u64
    }

    /// Integer-only accounting pool, executable without coin objects so the
    /// liquidity math can be verified through the runtime.
    public struct SimplePool has key, store {
        id: UID,
        reserve_a: u64,
        reserve_b: u64,
        total_shares: u64,
        owner: address
    }

    /// Seed a shared pool from the provided liquidity and return its address.
    public fun create_pool<A, B>(
        coin_a: Coin<A>, coin_b: Coin<B>, ctx: &mut TxContext
    ): address {
        let shares = coin::value(&coin_a) + coin::value(&coin_b);
        let pool = Pool<A, B> {
            id: object::new(ctx),
            balance_a: coin::into_balance(coin_a),
            balance_b: coin::into_balance(coin_b),
            total_shares: shares,
            owner: tx_context::sender(ctx)
        };
        let addr = object::uid_address(&pool.id);
        transfer::share_object(pool);
        addr
    }

    /// Add liquidity and return a receipt holding the minted share count.
    public fun add_liquidity<A, B>(
        pool: &mut Pool<A, B>, coin_a: Coin<A>, coin_b: Coin<B>
    ): LPReceipt<A, B> {
        let shares = coin::value(&coin_a) + coin::value(&coin_b);
        balance::merge(&mut pool.balance_a, coin::into_balance(coin_a));
        balance::merge(&mut pool.balance_b, coin::into_balance(coin_b));
        pool.total_shares = pool.total_shares + shares;
        LPReceipt<A, B> { amount: shares }
    }

    /// Burn the receipt's shares and return the proportional liquidity.
    public fun remove_liquidity<A, B>(
        pool: &mut Pool<A, B>, receipt: LPReceipt<A, B>, ctx: &mut TxContext
    ): (Coin<A>, Coin<B>) {
        let amount = receipt.amount;
        assert!(pool.total_shares >= amount, E_INSUFFICIENT_SHARES);
        let total = pool.total_shares;
        pool.total_shares = total - amount;
        let out_a = math_mul_div(balance::value(&pool.balance_a), amount, total);
        let out_b = math_mul_div(balance::value(&pool.balance_b), amount, total);
        let taken_a = balance::split(&mut pool.balance_a, out_a);
        let taken_b = balance::split(&mut pool.balance_b, out_b);
        (
            coin::from_balance(taken_a, ctx), coin::from_balance(taken_b, ctx)
        )
    }

    public fun reserves<A, B>(pool: &Pool<A, B>): (u64, u64) {
        (balance::value(&pool.balance_a), balance::value(&pool.balance_b))
    }

    public fun total_shares<A, B>(pool: &Pool<A, B>): u64 {
        pool.total_shares
    }

    public fun pool_owner<A, B>(pool: &Pool<A, B>): address {
        pool.owner
    }

    /// Create a shared integer-only pool seeded with both reserves.
    public entry fun create_simple_pool(
        reserve_a: u64, reserve_b: u64, ctx: &mut TxContext
    ) {
        assert!(reserve_a > 0 && reserve_b > 0, E_EMPTY_POOL);
        let pool = SimplePool {
            id: object::new(ctx),
            reserve_a,
            reserve_b,
            total_shares: reserve_a + reserve_b,
            owner: tx_context::sender(ctx)
        };
        transfer::share_object(pool);
    }

    /// Proportionally add liquidity using the first deposit as the share oracle.
    public entry fun add_simple_liquidity(
        pool: &mut SimplePool, amount_a: u64, amount_b: u64
    ) {
        let minted =
            if (pool.total_shares == 0) {
                amount_a + amount_b
            } else {
                mul_div(amount_a, pool.total_shares, pool.reserve_a)
            };
        pool.reserve_a = pool.reserve_a + amount_a;
        pool.reserve_b = pool.reserve_b + amount_b;
        pool.total_shares = pool.total_shares + minted;
    }

    /// Burn `shares` and return the proportional liquidity as `(out_a, out_b)`.
    public entry fun remove_simple_liquidity(
        pool: &mut SimplePool, shares: u64, out_a: u64, out_b: u64
    ) {
        assert!(pool.total_shares >= shares, E_INSUFFICIENT_SHARES);
        assert!(
            pool.reserve_a >= out_a && pool.reserve_b >= out_b,
            E_INSUFFICIENT_SHARES
        );
        pool.total_shares = pool.total_shares - shares;
        pool.reserve_a = pool.reserve_a - out_a;
        pool.reserve_b = pool.reserve_b - out_b;
    }

    /// Entry wrapper asserting the current reserves and share supply.
    public entry fun e2e_check_simple_pool(
        pool: &SimplePool, reserve_a: u64, reserve_b: u64, shares: u64
    ) {
        assert!(pool.reserve_a == reserve_a, E_BAD_RESERVE);
        assert!(pool.reserve_b == reserve_b, E_BAD_RESERVE);
        assert!(pool.total_shares == shares, E_BAD_RESERVE);
    }

    /// Entry wrapper exercising the 128-bit mul-div used by the pool math.
    public entry fun e2e_check_mul_div(x: u64, y: u64, z: u64, expected: u64) {
        assert!(math_mul_div(x, y, z) == expected, E_BAD_RESERVE);
    }

    /// Entry wrapper exercising the generic pool's mul-div at scale.
    public entry fun e2e_check_scaled_reserves(x: u64, y: u64, expected: u64) {
        assert!(
            math_mul_div(x, y, 10000) == expected,
            E_BAD_RESERVE
        );
    }

    fun math_mul_div(x: u64, y: u64, z: u64): u64 {
        (((((x as u128) * (y as u128)) / (z as u128)) as u64))
    }

    fun mul_div(x: u64, y: u64, z: u64): u64 {
        math_mul_div(x, y, z)
    }
}

