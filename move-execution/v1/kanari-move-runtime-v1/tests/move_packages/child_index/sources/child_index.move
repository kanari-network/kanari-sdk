module tester::child_index {
    use kanari_system::dynamic_object_field as dof;
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};
    use std::vector;

    struct Host has key, store {
        id: UID,
    }

    struct Child has key, store {
        id: UID,
        value: u64,
    }

    /// One-transaction cycle: add two children (pending visibility),
    /// remove one, re-check. Aborts on any mismatch.
    public entry fun full_cycle(ctx: &mut TxContext) {
        let host = Host { id: object::new(ctx) };
        let c1 = Child { id: object::new(ctx), value: 1 };
        let c2 = Child { id: object::new(ctx), value: 2 };
        let id1 = object::uid_address(&c1.id);
        let id2 = object::uid_address(&c2.id);
        dof::add(&mut host.id, 1u64, c1);
        dof::add(&mut host.id, 2u64, c2);

        let kids = dof::children(&host.id);
        assert!(vector::length(&kids) == 2, 0);
        assert!(vector::contains(&kids, &id1), 1);
        assert!(vector::contains(&kids, &id2), 2);

        let back = dof::remove<u64, Child>(&mut host.id, 1u64);
        assert!(back.value == 1, 3);
        let kids2 = dof::children(&host.id);
        assert!(vector::length(&kids2) == 1, 4);
        assert!(vector::contains(&kids2, &id2), 5);

        transfer::public_transfer(back, tx_context::sender(ctx));
        transfer::public_transfer(host, tx_context::sender(ctx));
    }

    /// Cross-transaction durability: exactly one child must survive.
    public entry fun check_persisted(host_id: address) {
        let host = object::borrow_global<Host>(host_id);
        let kids = dof::children(&host.id);
        assert!(vector::length(&kids) == 1, 0);
    }

    /// Delete the host object outright (children transfer out separately).
    public entry fun delete_host(host: Host) {
        let Host { id } = host;
        object::delete(id);
    }
}
