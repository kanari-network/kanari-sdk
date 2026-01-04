// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Block production logic for Kanari blockchain engine
// including parallel transaction execution using Move runtimes

use anyhow::Result;
use crossbeam_channel as cbchan;
use num_cpus;
use std::collections::{HashMap, VecDeque};

use super::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlockInfo {
    pub height: u64,
    pub hash: String,
    pub tx_count: usize,
    pub executed: usize,
    pub failed: usize,
    pub events: Vec<Event>,
}

pub(super) fn produce_block(engine: &super::BlockchainEngine) -> Result<BlockInfo> {
    let mut pending = engine.pending_txs.write().unwrap();

    if pending.is_empty() {
        anyhow::bail!("No pending transactions");
    }

    let transactions = pending.drain(..).collect::<Vec<_>>();
    let tx_count = transactions.len();

    let mut all_changesets: Vec<ChangeSet> = Vec::with_capacity(tx_count);
    let mut executed = 0usize;
    let mut failed = 0usize;
    let mut _total_gas_used = 0u64;

    if tx_count > 1 {
        let workers = std::cmp::min(num_cpus::get().max(1), tx_count);
        let (job_tx, job_rx) =
            cbchan::unbounded::<(usize, Transaction, Arc<RwLock<StateManager>>)>();
        let (res_tx, res_rx) = cbchan::unbounded::<(usize, Result<ChangeSet>)>();
        let mut handles = Vec::new();

        if let Some(pool) = &engine.runtime_pool {
            for i in 0..workers {
                let job_rx = job_rx.clone();
                let res_tx = res_tx.clone();
                let pool_entry = pool[i % pool.len()].clone();

                let handle = std::thread::spawn(move || {
                    while let Ok((idx, tx, state_arc)) = job_rx.recv() {
                        let mut guard = pool_entry.lock().unwrap();
                        let res = BlockchainEngine::execute_transaction_with_runtime(
                            &tx, &mut guard, &state_arc,
                        );
                        let _ = res_tx.send((idx, res));
                    }
                });
                handles.push(handle);
            }
        } else {
            let mut created = true;
            for _ in 0..workers {
                match kanari_move_runtime::move_runtime::MoveRuntime::new_with_kanari_natives() {
                    Ok(mut runtime) => {
                        let job_rx = job_rx.clone();
                        let res_tx = res_tx.clone();
                        let handle = std::thread::spawn(move || {
                            while let Ok((idx, tx, state_arc)) = job_rx.recv() {
                                let res = BlockchainEngine::execute_transaction_with_runtime(
                                    &tx,
                                    &mut runtime,
                                    &state_arc,
                                );
                                let _ = res_tx.send((idx, res));
                            }
                        });
                        handles.push(handle);
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to create runtime for worker: {}. Falling back to sequential.",
                            e
                        );
                        created = false;
                        break;
                    }
                }
            }

            if !created {
                for tx in &transactions {
                    match engine.execute_transaction(tx) {
                        Ok(changeset) => {
                            if changeset.success {
                                executed += 1;
                            } else {
                                failed += 1;
                            }
                            _total_gas_used += changeset.gas_used;
                            all_changesets.push(changeset);
                        }
                        Err(e) => {
                            eprintln!("Transaction execution error: {:?}", e);
                            failed += 1;
                        }
                    }
                }
            }
        }

        if !handles.is_empty() {
            let mut per_sender: HashMap<String, VecDeque<(usize, Transaction)>> = HashMap::new();
            for (i, tx) in transactions.iter().cloned().enumerate() {
                per_sender
                    .entry(tx.sender().to_string())
                    .or_default()
                    .push_back((i, tx));
            }

            // Reserve per-sender next-sequence numbers from the current global state
            // so that snapshots dispatched to workers contain the expected sequence
            // even if other threads update `engine.state` concurrently.
            let mut per_sender_next_seq: HashMap<String, u64> = HashMap::new();
            {
                let state_guard = engine.state.read().unwrap();
                for sender in per_sender.keys() {
                    if let Ok(addr) = AccountAddress::from_hex_literal(sender) {
                        if let Some(acct) = state_guard.get_account(&addr) {
                            per_sender_next_seq.insert(sender.clone(), acct.sequence_number);
                        } else {
                            per_sender_next_seq.insert(sender.clone(), 0u64);
                        }
                    } else {
                        per_sender_next_seq.insert(sender.clone(), 0u64);
                    }
                }
            }

            let mut results: Vec<Option<ChangeSet>> = vec![None; tx_count];
            let mut idx_to_sender: HashMap<usize, String> = HashMap::new();

            for (sender, queue) in per_sender.iter_mut() {
                if let Some((idx, tx)) = queue.pop_front() {
                    // create a snapshot of state for this job (no prior per-sender txs executed yet)
                    let mut state_snapshot = engine.state.read().unwrap().clone();
                    // Ensure the snapshot's account sequence matches the reserved sequence
                    if let Ok(addr) = AccountAddress::from_hex_literal(sender) {
                        let acct = state_snapshot.get_or_create_account(addr);
                        if let Some(next_seq) = per_sender_next_seq.get_mut(sender) {
                            acct.sequence_number = *next_seq;
                            // reserve the next sequence for subsequent txs for this sender
                            *next_seq = next_seq.wrapping_add(1);
                        }
                    }
                    let state_arc = Arc::new(RwLock::new(state_snapshot));
                    job_tx.send((idx, tx, state_arc)).unwrap();
                    idx_to_sender.insert(idx, sender.clone());
                }
            }

            let mut collected = 0usize;

            while collected < tx_count {
                if let Ok((idx, res)) = res_rx.recv() {
                    match res {
                        Ok(cs) => {
                            results[idx] = Some(cs);
                        }
                        Err(e) => {
                            eprintln!("Transaction execution error in worker: {:?}", e);
                            results[idx] = None;
                        }
                    }

                    if let Some(sender) = idx_to_sender.remove(&idx)
                        && let Some(queue) = per_sender.get_mut(&sender)
                        && let Some((next_idx, next_tx)) = queue.pop_front()
                    {
                        // Create a state snapshot for the next tx of this sender.
                        // Use the pre-reserved per-sender sequence number so the
                        // dispatched snapshot's account.sequence_number equals the
                        // expected sequence for the transaction.
                        let mut snapshot = engine.state.read().unwrap().clone();
                        if let Ok(addr) = AccountAddress::from_hex_literal(&sender) {
                            let acct = snapshot.get_or_create_account(addr);
                            if let Some(next_seq) = per_sender_next_seq.get_mut(&sender) {
                                acct.sequence_number = *next_seq;
                                *next_seq = next_seq.wrapping_add(1);
                            }
                        }
                        let state_arc = Arc::new(RwLock::new(snapshot));
                        job_tx.send((next_idx, next_tx, state_arc)).unwrap();
                        idx_to_sender.insert(next_idx, sender.clone());
                    }

                    collected += 1;
                }
            }

            drop(job_tx);
            drop(res_tx);
            for h in handles {
                let _ = h.join();
            }

            for opt in results.into_iter() {
                if let Some(cs) = opt {
                    if cs.success {
                        executed += 1;
                    } else {
                        failed += 1;
                    }
                    _total_gas_used += cs.gas_used;
                    all_changesets.push(cs);
                } else {
                    failed += 1;
                }
            }
        }
    } else {
        for tx in &transactions {
            match engine.execute_transaction(tx) {
                Ok(changeset) => {
                    if changeset.success {
                        executed += 1;
                    } else {
                        eprintln!("Transaction failed: {:?}", changeset.error_message);
                        failed += 1;
                    }
                    _total_gas_used += changeset.gas_used;
                    all_changesets.push(changeset);
                }
                Err(e) => {
                    eprintln!("Transaction execution error: {:?}", e);
                    failed += 1;
                }
            }
        }
    }

    let block_events: Vec<Event> = {
        let mut state = engine.state.write().unwrap();
        for changeset in &all_changesets {
            state
                .apply_changeset(changeset)
                .context("Failed to apply changeset to state")?;
        }

        state.drain_events()
    };

    let mut chain = engine.blockchain.write().unwrap();
    let prev_hash = chain.latest_block().hash();
    let height = chain.height() + 1;

    let state_root = {
        let state_guard = engine.state.read().unwrap();
        state_guard.compute_state_root()
    };

    let block = Block::new(
        height,
        prev_hash,
        state_root,
        transactions,
        block_events.clone(),
    );
    let block_hash = block.hash();

    chain.add_block(block)?;

    if let Some(store) = &engine.persistent_store {
        store
            .save("blockchain", &*chain)
            .context("Failed to persist blockchain")?;

        let state_guard = engine.state.read().unwrap();
        store
            .save("state_manager", &*state_guard)
            .context("Failed to persist state manager")?;

        let block_height = height;
        if let Err(e) = store.save_smt_snapshot(block_height) {
            eprintln!(
                "Failed to save SMT snapshot for height {}: {}",
                block_height, e
            );
        }
    }

    Ok(BlockInfo {
        height,
        hash: hex::encode(&block_hash),
        tx_count,
        executed,
        failed,
        events: block_events,
    })
}
