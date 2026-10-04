module kanari_e2e_tests::move_e2e_events {
    use kanari_system::event;
    use kanari_system::tx_context::{Self, TxContext};

    struct SimpleEvent has copy, drop {
        sender: address,
        message: vector<u8>,
        timestamp: u64
    }

    struct CounterEvent has copy, drop {
        counter: u64,
        action: vector<u8>
    }

    struct DataEvent has copy, drop {
        data_hash: vector<u8>,
        size: u64
    }

    public fun emit_simple(message: vector<u8>, ctx: &mut TxContext) {
        event::emit(
            SimpleEvent {
                sender: tx_context::sender(ctx),
                message,
                timestamp: tx_context::epoch(ctx)
            }
        );
    }

    public fun emit_counter(counter: u64, action: vector<u8>) {
        event::emit(CounterEvent { counter, action });
    }

    public fun emit_data(data_hash: vector<u8>, size: u64) {
        event::emit(DataEvent { data_hash, size });
    }

    public fun emit_batch(message: vector<u8>, ctx: &mut TxContext) {
        emit_simple(message, ctx);
        emit_counter(1, b"batch");
        emit_data(b"hash123", 100);
    }
}

