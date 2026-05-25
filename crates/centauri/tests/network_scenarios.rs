mod support;

use support::simulation::MultiNodeSimulationHarness;

#[test]
fn network_partition_buffers_cross_shard_messages_until_route_heals() {
    let mut harness = MultiNodeSimulationHarness::new(2);
    harness.isolate_route(0, 1);

    let sent = harness.send_cross_shard(0, 1, b"partitioned-message".to_vec());
    assert_eq!(harness.delayed_message_count(), 1);
    assert!(harness.drain_inbound(1).is_empty());

    harness.heal_route(0, 1);
    assert_eq!(harness.flush_delayed(), 1);
    assert_eq!(harness.delayed_message_count(), 0);

    let delivered = harness.drain_inbound(1);
    assert_eq!(delivered, vec![sent]);
}

#[test]
fn delayed_delivery_preserves_checkpoint_context_created_before_flush() {
    let mut harness = MultiNodeSimulationHarness::new(2);
    harness.advance_to_checkpoint(0);
    let source_checkpoint = harness.shard(0).local_dag().latest_checkpoint();

    harness.isolate_route(0, 1);
    let message = harness.send_cross_shard(0, 1, b"checkpoint-buffered".to_vec());

    assert_eq!(
        message.proof.checkpoint_sequence,
        source_checkpoint.sequence
    );
    assert_eq!(
        message.proof.checkpoint_hash,
        source_checkpoint.hash().unwrap()
    );

    harness.heal_route(0, 1);
    assert_eq!(harness.flush_delayed(), 1);

    let delivered = harness.drain_inbound(1);
    assert_eq!(delivered, vec![message]);
}

#[test]
fn mixed_shard_progress_allows_messages_with_different_checkpoint_heights() {
    let mut harness = MultiNodeSimulationHarness::new(3);
    harness.advance_to_checkpoint(0);
    harness.advance_to_checkpoint(2);

    let from_checkpointed = harness.send_cross_shard(0, 1, b"from-checkpointed".to_vec());
    let from_genesis = harness.send_cross_shard(1, 2, b"from-genesis".to_vec());

    assert_eq!(from_checkpointed.proof.checkpoint_sequence, 1);
    assert_eq!(from_genesis.proof.checkpoint_sequence, 0);

    let shard_one_messages = harness.drain_inbound(1);
    assert_eq!(shard_one_messages, vec![from_checkpointed]);

    let shard_two_messages = harness.drain_inbound(2);
    assert_eq!(shard_two_messages, vec![from_genesis]);
}
