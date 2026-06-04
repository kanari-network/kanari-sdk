use centauri::consensus::{AdaptiveQuorumConfig, DagConsensus, NetworkHealth};

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
fn blackbox_adaptive_quorum_changes_threshold_and_can_be_reverted() {
    let mut consensus = DagConsensus::new("auth1".to_string(), four_authorities());
    let static_quorum = consensus.committee().required_quorum();

    consensus.enable_adaptive_quorum(AdaptiveQuorumConfig::default());
    consensus.update_network_health(NetworkHealth {
        connectivity_ratio: 0.30,
        delivery_success_ratio: 0.40,
        timeout_ratio: 0.50,
        median_latency_ms: 4_000,
    });

    let elevated_quorum = consensus.committee().required_quorum();
    assert!(elevated_quorum > static_quorum);

    consensus.disable_adaptive_quorum();
    assert_eq!(consensus.committee().required_quorum(), static_quorum);
}
