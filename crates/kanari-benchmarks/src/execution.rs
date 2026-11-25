// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use kanari_core::{BlockchainEngine, CheckpointProductionInfo};
use kanari_types::env::{env_flag, env_usize_positive};
use kanari_types::transaction::SignedTransaction;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

const DEFAULT_PRODUCTION_CHUNK_SIZE: usize = 100_000;

pub fn execute_admission_path(
    engine: &BlockchainEngine,
    signed_txs: Vec<SignedTransaction>,
) -> Result<(CheckpointProductionInfo, f64)> {
    let requested = signed_txs.len();
    let submit_start = Instant::now();
    let accepted = engine.submit_transactions_batch(signed_txs)?;
    let submit_secs = submit_start.elapsed().as_secs_f64();

    Ok((
        CheckpointProductionInfo {
            vertex_id: "admission-mode".to_string(),
            round: 0,
            tx_count: accepted.len(),
            executed: 0,
            failed: requested.saturating_sub(accepted.len()),
            events: vec![],
            checkpoint: None,
            vertex: None,
        },
        submit_secs,
    ))
}

pub fn execute_production_path(
    engine: &BlockchainEngine,
    signed_txs: Vec<SignedTransaction>,
) -> Result<(CheckpointProductionInfo, f64, f64)> {
    execute_production_pipeline(engine, signed_txs, false)
}

pub fn execute_owned_fast_path(
    engine: &BlockchainEngine,
    signed_txs: Vec<SignedTransaction>,
) -> Result<(CheckpointProductionInfo, f64, f64)> {
    execute_production_pipeline(engine, signed_txs, true)
}

fn execute_production_pipeline(
    engine: &BlockchainEngine,
    signed_txs: Vec<SignedTransaction>,
    owned_fast: bool,
) -> Result<(CheckpointProductionInfo, f64, f64)> {
    if !pipeline_enabled() {
        return execute_production_sequential(engine, signed_txs, owned_fast);
    }

    let chunk_size = production_chunk_size(signed_txs.len());
    let mut remaining = signed_txs;

    thread::scope(|scope| -> Result<(CheckpointProductionInfo, f64, f64)> {
        let (submit_tx, submit_rx) = mpsc::channel::<(usize, f64)>();

        // A single submitter and a single producer. The engine APIs are &self
        // with internal locks, and finalization drains only committed hashes,
        // so transactions submitted while a checkpoint is being produced stay
        // pending for the next checkpoint instead of being silently committed.
        let submitter = scope.spawn(move || -> Result<()> {
            loop {
                let chunk = take_next_chunk(&mut remaining, chunk_size);
                if chunk.is_empty() {
                    break;
                }
                let chunk_len = chunk.len();
                let chunk_submit_secs = submit_chunk(engine, chunk)?;
                if chunk_profile_enabled() {
                    eprintln!(
                        "benchmark chunk submitted: txs={} submit={:.6}s",
                        chunk_len, chunk_submit_secs
                    );
                }
                submit_tx
                    .send((chunk_len, chunk_submit_secs))
                    .context("Checkpoint producer stopped before all chunks were submitted")?;
            }
            Ok(())
        });

        let producer = scope.spawn(|| -> Result<(CheckpointProductionInfo, f64, f64)> {
            let mut submit_secs = 0.0;
            let mut produce_secs = 0.0;
            let mut aggregate: Option<CheckpointProductionInfo> = None;

            for (_chunk_len, chunk_submit_secs) in submit_rx {
                submit_secs += chunk_submit_secs;

                // Wait for the submit signal before producing so an empty
                // mempool never turns into a spurious DAG/checkpoint round.
                while engine.pending_transaction_len() > 0 {
                    let produce_start = Instant::now();
                    let block_info = if owned_fast {
                        engine.produce_owned_fast_checkpoint()?
                    } else {
                        engine.produce_checkpoint()?
                    };
                    produce_secs += produce_start.elapsed().as_secs_f64();
                    if block_info.tx_count == 0 {
                        anyhow::bail!(
                            "checkpoint production made no progress with {} pending transactions",
                            engine.pending_transaction_len()
                        );
                    }
                    merge_checkpoint_info(&mut aggregate, block_info);
                }
            }

            let block_info =
                aggregate.context("No checkpoint was produced for submitted transactions")?;
            Ok((block_info, submit_secs, produce_secs))
        });

        let submit_result =
            submitter.join().map_err(|_| anyhow::anyhow!("Benchmark submitter thread panicked"))?;
        let produce_result =
            producer.join().map_err(|_| anyhow::anyhow!("Benchmark producer thread panicked"))?;
        submit_result?;
        produce_result
    })
}

