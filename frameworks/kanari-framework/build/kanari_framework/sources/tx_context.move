module kanari_framework::tx_context {
    use std::vector;

    const TX_HASH_LENGTH: u64 = 32;
    const EBadTxHashLength: u64 = 0;
    const ENoIDsCreated: u64 = 1;

    struct TxContext has drop {
        sender: address,
        tx_hash: vector<u8>,
        epoch: u64, 
        epoch_timestamp_ms: u64,
        ids_created: u64,
    }

    public fun sender(self: &TxContext): address {
        self.sender
    }

    public fun digest(self: &TxContext): &vector<u8> {
        &self.tx_hash
    }

    public fun epoch(self: &TxContext): u64 {
        self.epoch
    }

    public fun epoch_timestamp_ms(self: &TxContext): u64 {
        self.epoch_timestamp_ms
    }

    public fun fresh_object_address(ctx: &mut TxContext): address {
        let ids_created = ctx.ids_created;
        let id = derive_id(*&ctx.tx_hash, ids_created);
        ctx.ids_created = ids_created + 1;
        id
    }

    native fun derive_id(tx_hash: vector<u8>, ids_created: u64): address;

    #[test_only]
    public fun new(
        sender: address, 
        tx_hash: vector<u8>,
        epoch: u64,
        epoch_timestamp_ms: u64,
        ids_created: u64,
    ): TxContext {
        assert!(vector::length(&tx_hash) == TX_HASH_LENGTH, EBadTxHashLength);
        TxContext { sender, tx_hash, epoch, epoch_timestamp_ms, ids_created }
    }

    #[test_only]
    public fun dummy(): TxContext {
        let tx_hash = x"3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532";
        new(@0x0, tx_hash, 0, 0, 0)
    }
}