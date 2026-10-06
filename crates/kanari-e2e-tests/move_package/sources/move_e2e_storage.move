module kanari_e2e_tests::move_e2e_storage {
    use std::string::{Self, String};
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    const E_NOT_OWNER: u64 = 300;
    const E_BAD_VALUE: u64 = 301;

    /// Parallel-array key/value store with owner-gated writes.
    public struct KeyValueStore has key, store {
        id: UID,
        owner: address,
        keys: vector<String>,
        values: vector<vector<u8>>,
    }

    /// Versioned blob with a caller-supplied content hash.
    public struct DataBlob has key, store {
        id: UID,
        data: vector<u8>,
        hash: vector<u8>,
        version: u64,
    }

    public fun create_store(ctx: &mut TxContext): address {
        let store = e2e_new_store(ctx);
        let addr = object::uid_address(&store.id);
        transfer::share_object(store);
        addr
    }

    /// Owner-only insert or overwrite.
    public entry fun set_value(
        store: &mut KeyValueStore,
        key: vector<u8>,
        value: vector<u8>,
        ctx: &mut TxContext,
    ) {
        assert!(store.owner == tx_context::sender(ctx), E_NOT_OWNER);
        let key_str = string::utf8(key);
        let mut pos = vector::length(&store.keys);
        while (pos > 0) {
            pos = pos - 1;
            if (*vector::borrow(&store.keys, pos) == key_str) {
                *vector::borrow_mut(&mut store.values, pos) = value;
                return
            };
        };
        vector::push_back(&mut store.keys, key_str);
        vector::push_back(&mut store.values, value);
    }

    /// Read a value, returning `none` when the key is absent.
    public fun get_value(store: &KeyValueStore, key: vector<u8>): option::Option<vector<u8>> {
        let key_str = string::utf8(key);
        let mut pos = vector::length(&store.keys);
        while (pos > 0) {
            pos = pos - 1;
            if (*vector::borrow(&store.keys, pos) == key_str) {
                return option::some(*vector::borrow(&store.values, pos))
            };
        };
        option::none()
    }

    /// Owner-only delete; no-op when the key is absent.
    public entry fun remove_value(
        store: &mut KeyValueStore, key: vector<u8>, ctx: &mut TxContext
    ) {
        assert!(store.owner == tx_context::sender(ctx), E_NOT_OWNER);
        let key_str = string::utf8(key);
        let mut pos = vector::length(&store.keys);
        while (pos > 0) {
            pos = pos - 1;
            if (*vector::borrow(&store.keys, pos) == key_str) {
                vector::remove(&mut store.keys, pos);
                vector::remove(&mut store.values, pos);
                return
            };
        };
    }

    public fun store_size(store: &KeyValueStore): u64 {
        vector::length(&store.keys)
    }

    public fun create_blob(
        data: vector<u8>,
        hash: vector<u8>,
        ctx: &mut TxContext,
    ): address {
        let blob = e2e_new_blob(data, hash, ctx);
        let addr = object::uid_address(&blob.id);
        transfer::share_object(blob);
        addr
    }

    /// Replace the payload and bump the version counter.
    public entry fun update_blob(
        blob: &mut DataBlob,
        data: vector<u8>,
        hash: vector<u8>,
    ) {
        blob.data = data;
        blob.hash = hash;
        blob.version = blob.version + 1;
    }

    public fun blob_data(blob: &DataBlob): &vector<u8> {
        &blob.data
    }

    public fun blob_hash(blob: &DataBlob): &vector<u8> {
        &blob.hash
    }

    public fun blob_version(blob: &DataBlob): u64 {
        blob.version
    }

    /// Builds an unshared store owned by the sender.
    fun e2e_new_store(ctx: &mut TxContext): KeyValueStore {
        KeyValueStore {
            id: object::new(ctx),
            owner: tx_context::sender(ctx),
            keys: vector::empty<String>(),
            values: vector::empty<vector<u8>>(),
        }
    }

    /// Builds an unshared version-1 blob.
    fun e2e_new_blob(
        data: vector<u8>, hash: vector<u8>, ctx: &mut TxContext
    ): DataBlob {
        DataBlob {
            id: object::new(ctx),
            data,
            hash,
            version: 1,
        }
    }

    /// Entry wrapper for `create_store`.
    public entry fun e2e_create_store(ctx: &mut TxContext) {
        create_store(ctx);
    }

    /// Entry wrapper for `create_blob`.
    public entry fun e2e_create_blob(
        data: vector<u8>, hash: vector<u8>, ctx: &mut TxContext
    ) {
        create_blob(data, hash, ctx);
    }

    /// Entry wrapper asserting the store size.
    public entry fun e2e_check_store_size(store: &KeyValueStore, expected: u64) {
        assert!(store_size(store) == expected, E_BAD_VALUE);
    }

    /// Entry wrapper asserting a stored value.
    public entry fun e2e_check_get_value(
        store: &KeyValueStore, key: vector<u8>, expected: vector<u8>
    ) {
        let mut found = get_value(store, key);
        assert!(option::is_some(&found), E_BAD_VALUE);
        let stored = option::extract(&mut found);
        assert!(stored == expected, E_BAD_VALUE);
    }

    /// Entry wrapper asserting that a key is absent.
    public entry fun e2e_check_get_value_absent(
        store: &KeyValueStore, key: vector<u8>
    ) {
        assert!(option::is_none(&get_value(store, key)), E_BAD_VALUE);
    }

    /// Entry wrapper asserting the blob version.
    public entry fun e2e_check_blob_version(blob: &DataBlob, expected: u64) {
        assert!(blob_version(blob) == expected, E_BAD_VALUE);
    }

    /// Full CRUD round trip over the store and the blob, asserted in-transaction.
    public entry fun e2e_store_roundtrip(ctx: &mut TxContext) {
        let mut store = e2e_new_store(ctx);
        assert!(store_size(&store) == 0, E_BAD_VALUE);

        set_value(&mut store, b"alpha", b"one", ctx);
        set_value(&mut store, b"beta", b"two", ctx);
        assert!(store_size(&store) == 2, E_BAD_VALUE);

        // Writing an existing key overwrites in place instead of appending.
        set_value(&mut store, b"alpha", b"uno", ctx);
        assert!(store_size(&store) == 2, E_BAD_VALUE);

        let mut found = get_value(&store, b"alpha");
        assert!(option::is_some(&found), E_BAD_VALUE);
        assert!(option::extract(&mut found) == b"uno", E_BAD_VALUE);
        assert!(option::is_none(&get_value(&store, b"missing")), E_BAD_VALUE);

        remove_value(&mut store, b"alpha", ctx);
        assert!(store_size(&store) == 1, E_BAD_VALUE);
        assert!(option::is_none(&get_value(&store, b"alpha")), E_BAD_VALUE);

        // Removing an absent key is a no-op.
        remove_value(&mut store, b"alpha", ctx);
        assert!(store_size(&store) == 1, E_BAD_VALUE);

        let mut blob = e2e_new_blob(b"payload", b"hash0", ctx);
        assert!(blob_version(&blob) == 1, E_BAD_VALUE);
        update_blob(&mut blob, b"payload2", b"hash1");
        assert!(blob_version(&blob) == 2, E_BAD_VALUE);
        assert!(*blob_data(&blob) == b"payload2", E_BAD_VALUE);
        assert!(*blob_hash(&blob) == b"hash1", E_BAD_VALUE);
        update_blob(&mut blob, b"payload3", b"hash2");
        assert!(blob_version(&blob) == 3, E_BAD_VALUE);

        transfer::share_object(store);
        transfer::share_object(blob);
    }

    /// Creates a shared store owned by the sender so non-owner writes abort.
    public entry fun e2e_create_owned_store(ctx: &mut TxContext) {
        let store = e2e_new_store(ctx);
        transfer::share_object(store);
    }
}