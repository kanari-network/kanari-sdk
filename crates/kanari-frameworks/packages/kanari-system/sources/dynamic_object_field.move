// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Similar to `kanari_system::dynamic_field`, but the values bound to these
/// dynamic fields _must_ be objects themselves (`key + store`, enforced by
/// the type bounds below).
///
/// Implemented purely on top of `dynamic_field` (no natives of its own):
/// the field name is wrapped to prevent key collisions with direct
/// `dynamic_field` usage on the same object.
module kanari_system::dynamic_object_field {
    use std::option::{Self, Option};
    use kanari_system::dynamic_field;
    use kanari_system::object::{Self, UID};

    #[allow(unused_const)]
    /// The bound value is shorter than a UID: corrupt state, or a non-object
    /// field probed via hand-crafted bytecode (`field_id` is only declared
    /// in this module, but shares the `dynamic_field` native table).
    /// Raised by the native; never referenced in Move, like the codes below.
    const EInvalidChildId: u64 = 3;

    /// Internal wrapper for the field name. The separate type is necessary
    /// to prevent key collisions with direct usage of `dynamic_field`.
    struct Wrapper<Name> has copy, drop, store {
        name: Name,
    }

    /// Record a `(parent, name) -> child` link in the host child index.
    /// The link is metadata only; the value itself still flows through the
    /// regular dynamic-field path.
    native fun record_child<Name: copy + drop + store, Value: key + store>(
        object: &mut UID,
        name: Name,
        child: &Value,
    );

    /// Drop the `(parent, name)` link from the host child index.
    native fun unrecord_child<Name: copy + drop + store>(
        object: &mut UID,
        name: Name,
    );

    /// Enumerate the live children of `object` in insertion order, merging
    /// this transaction's pending writes over durable state.
    native fun list_children(object: &UID): vector<address>;

    /// Address of the child object bound under `name`. Aborts with the
    /// dynamic-field "does not exist" code if the field is missing, and with
    /// `EInvalidChildId` if the stored value cannot hold an object ID.
    /// Safe by construction for values written via `add`: `key` types are
    /// UID-first (publish-time object rule), so their BCS opens with the
    /// 32-byte UID.
    native fun field_id<Name: copy + drop + store>(
        object: &UID,
        name: Name,
    ): address;

    /// Adds a dynamic object field to `object` under `name`, binding the
    /// object `value`. Aborts if the field already exists.
    public fun add<Name: copy + drop + store, Value: key + store>(
        object: &mut UID,
        name: Name,
        value: Value,
    ) {
        record_child(object, name, &value);
        dynamic_field::add(object, Wrapper { name }, value);
    }

    /// Immutably borrows the dynamic object field `name` on `object`.
    /// Aborts if the field does not exist or has a different value type.
    public fun borrow<Name: copy + drop + store, Value: key + store>(
        object: &UID,
        name: Name,
    ): &Value {
        dynamic_field::borrow(object, Wrapper { name })
    }

    /// Mutably borrows the dynamic object field `name` on `object`.
    /// Aborts if the field does not exist or has a different value type.
    public fun borrow_mut<Name: copy + drop + store, Value: key + store>(
        object: &mut UID,
        name: Name,
    ): &mut Value {
        dynamic_field::borrow_mut(object, Wrapper { name })
    }

    /// Removes the dynamic object field `name` on `object` and returns the
    /// bound object. Aborts if the field does not exist.
    public fun remove<Name: copy + drop + store, Value: key + store>(
        object: &mut UID,
        name: Name,
    ): Value {
        unrecord_child(object, name);
        dynamic_field::remove(object, Wrapper { name })
    }

    /// Returns true iff `object` has a dynamic object field `name`.
    public fun exists_<Name: copy + drop + store>(
        object: &UID,
        name: Name,
    ): bool {
        dynamic_field::exists_(object, Wrapper { name })
    }

    /// Returns true iff `object` has a dynamic object field `name` whose
    /// bound value type is exactly `Value`.
    public fun exists_with_type<Name: copy + drop + store, Value: key + store>(
        object: &UID,
        name: Name,
    ): bool {
        dynamic_field::exists_with_type<Wrapper<Name>, Value>(object, Wrapper { name })
    }

    /// Returns the ID of the child object bound under `name`, or `none` if
    /// the field does not exist.
    public fun id<Name: copy + drop + store>(
        object: &UID,
        name: Name,
    ): Option<object::ID> {
        if (!exists_(object, name)) return option::none();
        option::some(object::id_from_address(field_id(object, name)))
    }

    /// Live child object addresses of `object` in insertion order.
    public fun children(object: &UID): vector<address> {
        list_children(object)
    }
}
