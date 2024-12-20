module ibc::ibc {
    use std::string::String;
    use kanari_framework::object::{UID, new};
    use kanari_framework::tx_context::TxContext;

    struct User has key {
        id: UID,
        name: String,
        age: u8,
    }

    public fun create_user(ctx: &mut TxContext, name: String, age: u8) : User {
        User {
            id: new(ctx),
            name,
            age,
        }
    }
}