// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! zkLogin session persistence.
//!
//! Sessions live under `<KANARI_HOME|~/.kanari>/zklogin/<address>.json` with
//! owner-only permissions on Unix. A session holds everything a later command
//! needs: the verified JWT, the salt (re-derives the address), and the
//! ephemeral secret (signs future transactions until `max_epoch`).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SESSION_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkLoginSession {
    pub version: u32,
    pub provider: String,
    pub iss: String,
    pub aud: String,
    pub sub: String,
    /// Derived `0x`-hex address.
    pub address: String,
    /// 32-byte salt, hex (backs up address derivation).
    pub salt_hex: String,
    /// Ephemeral public key, hex.
    pub ephemeral_pubkey_hex: String,
    /// Ephemeral secret, hex. SENSITIVE — owner-only file, short-lived by
    /// design (`max_epoch` bounds it).
    pub ephemeral_secret_hex: String,
    pub max_epoch: u64,
    /// Nonce randomness, hex.
    pub randomness_hex: String,
    /// Raw `id_token` JWT from the provider.
    pub id_token: String,
    pub obtained_at_unix: u64,
    /// `exp` claim of the JWT, when present.
    pub id_token_expires_at_unix: Option<u64>,
}

pub fn session_dir() -> Result<PathBuf> {
    if let Ok(home) = std::env::var("KANARI_HOME") {
        return Ok(PathBuf::from(home).join("zklogin"));
    }
    #[cfg(windows)]
    let home = std::env::var("USERPROFILE").context("USERPROFILE is not set")?;
    #[cfg(not(windows))]
    let home = std::env::var("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(".kanari").join("zklogin"))
}

pub(crate) fn session_path_in(dir: &std::path::Path, address: &str) -> Result<PathBuf> {
    let safe: String = address.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    anyhow::ensure!(!safe.is_empty(), "invalid session address");
    Ok(dir.join(format!("{safe}.json")))
}

pub fn save_session(session: &ZkLoginSession) -> Result<PathBuf> {
    save_session_in(&session_dir()?, session)
}

pub(crate) fn save_session_in(dir: &std::path::Path, session: &ZkLoginSession) -> Result<PathBuf> {
    std::fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let path = session_path_in(dir, &session.address)?;
    let json = serde_json::to_string_pretty(session).context("cannot serialize session")?;
    std::fs::write(&path, json).with_context(|| format!("cannot write {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .context("cannot restrict session permissions")?;
    }
    Ok(path)
}

pub fn load_session(address: &str) -> Result<ZkLoginSession> {
    load_session_in(&session_dir()?, address)
}

pub(crate) fn load_session_in(dir: &std::path::Path, address: &str) -> Result<ZkLoginSession> {
    let path = session_path_in(dir, address)?;
    let json = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "no session at {} (run `kanari zklogin login` first)",
            path.display()
        )
    })?;
    serde_json::from_str(&json).context("session file is corrupt")
}

/// Most recently obtained session (by `obtained_at_unix`), if any.
pub fn latest_session() -> Result<Option<ZkLoginSession>> {
    latest_session_in(&session_dir()?)
}

pub(crate) fn latest_session_in(dir: &std::path::Path) -> Result<Option<ZkLoginSession>> {
    let mut best: Option<ZkLoginSession> = None;
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(json) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(session): Result<ZkLoginSession, _> = serde_json::from_str(&json) else {
            continue;
        };
        let newer = best
            .as_ref()
            .map(|b| session.obtained_at_unix > b.obtained_at_unix)
            .unwrap_or(true);
        if newer {
            best = Some(session);
        }
    }
    Ok(best)
}

pub fn delete_session(address: &str) -> Result<bool> {
    delete_session_in(&session_dir()?, address)
}

pub(crate) fn delete_session_in(dir: &std::path::Path, address: &str) -> Result<bool> {
    let path = session_path_in(dir, address)?;
    if !path.exists() {
        return Ok(false);
    }
    std::fs::remove_file(&path).with_context(|| format!("cannot delete {}", path.display()))?;
    Ok(true)
}

/// Stable salt vault: one 32-byte salt per (iss, aud, sub).
///
/// The address derives from the salt, so a fresh random salt every login
/// would give a NEW address every time. Persisting the salt per account
/// keeps the address stable: same Google account -> same address, forever.
/// Filename = SHA256 over length-prefixed parts (no PII in the path).
/// Same 0600 treatment as sessions. Returns `(salt, is_new)`.
pub fn load_or_create_salt(iss: &str, aud: &str, sub: &str) -> Result<([u8; 32], bool)> {
    load_or_create_salt_in(&session_dir()?, iss, aud, sub)
}

