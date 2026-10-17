// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// A map backed by a vector, mirroring `sui::vec_map`.
module kanari_system::vec_map {
    use std::option::{Self, Option};
    use std::vector;

    /// This key already exists in the map
    const EKeyAlreadyExists: u64 = 0;

    /// This key does not exist in the map
    const EKeyDoesNotExist: u64 = 1;

    /// Trying to destroy a map that is not empty
    const EMapNotEmpty: u64 = 2;

    /// Trying to access an element of the map at an invalid index
    const EIndexOutOfBounds: u64 = 3;

    /// Trying to pop from a map that is empty
    const EMapEmpty: u64 = 4;

    /// Trying to construct a map from keys and values of different lengths
    const EUnequalLengths: u64 = 5;

    /// A map data structure backed by a vector. Entries are kept in
    /// insertion order (not sorted); all operations are O(N). For large
    /// maps prefer `table`/`linked_table` instead.
    struct VecMap<K: copy, V> has copy, drop, store {
        contents: vector<Entry<K, V>>,
    }

    /// An entry in the map
    struct Entry<K: copy, V> has copy, drop, store {
        key: K,
        value: V,
    }

    /// Create an empty `VecMap`
    public fun empty<K: copy, V>(): VecMap<K, V> {
        VecMap { contents: vector::empty() }
    }

    /// Insert `key` |-> `value`. Aborts `EKeyAlreadyExists` on duplicates.
    public fun insert<K: copy, V>(self: &mut VecMap<K, V>, key: K, value: V) {
        assert!(!contains(self, &key), EKeyAlreadyExists);
        vector::push_back(&mut self.contents, Entry { key, value });
    }

    /// Remove `key` and return `(key, value)`. Aborts `EKeyDoesNotExist`.
    public fun remove<K: copy, V>(self: &mut VecMap<K, V>, key: &K): (K, V) {
        let idx = get_idx(self, key);
        let Entry { key, value } = vector::remove(&mut self.contents, idx);
        (key, value)
    }

    /// Pop the most recently inserted entry. Aborts `EMapEmpty` when empty.
    public fun pop<K: copy, V>(self: &mut VecMap<K, V>): (K, V) {
        assert!(!vector::is_empty(&self.contents), EMapEmpty);
        let Entry { key, value } = vector::pop_back(&mut self.contents);
        (key, value)
    }

    /// Mutably borrow the value for `key`. Aborts `EKeyDoesNotExist`.
    public fun get_mut<K: copy, V>(self: &mut VecMap<K, V>, key: &K): &mut V {
        let idx = get_idx(self, key);
        let entry = vector::borrow_mut(&mut self.contents, idx);
        &mut entry.value
    }

    /// Borrow the value for `key`. Aborts `EKeyDoesNotExist`.
    public fun get<K: copy, V>(self: &VecMap<K, V>, key: &K): &V {
        let idx = get_idx(self, key);
        let entry = vector::borrow(&self.contents, idx);
        &entry.value
    }

    /// Borrow a copyable value for `key`, or `None` when absent.
    public fun try_get<K: copy, V: copy>(self: &VecMap<K, V>, key: &K): Option<V> {
        if (contains(self, key)) {
            option::some(*get(self, key))
        } else {
            option::none()
        }
    }

    /// True iff an entry exists for `key`.
    public fun contains<K: copy, V>(self: &VecMap<K, V>, key: &K): bool {
        option::is_some(&get_idx_opt(self, key))
    }

    /// Number of entries.
    public fun size<K: copy, V>(self: &VecMap<K, V>): u64 {
        vector::length(&self.contents)
    }

    /// True iff there are no entries.
    public fun is_empty<K: copy, V>(self: &VecMap<K, V>): bool {
        size(self) == 0
    }

    /// Destroy an empty map. Aborts `EMapNotEmpty` otherwise.
    public fun destroy_empty<K: copy, V>(self: VecMap<K, V>) {
        let VecMap { contents } = self;
        assert!(vector::is_empty(&contents), EMapNotEmpty);
        vector::destroy_empty(contents);
    }

