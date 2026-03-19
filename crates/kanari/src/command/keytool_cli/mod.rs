// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use clap::Subcommand;

pub mod add;
pub mod create;
pub mod info;
pub mod list;
pub mod load;

#[derive(Subcommand, Debug)]
pub enum KeytoolCommand {
    /// Create a new wallet with kanari-crypto
    Create(create::CreateWallet),
    /// Load an existing wallet
    Load(load::LoadWallet),
    /// List all wallets with balances
    List(list::ListWallets),
    /// Show detailed wallet information
    Info(info::WalletInfo),
    /// Import an existing wallet from private key or seed phrase
    Import(add::AddWallet),
}

impl KeytoolCommand {
    pub fn execute(&self) -> Result<()> {
        match self {
            KeytoolCommand::Create(cmd) => cmd.execute(),
            KeytoolCommand::Load(cmd) => cmd.execute(),
            KeytoolCommand::List(cmd) => cmd.execute(),
            KeytoolCommand::Info(cmd) => cmd.execute(),
            KeytoolCommand::Import(cmd) => cmd.execute(),
        }
    }
}
