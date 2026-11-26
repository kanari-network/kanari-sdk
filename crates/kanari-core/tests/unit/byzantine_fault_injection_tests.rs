// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0
//
// Focused Byzantine fault-injection regressions for kanari-core.
//
// This suite deliberately targets guards that are not already exercised by
// `attack_tests.rs` or the top-level DAG tests: tampered checkpoint state roots
// paired with signed DAG evidence, persistent nonce-watermark replay with a new
// payload, duplicated committed-index entries that could inflate history counts,
// genuinely signed same-author/same-round DAG equivocation, and a peer-authority
// policy map that must never rewrite the consensus committee. All cases are
// fail-closed: invalid input is rejected, or consensus/state is left unchanged.

use super::*;
use kanari_move_runtime_v2::storage::persistent_store::PersistentStore;
use std::collections::BTreeMap;

/// A checkpoint whose state root is tampered after its signed DAG evidence was
/// produced must be rejected by both sync and apply paths without mutating the
/// committed chain.
#[test]
fn byzantine_tampered_checkpoint_state_root_and_certificate_are_rejected() {
    let mut engine = BlockchainEngine::new_in_memory().unwrap();
    let authorities = vec!["auth1".to_string(), "auth2".to_string()];
    let (local_key, public_keys) = secure_consensus_keys(&authorities, "auth1");
    engine.set_authorities("auth1".to_string(), authorities);
    engine.set_consensus_signing_key(local_key, public_keys).unwrap();

    let prev_hash = engine.blockchain_read().latest_checkpoint().hash().unwrap();
    let valid_checkpoint = Checkpoint::new(
        1,
        Vec::new(),
        Vec::<SignedTransaction>::new(),
        vec![5u8; 32],
        1,
        prev_hash,
    );
    engine
        .blockchain_write()
        .add_checkpoint_with_validation(valid_checkpoint.clone(), true)
        .unwrap();
    assert_eq!(engine.get_stats().height, 1);
    let committed_hash = engine.blockchain_read().latest_checkpoint().hash().unwrap();

    // The attacker keeps a genuine signed vertex (the certificate) but swaps the
    // checkpoint state root, changing the checkpoint hash out from under it.
    let mut tampered = valid_checkpoint;
    tampered.state_root = vec![0xde; 32];
    let certificate = signed_network_vertex("auth1", &authority_key(11), 1, vec![]);
    let sync_data =
        CheckpointSyncData { checkpoint: tampered.clone(), dag_vertices: vec![certificate] };

    let err = engine.sync_checkpoint_from_data(&sync_data).unwrap_err();
    assert!(err.to_string().contains("Conflicting checkpoint"), "{err:#}");
    assert_eq!(engine.blockchain_read().latest_checkpoint().hash().unwrap(), committed_hash);
    assert_eq!(engine.get_stats().height, 1);

    // A clean node must also refuse to apply the forged state root.
    let fresh = BlockchainEngine::new_in_memory().unwrap();
    let err = fresh.apply_checkpoint(tampered).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("state root mismatch"), "{err:#}");
    assert_eq!(fresh.get_stats().height, 0);
}

/// A committed nonce watermark must reject a different payload reusing an
/// already committed nonce, while still admitting a strictly newer nonce.
#[test]
fn byzantine_stale_nonce_watermark_rejects_replayed_payload() {
    let mut engine = BlockchainEngine::new_in_memory().unwrap();
    let store = Arc::new(PersistentStore::open_in_memory().unwrap());
    engine.persistent_store = Some(store.clone());

    let sender = generate_keypair(CurveType::Ed25519).unwrap();
    let recipient = generate_keypair(CurveType::Ed25519).unwrap().address;
    for (object_id, balance) in
        [("0xaaaa", 1_000_000), ("0x1000", 1_000_000), ("0x1001", 1_000_000)]
    {
        fund_sender_with_coin(&engine, &sender.address, object_id, balance);
    }

    let committed =
        signed_transfer_with_refs(&sender, &recipient, "0xaaaa", 1_000_000, "0x1000", 1_000_000, 7);
    let checkpoint =
        Checkpoint::new(1, vec![[2u8; 32]], vec![committed], vec![7u8; 32], 1, vec![0u8; 32]);
    BlockchainEngine::persist_checkpoint_transactions(&store, &checkpoint).unwrap();

    // Same sender and nonce, different gas object => different transaction hash.
    let replay_equal_nonce =
        signed_transfer_with_refs(&sender, &recipient, "0xaaaa", 1_000_000, "0x1001", 1_000_000, 7);
    let err = engine.submit_transactions_batch(vec![replay_equal_nonce]).unwrap_err();
    assert!(err.to_string().contains("is stale"), "{err:#}");

    let replay_lower_nonce =
        signed_transfer_with_refs(&sender, &recipient, "0xaaaa", 1_000_000, "0x1001", 1_000_000, 6);
    let err = engine.submit_transactions_batch(vec![replay_lower_nonce]).unwrap_err();
    assert!(err.to_string().contains("is stale"), "{err:#}");

    assert_eq!(engine.pending_transaction_len(), 0);

    let newer_nonce =
        signed_transfer_with_refs(&sender, &recipient, "0xaaaa", 1_000_000, "0x1001", 1_000_000, 8);
    engine.submit_transactions_batch(vec![newer_nonce]).unwrap();
    assert_eq!(engine.pending_transaction_len(), 1);
}

