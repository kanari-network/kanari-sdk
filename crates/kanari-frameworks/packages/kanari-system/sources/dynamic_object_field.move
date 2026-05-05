// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

module kanari_system::dynamic_object_field {
    use kanari_system::object::UID;

    #[allow(unused_const)]
    /// Error codes
    const EFieldAlreadyExists: u64 = 1;
    #[allow(unused_const)]
    const EFieldDoesNotExist: u64 = 2;
    #[allow(unused_const)]
    const ENotObject: u64 = 3;

    public native fun add<Name: copy + drop + store, Value: key + store>(
        object: &mut UID,
        name: Name,
        value: Value,
    );

    public native fun borrow_mut<Name: copy + drop + store, Value: key + store>(
        object: &mut UID,
        name: Name,
    ): &mut Value;

    public native fun borrow<Name: copy + drop + store, Value: key + store>(
        object: &UID,
        name: Name,
    ): &Value;

    public native fun remove<Name: copy + drop + store, Value: key + store>(
        object: &mut UID,
        name: Name,
    ): Value;

    public native fun exists_<Name: copy + drop + store>(
        object: &UID,
        name: Name,
    ): bool;
}