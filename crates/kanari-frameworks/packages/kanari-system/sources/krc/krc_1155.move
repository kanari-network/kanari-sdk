// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// KRC-1155 is Kanari's semi-fungible token standard.
///
/// Each `TokenType` has an independent supply cap. Holder value is represented
/// by owned `Balance` objects, which can be split, joined, transferred and
/// burned without a global address-to-balance map. This preserves the object
/// ownership model while supporting many indistinguishable copies per type.
module kanari_system::krc_1155 {
    use std::string::{Self, String};
    use kanari_system::event;
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::url::{Self, Url};

    const E_TOKEN_CAP_MISMATCH: u64 = 0;
    const E_BALANCE_TYPE_MISMATCH: u64 = 1;
    const E_INSUFFICIENT_BALANCE: u64 = 2;
    const E_ZERO_AMOUNT: u64 = 3;
    const E_SUPPLY_OVERFLOW: u64 = 4;
    const E_SUPPLY_CAP_EXCEEDED: u64 = 5;

    /// Public metadata and supply accounting for one semi-fungible token type.
    struct TokenType has key, store {
        id: UID,
        name: String,
        description: String,
        image_url: Url,
        max_supply: u64,
        total_supply: u64,
    }

    /// Mint and burn authority for exactly one `TokenType`.
    struct TokenCap has key, store {
        id: UID,
        token_type: address,
    }

    /// A holder-owned quantity of one KRC-1155 token type.
    struct Balance has key, store {
        id: UID,
        token_type: address,
        amount: u64,
    }

    struct Minted has copy, drop {
        token_type: address,
        recipient: address,
        amount: u64,
    }

    struct Burned has copy, drop {
        token_type: address,
        owner: address,
        amount: u64,
    }

    struct Transferred has copy, drop {
        token_type: address,
        sender: address,
        recipient: address,
        amount: u64,
    }

    /// Create a token type and its issuer capability. The caller may share the
    /// token type for discovery, then retain or transfer `TokenCap`.
    public fun create(
        name: vector<u8>,
        description: vector<u8>,
        image_url: vector<u8>,
        max_supply: u64,
        ctx: &mut TxContext,
    ): (TokenType, TokenCap) {
        let id = object::new(ctx);
        let token_type = object::uid_address(&id);
        let token = TokenType {
            id,
            name: string::utf8(name),
            description: string::utf8(description),
            image_url: url::new_unsafe_from_bytes(image_url),
            max_supply,
            total_supply: 0,
        };
        let cap = TokenCap { id: object::new(ctx), token_type };
        (token, cap)
    }

    /// Share immutable-discovery state. Supply remains mutable through a
    /// shared `&mut TokenType` input to `mint` and `burn`.
    public fun share(token: TokenType) {
        transfer::share_object(token)
    }

    /// Mint `amount` units. Both the token and cap must identify the same type.
    public fun mint(
        token: &mut TokenType,
        cap: &TokenCap,
        amount: u64,
        ctx: &mut TxContext,
    ): Balance {
        assert!(amount > 0, E_ZERO_AMOUNT);
        let token_type = object::uid_address(&token.id);
        assert!(cap.token_type == token_type, E_TOKEN_CAP_MISMATCH);
        let next_supply = token.total_supply + amount;
        assert!(next_supply >= token.total_supply, E_SUPPLY_OVERFLOW);
        assert!(next_supply <= token.max_supply, E_SUPPLY_CAP_EXCEEDED);
        token.total_supply = next_supply;
        object::save_object(token);
        Balance { id: object::new(ctx), token_type, amount }
    }

    /// Mint and transfer directly to `recipient`, with an indexer event.
    public entry fun mint_to(
        token: &mut TokenType,
        cap: &TokenCap,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext,
    ) {
        let balance = mint(token, cap, amount, ctx);
        event::emit(Minted { token_type: balance.token_type, recipient, amount });
        transfer::public_transfer(balance, recipient);
    }

    /// Split a holder balance. The returned object has `amount`; the original
    /// retains the remainder.
    public fun split(balance: &mut Balance, amount: u64, ctx: &mut TxContext): Balance {
        assert!(amount > 0, E_ZERO_AMOUNT);
        assert!(balance.amount >= amount, E_INSUFFICIENT_BALANCE);
        balance.amount = balance.amount - amount;
        object::save_object(balance);
        Balance { id: object::new(ctx), token_type: balance.token_type, amount }
    }

    /// Transfer part of a holder balance and emit the canonical KRC-1155 event.
    public entry fun transfer(
        balance: &mut Balance,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext,
    ) {
        let token_type = balance.token_type;
        let sender = tx_context::sender(ctx);
        let sent = split(balance, amount, ctx);
        event::emit(Transferred { token_type, sender, recipient, amount });
        transfer::public_transfer(sent, recipient);
    }

    /// Merge two balances of the same token type.
    public fun join(balance: &mut Balance, other: Balance) {
        let Balance { id, token_type, amount } = other;
        assert!(balance.token_type == token_type, E_BALANCE_TYPE_MISMATCH);
        let next_amount = balance.amount + amount;
        assert!(next_amount >= balance.amount, E_SUPPLY_OVERFLOW);
        balance.amount = next_amount;
        object::save_object(balance);
        object::delete(id);
    }

    /// Burn a balance and lower the token type's total supply.
    public fun burn(token: &mut TokenType, cap: &TokenCap, balance: Balance, ctx: &TxContext) {
        let Balance { id, token_type, amount } = balance;
        assert!(cap.token_type == token_type, E_TOKEN_CAP_MISMATCH);
        assert!(object::uid_address(&token.id) == token_type, E_TOKEN_CAP_MISMATCH);
        assert!(token.total_supply >= amount, E_SUPPLY_OVERFLOW);
        token.total_supply = token.total_supply - amount;
        object::save_object(token);
        event::emit(Burned { token_type, owner: tx_context::sender(ctx), amount });
        object::delete(id);
    }

    public fun token_type(token: &TokenType): address { object::uid_address(&token.id) }
    public fun balance_token_type(balance: &Balance): address { balance.token_type }
    public fun balance_value(balance: &Balance): u64 { balance.amount }
    public fun total_supply(token: &TokenType): u64 { token.total_supply }
    public fun max_supply(token: &TokenType): u64 { token.max_supply }
    public fun name(token: &TokenType): String { token.name }
    public fun description(token: &TokenType): String { token.description }
    public fun image_url(token: &TokenType): Url { token.image_url }

    #[test]
    fun test_mint_split_join_and_burn_conserves_supply() {
        let ctx = tx_context::dummy();
        let (token, cap) = create(b"Tickets", b"", b"https://image", 100, &mut ctx);
        let balance = mint(&mut token, &cap, 40, &mut ctx);
        let part = split(&mut balance, 15, &mut ctx);
        assert!(balance_value(&balance) == 25, 0);
        join(&mut balance, part);
        assert!(balance_value(&balance) == 40, 1);
        burn(&mut token, &cap, balance, &ctx);
        assert!(total_supply(&token) == 0, 2);
        transfer::public_freeze_object(token);
        transfer::public_freeze_object(cap);
    }
}
