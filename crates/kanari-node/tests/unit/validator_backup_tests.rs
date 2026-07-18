use std::sync::Mutex;

use kanari_core::BlockchainEngine;

use super::{export_validator_backup, import_validator_backup, safe_restore_file};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn restore_rejects_path_traversal() {
    let temp = tempfile::tempdir().unwrap();
    assert!(safe_restore_file(temp.path(), "../secret", b"no").is_err());
    assert!(safe_restore_file(temp.path(), "/absolute", b"no").is_err());
}

#[test]
fn encrypted_validator_backup_round_trips_all_recovery_material() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    unsafe {
        std::env::set_var(
            "KANARI_VALIDATOR_BACKUP_PASSWORD",
            "backup password for regression test",
        );
    }

    let source = tempfile::tempdir().unwrap();
    std::fs::write(source.path().join("p2p-identity.key"), b"identity").unwrap();
    std::fs::write(source.path().join("mysticeti-0.wal"), b"wal").unwrap();
    let private_key = source.path().join("private.key");
    let public_keys = source.path().join("public.json");
    let genesis = source.path().join("genesis.json");
    std::fs::write(&private_key, b"private").unwrap();
    std::fs::write(&public_keys, b"public").unwrap();
    std::fs::write(&genesis, b"genesis").unwrap();
    let backup = source.path().join("validator-backup.json");
    let engine = BlockchainEngine::new_in_memory().unwrap();

    let exported = export_validator_backup(
        &engine,
        "devnet",
        source.path(),
        &private_key,
        &public_keys,
        &genesis,
        &backup,
    )
    .unwrap();
    let restored_data = tempfile::tempdir().unwrap();
    let restored_recovery = tempfile::tempdir().unwrap();
    let imported = import_validator_backup(
        &backup,
        "devnet",
        restored_data.path(),
        restored_recovery.path(),
    )
    .unwrap();

    assert_eq!(exported.checkpoint_height, imported.checkpoint_height);
    assert_eq!(exported.state_root, imported.state_root);
    assert_eq!(
        std::fs::read(restored_data.path().join("mysticeti-0.wal")).unwrap(),
        b"wal"
    );
    assert_eq!(
        std::fs::read(restored_recovery.path().join("private-key.key")).unwrap(),
        b"private"
    );

    unsafe {
        std::env::remove_var("KANARI_VALIDATOR_BACKUP_PASSWORD");
    }
}
