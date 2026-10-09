// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Safe borrowing helpers over `object::borrow_global[_mut]`.
///
/// Raw natives return a reference and leave persistence to the caller:
/// forgetting `save_object` after a `borrow_global_mut` silently drops the
/// mutation on commit (this exact bug once lost multisig approvals).
module kanari_system::borrow {
    use kanari_system::object::{Self, ID};

    /// Alias check for the two-object helper.
    const E_SAME_OBJECT: u64 = 1;

    /// Read an object without mutation authority.
    public fun borrow<T: key>(addr: address): &T {
        object::borrow_global<T>(addr)
    }

    /// Borrow mutably. The caller MUST call `save` (same module) after
    /// mutating, otherwise the change is lost on commit.
    public fun borrow_mut<T: key>(addr: address): &mut T {
        object::borrow_global_mut<T>(addr)
    }

    /// Persist a borrowed object. Call exactly once after mutating a
    /// `borrow_mut` reference.
    public fun save<T: key>(obj: &T) {
        object::save_object(obj);
    }

    /// Assert two object IDs are distinct before borrowing both mutably.
    /// Move cannot hold two `&mut` to the same object.
    public fun assert_distinct(a: address, b: address) {
        assert!(a != b, E_SAME_OBJECT);
    }

    /// Resolve an `ID` to its address, then borrow immutably.
    public fun borrow_id<T: key>(id: &ID): &T {
        borrow<T>(object::id_to_address(id))
    }
}
