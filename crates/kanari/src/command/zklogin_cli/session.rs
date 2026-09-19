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

/// Standard address-salt: one 32-byte value per (iss, aud, sub), derived
/// purely from [`kanari_crypto::signatures::zklogin::deterministic_salt`].
///
/// This is THE standard salt — the exact same value the Android app derives
/// via FFI — so CLI and app always agree on the address for the same Google
/// account, on any machine, with nothing stored: no vault files, no random
/// salts, no backups. Stale `salts/` entries left by older versions are
/// intentionally never read. (The address derives from the salt, so any
/// per-device randomness here would mint a NEW address every reinstall.)
pub fn standard_salt(iss: &str, aud: &str, sub: &str) -> Result<[u8; 32]> {
    kanari_crypto::signatures::zklogin::deterministic_salt(iss, aud, sub)
        .map_err(|e| anyhow::anyhow!("cannot derive standard salt: {e:?}"))
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
    fn standard_salt_matches_shared_vector() {
        // No filesystem involved: the standard salt is pure derivation,
        // identical to what the Android app gets from kanari-crypto FFI.
        // Same fixture as `deterministic_salt_is_stable_and_shared_vector`.
        let s1 =
            standard_salt("https://accounts.google.com", "kanari-test-client", "1234").unwrap();
        assert_eq!(
            hex::encode(s1),
            "82364ea8d3563ad90cd8876b4242a2d44be88114b1b4ca114b39ca7027617fe0"
        );
        // Same account -> same salt, every call, every machine.
        let s2 =
            standard_salt("https://accounts.google.com", "kanari-test-client", "1234").unwrap();
        assert_eq!(s1, s2);
        // Different sub -> different salt. Different aud -> different salt.
        let s3 = standard_salt("https://accounts.google.com", "kanari-test-client", "u2").unwrap();
        assert_ne!(s1, s3);
        let s4 = standard_salt("https://accounts.google.com", "other", "1234").unwrap();
        assert_ne!(s1, s4);
    }
}
