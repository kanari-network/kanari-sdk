// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

module kanari_system::deny_list {
    use std::vector;
    use kanari_system::object;
    use kanari_system::tx_context::TxContext;

    /// Deny list resource storing addresses
    struct DenyList has key, store, drop {
        addresses: vector<address>,
    }

    /// Capability to mutate a DenyList for a specific coin type
    struct DenyCap<phantom T> has key, store, drop {
        id: object::UID,
    }

    /// Create a new empty DenyList
    public fun new_denylist(): DenyList {
        DenyList { addresses: vector::empty<address>() }
    }

    /// Create a new DenyCap object
    public fun new_denycap<T>(ctx: &mut TxContext): DenyCap<T> {
        DenyCap<T> { id: object::new(ctx) }
    }

    /// Add an address to the deny list. (No-op implementation: placeholder)
    public fun deny_list_add<T>(_d: &mut DenyList, _cap: &mut DenyCap<T>, _addr: address, _ctx: &mut TxContext) {
        // Placeholder: Implement presence checks and vector insert if needed.
    }

    /// Remove an address from the deny list. (No-op implementation: placeholder)
    public fun deny_list_remove<T>(_d: &mut DenyList, _cap: &mut DenyCap<T>, _addr: address, _ctx: &mut TxContext) {
        // Placeholder: Implement removal logic if desired.
    }
}
