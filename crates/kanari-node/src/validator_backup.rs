// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use kanari_core::{BlockchainEngine, engine::StateSnapshot};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use zeroize::Zeroizing;

const BACKUP_FORMAT_VERSION: u32 = 1;
const BACKUP_PASSWORD_ENV: &str = "KANARI_VALIDATOR_BACKUP_PASSWORD";

#[derive(Debug, Serialize, Deserialize)]
struct ValidatorBackupPayload {
    format_version: u32,
    network: String,
    created_unix_seconds: u64,
    state_snapshot: Vec<u8>,
    files: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedValidatorBackup {
    format_version: u32,
    payload_sha3_256: String,
    encrypted_payload: kanari_crypto::EncryptedData,
}

#[derive(Debug)]
pub struct ValidatorBackupSummary {
    pub checkpoint_height: u64,
    pub state_root: String,
    pub included_files: usize,
}

fn backup_password() -> Result<Zeroizing<String>> {
    let password = Zeroizing::new(std::env::var(BACKUP_PASSWORD_ENV).map_err(|_| {
        anyhow::anyhow!("{BACKUP_PASSWORD_ENV} is required for validator backup and restore")
    })?);
    if password.len() < 12 {
        anyhow::bail!("{BACKUP_PASSWORD_ENV} must contain at least 12 characters");
    }
    Ok(password)
}

fn add_required_file(
    files: &mut BTreeMap<String, Vec<u8>>,
    archive_name: &str,
    source: &Path,
) -> Result<()> {
    let bytes = fs::read(source)
        .with_context(|| format!("Failed to read required backup file {}", source.display()))?;
    files.insert(archive_name.to_string(), bytes);
    Ok(())
}

pub fn export_validator_backup(
    engine: &BlockchainEngine,
    network: &str,
    data_dir: &Path,
    consensus_private_key: &Path,
    consensus_public_keys: &Path,
    genesis: &Path,
    output: &Path,
) -> Result<ValidatorBackupSummary> {
    if output.exists() {
        anyhow::bail!(
            "Refusing to overwrite validator backup: {}",
            output.display()
        );
    }
    let password = backup_password()?;
    let temp_dir = tempfile::tempdir()?;
    let snapshot_path = temp_dir.path().join("state-snapshot.json");
    let snapshot = engine.export_state_snapshot(&snapshot_path, network)?;
    let state_snapshot = fs::read(&snapshot_path)?;

    let mut files = BTreeMap::new();
    add_required_file(
        &mut files,
        "identity/p2p-identity.key",
        &data_dir.join("p2p-identity.key"),
    )?;
    add_required_file(
        &mut files,
        "consensus/private-key.key",
        consensus_private_key,
    )?;
    add_required_file(
        &mut files,
        "consensus/public-keys.json",
        consensus_public_keys,
    )?;
    add_required_file(&mut files, "network/genesis.json", genesis)?;

    for entry in fs::read_dir(data_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("mysticeti-") && name.ends_with(".wal") {
            files.insert(format!("wal/{name}"), fs::read(entry.path())?);
        }
    }

    let payload = ValidatorBackupPayload {
        format_version: BACKUP_FORMAT_VERSION,
        network: network.to_string(),
        created_unix_seconds: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        state_snapshot,
        files,
    };
    let encoded = bincode::serde::encode_to_vec(&payload, bincode::config::standard())?;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&encoded)?;
    let compressed = Zeroizing::new(encoder.finish()?);
    let payload_sha3_256 = hex::encode(Sha3_256::digest(&compressed));
    let encrypted_payload = kanari_crypto::encrypt_data(&compressed, password.as_str())
        .map_err(|error| anyhow::anyhow!("Failed to encrypt validator backup: {error}"))?;
    let archive = EncryptedValidatorBackup {
        format_version: BACKUP_FORMAT_VERSION,
        payload_sha3_256,
        encrypted_payload,
    };

    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let temporary = output.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(&archive)?)?;
    fs::rename(&temporary, output)?;

    Ok(ValidatorBackupSummary {
        checkpoint_height: snapshot.checkpoint_height,
        state_root: snapshot.state_root,
        included_files: payload.files.len(),
    })
}

