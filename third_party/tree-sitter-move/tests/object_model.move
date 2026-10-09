module test::object_model {
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;

    public struct Item has key, store {
        id: UID,
        owner: address,
    }

    public entry fun create(owner: address, ctx: &mut TxContext) {
        let item = Item {
            id: object::new(ctx),
            owner,
        };
        transfer::public_transfer(item, owner);
    }

    public fun borrow(item_id: address): &Item {
        object::borrow_global<Item>(item_id)
    }

    public fun destroy(item: Item) {
        let Item { id, owner: _ } = item;
        object::delete(id);
    }
}
