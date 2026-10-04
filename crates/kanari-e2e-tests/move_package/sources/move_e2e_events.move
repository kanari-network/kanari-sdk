module kanari_e2e_tests::move_e2e_events {
    use kanari_system::event;
    use kanari_system::tx_context::{Self, TxContext};

    /// Emitted by `emit_message`, carries the sender and a free-form payload.
    public struct MessageEvent has copy, drop {
        sender: address,
        payload: vector<u8>,
        epoch: u64
    }

    /// Emitted by `bump_counter`, tracks a monotonic counter per transaction.
    public struct CounterEvent has copy, drop {
        counter: u64,
        tag: vector<u8>
    }

    /// Emitted by `emit_batch`, aggregates a payload digest and a size.
    public struct BatchEvent has copy, drop {
        digest: vector<u8>,
        size: u64,
        sender: address
    }

    /// Emit a message event bound to the current sender.
    public entry fun emit_message(payload: vector<u8>, ctx: &mut TxContext) {
        event::emit(
            MessageEvent {
                sender: tx_context::sender(ctx),
                payload,
                epoch: tx_context::epoch(ctx)
            }
        );
    }

    /// Emit a counter event.
    public entry fun bump_counter(counter: u64, tag: vector<u8>) {
        event::emit(CounterEvent { counter, tag });
    }

    /// Emit three events from a single call to exercise batch emission.
    public entry fun emit_batch(payload: vector<u8>, ctx: &mut TxContext) {
        emit_message(payload, ctx);
        bump_counter(1, b"batch");
        event::emit(
            BatchEvent {
                digest: x"0102030405",
                size: 5,
                sender: tx_context::sender(ctx)
            }
        );
    }
}

