// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

module kanari_system::bag {
    use kanari_system::object::{Self, UID};
    use kanari_system::tx_context::TxContext;
    use kanari_system::dynamic_field;

    /// Error codes
    const EBagNotEmpty: u64 = 1;

    /// A Bag that stores heterogeneous key-value pairs dynamically
    struct Bag has key, store {
        id: UID,
        size: u64,
    }

    /// Creates a new, empty bag
    public fun new(ctx: &mut TxContext): Bag {
        Bag {
            id: object::new(ctx),
            size: 0,
        }
    }

    /// Adds a key-value pair to the bag. 
    /// Types K and V can be different for every entry.
    public fun add<K: copy + drop + store, V: store>(bag: &mut Bag, k: K, v: V) {
        dynamic_field::add(&mut bag.id, k, v);
        bag.size = bag.size + 1;
    }

    public fun borrow_mut<K: copy + drop + store, V: store>(bag: &mut Bag, k: K): &mut V {
        dynamic_field::borrow_mut(&mut bag.id, k)
    }

    public fun borrow<K: copy + drop + store, V: store>(bag: &Bag, k: K): &V {
        dynamic_field::borrow(&bag.id, k)
    }

    public fun remove<K: copy + drop + store, V: store>(bag: &mut Bag, k: K): V {
        let v = dynamic_field::remove(&mut bag.id, k);
        bag.size = bag.size - 1;
        v
    }

    public fun contains<K: copy + drop + store>(bag: &Bag, k: K): bool {
        dynamic_field::exists_(&bag.id, k)
    }

    public fun length(bag: &Bag): u64 {
        bag.size
    }

    public fun destroy_empty(bag: Bag) {
        let Bag { id, size } = bag;
        assert!(size == 0, EBagNotEmpty);
        object::delete(id);
    }
}