use centauri::consensus::{AdaptiveQuorumConfig, DagConsensus, NetworkHealth, ShardId, ShardedDag};

fn four_authorities() -> Vec<String> {
    vec![
        "auth1".to_string(),
        "auth2".to_string(),
        "auth3".to_string(),
        "auth4".to_string(),
    ]
}

fn single_authority() -> Vec<String> {
    vec!["auth1".to_string()]
}

fn routing_key_for_target(dag: &ShardedDag, target: ShardId) -> Vec<u8> {
    for candidate in 0u32..10_000 {
        let key = format!("integration-route-key-{candidate}");
        if dag.route_payload(key.as_bytes()) == target {
            return key.into_bytes();
        }
    }
    panic!("failed to find routing key for target shard {target}");
}

fn advance_consensus_to_checkpoint_one(consensus: &mut DagConsensus) {
    let round1 = consensus.create_vertex(vec![], vec![1u8; 32], 1).unwrap();
    consensus.add_vertex(round1).unwrap();

    let round2 = consensus.create_vertex(vec![], vec![2u8; 32], 2).unwrap();
    consensus.add_vertex(round2).unwrap();

    let round3 = consensus.create_vertex(vec![], vec![3u8; 32], 3).unwrap();
    consensus.add_vertex(round3).unwrap();

    let checkpoint = consensus
        .try_commit()
        .unwrap()
        .expect("checkpoint should be produced");
    consensus.add_checkpoint(checkpoint).unwrap();
}

#[test]
fn blackbox_consensus_state_roundtrip_preserves_progress_and_allows_further_commits() {
    let mut original = DagConsensus::new("auth1".to_string(), single_authority());
    advance_consensus_to_checkpoint_one(&mut original);

    let saved = original.save_state().unwrap();
    assert_eq!(saved.current_round, 3);
    assert_eq!(saved.last_checkpoint_round, 3);

    let mut restored = DagConsensus::new("auth1".to_string(), single_authority());
    restored.load_state(saved).unwrap();

    assert_eq!(restored.latest_checkpoint().sequence, 1);
    assert_eq!(restored.store().current_round(), 3);
    assert!(!restored.store().get_vertex_ids_in_round(1).is_empty());

    let round4 = restored.create_vertex(vec![], vec![4u8; 32], 4).unwrap();
    restored.add_vertex(round4).unwrap();
    assert_eq!(restored.store().current_round(), 4);
}

#[test]
fn blackbox_sharded_cross_shard_delivery_uses_latest_checkpoint_context() {
    let mut source = ShardedDag::new(0, 2, "auth1".to_string(), single_authority()).unwrap();
    let mut target = ShardedDag::new(1, 2, "auth1".to_string(), single_authority()).unwrap();

    advance_consensus_to_checkpoint_one(source.local_dag_mut());
    let source_checkpoint = source.local_dag().latest_checkpoint();
    let routing_key = routing_key_for_target(&source, 1);

    source
        .submit_payload(&routing_key, b"blackbox-cross-shard".to_vec())
        .unwrap();
    let drained = source.drain_outbound_for(1);
    assert_eq!(drained.len(), 1);

    let message = drained.into_iter().next().unwrap();
    assert_eq!(
        message.proof.checkpoint_sequence,
        source_checkpoint.sequence
    );
    assert_eq!(
        message.proof.checkpoint_hash,
        source_checkpoint.hash().unwrap()
    );

    target.receive_message(message.clone()).unwrap();
    assert_eq!(target.pop_inbound(), Some(message));
}

#[test]
fn blackbox_adaptive_quorum_changes_sharded_dag_threshold_and_can_be_reverted() {
    let mut dag = ShardedDag::new(0, 2, "auth1".to_string(), four_authorities()).unwrap();
    let static_quorum = dag.local_dag().committee().required_quorum();

    dag.enable_adaptive_quorum(AdaptiveQuorumConfig::default());
    dag.update_network_health(NetworkHealth {
        connectivity_ratio: 0.30,
        delivery_success_ratio: 0.40,
        timeout_ratio: 0.50,
        median_latency_ms: 4_000,
    });

    let elevated_quorum = dag.local_dag().committee().required_quorum();
    assert!(elevated_quorum > static_quorum);

    dag.local_dag_mut().disable_adaptive_quorum();
    assert_eq!(dag.local_dag().committee().required_quorum(), static_quorum);
}
