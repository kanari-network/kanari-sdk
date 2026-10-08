#[allow(unused_field)]
module tester::uniswap_amm {
    use kanari_system::balance::{Self, Balance};
    use kanari_system::coin::{Self, Coin};
    use kanari_system::event;
    use kanari_system::math;
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    const E_ZERO_AMOUNT: u64 = 1;
    const E_ZERO_SHARES: u64 = 2;
    const E_SLIPPAGE: u64 = 3;
    const E_INSUFFICIENT_SHARES: u64 = 4;

    /// Input-side swap fee in basis points (9970 = 0.30% withheld by the pool).
    const FEE_BPS: u128 = 9970;

    /// Test-only marker asset types for the two pool sides.
    struct USD_MOCK has drop {}

    struct KAN_MOCK has drop {}

    /// Constant-product pool: reserves are held as `Balance` values and the
    /// LP share supply tracks who may withdraw them.
    struct Pool<phantom X, phantom Y> has key, store {
        id: UID,
        reserve_x: Balance<X>,
        reserve_y: Balance<Y>,
        lp_supply: u64
    }

    /// LP position receipt; consumed (deleted) when liquidity is removed.
    /// Non-generic so entry binding can resolve its key ability.
    struct Receipt has key, store {
        id: UID,
        shares: u64
    }

    struct AddLiquidityEvent has copy, drop {
        sender: address,
        shares: u64
    }

    struct SwapEvent has copy, drop {
        sender: address,
        amount_in: u64,
        amount_out: u64
    }

    struct RemoveLiquidityEvent has copy, drop {
        sender: address,
        shares: u64,
        amount_x: u64,
        amount_y: u64
    }

    /// Test-only faucet: this package exists solely for runtime tests and is
    /// never deployed. Mints `amount` of `T` to the transaction sender.
    public entry fun mint_test_coin<T>(amount: u64, ctx: &mut TxContext) {
        assert!(amount > 0, E_ZERO_AMOUNT);
        let coin = coin::from_balance(balance::create<T>(amount), ctx);
        transfer::public_transfer(coin, tx_context::sender(ctx));
    }

    /// Create an empty `X`/`Y` pool owned by the sender.
    public entry fun create_pool<X, Y>(ctx: &mut TxContext) {
        let pool = Pool<X, Y> {
            id: object::new(ctx),
            reserve_x: balance::zero<X>(),
            reserve_y: balance::zero<Y>(),
            lp_supply: 0
        };
        transfer::public_transfer(pool, tx_context::sender(ctx));
    }

    /// Deposit both coins into the pool and mint an LP `Receipt`.
    ///
    /// First deposit sets the share price at `sqrt(x * y)`; later deposits
    /// mint `min(amount_x * supply / reserve_x, amount_y * supply / reserve_y)`.
    /// The input coins are fully drained (split to zero and merged into the
    /// pool reserves).
    public entry fun add_liquidity<X, Y>(
        pool: &mut Pool<X, Y>,
        coin_x: &mut Coin<X>,
        coin_y: &mut Coin<Y>,
        ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        let amount_x = coin::value(coin_x);
        let amount_y = coin::value(coin_y);
        assert!(amount_x > 0 && amount_y > 0, E_ZERO_AMOUNT);

        let shares =
            if (pool.lp_supply == 0) {
                math::sqrt_u128((amount_x as u128) * (amount_y as u128))
            } else {
                let sx =
                    math::mul_div_u128(
                        (amount_x as u128),
                        (pool.lp_supply as u128),
                        (balance::value(&pool.reserve_x) as u128)
                    );
                let sy =
                    math::mul_div_u128(
                        (amount_y as u128),
                        (pool.lp_supply as u128),
                        (balance::value(&pool.reserve_y) as u128)
                    );
                math::min_u128(sx, sy)
            };
        assert!(shares > 0, E_ZERO_SHARES);

        let bal_x = coin::into_balance(coin::split(coin_x, amount_x, ctx));
        let bal_y = coin::into_balance(coin::split(coin_y, amount_y, ctx));
        balance::merge(&mut pool.reserve_x, bal_x);
        balance::merge(&mut pool.reserve_y, bal_y);
        pool.lp_supply = pool.lp_supply + (shares as u64);
        object::save_object(pool);

        let receipt = Receipt {
            id: object::new(ctx),
            shares: (shares as u64)
        };
        transfer::public_transfer(receipt, sender);
        event::emit(AddLiquidityEvent { sender, shares: (shares as u64) });
    }

