use kanari_core::BlockchainEngine;

use super::{configure_consensus_signing_key, load_or_create_p2p_identity, queue_network_message};
use crate::p2p::P2PMessage;

#[test]
fn encrypted_p2p_identity_is_stable_across_restart() {
    let _guard = super::TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let temp = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var(
            "KANARI_NODE_IDENTITY_PASSWORD",
            "correct horse battery staple",
        );
    }

    let first = load_or_create_p2p_identity(temp.path(), "devnet").unwrap();
    let second = load_or_create_p2p_identity(temp.path(), "devnet").unwrap();
    assert_eq!(first.public().to_peer_id(), second.public().to_peer_id());
    let stored = std::fs::read(temp.path().join("p2p-identity.key")).unwrap();
    assert_eq!(stored.first(), Some(&b'{'));

    unsafe {
        std::env::remove_var("KANARI_NODE_IDENTITY_PASSWORD");
    }
}

#[test]
fn mainnet_requires_identity_encryption_password() {
    let _guard = super::TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let temp = tempfile::tempdir().unwrap();
    unsafe {
        std::env::remove_var("KANARI_NODE_IDENTITY_PASSWORD");
        std::env::remove_var("KANARI_CONSENSUS_KEY_PASSWORD");
    }

    let error = load_or_create_p2p_identity(temp.path(), "mainnet").unwrap_err();
    assert!(error.to_string().contains("Mainnet requires"));
    assert!(!temp.path().join("p2p-identity.key").exists());
}

#[test]
fn mainnet_rejects_plaintext_consensus_key_file() {
    let _guard = super::TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let temp = tempfile::tempdir().unwrap();
    let private_key = temp.path().join("private.key");
    std::fs::write(&private_key, "11".repeat(32)).unwrap();
    unsafe {
        std::env::set_var("KANARI_NETWORK", "mainnet");
    }

    let mut engine = BlockchainEngine::new_in_memory().unwrap();
    let error = configure_consensus_signing_key(
        &mut engine,
        &private_key,
        &temp.path().join("unused-public-keys.json"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("Mainnet refuses plaintext"));

    unsafe {
        std::env::remove_var("KANARI_NETWORK");
    }
}

#[test]
fn bounded_outgoing_queue_applies_backpressure() {
    let (sender, _receiver) = tokio::sync::mpsc::channel(1);
    assert!(queue_network_message(
        &sender,
        P2PMessage::CheckpointRequest(1, 1),
        "first"
    ));
    assert!(!queue_network_message(
        &sender,
        P2PMessage::CheckpointRequest(2, 2),
        "full"
    ));
}
