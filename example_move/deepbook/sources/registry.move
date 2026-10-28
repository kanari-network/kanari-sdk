// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Global order book registry for DeepBook.
//!
//! Deduplicates order books per token pair via a blake2b256 hash of the sorted
//! (Base, Quote) type names, so `BASE/QUOTE` and `QUOTE/BASE` cannot both
//! exist. The registry owner can change book admins; book admins can withdraw
//! collected fees.

module deepbook::registry {
    use kanari_system::event;
    use kanari_system::object;
    use kanari_system::table::{Self, Table};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};
    use deepbook::deepbook::{OrderBook, create_order_book_with_decimals, book_id, withdraw_fee_base, withdraw_fee_quote};
    use std::hash;
    use std::type_name;

    // Error codes
    const E_ORDERBOOK_ALREADY_EXISTS: u64 = 8;
    const E_SAME_TOKEN_PAIR: u64 = 9;
    const E_UNAUTHORIZED: u64 = 5;
    const E_INVALID_FEE: u64 = 6;
    const E_INVALID_DEPTH: u64 = 7;

    /// Registry to prevent duplicate order books for the same token pair
    public struct GlobalOrderBookRegistry has key, store {
        id: object::UID,
        // Maps type-pair hash (blake2b256 of concatenated type names) to book address
        books: Table<vector<u8>, address>,
        // owner of the registry (can change book admins)
        owner: address,
        // Maps book address -> admin address (who can withdraw fees)
        admins: Table<address, address>,
    }

    // Events
    public struct OrderBookRegistryCreated has copy, drop {
        registry_id: address,
    }

    public struct FeesWithdrawn has copy, drop {
        book_id: address,
        to: address,
        base_amount: u64,
        quote_amount: u64,
    }

    public struct BookAdminChanged has copy, drop {
        registry_id: address,
        book_id: address,
        new_admin: address,
    }

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------

    /// Helper function to compare two vectors lexicographically (returns true if v1 <= v2)
    public fun compare_vectors(v1: &vector<u8>, v2: &vector<u8>): bool {
        let len1 = vector::length(v1);
        let len2 = vector::length(v2);
        let min_len = if (len1 < len2) { len1 } else { len2 };

        let mut i = 0;
        while (i < min_len) {
        let b1 = *vector::borrow(v1, i);
        let b2 = *vector::borrow(v2, i);
        if (b1 < b2) {
            return true
        } else if (b1 > b2) {
            return false
        };
        i = i + 1;
    };

        // If all bytes equal up to min_len, shorter vector is "less"
        len1 <= len2
    }

    /// Deterministic blake2b256 hash of the sorted (Base, Quote) type names, so
    /// `BASE/QUOTE` and `QUOTE/BASE` map to the same registry key.
    fun pair_hash<Base, Quote>(): vector<u8> {
        let ty_x = type_name::get_with_original_ids<Base>().into_string().into_bytes();
        let ty_y = type_name::get_with_original_ids<Quote>().into_string().into_bytes();

        // Prevent creating order book with same base and quote token
        assert!(ty_x != ty_y, E_SAME_TOKEN_PAIR);

        // Sort type names so pair order doesn't matter
        let (first, second) = if (compare_vectors(&ty_x, &ty_y)) {
        (ty_x, ty_y)
    } else {
        (ty_y, ty_x)
    };

        let mut concat = first;
        vector::append(&mut concat, second);
        hash::blake2b256(&concat)
    }

    // ---------------------------------------------------------------------------
    // Registry lifecycle
    // ---------------------------------------------------------------------------

    /// Create global registry for order books (call once during deployment)
    public fun create_global_registry(ctx: &mut TxContext): address {
        let registry = GlobalOrderBookRegistry {
        id: object::new(ctx),
        books: table::new(ctx),
        owner: tx_context::sender(ctx),
        admins: table::new(ctx),
    };

        let registry_id = object::uid_address(&registry.id);
        event::emit(OrderBookRegistryCreated { registry_id });
        transfer::share_object(registry);
        registry_id
    }

    // ---------------------------------------------------------------------------
    // Book creation with registry
    // ---------------------------------------------------------------------------

    /// Check if an order book exists for the given type pair (in any order)
    public fun book_exists<Base, Quote>(registry: &GlobalOrderBookRegistry): bool {
        table::contains(&registry.books, pair_hash<Base, Quote>())
    }

    /// Get order book address for a given type pair (returns None if not exists, works with any order)
    public fun get_book_address<Base, Quote>(registry: &GlobalOrderBookRegistry): Option<address> {
        let pair_hash = pair_hash<Base, Quote>();
        if (table::contains(&registry.books, pair_hash)) {
        option::some(*table::borrow(&registry.books, pair_hash))
    } else {
        option::none<address>()
    }
    }

    /// Create an order book and register it in the provided registry to prevent duplicates
    public fun create_order_book_with_registry_with_decimals<Base, Quote>(
        registry: &mut GlobalOrderBookRegistry,
        fee_bps: u64,
        max_depth: u64,
        base_decimals: u8,
        quote_decimals: u8,
        ctx: &mut TxContext,
    ) {
        assert!(fee_bps <= 1000, E_INVALID_FEE); // max 10%
        assert!(max_depth > 0, E_INVALID_DEPTH);
        assert!(max_depth <= MAX_DEPTH, E_INVALID_DEPTH);

        let pair_hash = pair_hash<Base, Quote>();
        assert!(!table::contains(&registry.books, pair_hash), E_ORDERBOOK_ALREADY_EXISTS);

        let book_id = create_order_book_with_decimals<Base, Quote>(fee_bps, max_depth, base_decimals, quote_decimals, ctx);

        // Register book in registry
        table::add(&mut registry.books, pair_hash, book_id);
        // Set the creator as the admin for this book
        table::add(&mut registry.admins, book_id, tx_context::sender(ctx));
    }

    /// Factory: return existing book address for pair or create+register a new one.
    /// Not an entry function: Move entry functions cannot return values; callers
    /// that only need "ensure exists" can ignore the result in a wrapper.
    public fun get_or_create_order_book_with_decimals<Base, Quote>(
        registry: &mut GlobalOrderBookRegistry,
        fee_bps: u64,
        max_depth: u64,
        base_decimals: u8,
        quote_decimals: u8,
        ctx: &mut TxContext,
    ): address {
        let pair_hash = pair_hash<Base, Quote>();

        // If exists, return existing address
        if (table::contains(&registry.books, pair_hash)) {
        *table::borrow(&registry.books, pair_hash)
    } else {
        // create and register
        assert!(fee_bps <= 1000, E_INVALID_FEE);
        assert!(max_depth > 0, E_INVALID_DEPTH);
        assert!(max_depth <= MAX_DEPTH, E_INVALID_DEPTH);

        let book_id = create_order_book_with_decimals<Base, Quote>(fee_bps, max_depth, base_decimals, quote_decimals, ctx);
        table::add(&mut registry.books, pair_hash, book_id);
        // set creator as admin
        table::add(&mut registry.admins, book_id, tx_context::sender(ctx));
        book_id
    }
    }

    // ---------------------------------------------------------------------------
    // Registry administration
    // ---------------------------------------------------------------------------

    /// Get admin for a book from the registry (returns None if not set)
    public fun get_book_admin(
        registry: &GlobalOrderBookRegistry,
        book_addr: address,
    ): Option<address> {
        if (table::contains(&registry.admins, book_addr)) {
        option::some(*table::borrow(&registry.admins, book_addr))
    } else {
        option::none<address>()
    }
    }

    /// Change book admin (only registry owner can call)
    public entry fun set_book_admin(
        registry: &mut GlobalOrderBookRegistry,
        book_addr: address,
        new_admin: address,
        ctx: &mut TxContext,
    ) {
        let caller = tx_context::sender(ctx);
        assert!(caller == registry.owner, E_UNAUTHORIZED);

        if (table::contains(&registry.admins, book_addr)) {
        table::remove(&mut registry.admins, book_addr);
    };
        table::add(&mut registry.admins, book_addr, new_admin);

        event::emit(BookAdminChanged {
        registry_id: object::uid_address(&registry.id),
        book_id: book_addr,
        new_admin,
    });
    }

    /// Withdraw collected fees (only callable by book admin as set in registry)
    public entry fun withdraw_fees<Base, Quote>(
        registry: &mut GlobalOrderBookRegistry,
        book: &mut OrderBook<Base, Quote>,
        to: address,
        base_amount: u64,
        quote_amount: u64,
        ctx: &mut TxContext,
    ) {
        let caller = tx_context::sender(ctx);
        let book_addr = book_id(book);
        assert!(table::contains(&registry.admins, book_addr), E_UNAUTHORIZED);
        let admin = *table::borrow(&registry.admins, book_addr);
        assert!(caller == admin, E_UNAUTHORIZED);

        withdraw_fee_base<Base, Quote>(book, base_amount, to, ctx);
        withdraw_fee_quote<Base, Quote>(book, quote_amount, to, ctx);

        event::emit(FeesWithdrawn {
        book_id: book_addr,
        to,
        base_amount,
        quote_amount,
    });
    }

    // ---------------------------------------------------------------------------
    // Constants (shared with deepbook module)
    // ---------------------------------------------------------------------------
    const MAX_DEPTH: u64 = 1_000_000;
}
