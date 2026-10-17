// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0
#[test_only]
module kanari_system::linked_table_tests {
    use std::option;
    use kanari_system::linked_table;
    use kanari_system::tx_context;

    #[test]
    fun test_push_borrow_order() {
        let ctx = &mut tx_context::dummy();
        let t = linked_table::new<u64, u64>(ctx);
        let (k0, k1, k2, k9) = (0u64, 1u64, 2u64, 9u64);
        assert!(linked_table::is_empty(&t), 0);
        linked_table::push_back(&mut t, k1, 100);
        linked_table::push_back(&mut t, k2, 200);
        linked_table::push_front(&mut t, k0, 50);
        assert!(linked_table::length(&t) == 3, 1);
        assert!(*linked_table::borrow(&t, k0) == 50, 2);
        assert!(*linked_table::borrow(&t, k2) == 200, 3);
        assert!(linked_table::contains(&t, k1), 4);
        assert!(!linked_table::contains(&t, k9), 5);
        // Order: 0 -> 1 -> 2.
        assert!(option::destroy_some(*linked_table::front(&t)) == 0, 6);
        assert!(option::destroy_some(*linked_table::back(&t)) == 2, 7);
        assert!(option::destroy_some(*linked_table::next(&t, k0)) == 1, 8);
        assert!(option::destroy_some(*linked_table::prev(&t, k2)) == 1, 9);
        assert!(option::is_none(linked_table::prev(&t, k0)), 10);
        assert!(option::is_none(linked_table::next(&t, k2)), 11);
        let v = linked_table::borrow_mut(&mut t, k1);
        *v = 111;
        assert!(*linked_table::borrow(&t, k1) == 111, 12);
        kanari_system::transfer::public_freeze_object(t);
    }

    #[test]
    fun test_remove_splices_order() {
        let ctx = &mut tx_context::dummy();
        let t = linked_table::new<u64, u64>(ctx);
        let (k1, k2, k3) = (1u64, 2u64, 3u64);
        linked_table::push_back(&mut t, k1, 100);
        linked_table::push_back(&mut t, k2, 200);
        linked_table::push_back(&mut t, k3, 300);
        assert!(linked_table::remove(&mut t, k2) == 200, 0);
        // Order: 1 -> 3.
        assert!(option::destroy_some(*linked_table::next(&t, k1)) == 3, 1);
        assert!(option::destroy_some(*linked_table::prev(&t, k3)) == 1, 2);
        let (k, v) = linked_table::pop_front(&mut t);
        assert!(k == 1 && v == 100, 3);
        let (k2, v2) = linked_table::pop_back(&mut t);
        assert!(k2 == 3 && v2 == 300, 4);
        assert!(linked_table::is_empty(&t), 5);
        linked_table::destroy_empty(t);
    }

    #[test]
    #[expected_failure(location = kanari_system::linked_table, abort_code = 1)]
    fun test_pop_front_empty_aborts() {
        let ctx = &mut tx_context::dummy();
        let t = linked_table::new<u64, u64>(ctx);
        linked_table::pop_front(&mut t);
        linked_table::destroy_empty(t);
    }

    #[test]
    #[expected_failure(location = kanari_system::linked_table, abort_code = 0)]
    fun test_destroy_nonempty_aborts() {
        let ctx = &mut tx_context::dummy();
        let t = linked_table::new<u64, u64>(ctx);
        linked_table::push_back(&mut t, 1, 100);
        linked_table::destroy_empty(t);
    }

    #[test]
    fun test_drop_nonempty_with_droppable_value() {
        let ctx = &mut tx_context::dummy();
        let t = linked_table::new<u64, u64>(ctx);
        linked_table::push_back(&mut t, 1, 100);
        linked_table::drop(t);
    }
}
