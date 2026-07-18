use super::{encrypt_existing_consensus_key, validate_start_authority_config};

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
fn existing_consensus_seed_encrypts_without_key_rotation() {
    let _guard = crate::app::TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("private.hex");
    let output = temp.path().join("private.key");
    let seed = "42".repeat(32);
    std::fs::write(&input, &seed).unwrap();
    unsafe {
        std::env::set_var(
            "KANARI_CONSENSUS_KEY_PASSWORD",
            "migration regression password",
        );
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
