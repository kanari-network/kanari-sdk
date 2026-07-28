use std::sync::Mutex;

use kanari_core::BlockchainEngine;

use super::{
    BACKUP_V2_MAGIC, export_validator_backup, export_validator_backup_v1, import_validator_backup,
    safe_restore_file,
};

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
    let (private_key, public_keys, genesis) = write_backup_sources(source.path());
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
    let backup_bytes = std::fs::read(&backup).unwrap();
    assert!(backup_bytes.starts_with(BACKUP_V2_MAGIC));
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

#[test]
fn corrupted_streaming_validator_backup_is_rejected() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    unsafe {
        std::env::set_var(
            "KANARI_VALIDATOR_BACKUP_PASSWORD",
            "backup password for regression test",
        );
    }

    let source = tempfile::tempdir().unwrap();
    let (private_key, public_keys, genesis) = write_backup_sources(source.path());
    let backup = source.path().join("validator-backup-v2.bin");
    let engine = BlockchainEngine::new_in_memory().unwrap();
    export_validator_backup(
        &engine,
        "devnet",
        source.path(),
        &private_key,
        &public_keys,
        &genesis,
        &backup,
    )
    .unwrap();

    let mut bytes = std::fs::read(&backup).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x55;
    let corrupted = source.path().join("validator-backup-v2-corrupt.bin");
    std::fs::write(&corrupted, bytes).unwrap();

    let restored_data = tempfile::tempdir().unwrap();
    let restored_recovery = tempfile::tempdir().unwrap();
    assert!(
        import_validator_backup(
            &corrupted,
            "devnet",
            restored_data.path(),
            restored_recovery.path(),
        )
        .is_err()
    );

    unsafe {
        std::env::remove_var("KANARI_VALIDATOR_BACKUP_PASSWORD");
    }
}

#[test]
fn legacy_v1_validator_backup_still_imports() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    unsafe {
        std::env::set_var(
            "KANARI_VALIDATOR_BACKUP_PASSWORD",
            "backup password for regression test",
        );
    }

    let source = tempfile::tempdir().unwrap();
    let (private_key, public_keys, genesis) = write_backup_sources(source.path());
    let backup = source.path().join("validator-backup-v1.json");
    let engine = BlockchainEngine::new_in_memory().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let snapshot_path = temp.path().join("state-snapshot.json");
    let snapshot = engine
        .export_state_snapshot(&snapshot_path, "devnet")
        .unwrap();
    let files =
        super::required_backup_files(source.path(), &private_key, &public_keys, &genesis).unwrap();
    export_validator_backup_v1(
        "devnet",
        &backup,
        "backup password for regression test",
        &snapshot,
        &snapshot_path,
        &files,
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

    assert_eq!(snapshot.checkpoint_height, imported.checkpoint_height);
    assert_eq!(
        std::fs::read(restored_recovery.path().join("genesis.json")).unwrap(),
        b"genesis"
    );

    unsafe {
        std::env::remove_var("KANARI_VALIDATOR_BACKUP_PASSWORD");
    }
}

fn write_backup_sources(
    source: &std::path::Path,
) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    std::fs::write(source.join("p2p-identity.key"), b"identity").unwrap();
    std::fs::write(source.join("mysticeti-0.wal"), b"wal").unwrap();
    let private_key = source.join("private.key");
    let public_keys = source.join("public.json");
    let genesis = source.join("genesis.json");
    std::fs::write(&private_key, b"private").unwrap();
    std::fs::write(&public_keys, b"public").unwrap();
    std::fs::write(&genesis, b"genesis").unwrap();
    (private_key, public_keys, genesis)
}
