// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Example module demonstrating KRC-1155 semi-fungible token usage.
///
/// This module shows how to build a game-item system on top of
/// `kanari_system::krc_1155`. Each item type (e.g., "Health Potion",
/// "Iron Sword") is a KRC-1155 `TokenType` with its own max supply,
/// and each player's holdings are `Balance` objects they own directly.
module game_items::game_items {

    use std::string::{Self, String};
    use kanari_system::event;
    use kanari_system::krc_1155::{Self, Balance, TokenCap, TokenType};
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    /// A catalog entry — shared discovery pointer to a KRC-1155 token type.
    public struct ItemCatalog has key, store {
        id: UID,
        name: String,
        token_type: address,
        max_supply: u64
    }

    /// Player inventory — tracks which item types a player owns.
    public struct Inventory has key, store {
        id: UID,
        owner: address
    }

    /// Create a new game item type and register it in the catalog.
    /// The token type is shared for discovery; the TokenCap is transferred
    /// to the sender for mint/burn authority. ItemCatalog is also transferred
    /// to the sender for discovery.
    public entry fun create_item_type(
        name: vector<u8>,
        description: vector<u8>,
        image_url: vector<u8>,
        max_supply: u64,
        ctx: &mut TxContext
    ) {
        let (token, cap) = krc_1155::create(
            name,
            description,
            image_url,
            max_supply,
            ctx
        );
        let token_type = krc_1155::token_type(&token);
        let catalog = ItemCatalog {
            id: object::new(ctx),
            name: string::utf8(name),
            token_type,
            max_supply
        };
        // Share token type so anyone can discover it (read-only).
        krc_1155::share(token);
        // Transfer ownership of catalog and cap to the sender.
        let sender = tx_context::sender(ctx);
        transfer::public_transfer(catalog, sender);
        transfer::public_transfer(cap, sender);
    }

    /// Get or create a player's inventory object.
    public fun get_or_create_inventory(
        owner: address,
        ctx: &mut TxContext
    ): Inventory {
        Inventory {
            id: object::new(ctx),
            owner
        }
    }

    /// Mint `amount` of `item_type` directly to `recipient`.
    /// Requires the `TokenCap` for that item type.
    public entry fun mint_to_player(
        token: &mut TokenType,
        cap: &TokenCap,
        recipient: address,
        amount: u64,
        ctx: &mut TxContext
    ) {
        krc_1155::mint_to(token, cap, amount, recipient, ctx);
    }

    /// Transfer `amount` of an item type from sender to `recipient`.
    /// Sender must hold a `Balance` of that item type.
    public entry fun transfer_item(
        balance: &mut Balance,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        assert!(amount > 0, 0);
        let token_type = krc_1155::balance_token_type(balance);
        krc_1155::transfer(balance, amount, recipient, ctx);
        event::emit(
            ItemMoved {
                item_type: token_type,
                sender,
                recipient,
                amount,
                timestamp: tx_context::epoch_timestamp_ms(ctx)
            }
        );
    }

    /// Split `amount` off a player's balance, returning a new Balance object.
    public fun split_item(
        balance: &mut Balance,
        amount: u64,
        ctx: &mut TxContext
    ): Balance {
        krc_1155::split(balance, amount, ctx)
    }

    /// Merge two balances of the same item type into one.
    public fun join_items(
        primary: &mut Balance,
        other: Balance
    ) {
        krc_1155::join(primary, other);
    }

    /// Burn `amount` from a balance, reducing total supply.
    /// Requires the `TokenCap` for that item type.
    public entry fun burn_item(
        token: &mut TokenType,
        cap: &TokenCap,
        balance: Balance,
        ctx: &TxContext
    ) {
        let amount = krc_1155::balance_value(&balance);
        let item_type = krc_1155::token_type(token);
        krc_1155::burn(token, cap, balance, ctx);
        event::emit(
            ItemBurned {
                item_type,
                owner: tx_context::sender(ctx),
                amount,
                timestamp: tx_context::epoch_timestamp_ms(ctx)
            }
        );
    }

    /// Read-only helpers for UI / indexer consumption.
    public fun item_name(catalog: &ItemCatalog): String {
        catalog.name
    }

    /// Returns the token type address of the item.
    public fun item_type(catalog: &ItemCatalog): address {
        catalog.token_type
    }

    /// Returns the maximum supply of the item.
    public fun item_max_supply(catalog: &ItemCatalog): u64 {
        catalog.max_supply
    }

    /// Returns the total supply of the item token type.
    public fun item_total_supply(token: &TokenType): u64 {
        krc_1155::total_supply(token)
    }

    /// Returns the balance amount of the item.
    public fun item_balance_amount(balance: &Balance): u64 {
        krc_1155::balance_value(balance)
    }

    /// Returns the token type of the balance.
    public fun item_balance_token_type(balance: &Balance): address {
        krc_1155::balance_token_type(balance)
    }

    /// Events for indexer / frontend consumption.
    public struct ItemMoved has copy, drop {
        item_type: address,
        sender: address,
        recipient: address,
        amount: u64,
        timestamp: u64
    }

    public struct ItemBurned has copy, drop {
        item_type: address,
        owner: address,
        amount: u64,
        timestamp: u64
    }

    #[test]
    fun test_item_lifecycle() {
        let mut ctx = tx_context::dummy();

        // 1. Create an item type: 1000 Health Potions
        create_item_type(
            b"Health Potion",
            b"Restores 50 HP",
            b"https://cdn.example.com/potion.png",
            1000,
            &mut ctx
        );
        // Token type is shared; catalog and cap are transferred to sender.
        // In tests, we verify the function doesn't abort.
    }

    #[test]
    fun test_mint_split_join_and_burn_conserves_supply() {
        let mut ctx = tx_context::dummy();

        // Create item type directly (token stays in scope for the test)
        let (mut token, cap) = krc_1155::create(
            b"Health Potion",
            b"Restores 50 HP",
            b"https://cdn.example.com/potion.png",
            1000,
            &mut ctx
        );
        assert!(krc_1155::total_supply(&token) == 0, 0);

        // Mint 100 potions
        let mut alice_balance = krc_1155::mint(&mut token, &cap, 100, &mut ctx);
        assert!(krc_1155::balance_value(&alice_balance) == 100, 1);
        assert!(krc_1155::total_supply(&token) == 100, 2);

        // Split off 30
        let split_off = krc_1155::split(&mut alice_balance, 30, &mut ctx);
        assert!(krc_1155::balance_value(&alice_balance) == 70, 3);
        assert!(krc_1155::balance_value(&split_off) == 30, 4);

        // Join split_off back into alice_balance
        krc_1155::join(&mut alice_balance, split_off);
        assert!(krc_1155::balance_value(&alice_balance) == 100, 5);

        // Split 50 off, then burn that 50
        let burn_balance = krc_1155::split(&mut alice_balance, 50, &mut ctx);
        assert!(krc_1155::balance_value(&alice_balance) == 50, 5);
        krc_1155::burn(&mut token, &cap, burn_balance, &ctx);
        assert!(krc_1155::total_supply(&token) == 50, 6);

        transfer::public_freeze_object(token);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(alice_balance);
    }

}