    /// Unpack into `(keys, values)` in insertion order.
    public fun into_keys_values<K: copy, V>(self: VecMap<K, V>): (vector<K>, vector<V>) {
        let VecMap { contents } = self;
        let contents = contents;
        // Reverse so pops below yield insertion order.
        vector::reverse(&mut contents);
        let i = 0;
        let n = vector::length(&contents);
        let keys = vector::empty();
        let values = vector::empty();
        while (i < n) {
            let Entry { key, value } = vector::pop_back(&mut contents);
            vector::push_back(&mut keys, key);
            vector::push_back(&mut values, value);
            i = i + 1;
        };
        vector::destroy_empty(contents);
        (keys, values)
    }

    /// Build a map from parallel `keys`/`values` vectors.
    /// Aborts `EUnequalLengths` when the lengths differ.
    public fun from_keys_values<K: copy, V>(
        keys: vector<K>,
        values: vector<V>,
    ): VecMap<K, V> {
        let keys = keys;
        let values = values;
        assert!(vector::length(&keys) == vector::length(&values), EUnequalLengths);
        vector::reverse(&mut keys);
        vector::reverse(&mut values);
        let map = empty();
        while (!vector::is_empty(&keys)) {
            insert(&mut map, vector::pop_back(&mut keys), vector::pop_back(&mut values));
        };
        vector::destroy_empty(keys);
        vector::destroy_empty(values);
        map
    }

    /// All keys (insertion order; do not assume sorted).
    public fun keys<K: copy, V>(self: &VecMap<K, V>): vector<K> {
        let i = 0;
        let n = vector::length(&self.contents);
        let out = vector::empty();
        while (i < n) {
            let entry = vector::borrow(&self.contents, i);
            vector::push_back(&mut out, entry.key);
            i = i + 1;
        };
        out
    }

    /// Index of `key`, or `None` when absent (insertion order, not sorted).
    public fun get_idx_opt<K: copy, V>(self: &VecMap<K, V>, key: &K): Option<u64> {
        let i = 0;
        let n = size(self);
        while (i < n) {
            if (&vector::borrow(&self.contents, i).key == key) {
                return option::some(i)
            };
            i = i + 1;
        };
        option::none()
    }

    /// Index of `key`. Aborts `EKeyDoesNotExist` when absent.
    public fun get_idx<K: copy, V>(self: &VecMap<K, V>, key: &K): u64 {
        let idx_opt = get_idx_opt(self, key);
        assert!(option::is_some(&idx_opt), EKeyDoesNotExist);
        option::destroy_some(idx_opt)
    }

    /// The `idx`th entry. Aborts `EIndexOutOfBounds` when out of range.
    public fun get_entry_by_idx<K: copy, V>(self: &VecMap<K, V>, idx: u64): (&K, &V) {
        assert!(idx < size(self), EIndexOutOfBounds);
        let entry = vector::borrow(&self.contents, idx);
        (&entry.key, &entry.value)
    }

    /// Mutably borrow the `idx`th entry. Aborts `EIndexOutOfBounds`.
    public fun get_entry_by_idx_mut<K: copy, V>(
        self: &mut VecMap<K, V>,
        idx: u64,
    ): (&K, &mut V) {
        assert!(idx < size(self), EIndexOutOfBounds);
        let entry = vector::borrow_mut(&mut self.contents, idx);
        (&entry.key, &mut entry.value)
    }

    /// Remove the entry at `idx`. Aborts `EIndexOutOfBounds`.
    public fun remove_entry_by_idx<K: copy, V>(self: &mut VecMap<K, V>, idx: u64): (K, V) {
        assert!(idx < size(self), EIndexOutOfBounds);
        let Entry { key, value } = vector::remove(&mut self.contents, idx);
        (key, value)
    }
}
