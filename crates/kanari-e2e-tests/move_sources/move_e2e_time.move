module kanari_e2e_tests::move_e2e_time {
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::clock::{Self, Clock};

    struct TimestampRecord has key {
        id: kanari_system::object::UID,
        created_at: u64,
        last_updated: u64,
        owner: address,
    }

    public fun create_record(clock: &Clock, ctx: &mut TxContext): address {
        let record = TimestampRecord {
            id: kanari_system::object::new(ctx),
            created_at: clock::timestamp_ms(clock),
            last_updated: clock::timestamp_ms(clock),
            owner: tx_context::sender(ctx),
        };
        let addr = kanari_system::object::uid_to_address(&record.id);
        kanari_system::transfer::share_object(record);
        addr
    }

    public fun update_record(record: &mut TimestampRecord, clock: &Clock, ctx: &mut TxContext) {
        assert!(record.owner == tx_context::sender(ctx), 400);
        record.last_updated = clock::timestamp_ms(clock);
    }

    public fun get_created_at(record: &TimestampRecord): u64 {
        record.created_at
    }

    public fun get_last_updated(record: &TimestampRecord): u64 {
        record.last_updated
    }

    public fun is_fresh(record: &TimestampRecord, clock: &Clock, threshold_ms: u64): bool {
        clock::timestamp_ms(clock) - record.last_updated <= threshold_ms
    }
}
