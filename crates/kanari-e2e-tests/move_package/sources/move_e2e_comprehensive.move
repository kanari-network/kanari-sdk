module kanari_e2e_tests::move_e2e_comprehensive {
    use std::string::{Self, String};
    use kanari_system::balance::{Self, Balance};
    use kanari_system::coin::{Self, Coin, CoinMetadata};
    use kanari_system::event;
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::url;

    const E_NOT_OWNER: u64 = 100;
    const E_INSUFFICIENT: u64 = 101;
    const E_BAD_BALANCE: u64 = 102;
    const E_BAD_MINT: u64 = 103;

    public struct COMP_COIN has drop {}

    /// Shared vault holding a single coin type.
    public struct Vault has key, store {
        id: UID,
        balance: Balance<COMP_COIN>,
        owner: address,
        label: String
    }

    /// Emitted on every successful deposit.
    public struct DepositEvent has copy, drop {
        vault: address,
        depositor: address,
        amount: u64
    }

    /// Emitted on every successful withdrawal.
    public struct WithdrawEvent has copy, drop {
        vault: address,
        withdrawer: address,
        amount: u64
    }

    /// Create the COMP_COIN currency and fund the sender.
    public entry fun init_comp_coin(ctx: &mut TxContext) {
        let (mut treasury_cap, metadata) = e2e_create(ctx);
        let sender = tx_context::sender(ctx);
        let minted = coin::mint<COMP_COIN>(&mut treasury_cap, 2000000000000, ctx);
        transfer::share_object(treasury_cap);
        transfer::share_object(metadata);
        transfer::public_transfer(minted, sender);
    }

    /// Create a shared vault with a human-readable label.
    public fun create_vault(label: vector<u8>, ctx: &mut TxContext): address {
        let vault = e2e_new_vault(label, ctx);
        let addr = object::uid_address(&vault.id);
        transfer::share_object(vault);
        addr
    }

    /// Deposit coins into the vault and emit a deposit event.
    public entry fun deposit(
        vault: &mut Vault, payment: Coin<COMP_COIN>, ctx: &mut TxContext
    ) {
        let amount = coin::value(&payment);
        balance::merge(&mut vault.balance, coin::into_balance(payment));
        event::emit(
            DepositEvent {
                vault: object::uid_address(&vault.id),
                depositor: tx_context::sender(ctx),
                amount
            }
        );
    }

    /// Owner-only withdrawal that emits a withdraw event.
    public fun withdraw(vault: &mut Vault, amount: u64, ctx: &mut TxContext)
        : Coin<COMP_COIN> {
        let sender = tx_context::sender(ctx);
        assert!(vault.owner == sender, E_NOT_OWNER);
        assert!(balance::value(&vault.balance) >= amount, E_INSUFFICIENT);
        let withdrawn = balance::split(&mut vault.balance, amount);
        event::emit(
            WithdrawEvent {
                vault: object::uid_address(&vault.id),
                withdrawer: sender,
                amount
            }
        );
        coin::from_balance(withdrawn, ctx)
    }

    public fun vault_balance(vault: &Vault): u64 {
        balance::value(&vault.balance)
    }

    public fun vault_label(vault: &Vault): &String {
        &vault.label
    }

    public fun vault_owner(vault: &Vault): address {
        vault.owner
    }

    /// Creates the COMP_COIN currency with the canonical E2E parameters.
    fun e2e_create(ctx: &mut TxContext): (coin::TreasuryCap<COMP_COIN>, CoinMetadata<COMP_COIN>) {
        coin::create_currency<COMP_COIN>(
            COMP_COIN {},
            9,
            b"COMP",
            b"Comp Coin",
            b"Kanari E2E comprehensive test coin",
            option::none<url::Url>(),
            ctx
        )
    }

    /// Builds an unshared vault owned by the transaction sender.
    fun e2e_new_vault(label: vector<u8>, ctx: &mut TxContext): Vault {
        Vault {
            id: object::new(ctx),
            balance: balance::zero<COMP_COIN>(),
            owner: tx_context::sender(ctx),
            label: string::utf8(label)
        }
    }

    /// Entry wrapper for `create_vault`.
    public entry fun e2e_create_vault(label: vector<u8>, ctx: &mut TxContext) {
        create_vault(label, ctx);
    }

    /// Entry wrapper for `withdraw` that forwards the coin to the sender.
    public entry fun e2e_withdraw(
        vault: &mut Vault, amount: u64, ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        let taken = withdraw(vault, amount, ctx);
        transfer::public_transfer(taken, sender);
    }

    /// Entry wrapper asserting the vault balance.
    public entry fun e2e_check_vault_balance(vault: &Vault, expected: u64) {
        assert!(vault_balance(vault) == expected, E_BAD_BALANCE);
    }

    /// Entry wrapper asserting the vault owner.
    public entry fun e2e_check_vault_owner(vault: &Vault, expected: address) {
        assert!(vault_owner(vault) == expected, E_BAD_BALANCE);
    }

    /// Full deposit/withdraw round trip, asserted inside a single transaction.
    public entry fun e2e_vault_roundtrip(ctx: &mut TxContext) {
        let (mut treasury_cap, metadata) = e2e_create(ctx);
        let sender = tx_context::sender(ctx);
        let mut vault = e2e_new_vault(b"roundtrip", ctx);

        assert!(vault_balance(&vault) == 0, E_BAD_BALANCE);

        let payment = coin::mint<COMP_COIN>(&mut treasury_cap, 1000, ctx);
        deposit(&mut vault, payment, ctx);
        assert!(vault_balance(&vault) == 1000, E_BAD_BALANCE);

        // A second deposit accumulates on top of the first.
        let top_up = coin::mint<COMP_COIN>(&mut treasury_cap, 500, ctx);
        deposit(&mut vault, top_up, ctx);
        assert!(vault_balance(&vault) == 1500, E_BAD_BALANCE);

        let taken = withdraw(&mut vault, 600, ctx);
        assert!(coin::value(&taken) == 600, E_BAD_MINT);
        assert!(vault_balance(&vault) == 900, E_BAD_BALANCE);

        // Draining the vault must leave it empty, not negative.
        let rest = withdraw(&mut vault, 900, ctx);
        assert!(coin::value(&rest) == 900, E_BAD_MINT);
        assert!(vault_balance(&vault) == 0, E_BAD_BALANCE);

        coin::destroy_zero(coin::zero<COMP_COIN>(ctx));

        transfer::share_object(treasury_cap);
        transfer::share_object(metadata);
        transfer::share_object(vault);
        transfer::public_transfer(taken, sender);
        transfer::public_transfer(rest, sender);
    }
}