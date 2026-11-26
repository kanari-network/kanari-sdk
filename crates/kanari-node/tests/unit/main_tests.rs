use super::{
    encrypt_existing_consensus_key, load_peer_authority_map, p2p_peer_id_for_data_dir,
    require_peer_authority_map, validate_start_authority_config,
};
use crate::NetworkMode;
use libp2p_identity::Keypair;

fn test_peer_id() -> String {
    Keypair::generate_ed25519().public().to_peer_id().to_string()
}

#[test]
fn start_requires_both_authority_fields() {
    assert!(validate_start_authority_config(&None, &None).is_err());
    assert!(validate_start_authority_config(&Some("0x1".to_string()), &None).is_err());
    assert!(validate_start_authority_config(&None, &Some(vec!["0x1".to_string()])).is_err());
}

#[test]
fn start_rejects_empty_authority_list() {
    assert!(validate_start_authority_config(&Some("0x1".to_string()), &Some(vec![])).is_err());
}

#[test]
fn start_accepts_complete_authority_config() {
    assert!(
        validate_start_authority_config(
            &Some("0x1".to_string()),
            &Some(vec!["0x1".to_string(), "0x2".to_string()]),
        )
        .is_ok()
    );
}

#[test]
fn peer_authority_entries_are_canonicalized_and_validated() {
    let peer = test_peer_id();
    let authorities = vec!["0x1".to_string(), "0x2".to_string()];
    let entries = vec![format!("{peer}=1"), format!("{}=0x2", test_peer_id())];

    let map = load_peer_authority_map(Some(&entries), None, &authorities).unwrap();
    assert_eq!(map.get(&peer).map(String::as_str), Some("0x1"));
    assert_eq!(map.len(), 2);
}

#[test]
fn peer_authority_rejects_invalid_peer_and_unknown_authority() {
    let authorities = vec!["0x1".to_string()];
    assert!(
        load_peer_authority_map(Some(&vec!["not-a-peer=0x1".into()]), None, &authorities).is_err()
    );
    assert!(load_peer_authority_map(
        Some(&vec![format!("{}=0x9", test_peer_id())]),
        None,
        &authorities,
    )
    .is_err());
}

#[test]
fn peer_authority_rejects_duplicate_authority_mapping() {
    let authorities = vec!["0x1".to_string()];
    let entries = vec![format!("{}=0x1", test_peer_id()), format!("{}=1", test_peer_id())];
    assert!(load_peer_authority_map(Some(&entries), None, &authorities).is_err());
}

#[test]
fn validator_start_rejects_an_empty_peer_authority_map() {
    assert!(require_peer_authority_map(&Default::default()).is_err());
    let map = std::collections::BTreeMap::from([("peer".to_string(), "0x1".to_string())]);
    assert!(require_peer_authority_map(&map).is_ok());
}

#[test]
fn p2p_peer_id_is_stable_for_a_data_dir() {
    // Other tests in this binary mutate the identity-password environment.
    // Without this lock, the first call can write an encrypted identity while
    // another test removes the password before the second call reads it.
    let _guard = crate::app::TEST_ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    unsafe {
        std::env::remove_var("KANARI_NODE_IDENTITY_PASSWORD");
        std::env::remove_var("KANARI_CONSENSUS_KEY_PASSWORD");
    }

    let temp = tempfile::tempdir().unwrap();
    let data_dir = Some(temp.path().to_path_buf());
    let first = p2p_peer_id_for_data_dir(&data_dir, &NetworkMode::Testnet).unwrap();
    let second = p2p_peer_id_for_data_dir(&data_dir, &NetworkMode::Testnet).unwrap();
    assert_eq!(first, second);
    assert!(first.starts_with("12D3Koo"));
    assert!(temp.path().join("p2p-identity.key").exists());
}

#[test]
fn existing_consensus_seed_encrypts_without_key_rotation() {
    let _guard = crate::app::TEST_ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("private.hex");
    let output = temp.path().join("private.key");
    let seed = "42".repeat(32);
    std::fs::write(&input, &seed).unwrap();
    unsafe {
        std::env::set_var("KANARI_CONSENSUS_KEY_PASSWORD", "migration regression password");
    }

    encrypt_existing_consensus_key(&input, &output, false).unwrap();
    let encrypted: kanari_crypto::EncryptedData =
        serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    let decrypted =
        kanari_crypto::decrypt_string(&encrypted, "migration regression password").unwrap();
    assert_eq!(decrypted, seed);

    unsafe {
        std::env::remove_var("KANARI_CONSENSUS_KEY_PASSWORD");
    }
}