    /// Swap an exact input of `X` for `Y` with 0.30% input fee, aborting with
    /// `E_SLIPPAGE` when the quote is below `min_out`. The input coin is fully
    /// drained and the output coin is transferred to the sender.
    public entry fun swap_exact_x_for_y<X, Y>(
        pool: &mut Pool<X, Y>,
        coin_in: &mut Coin<X>,
        min_out: u64,
        ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        let amount_in = coin::value(coin_in);
        assert!(amount_in > 0, E_ZERO_AMOUNT);

        let amount_in_after_fee = math::bps_mul_floor_u128((amount_in as u128), FEE_BPS);
        let amount_out =
            math::quote_amount_out(
                amount_in_after_fee,
                (balance::value(&pool.reserve_x) as u128),
                (balance::value(&pool.reserve_y) as u128)
            );
        assert!(amount_out > 0, E_ZERO_AMOUNT);
        assert!(amount_out >= (min_out as u128), E_SLIPPAGE);

        let in_bal = coin::into_balance(coin::split(coin_in, amount_in, ctx));
        balance::merge(&mut pool.reserve_x, in_bal);
        let out_bal = balance::split(&mut pool.reserve_y, (amount_out as u64));
        transfer::public_transfer(coin::from_balance(out_bal, ctx), sender);
        object::save_object(pool);
        event::emit(SwapEvent { sender, amount_in, amount_out: (amount_out as u64) });
    }

    /// Burn an LP `Receipt` and withdraw the pro-rata share of both reserves.
    /// The receipt is consumed. The receipt parameter comes first so the
    /// runtime's declared-object binding (which only resolves type-tag-bearing
    /// parameters through the explicit list) consumes it before the pool binds
    /// through the raw object-id path.
    public entry fun remove_liquidity<X, Y>(
        receipt: Receipt, pool: &mut Pool<X, Y>, ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        let Receipt { id, shares } = receipt;
        object::delete(id);
        assert!(shares > 0, E_ZERO_SHARES);
        let supply = pool.lp_supply;
        assert!(shares <= supply, E_INSUFFICIENT_SHARES);

        let amount_x =
            math::mul_div_u128(
                (balance::value(&pool.reserve_x) as u128),
                (shares as u128),
                (supply as u128)
            );
        let amount_y =
            math::mul_div_u128(
                (balance::value(&pool.reserve_y) as u128),
                (shares as u128),
                (supply as u128)
            );
        assert!(amount_x > 0 && amount_y > 0, E_ZERO_AMOUNT);

        pool.lp_supply = supply - shares;
        let out_x = balance::split(&mut pool.reserve_x, (amount_x as u64));
        let out_y = balance::split(&mut pool.reserve_y, (amount_y as u64));
        transfer::public_transfer(coin::from_balance(out_x, ctx), sender);
        transfer::public_transfer(coin::from_balance(out_y, ctx), sender);
        object::save_object(pool);
        event::emit(
            RemoveLiquidityEvent {
                sender,
                shares,
                amount_x: (amount_x as u64),
                amount_y: (amount_y as u64)
            }
        );
    }

    // ------------------------------------------------------------------
    // Views
    // ------------------------------------------------------------------
    public fun get_reserves<X, Y>(pool: &Pool<X, Y>): (u64, u64) {
        (balance::value(&pool.reserve_x), balance::value(&pool.reserve_y))
    }

    public fun lp_total<X, Y>(pool: &Pool<X, Y>): u64 {
        pool.lp_supply
    }

    public fun quote_swap<X, Y>(pool: &Pool<X, Y>, amount_in: u64): u64 {
        let amount_in_after_fee = math::bps_mul_floor_u128((amount_in as u128), FEE_BPS);
        let amount_out =
            math::quote_amount_out(
                amount_in_after_fee,
                (balance::value(&pool.reserve_x) as u128),
                (balance::value(&pool.reserve_y) as u128)
            );
        (amount_out as u64)
    }
}

