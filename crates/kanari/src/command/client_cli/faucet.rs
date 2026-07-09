// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::command::common::get_rpc_endpoint;
use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Faucet {
    /// Recipient address (optional). If omitted, uses configured active_address
    #[arg(short, long)]
    pub to: Option<String>,

    /// Amount in Kanari
    #[arg(short, long)]
    pub amount: f64,

    /// Dev wallet address override (optional)
    #[arg(long)]
    pub dev_address: Option<String>,

    /// Dev wallet password (optional; falls back to KANARI_PASSWORD env)
    #[arg(long)]
    pub dev_password: Option<String>,

    /// RPC endpoint
    #[clap(long = "rpc")]
    pub rpc_endpoint: Option<String>,
}

impl Faucet {
    pub async fn execute(&self) -> Result<()> {
        let rpc = get_rpc_endpoint(self.rpc_endpoint.clone());

        let status = kanari_faucet::request_from_dev(
            self.dev_address.as_deref(),
            self.dev_password.as_deref(),
            self.to.as_deref(),
            self.amount,
            &rpc,
        )
        .await
        .context("Faucet request failed")?;

        eprintln!(
            "Faucet tx submitted: hash={} status={} success={} previewed={} submitted={} committed={}",
            status.hash,
            status.status,
            status.success,
            status.previewed,
            status.submitted,
            status.committed
        );

        Ok(())
    }
}