/// A corrupted recent-transaction index containing duplicate hashes must not
/// inflate the committed-transaction count returned by history queries.
#[test]
fn byzantine_duplicate_committed_index_entries_cannot_inflate_counts() {
    let mut engine = BlockchainEngine::new_in_memory().unwrap();
    let store = Arc::new(PersistentStore::open_in_memory().unwrap());
    engine.persistent_store = Some(store.clone());

    let tx_a = signed_transfer(0);
    let tx_b = signed_transfer(0);
    let hash_a = tx_a.transaction_hash().to_vec();
    let hash_b = tx_b.transaction_hash().to_vec();

    let checkpoint =
        Checkpoint::new(1, vec![[3u8; 32]], vec![tx_a, tx_b], vec![9u8; 32], 1, vec![0u8; 32]);
    BlockchainEngine::persist_checkpoint_transactions(&store, &checkpoint).unwrap();

    // Inject the same committed hashes twice through the durable recent index.
    store
        .save(
            BlockchainEngine::recent_transaction_hashes_key(),
            &vec![hash_a.clone(), hash_a.clone(), hash_b.clone(), hash_b.clone()],
        )
        .unwrap();

    assert!(engine.try_is_transaction_committed(&hash_a).unwrap());
    assert!(engine.try_is_transaction_committed(&hash_b).unwrap());

    let listed = engine.list_committed_transactions_from_history(usize::MAX, None, |_| true);
    assert_eq!(listed.len(), 2, "duplicated index rows must be de-duplicated");

    let mut actual =
        listed.iter().map(|(tx, _, _)| tx.transaction_hash().to_vec()).collect::<Vec<_>>();
    actual.sort();
    let mut expected = vec![hash_a, hash_b];
    expected.sort();
    assert_eq!(actual, expected);
    assert_eq!(engine.get_stats().height, 0, "index queries must not advance the chain");
}

/// Two validly signed vertices from one authority at one round are an
/// equivocation. Mysticeti may retain both twins, but a lone equivocating
/// validator must never be able to finalize a kanari checkpoint.
#[test]
fn byzantine_signed_equivocation_cannot_finalize_a_checkpoint() {
    let (engine, dag_engine, remote_key) =
        build_test_dag_engine(vec!["auth1".to_string(), "auth2".to_string()], "auth1");

    let first = signed_network_vertex("auth2", &remote_key, 1, vec![]);
    let twin = signed_network_vertex("auth2", &remote_key, 1, vec![]);
    assert_eq!(first.author, twin.author);
    assert_eq!(first.round, twin.round);
    assert_ne!(first.id, twin.id, "twins must be distinct signed blocks");

    dag_engine.add_network_vertex(first).unwrap();
    // A genuine twin is not a malformed block; the safety assertion is that
    // accepting it into the DAG cannot by itself commit any checkpoint.
    let _ = dag_engine.add_network_vertex(twin);

    assert_eq!(engine.get_stats().height, 0);
    assert!(engine.blockchain_read().get_checkpoint(1).is_none());
}

/// A peer-authority map is transport policy only. Even a map that assigns many
/// peers to one authority (or an unknown peer) must leave the consensus
/// committee and quorum untouched; deriving a committee from those policy
/// values is rejected.
#[test]
fn byzantine_peer_policy_map_cannot_rewrite_committee_or_quorum() {
    let authorities = vec!["auth1".to_string(), "auth2".to_string(), "auth3".to_string()];
    let (local_key, public_keys) = secure_consensus_keys(&authorities, "auth1");
    let engine = Arc::new(BlockchainEngine::new_in_memory().unwrap());
    let original = DagEngine::new_secure(
        engine.clone(),
        "auth1".to_string(),
        authorities.clone(),
        local_key.clone(),
        public_keys.clone(),
    )
    .unwrap();
    let baseline = original.production_policy();

    // Attacker-controlled peer policy: two peers claim the same authority.
    let policy = BTreeMap::from([
        ("peer-a".to_string(), "auth1".to_string()),
        ("peer-b".to_string(), "auth1".to_string()),
        ("peer-c".to_string(), "auth2".to_string()),
    ]);
    assert_eq!(policy.values().collect::<std::collections::BTreeSet<_>>().len(), 2);

    let policy_authorities = policy.values().cloned().collect::<Vec<_>>();
    let forged = DagEngine::new_secure(
        engine,
        "auth1".to_string(),
        policy_authorities,
        local_key,
        public_keys,
    );
    let err = match forged {
        Ok(_) => panic!("committee derived from a peer policy map must be rejected"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("duplicates"), "{err:#}");

    let after = original.production_policy();
    assert_eq!(after.quorum_size, baseline.quorum_size);
    assert_eq!(after.missing_parent_authors, baseline.missing_parent_authors);
    // Genesis blocks are retained for every committee member, so the committee
    // is observable as the parent-author set and must remain exactly as configured.
    assert_eq!(baseline.parent_authors, authorities);
    assert_eq!(after.parent_authors, authorities);
}
