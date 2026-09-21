// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0
module kanari_system::pay {
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::coin::{Self, Coin};
    use kanari_system::deny_list::{Self, DenyList};
    use kanari_system::transfer;
    use std::vector;

    /// For when empty vector is supplied into join function.
    const ENoCoins: u64 = 0;
    /// Recipient is on the deny list.
    const EDENIED: u64 = 1;
    /// Vector fan-out exceeds `coin::max_split_parts`.
    const ETOO_MANY_COINS: u64 = 2;
    /// Sending to the sender themselves: use `keep`/`split` instead.
    const ESELF_PAY: u64 = 3;
    /// Amount specified must be greater than zero.
    const EZERO_AMOUNT: u64 = 4;
    /// Division by zero is not allowed.
    const EDIVISION_BY_ZERO: u64 = 5;

    // #[allow(lint(self_transfer))]
    /// Transfer `c` to the sender of the current transaction
    public fun keep<T>(c: Coin<T>, ctx: &TxContext) {
        transfer::public_transfer(c, tx_context::sender(ctx))
    }

    /// Split coin `self` to two coins, one with balance `split_amount`,
    /// and the remaining balance is left is `self`.
    public entry fun split<T>(
        self: &mut Coin<T>, split_amount: u64, ctx: &mut TxContext
    ) {
        assert!(split_amount > 0, EZERO_AMOUNT);
        keep(coin::split(self, split_amount, ctx), ctx)
    }

    /// Split coin `self` into multiple coins, each with balance specified
    /// in `split_amounts`. Remaining balance is left in `self`.
    public entry fun split_vec<T>(
        self: &mut Coin<T>, split_amounts: vector<u64>, ctx: &mut TxContext
    ) {
        assert!(
            vector::length(&split_amounts) <= coin::max_split_parts(),
            ETOO_MANY_COINS
        );
        let (i, len) = (0, vector::length(&split_amounts));
        while (i < len) {
            split(self, *vector::borrow(&split_amounts, i), ctx);
            i = i + 1;
        };
    }

    /// Send `amount` units of `c` to `recipient`
    /// Aborts `ESELF_PAY` when `recipient` is the transaction sender; keep
    /// the coin with `keep`/`split` instead. Aborts `EZERO_AMOUNT` if `amount` is 0.
    public entry fun split_and_transfer<T>(
        c: &mut Coin<T>, amount: u64, recipient: address, ctx: &mut TxContext
    ) {
        assert!(amount > 0, EZERO_AMOUNT);
        assert!(recipient != tx_context::sender(ctx), ESELF_PAY);
        transfer::public_transfer(coin::split(c, amount, ctx), recipient)
    }

    /// Regulated send: like `split_and_transfer`, but aborts `EDENIED` when
    /// `recipient` is on the given deny list. Plain `split_and_transfer`
    /// deliberately skips the check (permissionless coin path).
    public entry fun split_and_transfer_checked<T>(
        c: &mut Coin<T>,
        amount: u64,
        recipient: address,
        deny: &DenyList,
        ctx: &mut TxContext
    ) {
        assert!(!deny_list::contains(deny, recipient), EDENIED);
        split_and_transfer(c, amount, recipient, ctx)
    }

    // #[allow(lint(self_transfer))]
    /// Divide coin `self` into `n` coins with equal balances. If the balance is
    /// not evenly divisible by `n`, the remainder is left in `self`.
    public entry fun divide_and_keep<T>(
        self: &mut Coin<T>, n: u64, ctx: &mut TxContext
    ) {
        assert!(n > 0, EDIVISION_BY_ZERO);
        let vec: vector<Coin<T>> = coin::divide_into_n(self, n, ctx);
        let (i, len) = (0, vector::length(&vec));
        while (i < len) {
            transfer::public_transfer(
                vector::pop_back(&mut vec), tx_context::sender(ctx)
            );
            i = i + 1;
        };
        vector::destroy_empty(vec);
    }

    /// Join `coin` into `self`. Re-exports `coin::join` function.
    public entry fun join<T>(self: &mut Coin<T>, coin: Coin<T>) {
        coin::join(self, coin)
    }

    /// Join everything in `coins` with `self`
    public entry fun join_vec<T>(self: &mut Coin<T>, coins: vector<Coin<T>>) {
        assert!(vector::length(&coins) <= coin::max_split_parts(), ETOO_MANY_COINS);
        let (i, len) = (0, vector::length(&coins));
        while (i < len) {
            let coin = vector::pop_back(&mut coins);
            coin::join(self, coin);
            i = i + 1
        };
        // safe because we've drained the vector
        vector::destroy_empty(coins)
    }

    /// Join a vector of `Coin` into a single object and transfer it to `receiver`.
    public entry fun join_vec_and_transfer<T>(
        coins: vector<Coin<T>>, receiver: address, ctx: &TxContext
    ) {
        assert!(receiver != tx_context::sender(ctx), ESELF_PAY);
        assert!(vector::length(&coins) > 0, ENoCoins);
        assert!(
            vector::length(&coins) <= coin::max_split_parts() + 1,
            ETOO_MANY_COINS
        );

        let self = vector::pop_back(&mut coins);
        join_vec(&mut self, coins);
        transfer::public_transfer(self, receiver)
    }
}

