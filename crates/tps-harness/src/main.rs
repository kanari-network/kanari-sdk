// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use kanari_core::{BlockchainEngine, Transaction};
use move_core_types::account_address::AccountAddress;
use std::time::Instant;

fn main() -> Result<()> {
    // Simple harness: create engine, pre-fund accounts, submit N transfers into pending_txs,
    // call produce_block() once and measure duration.
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1000);

    eprintln!("TPS harness: creating engine and preparing {} txs", n);
    let engine = BlockchainEngine::new()?;

    // Pre-fund accounts
    {
        let mut state = engine.state.write().unwrap();
        for i in 0..n {
            let addr_str = format!("0x{:x}", i + 1);
            if let Ok(addr) = AccountAddress::from_hex_literal(&addr_str) {
                let acc = state.get_or_create_account(addr);
                acc.balance = 1_000_000_000_000; // large balance
            }
        }
    }

    // Prepare transactions and push to pending_txs
    {
        let mut pending = engine.pending_txs.write().unwrap();
        for i in 0..n {
            let from = format!("0x{:x}", i + 1);
            let to = format!("0x{:x}", i + 1000000);
            let tx = Transaction::new_transfer(from, to, 1);
            pending.push(tx);
        }
    }

    eprintln!("Starting produce_block() for {} txs...", n);
    let start = Instant::now();
    let info = engine.produce_block()?;
    let dur = start.elapsed();
    let tps = (info.tx_count as f64) / dur.as_secs_f64();

    eprintln!(
        "Done: executed={} failed={} tx_count={}",
        info.executed, info.failed, info.tx_count
    );
    eprintln!("Duration: {:.3}s — TPS: {:.2}", dur.as_secs_f64(), tps);

    Ok(())
}
