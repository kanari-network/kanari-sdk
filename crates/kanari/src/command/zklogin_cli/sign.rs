// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! `kanari zklogin sign` — sign bytes with a session's ephemeral secret.
//!
//! This is the client half of "send a tx as the zkLogin address": the CLI
//! proves possession of the session key by signing the transaction payload.
//! Chain acceptance of such signatures (sender auth for zkLogin addresses)
//! is NOT implemented in the node yet — see the module docs in
//! `kanari_system::zklogin` for what contracts can verify on-chain today.

use anyhow::{Context, Result};
use clap::Parser;
use kanari_crypto::signatures::zklogin::EphemeralKeypair;

use super::session;

#[derive(Parser, Debug)]
pub struct Sign {
    /// Address of the session to sign with (default: latest session).
    #[arg(long)]
    pub address: Option<String>,
    /// Message to sign, as UTF-8 text (exactly one of --message/--message-hex).
    #[arg(long)]
    pub message: Option<String>,
    /// Message to sign, as hex bytes (exactly one of --message/--message-hex).
    #[arg(long)]
    pub message_hex: Option<String>,
}

impl Sign {
    pub fn execute(&self) -> Result<()> {
        let msg = match (&self.message, &self.message_hex) {
            (Some(text), None) => text.as_bytes().to_vec(),
            (None, Some(hex_str)) => hex::decode(hex_str.trim_start_matches("0x"))
                .context("message-hex is not valid hex")?,
            _ => anyhow::bail!("pass exactly one of --message or --message-hex"),
        };
        let session = match &self.address {
            Some(addr) => session::load_session(addr)?,
            None => session::latest_session()?
                .context("no zkLogin session found (run `kanari zklogin login` first)")?,
        };
        let secret: [u8; 32] = hex::decode(&session.ephemeral_secret_hex)
            .context("session secret is corrupt")?
            .try_into()
            .map_err(|_| anyhow::anyhow!("session secret is not 32 bytes"))?;
        let keypair =
            EphemeralKeypair::from_secret(secret).context("session secret is degenerate")?;
        // Sanity: the stored pubkey must match the restored secret.
        anyhow::ensure!(
            hex::encode(keypair.public_bytes()) == session.ephemeral_pubkey_hex,
            "session pubkey does not match its secret (corrupt session file?)"
        );
        let sig = keypair.sign(&msg);
        // Local self-check before printing.
        EphemeralKeypair::verify(&keypair.public_bytes(), &msg, &sig)
            .map_err(|e| anyhow::anyhow!("self-verification failed: {e:?}"))?;

        eprintln!("address: {}", session.address);
        eprintln!("pubkey:  {}", session.ephemeral_pubkey_hex);
        eprintln!("sig:     {}", hex::encode(sig));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ephemeral_sign_roundtrip() {
        let kp = EphemeralKeypair::from_secret([5u8; 32]).unwrap();
        let msg = b"kanari tx payload";
        let sig = kp.sign(msg);
        assert!(EphemeralKeypair::verify(&kp.public_bytes(), msg, &sig).is_ok());
        assert!(EphemeralKeypair::verify(&kp.public_bytes(), b"tampered", &sig).is_err());
    }

    #[test]
    fn session_save_load_delete_roundtrip_isolated() {
        // Explicit dir: no env manipulation, race-free under parallel tests.
        let dir = std::env::temp_dir().join(format!("kanari-zklogin-test-{}", std::process::id()));
        let session = session::ZkLoginSession {
            version: session::SESSION_VERSION,
            provider: "google".to_string(),
            iss: "https://accounts.google.com".to_string(),
            aud: "cid".to_string(),
            sub: "u1".to_string(),
            address: "0xdeadbeef".to_string(),
            salt_hex: "00".repeat(32),
            ephemeral_pubkey_hex: "11".repeat(32),
            ephemeral_secret_hex: "22".repeat(32),
            max_epoch: 10,
            randomness_hex: "33".repeat(32),
            id_token: "h.p.s".to_string(),
            obtained_at_unix: 1,
            id_token_expires_at_unix: None,
        };
        let path = session::save_session_in(&dir, &session).unwrap();
        assert!(path.exists());
        let back = session::load_session_in(&dir, "0xdeadbeef").unwrap();
        assert_eq!(back.sub, "u1");
        assert_eq!(
            session::latest_session_in(&dir).unwrap().unwrap().address,
            "0xdeadbeef"
        );
        assert!(session::delete_session_in(&dir, "0xdeadbeef").unwrap());
        assert!(!session::delete_session_in(&dir, "0xdeadbeef").unwrap());
        std::fs::remove_dir_all(&dir).ok();
    }
}
