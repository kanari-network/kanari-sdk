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
    CreateWallet(create::CreateWallet),
    /// Load an existing wallet
    LoadWallet(load::LoadWallet),
    /// List all wallets with balances
    ListWallets(list::ListWallets),
    /// Show detailed wallet information
    WalletInfo(info::WalletInfo),
    /// Import an existing wallet from private key or seed phrase
    AddWallet(add::AddWallet),
}

impl KeytoolCommand {
    pub fn execute(&self) -> Result<()> {
        match self {
            KeytoolCommand::CreateWallet(cmd) => cmd.execute(),
            KeytoolCommand::LoadWallet(cmd) => cmd.execute(),
            KeytoolCommand::ListWallets(cmd) => cmd.execute(),
            KeytoolCommand::WalletInfo(cmd) => cmd.execute(),
            KeytoolCommand::AddWallet(cmd) => cmd.execute(),
        }
    }
}
