module tester::gas_meter_transfer_e2e {
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    struct Blob has key, store {
        id: UID,
        data: vector<u8>
    }

    public entry fun transfer_blob(data: vector<u8>, ctx: &mut TxContext) {
        let blob = Blob { id: object::new(ctx), data };
        transfer::public_transfer(blob, tx_context::sender(ctx));
    }
}

