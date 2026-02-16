// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::command::common::normalize_addr;
use anyhow::{Context, Result};
use clap::Parser;
use kanari_crypto::wallet::{list_wallet_files, set_selected_wallet};

#[derive(Parser, Debug)]
pub struct LoadWallet {
    /// Wallet address to load
    #[arg(short, long)]
    pub address: String,
}

impl LoadWallet {
    pub fn execute(&self) -> Result<()> {
        let address_normalized = normalize_addr(&self.address)?;

        // Check if wallet exists without loading (decrypting) it
        let wallets = list_wallet_files().context("Failed to list wallets")?;
        let exists = wallets.iter().any(|(addr, _)| {
            if let Ok(norm) = normalize_addr(addr) {
                norm == address_normalized
            } else {
                false
            }
        });

        if !exists {
            anyhow::bail!("Wallet {} not found", address_normalized);
        }

        // Mark this wallet as selected in the kanari config so `list-wallets`
        // shows the expected selected address.
        set_selected_wallet(&address_normalized).context("Failed to set selected wallet")?;
        eprintln!("Selected wallet: {}", address_normalized);

        Ok(())
    }
}
