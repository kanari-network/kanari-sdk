module tester::deny_enforcement {
    use kanari_system::coin::{Self, Coin};
    use kanari_system::deny_list::{DenyCap, DenyList};
    use kanari_system::object;
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    /// Regulated test coin; this package exists solely for runtime tests
    /// and is never deployed.
    struct MOCK has drop {}

    /// Create a regulated currency, mint 1000 to the sender, and hand the
    /// mint/deny caps to the sender for later transactions.
    public entry fun setup(ctx: &mut TxContext) {
        let (cap, denycap, meta) = coin::create_regulated_currency(
            MOCK {},
            6,
            b"MOCK",
            b"Mock coin",
            b"deny enforcement e2e",
            std::option::none(),
            ctx
        );
        let sender = tx_context::sender(ctx);
        let minted = coin::mint(&mut cap, 1000, ctx);
        transfer::public_transfer(minted, sender);
        transfer::public_transfer(cap, sender);
        transfer::public_transfer(denycap, sender);
        transfer::public_freeze_object(meta);
    }

    /// Deny `addr` for MOCK on the shared list (resolved by id, like the
    /// multisig entry points do for off-path objects).
    public entry fun deny(
        deny_id: address,
        cap: &DenyCap<MOCK>,
        addr: address,
        ctx: &mut TxContext,
    ) {
        let list = object::borrow_global_mut<DenyList>(deny_id);
        coin::deny_list_add(list, cap, addr, ctx);
        object::save_object(list);
    }

    /// Undeny `addr` for MOCK.
    public entry fun undeny(
        deny_id: address,
        cap: &DenyCap<MOCK>,
        addr: address,
        ctx: &mut TxContext,
    ) {
        let list = object::borrow_global_mut<DenyList>(deny_id);
        coin::deny_list_remove(list, cap, addr, ctx);
        object::save_object(list);
    }

    /// Pause all regulated MOCK flows.
    public entry fun pause(
        deny_id: address,
        cap: &DenyCap<MOCK>,
        ctx: &mut TxContext,
    ) {
        let list = object::borrow_global_mut<DenyList>(deny_id);
        coin::deny_list_pause(list, cap, ctx);
        object::save_object(list);
    }

    /// Lift the MOCK pause.
    public entry fun unpause(
        deny_id: address,
        cap: &DenyCap<MOCK>,
        ctx: &mut TxContext,
    ) {
        let list = object::borrow_global_mut<DenyList>(deny_id);
        coin::deny_list_unpause(list, cap, ctx);
        object::save_object(list);
    }

    /// Deny `recipient` and send to them in the SAME transaction. The
    /// runtime reads pending deny writes, so this must abort — and abort
    /// atomically (the deny write itself must not persist either).
    public entry fun deny_and_send(
        deny_id: address,
        cap: &DenyCap<MOCK>,
        c: &mut Coin<MOCK>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext,
    ) {
        let list = object::borrow_global_mut<DenyList>(deny_id);
        coin::deny_list_add(list, cap, recipient, ctx);
        object::save_object(list);
        transfer::public_transfer(coin::split(c, amount, ctx), recipient);
    }
}
