#[test_only]
module kanari_system::dynamic_object_field_tests {
    use std::option;
    use kanari_system::dynamic_field;
    use kanari_system::dynamic_object_field as dof;
    use kanari_system::object::{Self, UID};
    use kanari_system::tx_context;

    struct Host has key, store {
        id: UID,
    }

    struct Child has key, store {
        id: UID,
        value: u64,
    }

    // `id` is required as the first field of a `key` struct but never read: this type only
    // exists to be attached and queried as a dynamic object field.
    #[allow(unused_field)]
    struct Other has key, store {
        id: UID,
    }

    fun new_host(ctx: &mut tx_context::TxContext): Host {
        Host { id: object::new(ctx) }
    }

    #[test]
    fun dof_typed_queries_round_trip() {
        let ctx = &mut tx_context::dummy();
        let host = new_host(ctx);

        // Missing field: every query reports absence.
        assert!(!dof::exists_<u64>(&host.id, 3), 0);
        assert!(!dof::exists_with_type<u64, Child>(&host.id, 3), 1);
        assert!(option::is_none(&dof::id(&host.id, 3)), 2);

        let child = Child { id: object::new(ctx), value: 500 };
        let want = object::uid_address(&child.id);
        dof::add<u64, Child>(&mut host.id, 3, child);

        // Present with the exact value type; a different `key` type reports
        // false even though the field exists.
        assert!(dof::exists_<u64>(&host.id, 3), 3);
        assert!(dof::exists_with_type<u64, Child>(&host.id, 3), 4);
        assert!(!dof::exists_with_type<u64, Other>(&host.id, 3), 5);

        // `id` resolves to the bound child's object ID.
        let got = option::destroy_some(dof::id(&host.id, 3));
        assert!(object::id_to_address(&got) == want, 6);

        // Mutating the child must not move its ID (cached-dirty-bytes path).
        dof::borrow_mut<u64, Child>(&mut host.id, 3).value = 777;
        let got_again = option::destroy_some(dof::id(&host.id, 3));
        assert!(object::id_to_address(&got_again) == want, 7);
        assert!(dof::exists_with_type<u64, Child>(&host.id, 3), 8);

        // Removal restores absence on every query.
        let removed = dof::remove<u64, Child>(&mut host.id, 3);
        assert!(removed.value == 777, 9);
        assert!(!dof::exists_<u64>(&host.id, 3), 10);
        assert!(!dof::exists_with_type<u64, Child>(&host.id, 3), 11);
        assert!(option::is_none(&dof::id(&host.id, 3)), 12);

        let Child { id, value: _ } = removed;
        object::delete(id);
        let Host { id } = host;
        object::delete(id);
    }

    #[test]
    fun df_exists_with_type_direct_usage() {
        let ctx = &mut tx_context::dummy();
        let host = new_host(ctx);

        assert!(!dynamic_field::exists_with_type<u64, u64>(&host.id, 9), 20);

        dynamic_field::add<u64, u64>(&mut host.id, 9, 44);
        assert!(dynamic_field::exists_with_type<u64, u64>(&host.id, 9), 21);
        assert!(!dynamic_field::exists_with_type<u64, bool>(&host.id, 9), 22);
        assert!(!dynamic_field::exists_with_type<bool, u64>(&host.id, true), 23);

        let removed = dynamic_field::remove<u64, u64>(&mut host.id, 9);
        assert!(removed == 44, 24);
        assert!(!dynamic_field::exists_with_type<u64, u64>(&host.id, 9), 25);

        let Host { id } = host;
        object::delete(id);
    }
}
