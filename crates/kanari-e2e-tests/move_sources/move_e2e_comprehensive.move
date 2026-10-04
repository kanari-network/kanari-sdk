module kanari_e2e_tests::move_e2e_comprehensive {
    use kanari_system::balance::{Self, Balance};
    use kanari_system::coin::{Self, Coin, TreasuryCap};
    use kanari_system::event;
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};
    use std::option;
    use std::string::{Self, String};

    struct COMP_COIN has drop {}

    struct RES_COIN has drop {}

    struct Vault has key {
        id: UID,
        balance: Balance<COMP_COIN>,
        owner: address,
        name: String
    }

    struct DepositEvent has copy, drop {
        vault_id: address,
        depositor: address,
        amount: u64
    }

    struct WithdrawEvent has copy, drop {
        vault_id: address,
        withdrawer: address,
        amount: u64
    }

    public fun init_comp_coin(ctx: &mut TxContext) {
        let (treasury, metadata) =
            coin::create_currency<COMP_COIN>(
                COMP_COIN {},
                9,
                b"COMP",
                b"Comp Coin",
                b"Comprehensive test coin",
                option::none(),
                ctx
            );
        let sender = tx_context::sender(ctx);
        transfer::public_share_object(treasury);
        transfer::public_share_object(metadata);
        transfer::public_transfer(
            coin::mint<COMP_COIN>(&mut treasury, 2_000_000_000_000, ctx),
            sender
        );
    }

    public fun init_res_coin(ctx: &mut TxContext) {
        let (treasury, metadata) =
            coin::create_currency<RES_COIN>(
                RES_COIN {},
                6,
                b"RES",
                b"Res Coin",
                b"Resource test coin",
                option::none(),
                ctx
            );
        let sender = tx_context::sender(ctx);
        transfer::public_share_object(treasury);
        transfer::public_share_object(metadata);
        transfer::public_transfer(
            coin::mint<RES_COIN>(&mut treasury, 500_000_000, ctx),
            sender
        );
    }

    public fun create_vault(name: vector<u8>, ctx: &mut TxContext): address {
        let vault = Vault {
            id: object::new(ctx),
            balance: balance::zero<COMP_COIN>(),
            owner: tx_context::sender(ctx),
            name: string::utf8(name)
        };
        let vault_id = object::uid_to_address(&vault.id);
        transfer::share_object(vault);
        vault_id
    }

    public fun deposit(
        vault: &mut Vault, coin: Coin<COMP_COIN>, ctx: &mut TxContext
    ) {
        let amount = coin::value(&coin);
        let balance_coin = coin::into_balance(coin);
        balance::join(&mut vault.balance, balance_coin);
        event::emit(
            DepositEvent {
                vault_id: object::uid_to_address(&vault.id),
                depositor: tx_context::sender(ctx),
                amount
            }
        );
    }

    public fun withdraw(vault: &mut Vault, amount: u64, ctx: &mut TxContext)
        : Coin<COMP_COIN> {
        assert!(vault.owner == tx_context::sender(ctx), 100);
        assert!(balance::value(&vault.balance) >= amount, 101);
        let withdrawn = coin::take(&mut vault.balance, amount, ctx);
        event::emit(
            WithdrawEvent {
                vault_id: object::uid_to_address(&vault.id),
                withdrawer: tx_context::sender(ctx),
                amount
            }
        );
        withdrawn
    }

    public fun get_balance(vault: &Vault): u64 {
        balance::value(&vault.balance)
    }

    public fun get_vault_name(vault: &Vault): &String {
        &vault.name
    }
}

