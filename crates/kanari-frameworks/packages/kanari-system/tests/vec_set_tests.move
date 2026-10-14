// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0
#[test_only]
module kanari_system::vec_set_tests {
    use kanari_system::vec_set;

    #[test]
    fun test_empty_singleton_contains() {
        let s = vec_set::empty<u64>();
        assert!(vec_set::is_empty(&s), 0);
        assert!(vec_set::size(&s) == 0, 1);
        assert!(!vec_set::contains(&s, &7), 2);

        let t = vec_set::singleton(@0x100);
        assert!(!vec_set::is_empty(&t), 3);
        assert!(vec_set::size(&t) == 1, 4);
        assert!(vec_set::contains(&t, &@0x100), 5);
    }

    #[test]
    fun test_insert_remove_cycle() {
        let s = vec_set::empty<u64>();
        vec_set::insert(&mut s, 1);
        vec_set::insert(&mut s, 2);
        assert!(vec_set::size(&s) == 2, 0);
        vec_set::remove(&mut s, &1);
        assert!(!vec_set::contains(&s, &1), 1);
        assert!(vec_set::contains(&s, &2), 2);
        vec_set::remove(&mut s, &2);
        assert!(vec_set::is_empty(&s), 3);
    }

    #[test]
    #[expected_failure(location = kanari_system::vec_set, abort_code = 0)]
    fun test_insert_duplicate_aborts() {
        let s = vec_set::empty<u64>();
        vec_set::insert(&mut s, 1);
        vec_set::insert(&mut s, 1);
    }

    #[test]
    #[expected_failure(location = kanari_system::vec_set, abort_code = 1)]
    fun test_remove_missing_aborts() {
        let s = vec_set::empty<u64>();
        vec_set::remove(&mut s, &1);
    }

    #[test]
    fun test_keys_insertion_order() {
        let s = vec_set::empty<u64>();
        vec_set::insert(&mut s, 30);
        vec_set::insert(&mut s, 10);
        vec_set::insert(&mut s, 20);
        let keys = vec_set::keys(&s);
        assert!(std::vector::length(&keys) == 3, 0);
        assert!(*std::vector::borrow(&keys, 0) == 30, 1);
        assert!(*std::vector::borrow(&keys, 1) == 10, 2);
        assert!(*std::vector::borrow(&keys, 2) == 20, 3);
        let moved = vec_set::into_keys(s);
        assert!(std::vector::length(&moved) == 3, 4);
        assert!(*std::vector::borrow(&moved, 0) == 30, 5);
    }
}