fn execute_production_sequential(
    engine: &BlockchainEngine,
    signed_txs: Vec<SignedTransaction>,
    owned_fast: bool,
) -> Result<(CheckpointProductionInfo, f64, f64)> {
    let chunk_size = production_chunk_size(signed_txs.len());
    let mut remaining = signed_txs;
    let mut submit_secs = 0.0;
    let mut produce_secs = 0.0;
    let mut aggregate: Option<CheckpointProductionInfo> = None;

    loop {
        let chunk = take_next_chunk(&mut remaining, chunk_size);
        if chunk.is_empty() {
            break;
        }
        let chunk_len = chunk.len();
        let chunk_submit_secs = submit_chunk(engine, chunk)?;
        submit_secs += chunk_submit_secs;
        if chunk_profile_enabled() {
            eprintln!(
                "benchmark chunk submitted: txs={} submit={:.6}s",
                chunk_len, chunk_submit_secs
            );
        }

        while engine.pending_transaction_len() > 0 {
            let produce_start = Instant::now();
            let block_info = if owned_fast {
                engine.produce_owned_fast_checkpoint()?
            } else {
                engine.produce_checkpoint()?
            };
            produce_secs += produce_start.elapsed().as_secs_f64();
            if block_info.tx_count == 0 {
                anyhow::bail!(
                    "checkpoint production made no progress with {} pending transactions",
                    engine.pending_transaction_len()
                );
            }
            merge_checkpoint_info(&mut aggregate, block_info);
        }

        if remaining.is_empty() {
            break;
        }
    }
    let block_info = aggregate.context("No checkpoint was produced for submitted transactions")?;

    Ok((block_info, submit_secs, produce_secs))
}

pub fn execute_immediate(
    engine: &BlockchainEngine,
    signed_txs: Vec<SignedTransaction>,
) -> Result<CheckpointProductionInfo> {
    let mut executed = 0usize;
    let mut failed = 0usize;
    let mut failure_samples = Vec::new();

    for signed_tx in signed_txs {
        match engine.execute_transaction_immediate(signed_tx) {
            Ok((_tx_hash, changeset)) => {
                if changeset.success {
                    let mut state = engine.state_write();
                    state.apply_changeset(&changeset)?;
                    state.commit()?;
                    executed += 1;
                } else {
                    if failure_samples.len() < 3 {
                        failure_samples.push(Some("changeset marked failed".to_string()));
                    }
                    failed += 1;
                }
            }
            Err(err) => {
                if failure_samples.len() < 3 {
                    failure_samples.push(Some(err.to_string()));
                }
                failed += 1;
            }
        }
    }

    if !failure_samples.is_empty() {
        eprintln!("Immediate-mode failure samples:");
        for sample in failure_samples.into_iter().flatten() {
            eprintln!("  - {sample}");
        }
    }

    Ok(mode_info("immediate-mode", executed, failed))
}

fn mode_info(vertex_id: &str, executed: usize, failed: usize) -> CheckpointProductionInfo {
    CheckpointProductionInfo {
        vertex_id: vertex_id.to_string(),
        round: 0,
        tx_count: executed + failed,
        executed,
        failed,
        events: vec![],
        checkpoint: None,
        vertex: None,
    }
}

fn production_chunk_size(tx_count: usize) -> usize {
    env_usize_positive("KANARI_BENCH_CHUNK_SIZE", DEFAULT_PRODUCTION_CHUNK_SIZE)
        .min(tx_count.max(1))
}

fn chunk_profile_enabled() -> bool {
    env_flag("KANARI_BENCH_CHUNK_PROFILE")
}

fn pipeline_enabled() -> bool {
    // Measured slower here: every extra checkpoint repeats the in-memory full
    // state-root fallback, so overlapping submission does not cover the extra
    // roots. Keep this as an opt-in experiment for faster state-root backends.
    env_flag("KANARI_BENCH_PIPELINE")
}

fn take_next_chunk(
    remaining: &mut Vec<SignedTransaction>,
    chunk_size: usize,
) -> Vec<SignedTransaction> {
    if remaining.len() <= chunk_size {
        return std::mem::take(remaining);
    }
    let tail = remaining.split_off(chunk_size);
    let chunk = std::mem::take(remaining);
    *remaining = tail;
    chunk
}

fn submit_chunk(engine: &BlockchainEngine, chunk: Vec<SignedTransaction>) -> Result<f64> {
    let submit_start = Instant::now();
    engine.submit_transactions_batch(chunk)?;
    Ok(submit_start.elapsed().as_secs_f64())
}

fn merge_checkpoint_info(
    aggregate: &mut Option<CheckpointProductionInfo>,
    block_info: CheckpointProductionInfo,
) {
    if let Some(total) = aggregate {
        total.tx_count += block_info.tx_count;
        total.executed += block_info.executed;
        total.failed += block_info.failed;
        total.vertex_id = block_info.vertex_id;
        total.round = block_info.round;
        total.checkpoint = block_info.checkpoint;
        total.vertex = block_info.vertex;
    } else {
        *aggregate = Some(block_info);
    }
}
