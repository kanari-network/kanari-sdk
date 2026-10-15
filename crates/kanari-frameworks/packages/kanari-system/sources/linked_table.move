// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Ordered table mirroring `sui::linked_table`: like `table`, but entries
/// are linked for ordered insertion and removal.
///
/// NOTE: `contains` uses name-based existence (`dynamic_field::exists_`)
/// because this runtime exposes no typed existence check; a field with the
/// same name but a different value type would report present and then abort
/// on `borrow` with a type mismatch instead.
module kanari_system::linked_table {
    use std::option::{Self, Option};
    use kanari_system::dynamic_field;
    use kanari_system::object::{Self, UID};
    use kanari_system::tx_context::TxContext;

    // Attempted to destroy a non-empty table
    const ETableNotEmpty: u64 = 0;

    // Attempted to remove the front or back of an empty table
    const ETableIsEmpty: u64 = 1;

    struct LinkedTable<K: copy + drop + store, phantom V: store> has key, store {
        /// the ID of this table
        id: UID,
        /// the number of key-value pairs in the table
        size: u64,
        /// the front of the table, i.e. the key of the first entry
        head: Option<K>,
        /// the back of the table, i.e. the key of the last entry
        tail: Option<K>,
    }

    struct Node<K: copy + drop + store, V: store> has store {
        /// the previous key
        prev: Option<K>,
        /// the next key
        next: Option<K>,
        /// the value being stored
        value: V,
    }

    /// Creates a new, empty table
    public fun new<K: copy + drop + store, V: store>(ctx: &mut TxContext): LinkedTable<K, V> {
        LinkedTable {
            id: object::new(ctx),
            size: 0,
            head: option::none(),
            tail: option::none(),
        }
    }

    /// The key of the first element, or `None` when empty.
    public fun front<K: copy + drop + store, V: store>(table: &LinkedTable<K, V>): &Option<K> {
        &table.head
    }

    /// The key of the last element, or `None` when empty.
    public fun back<K: copy + drop + store, V: store>(table: &LinkedTable<K, V>): &Option<K> {
        &table.tail
    }

    /// Insert at the front. Aborts when `k` already exists.
    public fun push_front<K: copy + drop + store, V: store>(
        table: &mut LinkedTable<K, V>,
        k: K,
        value: V,
    ) {
        let old_head = option::swap_or_fill(&mut table.head, k);
        if (option::is_none(&table.tail)) {
            option::fill(&mut table.tail, k);
        };
        let prev = option::none();
        let next = if (option::is_some(&old_head)) {
            let old_head_k = option::destroy_some(old_head);
            dynamic_field::borrow_mut<K, Node<K, V>>(&mut table.id, old_head_k).prev =
                option::some(k);
            option::some(old_head_k)
        } else {
            option::none()
        };
        dynamic_field::add(&mut table.id, k, Node { prev, next, value });
        table.size = table.size + 1;
    }

    /// Insert at the back. Aborts when `k` already exists.
    public fun push_back<K: copy + drop + store, V: store>(
        table: &mut LinkedTable<K, V>,
        k: K,
        value: V,
    ) {
        if (option::is_none(&table.head)) {
            option::fill(&mut table.head, k);
        };
        let old_tail = option::swap_or_fill(&mut table.tail, k);
        let prev = if (option::is_some(&old_tail)) {
            let old_tail_k = option::destroy_some(old_tail);
            dynamic_field::borrow_mut<K, Node<K, V>>(&mut table.id, old_tail_k).next =
                option::some(k);
            option::some(old_tail_k)
        } else {
            option::none()
        };
        let next = option::none();
        dynamic_field::add(&mut table.id, k, Node { prev, next, value });
        table.size = table.size + 1;
    }

    /// Immutably borrow the value for `k`. Aborts when absent.
    public fun borrow<K: copy + drop + store, V: store>(
        table: &LinkedTable<K, V>,
        k: K,
    ): &V {
        &dynamic_field::borrow<K, Node<K, V>>(&table.id, k).value
    }