fn safe_restore_file(base: &Path, relative_name: &str, bytes: &[u8]) -> Result<PathBuf> {
    let relative = Path::new(relative_name);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        anyhow::bail!("Unsafe validator backup path: {relative_name}");
    }
    let target = base.join(relative);
    if target.exists() {
        anyhow::bail!("Restore target already exists: {}", target.display());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&target, bytes)?;
    Ok(target)
}

pub fn import_validator_backup(
    backup: &Path,
    expected_network: &str,
    data_dir: &Path,
    recovery_dir: &Path,
) -> Result<ValidatorBackupSummary> {
    if data_dir.exists() && fs::read_dir(data_dir)?.next().is_some() {
        anyhow::bail!(
            "Validator restore data directory must be empty: {}",
            data_dir.display()
        );
    }
    if recovery_dir.exists() && fs::read_dir(recovery_dir)?.next().is_some() {
        anyhow::bail!(
            "Validator recovery directory must be empty: {}",
            recovery_dir.display()
        );
    }

    let password = backup_password()?;
    let archive: EncryptedValidatorBackup = serde_json::from_slice(&fs::read(backup)?)?;
    if archive.format_version != BACKUP_FORMAT_VERSION {
        anyhow::bail!(
            "Unsupported validator backup format {}",
            archive.format_version
        );
    }
    let compressed = Zeroizing::new(
        kanari_crypto::decrypt_data(&archive.encrypted_payload, password.as_str())
            .map_err(|error| anyhow::anyhow!("Failed to decrypt validator backup: {error}"))?,
    );
    let actual_hash = hex::encode(Sha3_256::digest(&compressed));
    if actual_hash != archive.payload_sha3_256 {
        anyhow::bail!("Validator backup integrity checksum mismatch");
    }

    let mut decoder = GzDecoder::new(compressed.as_slice());
    let mut decoded = Zeroizing::new(Vec::new());
    decoder.read_to_end(&mut decoded)?;
    let (payload, consumed): (ValidatorBackupPayload, usize) =
        bincode::serde::decode_from_slice(&decoded, bincode::config::standard())?;
    if consumed != decoded.len() {
        anyhow::bail!("Validator backup payload contains trailing data");
    }
    if payload.format_version != BACKUP_FORMAT_VERSION {
        anyhow::bail!(
            "Unsupported validator payload format {}",
            payload.format_version
        );
    }
    if payload.network != expected_network {
        anyhow::bail!(
            "Validator backup network mismatch: expected {}, got {}",
            expected_network,
            payload.network
        );
    }

    fs::create_dir_all(data_dir)?;
    fs::create_dir_all(recovery_dir)?;
    let temp_dir = tempfile::tempdir()?;
    let snapshot_path = temp_dir.path().join("state-snapshot.json");
    fs::write(&snapshot_path, &payload.state_snapshot)?;
    let snapshot: StateSnapshot = serde_json::from_slice(&payload.state_snapshot)?;
    let imported =
        BlockchainEngine::import_state_snapshot(&snapshot_path, data_dir, expected_network)?;

    for (name, bytes) in &payload.files {
        if let Some(wal_name) = name.strip_prefix("wal/") {
            safe_restore_file(data_dir, wal_name, bytes)?;
        } else if name == "identity/p2p-identity.key" {
            safe_restore_file(data_dir, "p2p-identity.key", bytes)?;
        } else if let Some(recovery_name) = name.strip_prefix("consensus/") {
            safe_restore_file(recovery_dir, recovery_name, bytes)?;
        } else if name == "network/genesis.json" {
            safe_restore_file(recovery_dir, "genesis.json", bytes)?;
        } else {
            anyhow::bail!("Unknown validator backup entry: {name}");
        }
    }

    if imported.checkpoint_height != snapshot.checkpoint_height
        || imported.state_root != snapshot.state_root
    {
        anyhow::bail!("Restored validator state does not match backup manifest");
    }

    Ok(ValidatorBackupSummary {
        checkpoint_height: imported.checkpoint_height,
        state_root: imported.state_root,
        included_files: payload.files.len(),
    })
}

#[cfg(test)]
mod tests {
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
}
