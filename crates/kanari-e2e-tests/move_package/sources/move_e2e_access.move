module kanari_e2e_tests::move_e2e_access {
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    const E_NOT_OWNER: u64 = 200;
    const E_NOT_ADMIN: u64 = 201;
    const E_BAD_COUNT: u64 = 202;

    /// Shared access-control object: an owner plus a mutable admin list.
    public struct AccessControl has key, store {
        id: UID,
        owner: address,
        admins: vector<address>,
        open: bool
    }

    /// Owned resource guarded by the access-control object.
    public struct Resource has key, store {
        id: UID,
        value: u64,
        metadata: vector<u8>
    }

    /// Create a shared access-control object and return its address.
    public fun create_access_control(open: bool, ctx: &mut TxContext): address {
        let control = e2e_new_control(open, ctx);
        let addr = object::uid_address(&control.id);
        transfer::share_object(control);
        addr
    }

    /// Owner-only: append an admin address.
    public entry fun add_admin(
        control: &mut AccessControl, admin: address, ctx: &mut TxContext
    ) {
        assert!(control.owner == tx_context::sender(ctx), E_NOT_OWNER);
        vector::push_back(&mut control.admins, admin);
    }

    /// Owner-only: remove an admin address if present.
    public entry fun remove_admin(
        control: &mut AccessControl, admin: address, ctx: &mut TxContext
    ) {
        assert!(control.owner == tx_context::sender(ctx), E_NOT_OWNER);
        let mut pos = vector::length(&control.admins);
        while (pos > 0) {
            pos = pos - 1;
            if (*vector::borrow(&control.admins, pos) == admin) {
                vector::remove(&mut control.admins, pos);
                return
            };
        };
    }

    /// Membership check over the admin list.
    public fun is_admin(control: &AccessControl, addr: address): bool {
        let mut pos = vector::length(&control.admins);
        while (pos > 0) {
            pos = pos - 1;
            if (*vector::borrow(&control.admins, pos) == addr) {
                return true
            };
        };
        false
    }

    /// Create a shared guarded resource and return its address.
    public fun create_resource(
        value: u64, metadata: vector<u8>, ctx: &mut TxContext
    ): address {
        let res = e2e_new_resource(value, metadata, ctx);
        let addr = object::uid_address(&res.id);
        transfer::share_object(res);
        addr
    }

    /// Owner or admin or open: mutate a guarded resource.
    public entry fun update_resource(
        control: &AccessControl,
        res: &mut Resource,
        new_value: u64,
        new_metadata: vector<u8>,
        ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        assert!(
            control.open
                || control.owner == sender
                || is_admin(control, sender),
            E_NOT_ADMIN
        );
        res.value = new_value;
        res.metadata = new_metadata;
    }

    public fun resource_value(res: &Resource): u64 {
        res.value
    }

    public fun resource_metadata(res: &Resource): &vector<u8> {
        &res.metadata
    }

    public fun admin_count(control: &AccessControl): u64 {
        vector::length(&control.admins)
    }

    /// Builds an unshared access-control object owned by the sender.
    fun e2e_new_control(open: bool, ctx: &mut TxContext): AccessControl {
        AccessControl {
            id: object::new(ctx),
            owner: tx_context::sender(ctx),
            admins: vector::empty<address>(),
            open
        }
    }

    /// Builds an unshared guarded resource.
    fun e2e_new_resource(
        value: u64, metadata: vector<u8>, ctx: &mut TxContext
    ): Resource {
        Resource { id: object::new(ctx), value, metadata }
    }

    /// Entry wrapper for `create_access_control`.
    public entry fun e2e_create_access_control(
        open: bool, ctx: &mut TxContext
    ) {
        create_access_control(open, ctx);
    }

    /// Entry wrapper for `create_resource`.
    public entry fun e2e_create_resource(
        value: u64, metadata: vector<u8>, ctx: &mut TxContext
    ) {
        create_resource(value, metadata, ctx);
    }

    /// Entry wrapper asserting the admin-list size.
    public entry fun e2e_check_admin_count(control: &AccessControl, expected: u64) {
        assert!(admin_count(control) == expected, E_BAD_COUNT);
    }

    /// Entry wrapper asserting a guarded resource value.
    public entry fun e2e_check_resource_value(res: &Resource, expected: u64) {
        assert!(resource_value(res) == expected, E_BAD_COUNT);
    }

    /// Entry wrapper asserting admin membership.
    public entry fun e2e_check_is_admin(
        control: &AccessControl, addr: address, expected: bool
    ) {
        assert!(is_admin(control, addr) == expected, E_BAD_COUNT);
    }

    /// Full admin lifecycle verified in a single transaction.
    public entry fun e2e_access_roundtrip(ctx: &mut TxContext) {
        let sender = tx_context::sender(ctx);
        let mut control = e2e_new_control(false, ctx);
        let mut res = e2e_new_resource(1, b"initial", ctx);

        assert!(admin_count(&control) == 0, E_BAD_COUNT);
        assert!(!is_admin(&control, sender), E_BAD_COUNT);

        add_admin(&mut control, sender, ctx);
        assert!(admin_count(&control) == 1, E_BAD_COUNT);
        assert!(is_admin(&control, sender), E_BAD_COUNT);

        // A listed admin passes the guard even though the control is closed.
        update_resource(&control, &mut res, 42, b"updated", ctx);
        assert!(resource_value(&res) == 42, E_BAD_COUNT);
        assert!(*resource_metadata(&res) == b"updated", E_BAD_COUNT);

        // Revoking admin access closes the control again.
        remove_admin(&mut control, sender, ctx);
        assert!(admin_count(&control) == 0, E_BAD_COUNT);
        assert!(!is_admin(&control, sender), E_BAD_COUNT);

        transfer::share_object(control);
        transfer::share_object(res);
    }

    /// Creates a closed control that already lists `admin`, so membership can
    /// be asserted from a later transaction.
    public entry fun e2e_create_control_with_admin(admin: address, ctx: &mut TxContext) {
        let mut control = e2e_new_control(false, ctx);
        vector::push_back(&mut control.admins, admin);
        transfer::share_object(control);
    }

    /// Creates a closed control with no admins so a non-owner call must abort.
    public entry fun e2e_create_closed_control(ctx: &mut TxContext) {
        let control = e2e_new_control(false, ctx);
        let res = e2e_new_resource(1, b"guarded", ctx);
        transfer::share_object(control);
        transfer::share_object(res);
    }
}