// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Session inspection without network access (besides nothing at all):
//! reads the saved session, re-checks JWT expiry against the local clock.

use anyhow::{Context, Result};
use clap::Parser;
use kanari_crypto::signatures::zklogin::decode_jwt_claims;

use super::session;

#[derive(Parser, Debug)]
pub struct Status {
    /// Address of the session to show (default: latest session).
    #[arg(long)]
    pub address: Option<String>,
}

impl Status {
    pub fn execute(&self) -> Result<()> {
        let session = match &self.address {
            Some(addr) => session::load_session(addr)?,
            None => session::latest_session()?
                .context("no zkLogin session found (run `kanari zklogin login` first)")?,
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // Decode (not verify) to report claims; signature was verified at login.
        let claims = decode_jwt_claims(&session.id_token)
            .map_err(|e| anyhow::anyhow!("stored id_token is corrupt: {e:?}"))?;
        let live = session
            .id_token_expires_at_unix
            .map(|exp| now <= exp.saturating_add(60))
            .unwrap_or(false);

        println!("address:   {}", session.address);
        println!("provider:   {}", session.provider);
        println!("sub:        {}", session.sub);
        println!("max_epoch:  {}", session.max_epoch);
        println!(
            "id_token:   {}",
            if live {
                "valid"
            } else {
                "EXPIRED (log in again)"
            }
        );
        if let Some(exp) = session.id_token_expires_at_unix {
            println!("expires_at: {exp} (unix secs)");
        }
        println!(
            "nonce bound: {}",
            claims.nonce.as_deref().unwrap_or("<missing>")
        );
        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct Logout {
    /// Address of the session to delete (default: latest session).
    #[arg(long)]
    pub address: Option<String>,
}

impl Logout {
    pub fn execute(&self) -> Result<()> {
        let address = match &self.address {
            Some(addr) => addr.clone(),
            None => session::latest_session()?
                .map(|s| s.address)
                .context("no zkLogin session found")?,
        };
        if session::delete_session(&address)? {
            println!("deleted session {address}");
        } else {
            println!("no session for {address}");
        }
        Ok(())
    }
}
