module tester::gas_meter_save_object_e2e {
    use kanari_system::object::{Self, UID};
    use kanari_system::tx_context::TxContext;

    struct Blob has key, store {
        id: UID,
        data: vector<u8>
    }

    public entry fun save_blob(data: vector<u8>, ctx: &mut TxContext) {
        let blob = Blob { id: object::new(ctx), data };
        object::save_object(&blob);
        let Blob { id, data: _ } = blob;
        object::delete(id);
    }
}

