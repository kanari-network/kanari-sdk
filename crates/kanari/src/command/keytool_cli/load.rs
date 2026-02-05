// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use clap::Parser;
use kanari_crypto::wallet::{Wallet, load_wallet, set_selected_wallet};

#[derive(Parser, Debug)]
pub struct LoadWallet {
    /// Wallet address to load
    #[arg(short, long)]
    pub address: String,
    /// Password to decrypt wallet
    #[arg(short, long)]
    pub password: String,
}

impl LoadWallet {
    pub fn execute(&self) -> Result<()> {
        let wallet: Wallet =
            load_wallet(&self.address, &self.password).context("Failed to load wallet")?;
        eprintln!(
            "Wallet loaded: {} (curve: {})",
            self.address, wallet.curve_type
        );

        // Mark this wallet as selected in the kanari config so `list-wallets`
        // shows the expected selected address.
        set_selected_wallet(&self.address).context("Failed to set selected wallet")?;
        eprintln!("Selected wallet: {}", self.address);

        Ok(())
    }
}
