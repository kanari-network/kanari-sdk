// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#[test_only]
module kanari_system::table_bag_tests {
    use kanari_system::bag;
    use kanari_system::table;
    use kanari_system::tx_context;

    #[test]
    fun test_table_crud() {
        let ctx = tx_context::dummy();
        let tab = table::new<u64, u64>(&mut ctx);
        let t = &mut tab;
        assert!(table::length(t) == 0, 0);
        assert!(!table::contains(t, 1), 1);

        table::add(t, 1, 100);
        table::add(t, 2, 200);
        assert!(table::length(t) == 2, 2);
        assert!(table::contains(t, 1), 3);
        assert!(*table::borrow(t, 1) == 100, 4);

        *table::borrow_mut(t, 1) = 150;
        assert!(*table::borrow(t, 1) == 150, 5);

        assert!(table::remove(t, 1) == 150, 6);
        assert!(table::length(t) == 1, 7);
        assert!(!table::contains(t, 1), 8);

        assert!(table::remove(t, 2) == 200, 9);
        table::destroy_empty(tab);
    }

    #[test]
    #[expected_failure(abort_code = 1)]
    fun test_table_destroy_nonempty_fails() {
        // ETableNotEmpty.
        let ctx = tx_context::dummy();
        let t = table::new<u64, u64>(&mut ctx);
        table::add(&mut t, 1, 100);
        table::destroy_empty(t);
    }

    #[test]
    fun test_bag_heterogeneous() {
        let ctx = tx_context::dummy();
        let bg = bag::new(&mut ctx);
        let b = &mut bg;
        assert!(bag::length(b) == 0, 0);

        // Different value types under different keys.
        bag::add(b, 1u8, 100u64);
        bag::add(b, 2u8, true);
        assert!(bag::length(b) == 2, 1);
        assert!(bag::contains<u8>(b, 1u8), 2);
        assert!(*bag::borrow<u8, u64>(b, 1u8) == 100, 3);
        assert!(*bag::borrow<u8, bool>(b, 2u8), 4);

        *bag::borrow_mut<u8, u64>(b, 1u8) = 111;
        assert!(*bag::borrow<u8, u64>(b, 1u8) == 111, 5);

        assert!(bag::remove<u8, bool>(b, 2u8), 6);
        assert!(bag::remove<u8, u64>(b, 1u8) == 111, 7);
        assert!(bag::length(b) == 0, 8);
        bag::destroy_empty(bg);
    }

    #[test]
    #[expected_failure(abort_code = 1)]
    fun test_bag_destroy_nonempty_fails() {
        // EBagNotEmpty.
        let ctx = tx_context::dummy();
        let b = bag::new(&mut ctx);
        bag::add(&mut b, 1u8, 1u64);
        bag::destroy_empty(b);
    }
}
