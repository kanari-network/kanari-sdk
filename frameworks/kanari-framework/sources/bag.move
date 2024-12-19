module kanari_framework::bag {
    use kanari_framework::object::{Self, UID};
    use kanari_framework::dynamic_field as field;
    use kanari_framework::tx_context::TxContext;

    // Attempted to destroy a non-empty bag
    const EBagNotEmpty: u64 = 0;

    struct Bag has key, store {
        /// the ID of this bag
        id: UID,
        /// the number of key-value pairs in the bag
        size: u64,
    }

    /// Creates a new, empty bag
    public fun new(ctx: &mut TxContext): Bag {
        Bag {
            id: object::new(ctx),
            size: 0,
        }
    }

    /// Adds a key-value pair to the bag `bag: &mut Bag`
    /// Aborts with `kanari_framework::dynamic_field::EFieldAlreadyExists` if the bag already has an entry with
    /// that key `k: K`.
    public fun add<K: copy + drop + store, V: store>(bag: &mut Bag, k: K, v: V) {
        field::add(&mut bag.id, k, v);
        bag.size = bag.size + 1;
    }

    /// Immutable borrows the value associated with the key in the bag `bag: &Bag`.
    /// Aborts with `kanari_framework::dynamic_field::EFieldDoesNotExist` if the bag does not have an entry with
    /// that key `k: K`.
    /// Aborts with `kanari_framework::dynamic_field::EFieldTypeMismatch` if the bag has an entry for the key, but
    /// the value does not have the specified type.
    public fun borrow<K: copy + drop + store, V: store>(bag: &Bag, k: K): &V {
        field::borrow(&bag.id, k)
    }

    /// Mutably borrows the value associated with the key in the bag `bag: &mut Bag`.
    /// Aborts with `kanari_framework::dynamic_field::EFieldDoesNotExist` if the bag does not have an entry with
    /// that key `k: K`.
    /// Aborts with `kanari_framework::dynamic_field::EFieldTypeMismatch` if the bag has an entry for the key, but
    /// the value does not have the specified type.
    public fun borrow_mut<K: copy + drop + store, V: store>(bag: &mut Bag, k: K): &mut V {
        field::borrow_mut(&mut bag.id, k)
    }

    /// Mutably borrows the key-value pair in the bag `bag: &mut Bag` and returns the value.
    /// Aborts with `kanari_framework::dynamic_field::EFieldDoesNotExist` if the bag does not have an entry with
    /// that key `k: K`.
    /// Aborts with `kanari_framework::dynamic_field::EFieldTypeMismatch` if the bag has an entry for the key, but
    /// the value does not have the specified type.
    public fun remove<K: copy + drop + store, V: store>(bag: &mut Bag, k: K): V {
        let v = field::remove(&mut bag.id, k);
        bag.size = bag.size - 1;
        v
    }

    /// Returns true iff there is an value associated with the key `k: K` in the bag `bag: &Bag`
    public fun contains<K: copy + drop + store>(bag: &Bag, k: K): bool {
        field::exists_<K>(&bag.id, k)
    }

    /// Returns true iff there is an value associated with the key `k: K` in the bag `bag: &Bag`
    /// with an assigned value of type `V`
    public fun contains_with_type<K: copy + drop + store, V: store>(bag: &Bag, k: K): bool {
        field::exists_with_type<K, V>(&bag.id, k)
    }

    /// Returns the size of the bag, the number of key-value pairs
    public fun length(bag: &Bag): u64 {
        bag.size
    }

    /// Returns true iff the bag is empty (if `length` returns `0`)
    public fun is_empty(bag: &Bag): bool {
        bag.size == 0
    }

    /// Destroys an empty bag
    /// Aborts with `EBagNotEmpty` if the bag still contains values
    public fun destroy_empty(bag: Bag) {
        let Bag { id, size } = bag;
        assert!(size == 0, EBagNotEmpty);
        object::delete(id)
    }    
}