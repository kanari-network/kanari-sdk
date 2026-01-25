// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use kanari_core::BlockchainEngine;
use kanari_crypto::keys::{CurveType, generate_keypair};
use kanari_types::transaction::{SignedTransaction, Transaction};
use move_core_types::account_address::AccountAddress;
use std::time::Instant;

fn main() -> Result<()> {
    // Simple harness: create engine, pre-fund accounts, submit N transfers into pending_txs,
    // call produce_block() once and measure duration.
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2000);

    eprintln!("TPS harness: creating engine and preparing {} txs", n);
    let engine = BlockchainEngine::new()?;

    // Generate keypairs for senders and recipients, pre-fund senders,
    // sign each transaction and push into pending_txs.
    let mut senders: Vec<_> = Vec::with_capacity(n);
    let mut recipients: Vec<_> = Vec::with_capacity(n);
    for _ in 0..n {
        senders.push(generate_keypair(CurveType::Ed25519)?);
    }
    for _ in 0..n {
        recipients.push(generate_keypair(CurveType::Ed25519)?);
    }

    // Pre-fund sender accounts
    {
        let mut state = engine.state.write().unwrap();
        for kp in &senders {
            if let Ok(addr) = AccountAddress::from_hex_literal(kp.address.as_str()) {
                let acc = state.get_or_create_account(addr);
                acc.balance = 1_000_000_000_000; // large balance
            }
        }
    }

    // Prepare transactions and push signed transactions to pending_txs
    {
        let mut pending = engine.pending_txs.write().unwrap();
        for i in 0..n {
            let from = senders[i].address.clone();
            let to = recipients[i].address.clone();
            let tx = Transaction::new_transfer(from.clone(), to, 1);
            let mut signed_tx = SignedTransaction::new(tx);
            signed_tx.sign(&senders[i].private_key, senders[i].curve_type)?;
            pending.push(signed_tx);
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