    /// Mutably borrow the value for `k`. Aborts when absent.
    public fun borrow_mut<K: copy + drop + store, V: store>(
        table: &mut LinkedTable<K, V>,
        k: K,
    ): &mut V {
        &mut dynamic_field::borrow_mut<K, Node<K, V>>(&mut table.id, k).value
    }

    /// The key before `k`, or `None` for the front element.
    public fun prev<K: copy + drop + store, V: store>(
        table: &LinkedTable<K, V>,
        k: K,
    ): &Option<K> {
        &dynamic_field::borrow<K, Node<K, V>>(&table.id, k).prev
    }

    /// The key after `k`, or `None` for the back element.
    public fun next<K: copy + drop + store, V: store>(
        table: &LinkedTable<K, V>,
        k: K,
    ): &Option<K> {
        &dynamic_field::borrow<K, Node<K, V>>(&table.id, k).next
    }

    /// Remove `k` (splicing the ordering) and return its value.
    /// Aborts when absent.
    public fun remove<K: copy + drop + store, V: store>(
        table: &mut LinkedTable<K, V>,
        k: K,
    ): V {
        let Node { prev, next, value } = dynamic_field::remove(&mut table.id, k);
        table.size = table.size - 1;
        if (option::is_some(&prev)) {
            dynamic_field::borrow_mut<K, Node<K, V>>(
                &mut table.id,
                *option::borrow(&prev),
            ).next = next;
        };
        if (option::is_some(&next)) {
            dynamic_field::borrow_mut<K, Node<K, V>>(
                &mut table.id,
                *option::borrow(&next),
            ).prev = prev;
        };
        if (option::borrow(&table.head) == &k) {
            table.head = next;
        };
        if (option::borrow(&table.tail) == &k) {
            table.tail = prev;
        };
        value
    }

    /// Remove the front entry. Aborts `ETableIsEmpty` when empty.
    public fun pop_front<K: copy + drop + store, V: store>(
        table: &mut LinkedTable<K, V>,
    ): (K, V) {
        assert!(option::is_some(&table.head), ETableIsEmpty);
        let head = *option::borrow(&table.head);
        (head, remove(table, head))
    }

    /// Remove the back entry. Aborts `ETableIsEmpty` when empty.
    public fun pop_back<K: copy + drop + store, V: store>(
        table: &mut LinkedTable<K, V>,
    ): (K, V) {
        assert!(option::is_some(&table.tail), ETableIsEmpty);
        let tail = *option::borrow(&table.tail);
        (tail, remove(table, tail))
    }

    /// True iff a value is associated with `k`. See the module note about
    /// name-based (not typed) existence.
    public fun contains<K: copy + drop + store, V: store>(
        table: &LinkedTable<K, V>,
        k: K,
    ): bool {
        dynamic_field::exists_(&table.id, k)
    }

    /// Number of entries.
    public fun length<K: copy + drop + store, V: store>(table: &LinkedTable<K, V>): u64 {
        table.size
    }

    /// True iff there are no entries.
    public fun is_empty<K: copy + drop + store, V: store>(table: &LinkedTable<K, V>): bool {
        table.size == 0
    }

    /// Destroy an empty table. Aborts `ETableNotEmpty` otherwise.
    public fun destroy_empty<K: copy + drop + store, V: store>(table: LinkedTable<K, V>) {
        let LinkedTable { id, size, head: _, tail: _ } = table;
        assert!(size == 0, ETableNotEmpty);
        object::delete(id);
    }

    /// Drop a possibly non-empty table. Usable only when `V` has `drop`.
    public fun drop<K: copy + drop + store, V: drop + store>(table: LinkedTable<K, V>) {
        let LinkedTable { id, size: _, head: _, tail: _ } = table;
        object::delete(id);
    }
}
