module kanari_e2e_tests::move_e2e_time {
    use kanari_system::clock;
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    const E_NOT_OWNER: u64 = 400;
    const E_BAD_CLOCK: u64 = 401;
    const E_BAD_TIME: u64 = 402;

    /// Timestamp record seeded from the transaction context and refreshed from the Clock.
    public struct TimestampRecord has key, store {
        id: UID,
        created_at: u64,
        last_updated: u64,
        owner: address
    }

    /// Shared address of the protocol `Clock` object.
    public fun clock_address(): address {
        @0x6
    }

    /// Create a shared timestamp record.
    public fun create_record(ctx: &mut TxContext): address {
        let now = tx_context::epoch_timestamp_ms(ctx);
        let record = TimestampRecord {
            id: object::new(ctx),
            created_at: now,
            last_updated: now,
            owner: tx_context::sender(ctx)
        };
        let addr = object::uid_address(&record.id);
        transfer::share_object(record);
        addr
    }

    /// Owner-only refresh of the `last_updated` field from an explicit clock value.
    public entry fun update_record(
        record: &mut TimestampRecord, now_ms: u64, ctx: &mut TxContext
    ) {
        assert!(record.owner == tx_context::sender(ctx), E_NOT_OWNER);
        record.last_updated = now_ms;
    }

    /// Read the protocol clock through its shared address.
    public fun now_ms(): u64 {
        clock::timestamp_ms_by_address(clock_address())
    }

    /// True when the record was refreshed within `threshold_ms`.
    public fun is_fresh(
        record: &TimestampRecord, now_ms: u64, threshold_ms: u64
    ): bool {
        now_ms >= record.last_updated && now_ms - record.last_updated <= threshold_ms
    }

    public fun created_at(record: &TimestampRecord): u64 {
        record.created_at
    }

    public fun last_updated(record: &TimestampRecord): u64 {
        record.last_updated
    }

    /// Entry wrapper for `create_record`.
    public entry fun e2e_create_record(ctx: &mut TxContext) {
        create_record(ctx);
    }

    /// Entry wrapper asserting the well-known protocol clock address.
    public entry fun e2e_check_clock_address() {
        assert!(clock_address() == @0x6, E_BAD_CLOCK);
    }

    /// Entry wrapper asserting the last-updated timestamp.
    public entry fun e2e_check_last_updated(
        record: &TimestampRecord, expected: u64
    ) {
        assert!(last_updated(record) == expected, E_BAD_TIME);
    }

    /// Entry wrapper asserting the creation timestamp.
    public entry fun e2e_check_created_at(
        record: &TimestampRecord, expected: u64
    ) {
        assert!(created_at(record) == expected, E_BAD_TIME);
    }

    /// Entry wrapper asserting the freshness predicate.
    public entry fun e2e_check_is_fresh(
        record: &TimestampRecord, now_ms: u64, threshold_ms: u64, expected: bool
    ) {
        assert!(
            is_fresh(record, now_ms, threshold_ms) == expected,
            E_BAD_TIME
        );
    }
}

