// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! `kanari zklogin` — Google OIDC login bound to an ephemeral key.
//!
//! The heavy lifting (nonce binding, JWT verification, address derivation)
//! lives in `kanari_crypto::signatures::zklogin`; this module is the network
//! + browser + session glue.

pub mod circuits;
pub mod login;
pub mod session;
pub mod sign;
pub mod status;

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum ZkLoginCommand {
    /// Log in with Google: opens the browser, verifies the id_token against
    /// Google JWKS, derives the zkLogin address, saves the session
    Login(login::Login),
    /// Show the current (or a given) session: address, expiry, validity
    Status(status::Status),
    /// Delete a session (default: the latest one)
    Logout(status::Logout),
    /// Sign bytes with a session's ephemeral secret (client half of
    /// sending a tx as the zkLogin address)
    Sign(sign::Sign),
    /// Demo-grade Groth16 setup for the binding circuit (toxic waste!
    /// production needs an MPC ceremony)
    SetupCircuit(circuits::SetupCircuit),
    /// Prove session binding (salt/sub + randomness) with a setup PK
    Prove(circuits::Prove),
    /// Verify a proof package offline (pin check + Groth16 verify)
    VerifyProof(circuits::VerifyProof),
}

impl ZkLoginCommand {
    pub fn execute(&self) -> Result<()> {
        match self {
            ZkLoginCommand::Login(cmd) => cmd.execute(),
            ZkLoginCommand::Status(cmd) => cmd.execute(),
            ZkLoginCommand::Logout(cmd) => cmd.execute(),
            ZkLoginCommand::Sign(cmd) => cmd.execute(),
            ZkLoginCommand::SetupCircuit(cmd) => cmd.execute(),
            ZkLoginCommand::Prove(cmd) => cmd.execute(),
            ZkLoginCommand::VerifyProof(cmd) => cmd.execute(),
        }
    }
}
