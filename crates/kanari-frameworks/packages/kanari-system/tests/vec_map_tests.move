// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0
#[test_only]
module kanari_system::vec_map_tests {
    use std::option;
    use std::vector;
    use kanari_system::vec_map;

    #[test]
    fun test_insert_get_remove() {
        let m = vec_map::empty<u64, u64>();
        assert!(vec_map::is_empty(&m), 0);
        vec_map::insert(&mut m, 1, 100);
        vec_map::insert(&mut m, 2, 200);
        assert!(vec_map::size(&m) == 2, 1);
        assert!(*vec_map::get(&m, &1) == 100, 2);
        assert!(vec_map::contains(&m, &2), 3);
        assert!(!vec_map::contains(&m, &3), 4);
        let (k, v) = vec_map::remove(&mut m, &1);
        assert!(k == 1 && v == 100, 5);
        assert!(vec_map::size(&m) == 1, 6);
        vec_map::destroy_empty(vec_map::empty<u64, u64>());
        let (_, _) = vec_map::remove(&mut m, &2);
        vec_map::destroy_empty(m);
    }

    #[test]
    #[expected_failure(location = kanari_system::vec_map, abort_code = 0)]
    fun test_insert_duplicate_aborts() {
        let m = vec_map::empty<u64, u64>();
        vec_map::insert(&mut m, 1, 100);
        vec_map::insert(&mut m, 1, 200);
        vec_map::destroy_empty(m);
    }

    #[test]
    #[expected_failure(location = kanari_system::vec_map, abort_code = 1)]
    fun test_remove_missing_aborts() {
        let m = vec_map::empty<u64, u64>();
        vec_map::remove(&mut m, &9);
        vec_map::destroy_empty(m);
    }

    #[test]
    fun test_keys_values_round_trip() {
        let keys = vector::empty<u64>();
        vector::push_back(&mut keys, 30);
        vector::push_back(&mut keys, 10);
        let values = vector::empty<u64>();
        vector::push_back(&mut values, 300);
        vector::push_back(&mut values, 100);
        let m = vec_map::from_keys_values(keys, values);
        assert!(vec_map::size(&m) == 2, 0);
        assert!(option::destroy_some(vec_map::try_get(&m, &10)) == 100, 1);
        assert!(option::is_none(&vec_map::try_get(&m, &99)), 2);
        let got = vec_map::get_mut(&mut m, &30);
        *got = 333;
        let (ks, vs) = vec_map::into_keys_values(m);
        assert!(*vector::borrow(&ks, 0) == 30, 3);
        assert!(*vector::borrow(&vs, 1) == 100, 4);
    }

    #[test]
    #[expected_failure(location = kanari_system::vec_map, abort_code = 5)]
    fun test_from_keys_values_length_mismatch_aborts() {
        let keys = vector::empty<u64>();
        vector::push_back(&mut keys, 1);
        let values = vector::empty<u64>();
        vec_map::destroy_empty(vec_map::from_keys_values(keys, values));
    }

    #[test]
    fun test_idx_accessors() {
        let m = vec_map::empty<u64, u64>();
        vec_map::insert(&mut m, 7, 70);
        vec_map::insert(&mut m, 8, 80);
        assert!(vec_map::get_idx(&m, &8) == 1, 0);
        let (k, v) = vec_map::get_entry_by_idx(&m, 0);
        assert!(*k == 7 && *v == 70, 1);
        let (k2, v2) = vec_map::remove_entry_by_idx(&mut m, 0);
        assert!(k2 == 7 && v2 == 70, 2);
        let (k3, v3) = vec_map::pop(&mut m);
        assert!(k3 == 8 && v3 == 80, 3);
        vec_map::destroy_empty(m);
    }
}
