// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! DeepBook — a central limit order book (CLOB) for Kanari, ported and
//! adapted from the Iota `DeepBook.move` reference design.
//!
//! Design summary:
//! * `OrderBook<Base, Quote>` is a **shared object** holding sorted `bids`
//!   (price desc) and `asks` (price asc) vectors plus escrowed
//!   `Balance<Base>` / `Balance<Quote>` for locked maker funds and fee
//!   balances. Depth per side is capped by `max_depth`; when full the worst
//!   resting order is evicted and refunded proportionally.
//! * Order types: `0` limit, `1` IOC (immediate-or-cancel), `2` FOK
//!   (fill-or-kill), `3` post-only (aborts if it would cross the book).
//! * Prices are normalized to `PRICE_SCALE` (1e9) quote-per-base units;
//!   per-book `base_decimals` / `quote_decimals` convert to raw token units.
//! * Matching is price-time priority; taker pays a `fee_bps` fee on the quote
//!   leg, collected into `fee_balance_quote` and withdrawable by the book
//!   admin registered in `GlobalOrderBookRegistry`.
//! * The registry deduplicates books per token pair via a blake3-256 hash of
//!   the sorted type names, so `BASE/QUOTE` and `QUOTE/BASE` cannot both exist.
//!
//! All price/fee/refund arithmetic routes through `kanari_system::math`
//! (U256-intermediate mul_div, bps helpers, checked narrowing casts) so no
//! hand-rolled overflow logic lives in this module.
//!
//! Kanari adaptations vs the Iota original: `iota::*` modules map to
//! `kanari_system::*` (`balance::join` -> `balance::merge`,
//! `object::uid_to_address` -> `object::uid_address`), hashing comes from
//! `std::hash`, and `get_or_create_order_book_with_decimals` is a plain
//! `public fun` because Move entry functions may not return values.

