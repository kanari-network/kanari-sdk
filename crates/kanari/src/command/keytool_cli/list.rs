// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use clap::Parser;
use kanari_crypto::wallet::list_wallet_files;

#[derive(Parser, Debug)]
pub struct ListWallets;

impl ListWallets {
    pub fn execute(&self) -> Result<()> {
        let wallets = list_wallet_files().context("Failed to list wallets")?;
        eprintln!("Found {} wallets", wallets.len());
        if wallets.is_empty() {
            eprintln!("No wallets found.");
        } else {
            for (addr, selected) in wallets {
                if selected {
                    eprintln!("- {}  (selected)", addr);
                } else {
                    eprintln!("- {}", addr);
                }
            }
        }
        Ok(())
    }
}
