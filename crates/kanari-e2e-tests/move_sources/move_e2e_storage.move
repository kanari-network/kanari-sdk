module kanari_e2e_tests::move_e2e_storage {
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};
    use std::string::{Self, String};
    use std::vector;

    struct KeyValueStore has key {
        id: UID,
        owner: address,
        keys: vector<String>,
        values: vector<vector<u8>>,
    }

    struct DataBlob has key, store {
        id: UID,
        data: vector<u8>,
        hash: vector<u8>,
        version: u64,
    }

    public fun create_store(ctx: &mut TxContext): address {
        let store = KeyValueStore {
            id: object::new(ctx),
            owner: tx_context::sender(ctx),
            keys: vector::empty<String>(),
            values: vector::empty<vector<u8>>(),
        };
        let addr = object::uid_to_address(&store.id);
        transfer::share_object(store);
        addr
    }

    public fun set_value(store: &mut KeyValueStore, key: vector<u8>, value: vector<u8>, ctx: &mut TxContext) {
        assert!(store.owner == tx_context::sender(ctx), 300);
        let key_str = string::utf8(key);
        let mut i = 0;
        let len = vector::length(&store.keys);
        while (i < len) {
            if (*vector::borrow(&store.keys, i) == key_str) {
                *vector::borrow_mut(&mut store.values, i) = value;
                return
            };
            i = i + 1;
        };
        vector::push_back(&mut store.keys, key_str);
        vector::push_back(&mut store.values, value);
    }

    public fun get_value(store: &KeyValueStore, key: vector<u8>): option::Option<vector<u8>> {
        let key_str = string::utf8(key);
        let mut i = 0;
        let len = vector::length(&store.keys);
        while (i < len) {
            if (*vector::borrow(&store.keys, i) == key_str) {
                return option::some(*vector::borrow(&store.values, i))
            };
            i = i + 1;
        };
        option::none()
    }

    public fun remove_value(store: &mut KeyValueStore, key: vector<u8>, ctx: &mut TxContext) {
        assert!(store.owner == tx_context::sender(ctx), 301);
        let key_str = string::utf8(key);
        let mut i = 0;
        let len = vector::length(&store.keys);
        while (i < len) {
            if (*vector::borrow(&store.keys, i) == key_str) {
                vector::remove(&mut store.keys, i);
                vector::remove(&mut store.values, i);
                return
            };
            i = i + 1;
        };
    }

    public fun create_blob(data: vector<u8>, hash: vector<u8>, ctx: &mut TxContext): address {
        let blob = DataBlob {
            id: object::new(ctx),
            data,
            hash,
            version: 1,
        };
        let addr = object::uid_to_address(&blob.id);
        transfer::public_share_object(blob);
        addr
    }

    public fun update_blob(blob: &mut DataBlob, data: vector<u8>, hash: vector<u8>) {
        blob.data = data;
        blob.hash = hash;
        blob.version = blob.version + 1;
    }

    public fun get_blob_data(blob: &DataBlob): &vector<u8> {
        &blob.data
    }

    public fun get_blob_version(blob: &DataBlob): u64 {
        blob.version
    }

    public fun get_blob_hash(blob: &DataBlob): &vector<u8> {
        &blob.hash
    }
}