pub(crate) fn load_or_create_salt_in(
    base: &std::path::Path,
    iss: &str,
    aud: &str,
    sub: &str,
) -> Result<([u8; 32], bool)> {
    use sha2::{Digest, Sha256};

    let dir = base.join("salts");
    std::fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let mut h = Sha256::new();
    h.update(b"kanari-salt-lookup-v1");
    for part in [iss.as_bytes(), aud.as_bytes(), sub.as_bytes()] {
        h.update((part.len() as u64).to_be_bytes());
        h.update(part);
    }
    let path = dir.join(hex::encode(h.finalize()));
    if path.exists() {
        let hex_str = std::fs::read_to_string(&path).context("cannot read salt file")?;
        let bytes = hex::decode(hex_str.trim())
            .context("salt file is corrupt")?
            .try_into()
            .map_err(|_| anyhow::anyhow!("salt file is corrupt"))?;
        return Ok((bytes, false));
    }
    let salt = kanari_crypto::signatures::zklogin::generate_salt()
        .map_err(|e| anyhow::anyhow!("cannot sample salt: {e:?}"))?;
    std::fs::write(&path, hex::encode(salt))
        .with_context(|| format!("cannot write {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .context("cannot restrict salt permissions")?;
    }
    Ok((salt, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ZkLoginSession {
        ZkLoginSession {
            version: SESSION_VERSION,
            provider: "google".to_string(),
            iss: "https://accounts.google.com".to_string(),
            aud: "client123".to_string(),
            sub: "user456".to_string(),
            address: "0xabc123".to_string(),
            salt_hex: "00".repeat(32),
            ephemeral_pubkey_hex: "11".repeat(32),
            ephemeral_secret_hex: "22".repeat(32),
            max_epoch: 1000,
            randomness_hex: "33".repeat(32),
            id_token: "header.payload.sig".to_string(),
            obtained_at_unix: 1_700_000_000,
            id_token_expires_at_unix: Some(1_700_003_600),
        }
    }

    #[test]
    fn session_json_roundtrip() {
        let s = sample();
        let json = serde_json::to_string(&s).unwrap();
        let back: ZkLoginSession = serde_json::from_str(&json).unwrap();
        assert_eq!(back.address, s.address);
        assert_eq!(back.max_epoch, 1000);
        assert_eq!(back.id_token_expires_at_unix, Some(1_700_003_600));
    }

    #[test]
    fn session_path_sanitizes_address() {
        // Explicit dir, no env involved.
        let dir = std::path::PathBuf::from("testdir");
        let p = session_path_in(&dir, "0xabc123").unwrap();
        assert_eq!(p, dir.join("0abc123.json"));
        assert!(session_path_in(&dir, "").is_err());
        // Only hex survives: no traversal possible, ever.
        // ("../../etc/passwd" keeps e,c from "etc" and a,d from "passwd".)
        assert_eq!(
            session_path_in(&dir, "../../etc/passwd").unwrap(),
            dir.join("ecad.json")
        );
    }

    #[test]
    fn salt_vault_is_stable_per_account() {
        // Explicit dir: no env manipulation, race-free under parallel tests.
        let dir =
            std::env::temp_dir().join(format!("kanari-zklogin-salttest-{}", std::process::id()));
        let (s1, is_new1) = load_or_create_salt_in(&dir, "https://a.example", "cid", "u1").unwrap();
        assert!(is_new1);
        assert_eq!(s1.len(), 32);
        // Same account -> same salt (address stays put across logins).
        let (s2, is_new2) = load_or_create_salt_in(&dir, "https://a.example", "cid", "u1").unwrap();
        assert!(!is_new2);
        assert_eq!(s1, s2);
        // Different sub -> different salt. Different aud -> different salt.
        let (s3, _) = load_or_create_salt_in(&dir, "https://a.example", "cid", "u2").unwrap();
        assert_ne!(s1, s3);
        let (s4, _) = load_or_create_salt_in(&dir, "https://a.example", "other", "u1").unwrap();
        assert_ne!(s1, s4);
        std::fs::remove_dir_all(&dir).ok();
    }
}