module deepbook::deepbook {
    use kanari_system::balance::{Self, Balance};
    use kanari_system::coin::{Self, Coin};
    use kanari_system::event;
    use kanari_system::math;
    use kanari_system::object;
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};
    use std::type_name;

    // Error codes
    const E_INSUFFICIENT_LIQUIDITY: u64 = 1;
    const E_INVALID_PRICE: u64 = 2;
    const E_INVALID_QUANTITY: u64 = 3;
    const E_ORDER_NOT_FOUND: u64 = 4;
    const E_UNAUTHORIZED: u64 = 5;
    const E_INVALID_FEE: u64 = 6;
    const E_INVALID_DEPTH: u64 = 7;
    const E_SAME_TOKEN_PAIR: u64 = 9;
    const E_OVERFLOW: u64 = 10;
    const E_POST_ONLY_VIOLATION: u64 = 11;

    // depth limits
    const MAX_DEPTH: u64 = 1_000_000;

    // Price scale factor (normalize prices to 9 decimals)
    const PRICE_SCALE: u128 = 1_000_000_000;

    /// Limit order in the book
    public struct LimitOrder has copy, drop, store {
        id: u64,
        maker: address,
        is_bid: bool,
        price: u64,
        // price in PRICE_SCALE units
        quantity: u64,
        // quantity in base token units
        filled: u64,
        // filled amount
        locked_amount: u64,
        // locked funds (quote for bid, base for ask)
    }

    /// Central limit order book
    public struct OrderBook<phantom Base, phantom Quote> has key, store {
        id: object::UID,
        next_order_id: u64,
        bids: vector<LimitOrder>,
        asks: vector<LimitOrder>,
        base_balance: Balance<Base>,
        quote_balance: Balance<Quote>,
        fee_balance_base: Balance<Base>,
        // collected fees in base
        fee_balance_quote: Balance<Quote>,
        // collected fees in quote
        fee_bps: u64,
        // fee in basis points (e.g. 30 = 0.3%)
        max_depth: u64,
        base_decimals: u8,
        quote_decimals: u8,
    }

    // Events
    public struct OrderPlaced has copy, drop {
        book_id: address,
        order_id: u64,
        maker: address,
        is_bid: bool,
        price: u64,
        quantity: u64,
    }

    public struct OrderMatched has copy, drop {
        book_id: address,
        order_id: u64,
        taker: address,
        maker: address,
        price: u64,
        quantity: u64,
    }

    public struct OrderCancelled has copy, drop {
        book_id: address,
        order_id: u64,
    }

    public struct OrderBookCreated has copy, drop {
        book_id: address,
    }

    // ---------------------------------------------------------------------------
    // Arithmetic core (all via kanari_system::math)
    // ---------------------------------------------------------------------------

    /// Quote units (u128) for `quantity` base units at `price` (PRICE_SCALE
    /// units), honoring both decimal counts:
    /// `(quantity * price * 10^qd) / (PRICE_SCALE * 10^bd)`, floor.
    fun trade_quote_u128(
        price: u128,
        quantity: u128,
        base_decimals: u8,
        quote_decimals: u8,
    ): u128 {
        let numerator = math::mul_div_u128(quantity, price, 1);
        let denominator = PRICE_SCALE * math::pow_u128(10, (base_decimals as u32));
        math::mul_div_u128(
        numerator,
        math::pow_u128(10, (quote_decimals as u32)),
        denominator,
    )
    }

    /// Narrow a u128 amount to u64, aborting E_OVERFLOW instead of truncating.
    fun checked_u64(value: u128): u64 {
        let narrowed = math::try_as_u64_from_u128(value);
        assert!(option::is_some(&narrowed), E_OVERFLOW);
        option::destroy_some(narrowed)
    }

    /// Move the taker fee from the quote pool into the fee pool. A zero fee
    /// (fee_bps = 0, or a trade small enough that bps math floors to 0) is a
    /// legitimate no-op: `balance::split(0)` aborts, so it must be guarded.
    fun collect_fee<Base, Quote>(book: &mut OrderBook<Base, Quote>, fee_amount: u64) {
        if (fee_amount > 0) {
            let fee_collected = balance::split(&mut book.quote_balance, fee_amount);
            balance::merge(&mut book.fee_balance_quote, fee_collected);
        }
    }

    // ---------------------------------------------------------------------------
    // Constructors
    // ---------------------------------------------------------------------------

    /// Internal constructor: build an `OrderBook` without sharing or emitting.
    public fun new_order_book<Base, Quote>(
        fee_bps: u64,
        max_depth: u64,
        base_decimals: u8,
        quote_decimals: u8,
        ctx: &mut TxContext,
    ): OrderBook<Base, Quote> {
        assert!(fee_bps <= 1000, E_INVALID_FEE); // max 10%
        // validate depth (1 .. MAX_DEPTH)
        assert!(max_depth > 0, E_INVALID_DEPTH);
        assert!(max_depth <= MAX_DEPTH, E_INVALID_DEPTH);

        OrderBook<Base, Quote> {
        id: object::new(ctx),
        next_order_id: 1,
        bids: vector::empty(),
        asks: vector::empty(),
        base_balance: balance::zero(),
        quote_balance: balance::zero(),
        fee_balance_base: balance::zero(),
        fee_balance_quote: balance::zero(),
        fee_bps,
        max_depth,
        base_decimals,
        quote_decimals,
    }
    }

    /// Create a new order book with explicit decimals, share it, and emit
    /// `OrderBookCreated`. Returns the book address.
    public fun create_order_book_with_decimals<Base, Quote>(
        fee_bps: u64,
        max_depth: u64,
        base_decimals: u8,
        quote_decimals: u8,
        ctx: &mut TxContext,
    ): address {
        // Prevent creating order book with same base and quote token
        let base_type = type_name::get_with_original_ids<Base>().into_string().into_bytes();
        let quote_type = type_name::get_with_original_ids<Quote>().into_string().into_bytes();
        assert!(base_type != quote_type, E_SAME_TOKEN_PAIR);

        let book = new_order_book<Base, Quote>(fee_bps, max_depth, base_decimals, quote_decimals, ctx);

        let book_id = object::uid_address(&book.id);

        // Emit an event with the created book id so UIs can discover the object address
        event::emit(OrderBookCreated { book_id });
        transfer::share_object(book);
        book_id
    }

    /// Return the address of an order book.
    public fun book_id<Base, Quote>(book: &OrderBook<Base, Quote>): address {
        object::uid_address(&book.id)
    }

    // ---------------------------------------------------------------------------
    // Order placement
    // ---------------------------------------------------------------------------

    /// Place a buy order (supports order types)
    /// price: normalized price (quote_amount * PRICE_SCALE / base_amount)
    /// quantity: amount of base token to buy
    /// payment: quote token payment (must be >= quantity * price / PRICE_SCALE)
    /// order_type: 0 = limit, 1 = IOC, 2 = FOK, 3 = PostOnly
    public entry fun place_bid<Base, Quote>(
        book: &mut OrderBook<Base, Quote>,
        price: u64,
        quantity: u64,
        mut payment: Coin<Quote>,
        order_type: u8,
        ctx: &mut TxContext,
    ) {
        assert!(price > 0, E_INVALID_PRICE);
        assert!(quantity > 0, E_INVALID_QUANTITY);

        let maker = tx_context::sender(ctx);

        // Pre-checks for special order types
        // FOK: ensure there is sufficient available ask quantity at <= price before consuming funds
        if (order_type == 2u8) {
        let mut total_available = 0u64;
        let mut k = 0u64;
        while (k < vector::length(&book.asks)) {
            let a = vector::borrow(&book.asks, k);
            if (a.price <= price) {
                total_available = total_available + (a.quantity - a.filled);
                k = k + 1;
            } else {
                break
            };
        };
        assert!(total_available >= quantity, E_INSUFFICIENT_LIQUIDITY);
    };

        // PostOnly: if the order would match immediately, abort
        if (order_type == 3u8) {
        if (vector::length(&book.asks) > 0) {
            let best = vector::borrow(&book.asks, 0);
            assert!(best.price > price, E_POST_ONLY_VIOLATION);
        };
    };

        // Required quote with decimals; reject dust that rounds to zero.
        let required_quote_u128 = trade_quote_u128(
        (price as u128),
        (quantity as u128),
        book.base_decimals,
        book.quote_decimals,
    );
        assert!(required_quote_u128 > 0, E_INSUFFICIENT_LIQUIDITY);
        let required_quote = checked_u64(required_quote_u128);
        let payment_value = coin::value(&payment);
        assert!(payment_value >= required_quote, E_INSUFFICIENT_LIQUIDITY);

        // Split exact amount needed, return excess
        let locked_payment = if (payment_value > required_quote) {
        let excess = coin::split(&mut payment, payment_value - required_quote, ctx);
        transfer::public_transfer(excess, maker);
        payment
    } else {
        payment
    };

        // Add payment to book balance
        let payment_balance = coin::into_balance(locked_payment);
        balance::merge(&mut book.quote_balance, payment_balance);

        let order_id = book.next_order_id;
        book.next_order_id = order_id + 1;

        let mut remaining = quantity;
        let mut remaining_locked = required_quote;

        // Try to match with existing asks
        let mut i = 0;
        while (i < vector::length(&book.asks) && remaining > 0) {
        let ask = vector::borrow_mut(&mut book.asks, i);

        if (ask.price <= price) {
            let available = ask.quantity - ask.filled;
            let matched = if (available <= remaining) { available } else { remaining };

            // Execute at the resting ask price.
            let match_price = ask.price;
            let trade_value_u128 = trade_quote_u128(
                (match_price as u128),
                (matched as u128),
                book.base_decimals,
                book.quote_decimals,
            );
            assert!(trade_value_u128 > 0, E_INSUFFICIENT_LIQUIDITY);
            let quote_amount = checked_u64(trade_value_u128);

            // Taker fee on the quote leg.
            let fee_amount = checked_u64(math::bps_mul_floor_u128(
                trade_value_u128,
                (book.fee_bps as u128),
            ));
            let maker_receives = quote_amount - fee_amount;

            // Update order
            ask.filled = ask.filled + matched;
            ask.locked_amount = ask.locked_amount - matched;
            remaining = remaining - matched;
            remaining_locked = remaining_locked - quote_amount;

            // Transfer base to taker (buyer)
            let base_to_taker = balance::split(&mut book.base_balance, matched);
            transfer::public_transfer(coin::from_balance(base_to_taker, ctx), maker);

            // Transfer quote to maker (seller) minus fee
            let quote_to_maker = balance::split(&mut book.quote_balance, maker_receives);
            transfer::public_transfer(coin::from_balance(quote_to_maker, ctx), ask.maker);

            let maker_order = ask.maker;
            let maker_order_id = ask.id;
            let fully_filled = ask.filled == ask.quantity;

            // Collect fee (in quote tokens)
            collect_fee(book, fee_amount);

            event::emit(OrderMatched {
                book_id: object::uid_address(&book.id),
                order_id: maker_order_id,
                taker: maker,
                maker: maker_order,
                price: match_price,
                quantity: matched,
            });

            // Remove fully filled order
            if (fully_filled) {
                vector::remove(&mut book.asks, i);
            } else {
                i = i + 1;
            };
        } else {
            break // asks are sorted, no more matches possible
        };
    };
        // If there's remaining quantity, handle based on order type
        if (remaining > 0) {
        // IOC: refund remaining locked quote back to maker and do NOT add resting order
        if (order_type == 1u8) {
            if (remaining_locked > 0) {
                let refund = balance::split(&mut book.quote_balance, remaining_locked);
                transfer::public_transfer(coin::from_balance(refund, ctx), maker);
            };
        } else {
            let new_order = LimitOrder {
                id: order_id,
                maker,
                is_bid: true,
                price,
                quantity: remaining,
                filled: 0,
                locked_amount: remaining_locked,
            };

            // Insert in sorted order (highest price first)
            let mut len = vector::length(&book.bids);

            // If side is full, evict the worst bid and refund its unmatched share
            if (len >= book.max_depth) {
                let worst = vector::pop_back(&mut book.bids);
                let unmatched = worst.quantity - worst.filled;
                if (unmatched > 0) {
                    let refund_quote = checked_u64(math::mul_div_u128(
                        (worst.locked_amount as u128),
                        (unmatched as u128),
                        (worst.quantity as u128),
                    ));
                    if (refund_quote > 0) {
                        let refund = balance::split(&mut book.quote_balance, refund_quote);
                        transfer::public_transfer(coin::from_balance(refund, ctx), worst.maker);
                    };
                };
                len = len - 1;
            };

            let mut insert_pos = len;
            let mut j = 0;
            while (j < len) {
                let bid = vector::borrow(&book.bids, j);
                // insert before strictly lower-priced bids; equal price keeps FIFO
                if (price > bid.price || (price == bid.price && order_id < bid.id)) {
                    insert_pos = j;
                    break
                };
                j = j + 1;
            };

            if (insert_pos == len) {
                vector::push_back(&mut book.bids, new_order);
            } else {
                vector::insert(&mut book.bids, new_order, insert_pos);
            };

            event::emit(OrderPlaced {
                book_id: object::uid_address(&book.id),
                order_id,
                maker,
                is_bid: true,
                price,
                quantity: remaining,
            });
        };
    };
    }

    /// Place a sell order (supports order types)
    /// price: normalized price (quote_amount * PRICE_SCALE / base_amount)
    /// quantity: amount of base token to sell
    /// base_coin: base token to sell
    /// order_type: 0 = limit, 1 = IOC, 2 = FOK, 3 = PostOnly
    public entry fun place_ask<Base, Quote>(
        book: &mut OrderBook<Base, Quote>,
        price: u64,
        quantity: u64,
        mut base_coin: Coin<Base>,
        order_type: u8,
        ctx: &mut TxContext,
    ) {
        assert!(price > 0, E_INVALID_PRICE);
        assert!(quantity > 0, E_INVALID_QUANTITY);

        let coin_value = coin::value(&base_coin);
        assert!(coin_value >= quantity, E_INSUFFICIENT_LIQUIDITY);

        let maker = tx_context::sender(ctx);

        // FOK: ensure there is sufficient available bid quantity at >= price before consuming funds
        if (order_type == 2u8) {
        let mut total_available = 0u64;
        let mut k = 0u64;
        while (k < vector::length(&book.bids)) {
            let b = vector::borrow(&book.bids, k);
            if (b.price >= price) {
                total_available = total_available + (b.quantity - b.filled);
                k = k + 1;
            } else {
                break
            };
        };
        assert!(total_available >= quantity, E_INSUFFICIENT_LIQUIDITY);
    };

        // PostOnly: if the order would match immediately, abort
        if (order_type == 3u8) {
        if (vector::length(&book.bids) > 0) {
            let best = vector::borrow(&book.bids, 0);
            assert!(best.price < price, E_POST_ONLY_VIOLATION);
        };
    };

        // Defensive pre-check: reject orders whose quote value would round to
        // zero or exceed u64 bounds before any funds move.
        let potential_quote_u128 = trade_quote_u128(
        (price as u128),
        (quantity as u128),
        book.base_decimals,
        book.quote_decimals,
    );
        assert!(potential_quote_u128 > 0, E_INSUFFICIENT_LIQUIDITY);
        let _potential_quote = checked_u64(potential_quote_u128);

        // Split exact amount needed, return excess
        let locked_base = if (coin_value > quantity) {
        let excess = coin::split(&mut base_coin, coin_value - quantity, ctx);
        transfer::public_transfer(excess, maker);
        base_coin
    } else {
        base_coin
    };

        // Add base coin to book balance
        let base_balance = coin::into_balance(locked_base);
        balance::merge(&mut book.base_balance, base_balance);

        let order_id = book.next_order_id;
        book.next_order_id = order_id + 1;

        let mut remaining = quantity;

        // Try to match with existing bids
        let mut i = 0;
        while (i < vector::length(&book.bids) && remaining > 0) {
        let bid = vector::borrow_mut(&mut book.bids, i);

        if (bid.price >= price) {
            let available = bid.quantity - bid.filled;
            let matched = if (available <= remaining) { available } else { remaining };

            // Execute at the resting bid price.
            let match_price = bid.price;
            let trade_value_u128 = trade_quote_u128(
                (match_price as u128),
                (matched as u128),
                book.base_decimals,
                book.quote_decimals,
            );
            assert!(trade_value_u128 > 0, E_INSUFFICIENT_LIQUIDITY);
            let quote_amount = checked_u64(trade_value_u128);

            // Taker fee on the quote leg.
            let fee_amount = checked_u64(math::bps_mul_floor_u128(
                trade_value_u128,
                (book.fee_bps as u128),
            ));

            // Update order
            bid.filled = bid.filled + matched;
            if (quote_amount > 0) {
                bid.locked_amount = bid.locked_amount - quote_amount;
            };
            remaining = remaining - matched;

            // Transfer quote to taker (seller) minus fee
            let taker_receive = quote_amount - fee_amount;
            let quote_to_taker = balance::split(&mut book.quote_balance, taker_receive);
            transfer::public_transfer(coin::from_balance(quote_to_taker, ctx), maker);

            // Transfer base to maker (buyer)
            let base_to_maker = balance::split(&mut book.base_balance, matched);
            transfer::public_transfer(coin::from_balance(base_to_maker, ctx), bid.maker);

            let maker_order = bid.maker;
            let maker_order_id = bid.id;
            let fully_filled = bid.filled == bid.quantity;

            // Collect fee in quote tokens
            collect_fee(book, fee_amount);

            event::emit(OrderMatched {
                book_id: object::uid_address(&book.id),
                order_id: maker_order_id,
                taker: maker,
                maker: maker_order,
                price: match_price,
                quantity: matched,
            });

            // Remove fully filled order
            if (fully_filled) {
                vector::remove(&mut book.bids, i);
            } else {
                i = i + 1;
            };
        } else {
            break // bids are sorted, no more matches possible
        };
    };

        // If there's remaining quantity, handle based on order type
        if (remaining > 0) {
        // IOC: refund remaining locked base back to maker and do NOT add resting order
        if (order_type == 1u8) {
            if (remaining > 0) {
                let refund = balance::split(&mut book.base_balance, remaining);
                transfer::public_transfer(coin::from_balance(refund, ctx), maker);
            };
        } else {
            let new_order = LimitOrder {
                id: order_id,
                maker,
                is_bid: false,
                price,
                quantity: remaining,
                filled: 0,
                locked_amount: remaining, // locked base amount
            };

            // Insert in sorted order (lowest price first)
            let mut len = vector::length(&book.asks);

            // If side is full, evict the worst ask and refund its unmatched share
            if (len >= book.max_depth) {
                let worst = vector::pop_back(&mut book.asks);
                let unmatched = worst.quantity - worst.filled;
                if (unmatched > 0) {
                    let refund_base = checked_u64(math::mul_div_u128(
                        (worst.locked_amount as u128),
                        (unmatched as u128),
                        (worst.quantity as u128),
                    ));
                    if (refund_base > 0) {
                        let refund = balance::split(&mut book.base_balance, refund_base);
                        transfer::public_transfer(coin::from_balance(refund, ctx), worst.maker);
                    };
                };
                len = len - 1;
            };

            let mut insert_pos = len;
            let mut j = 0;
            while (j < len) {
                let ask = vector::borrow(&book.asks, j);
                // insert before strictly higher-priced asks; equal price keeps FIFO
                if (price < ask.price || (price == ask.price && order_id < ask.id)) {
                    insert_pos = j;
                    break
                };
                j = j + 1;
            };

            if (insert_pos == len) {
                vector::push_back(&mut book.asks, new_order);
            } else {
                vector::insert(&mut book.asks, new_order, insert_pos);
            };

            event::emit(OrderPlaced {
                book_id: object::uid_address(&book.id),
                order_id,
                maker,
                is_bid: false,
                price,
                quantity: remaining,
            });
        };
    };
    }

    /// Cancel an order and return locked funds to maker
    public entry fun cancel_order<Base, Quote>(
        book: &mut OrderBook<Base, Quote>,
        order_id: u64,
        ctx: &mut TxContext,
    ) {
        let caller = tx_context::sender(ctx);

        // Search in bids
        let mut i = 0;
        let bids_len = vector::length(&book.bids);
        while (i < bids_len) {
        let bid = vector::borrow(&book.bids, i);
        if (bid.id == order_id) {
            assert!(bid.maker == caller, E_UNAUTHORIZED);

            // Refund the unmatched share of the locked quote.
            let refund_amount = checked_u64(math::mul_div_u128(
                (bid.locked_amount as u128),
                ((bid.quantity - bid.filled) as u128),
                (bid.quantity as u128),
            ));

            vector::remove(&mut book.bids, i);

            if (refund_amount > 0) {
                let refund = balance::split(&mut book.quote_balance, refund_amount);
                transfer::public_transfer(coin::from_balance(refund, ctx), caller);
            };

            event::emit(OrderCancelled {
                book_id: object::uid_address(&book.id),
                order_id,
            });
            return
        };
        i = i + 1;
    };

        // Search in asks
        let mut j = 0;
        let asks_len = vector::length(&book.asks);
        while (j < asks_len) {
        let ask = vector::borrow(&book.asks, j);
        if (ask.id == order_id) {
            assert!(ask.maker == caller, E_UNAUTHORIZED);

            let refund_base = ask.locked_amount;
            vector::remove(&mut book.asks, j);

            if (refund_base > 0) {
                let refund = balance::split(&mut book.base_balance, refund_base);
                transfer::public_transfer(coin::from_balance(refund, ctx), caller);
            };

            event::emit(OrderCancelled {
                book_id: object::uid_address(&book.id),
                order_id,
            });
            return
        };
        j = j + 1;
    };

        abort E_ORDER_NOT_FOUND
    }

    // ---------------------------------------------------------------------------
    // Queries
    // ---------------------------------------------------------------------------

    /// Get best bid price
    public fun get_best_bid<Base, Quote>(book: &OrderBook<Base, Quote>): u64 {
        if (vector::length(&book.bids) > 0) {
        let best_bid = vector::borrow(&book.bids, 0);
        best_bid.price
    } else {
        0
    }
    }

    /// Get best ask price
    public fun get_best_ask<Base, Quote>(book: &OrderBook<Base, Quote>): u64 {
        if (vector::length(&book.asks) > 0) {
        let best_ask = vector::borrow(&book.asks, 0);
        best_ask.price
    } else {
        0
    }
    }

    /// Get spread (difference between best ask and best bid)
    public fun get_spread<Base, Quote>(book: &OrderBook<Base, Quote>): u64 {
        let best_bid = get_best_bid(book);
        let best_ask = get_best_ask(book);

        if (best_bid > 0 && best_ask > 0 && best_ask > best_bid) {
        best_ask - best_bid
    } else {
        0
    }
    }

    /// Get total number of open bids
    public fun get_bid_count<Base, Quote>(book: &OrderBook<Base, Quote>): u64 {
        vector::length(&book.bids)
    }

    /// Get total number of open asks
    public fun get_ask_count<Base, Quote>(book: &OrderBook<Base, Quote>): u64 {
        vector::length(&book.asks)
    }

    /// Get maximum depth per side
    public fun get_max_depth<Base, Quote>(book: &OrderBook<Base, Quote>): u64 {
        book.max_depth
    }

    /// Get order book depth (total liquidity)
    public fun get_book_depth<Base, Quote>(book: &OrderBook<Base, Quote>): (u64, u64) {
        let mut total_bid_quantity = 0u64;
        let mut total_ask_quantity = 0u64;

        let mut i = 0;
        while (i < vector::length(&book.bids)) {
        let bid = vector::borrow(&book.bids, i);
        total_bid_quantity = total_bid_quantity + (bid.quantity - bid.filled);
        i = i + 1;
    };

        let mut j = 0;
        while (j < vector::length(&book.asks)) {
        let ask = vector::borrow(&book.asks, j);
        total_ask_quantity = total_ask_quantity + (ask.quantity - ask.filled);
        j = j + 1;
    };

        (total_bid_quantity, total_ask_quantity)
    }

    /// Get current fee balances (base, quote) for a book
    public fun get_fee_balances<Base, Quote>(book: &OrderBook<Base, Quote>): (u64, u64) {
        (balance::value(&book.fee_balance_base), balance::value(&book.fee_balance_quote))
    }

    /// Get current locked balances (base, quote) in the order book
    public fun get_locked_balances<Base, Quote>(book: &OrderBook<Base, Quote>): (u64, u64) {
        (balance::value(&book.base_balance), balance::value(&book.quote_balance))
    }

    // ---------------------------------------------------------------------------
    // Calculations
    // ---------------------------------------------------------------------------

    /// Calculate quote amount for given base amount at price
    public fun calculate_quote_amount(base_amount: u64, price: u64): u64 {
        checked_u64(math::mul_div_u128(
        (base_amount as u128),
        (price as u128),
        PRICE_SCALE,
    ))
    }

    /// Calculate base amount for given quote amount at price
    public fun calculate_base_amount(quote_amount: u64, price: u64): u64 {
        checked_u64(math::mul_div_u128(
        (quote_amount as u128),
        PRICE_SCALE,
        (price as u128),
    ))
    }

    /// Decimals-aware quote amount calculation
    public fun calculate_quote_amount_with_decimals(
        base_amount: u64,
        price: u64,
        base_decimals: u8,
        quote_decimals: u8,
    ): u64 {
        checked_u64(trade_quote_u128(
        (price as u128),
        (base_amount as u128),
        base_decimals,
        quote_decimals,
    ))
    }

    /// Decimals-aware base amount calculation
    public fun calculate_base_amount_with_decimals(
        quote_amount: u64,
        price: u64,
        base_decimals: u8,
        quote_decimals: u8,
    ): u64 {
        let numerator = math::mul_div_u128(
        (quote_amount as u128),
        PRICE_SCALE,
        1,
    ) * math::pow_u128(10, (base_decimals as u32));
        let denominator =
        (price as u128) * math::pow_u128(10, (quote_decimals as u32));
        checked_u64(math::mul_div_u128(numerator, 1, denominator))
    }

    /// Checked pre-flight for place_bid quote capacity (same asserts as the entry).
    public fun validate_quote_capacity(
        price: u64,
        quantity: u64,
        base_decimals: u8,
        quote_decimals: u8,
    ) {
        let required_quote_u128 = trade_quote_u128(
        (price as u128),
        (quantity as u128),
        base_decimals,
        quote_decimals,
    );
        assert!(required_quote_u128 > 0, E_INSUFFICIENT_LIQUIDITY);
        let _fits = checked_u64(required_quote_u128);
    }

    /// Checked refund math used by eviction/cancel paths.
    public fun check_refund_overflow(
        unmatched: u128,
        price: u128,
        base_decimals: u8,
        quote_decimals: u8,
    ) {
        let refund_quote_u128 = trade_quote_u128(price, unmatched, base_decimals, quote_decimals);
        let _fits = checked_u64(refund_quote_u128);
    }

    // ---------------------------------------------------------------------------
    // Order accessors
    // ---------------------------------------------------------------------------

    /// Get all bids in the order book
    public fun get_all_bids<Base, Quote>(book: &OrderBook<Base, Quote>): vector<LimitOrder> {
        book.bids
    }

    /// Get all asks in the order book
    public fun get_all_asks<Base, Quote>(book: &OrderBook<Base, Quote>): vector<LimitOrder> {
        book.asks
    }

    /// Helper: get locked_amount for a bid at index
    public fun get_bid_locked_amount_at<Base, Quote>(
        book: &OrderBook<Base, Quote>,
        index: u64,
    ): u64 {
        let b = vector::borrow(&book.bids, index);
        b.locked_amount
    }

    /// Helper: get locked_amount for an ask at index
    public fun get_ask_locked_amount_at<Base, Quote>(
        book: &OrderBook<Base, Quote>,
        index: u64,
    ): u64 {
        let a = vector::borrow(&book.asks, index);
        a.locked_amount
    }

    /// Helper: return the order id (u64 counter) for bid at index
    public fun get_bid_id_at<Base, Quote>(book: &OrderBook<Base, Quote>, index: u64): u64 {
        let b = vector::borrow(&book.bids, index);
        b.id
    }

    /// Helper: return the order id (u64 counter) for ask at index
    public fun get_ask_id_at<Base, Quote>(book: &OrderBook<Base, Quote>, index: u64): u64 {
        let a = vector::borrow(&book.asks, index);
        a.id
    }

    // Per-level accessors: UIs (kanariapp) read the book level-by-level because
    // view-call results are BCS-decoded field-by-field; a whole `LimitOrder`
    // struct would come back as raw bytes.

    /// Helper: price for bid at index
    public fun get_bid_price_at<Base, Quote>(book: &OrderBook<Base, Quote>, index: u64): u64 {
        let b = vector::borrow(&book.bids, index);
        b.price
    }

    /// Helper: remaining (unfilled) quantity for bid at index
    public fun get_bid_remaining_at<Base, Quote>(book: &OrderBook<Base, Quote>, index: u64): u64 {
        let b = vector::borrow(&book.bids, index);
        b.quantity - b.filled
    }

    /// Helper: maker (owner) address for bid at index
    public fun get_bid_maker_at<Base, Quote>(book: &OrderBook<Base, Quote>, index: u64): address {
        let b = vector::borrow(&book.bids, index);
        b.maker
    }

    /// Helper: price for ask at index
    public fun get_ask_price_at<Base, Quote>(book: &OrderBook<Base, Quote>, index: u64): u64 {
        let a = vector::borrow(&book.asks, index);
        a.price
    }

    /// Helper: remaining (unfilled) quantity for ask at index
    public fun get_ask_remaining_at<Base, Quote>(book: &OrderBook<Base, Quote>, index: u64): u64 {
        let a = vector::borrow(&book.asks, index);
        a.quantity - a.filled
    }

    /// Helper: maker (owner) address for ask at index
    public fun get_ask_maker_at<Base, Quote>(book: &OrderBook<Base, Quote>, index: u64): address {
        let a = vector::borrow(&book.asks, index);
        a.maker
    }

    /// Withdraw base fee from the book (internal, used by registry)
    public fun withdraw_fee_base<Base, Quote>(
        book: &mut OrderBook<Base, Quote>,
        amount: u64,
        to: address,
        ctx: &mut TxContext,
    ) {
        if (amount > 0) {
            let base_part = balance::split(&mut book.fee_balance_base, amount);
            transfer::public_transfer(coin::from_balance(base_part, ctx), to);
        }
    }

    /// Withdraw quote fee from the book (internal, used by registry)
    public fun withdraw_fee_quote<Base, Quote>(
        book: &mut OrderBook<Base, Quote>,
        amount: u64,
        to: address,
        ctx: &mut TxContext,
    ) {
        if (amount > 0) {
            let quote_part = balance::split(&mut book.fee_balance_quote, amount);
            transfer::public_transfer(coin::from_balance(quote_part, ctx), to);
        }
    }

    /// Get a specific bid by index
    public fun get_bid_at<Base, Quote>(book: &OrderBook<Base, Quote>, index: u64): &LimitOrder {
        vector::borrow(&book.bids, index)
    }

    /// Get a specific ask by index
    public fun get_ask_at<Base, Quote>(book: &OrderBook<Base, Quote>, index: u64): &LimitOrder {
        vector::borrow(&book.asks, index)
    }

    #[test_only]
    /// Test-only destroy: consumes an OrderBook, destroying all balances and
    /// the underlying UID. Used by integration tests to avoid `drop` errors.
    public fun destroy_order_book_for_testing<Base, Quote>(
        book: OrderBook<Base, Quote>,
    ) {
        let OrderBook {
            id,
            bids: _,
            asks: _,
            base_balance,
            quote_balance,
            fee_balance_base,
            fee_balance_quote,
            ..
        } = book;
        let _b = kanari_system::balance::destroy_for_testing(base_balance);
        let _q = kanari_system::balance::destroy_for_testing(quote_balance);
        let _fb = kanari_system::balance::destroy_for_testing(fee_balance_base);
        let _fq = kanari_system::balance::destroy_for_testing(fee_balance_quote);
        object::delete(id);
        let _ = _b;
        let _ = _q;
        let _ = _fb;
        let _ = _fq;
    }
}
