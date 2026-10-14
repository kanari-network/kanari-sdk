// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// A set backed by a vector: O(n) membership, insertion-ordered iteration.
/// Mirrors `sui::vec_set`; used for the per-type address sets in
/// `kanari_system::deny_list` so that module stays free of raw vectors.
module kanari_system::vec_set {
    use std::vector;

    /// Key is already present.
    const EKeyAlreadyExists: u64 = 0;

    /// Key is absent.
    const EKeyDoesNotExist: u64 = 1;

    struct VecSet<K: copy + drop + store> has copy, drop, store {
        contents: vector<K>,
    }

    /// Create an empty set.
    public fun empty<K: copy + drop + store>(): VecSet<K> {
        VecSet { contents: vector::empty() }
    }

    /// Create a single-element set.
    public fun singleton<K: copy + drop + store>(key: K): VecSet<K> {
        VecSet { contents: vector::singleton(key) }
    }

    /// Insert `key`. Aborts `EKeyAlreadyExists` when already present.
    public fun insert<K: copy + drop + store>(self: &mut VecSet<K>, key: K) {
        assert!(!vector::contains(&self.contents, &key), EKeyAlreadyExists);
        vector::push_back(&mut self.contents, key);
    }

    /// Remove `key` and return nothing. Aborts `EKeyDoesNotExist` when absent.
    public fun remove<K: copy + drop + store>(self: &mut VecSet<K>, key: &K) {
        let (found, index) = vector::index_of(&self.contents, key);
        assert!(found, EKeyDoesNotExist);
        vector::remove(&mut self.contents, index);
    }

    /// True iff `key` is in the set.
    public fun contains<K: copy + drop + store>(self: &VecSet<K>, key: &K): bool {
        vector::contains(&self.contents, key)
    }

    /// Number of elements.
    public fun size<K: copy + drop + store>(self: &VecSet<K>): u64 {
        vector::length(&self.contents)
    }

    /// True iff there are no elements.
    public fun is_empty<K: copy + drop + store>(self: &VecSet<K>): bool {
        vector::is_empty(&self.contents)
    }

    /// Copy of the elements in insertion order.
    public fun keys<K: copy + drop + store>(self: &VecSet<K>): vector<K> {
        self.contents
    }

    /// Destructure into the elements in insertion order.
    public fun into_keys<K: copy + drop + store>(self: VecSet<K>): vector<K> {
        let VecSet { contents } = self;
        contents
    }
}
