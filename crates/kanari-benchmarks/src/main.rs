// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result, bail};
use kanari_core::{BlockInfo, BlockchainEngine};
use kanari_crypto::keys::{CurveType, generate_keypair};
use kanari_types::balance::BalanceRecord;
use kanari_types::kanari::KANARI_TOKEN_TYPE;
use kanari_types::transaction::{SignedTransaction, Transaction};
use move_core_types::account_address::AccountAddress;
use std::fmt;
use std::time::Instant;
use tempfile::TempDir;

#[derive(Debug, Clone, PartialEq, Eq)]
enum HarnessMode {
    Production,
    Immediate,
    Parallel,
    ParallelExecOnly,
}

impl HarnessMode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Production => "production",
            Self::Immediate => "immediate",
            Self::Parallel => "parallel",
            Self::ParallelExecOnly => "parallel-exec-only",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HarnessConfig {
    tx_count: usize,
    json: bool,
    mode: HarnessMode,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            tx_count: 512,
            json: false,
            mode: HarnessMode::Production,
        }
    }
}

#[derive(Debug, Clone)]
struct HarnessReport {
    requested_txs: usize,
    mode: HarnessMode,
    block_info: BlockInfo,
    duration_secs: f64,
    submit_secs: Option<f64>,
    produce_secs: Option<f64>,
    tps: f64,
}

impl HarnessReport {
    fn render_text(&self) -> String {
        let breakdown = match (self.submit_secs, self.produce_secs) {
            (Some(submit), Some(produce)) => {
                format!("\nsubmit_secs={submit:.6}\nproduce_secs={produce:.6}")
            }
            _ => String::new(),
        };
        format!(
            "Kanari benchmark completed\nmode={}\nrequested_txs={}\nexecuted={}\nfailed={}\ntx_count={}\nduration_secs={:.3}{}\ntps={:.2}",
            self.mode.as_str(),
            self.requested_txs,
            self.block_info.executed,
            self.block_info.failed,
            self.block_info.tx_count,
            self.duration_secs,
            breakdown,
            self.tps
        )
    }

    fn render_json(&self) -> String {
        let timing_fields = match (self.submit_secs, self.produce_secs) {
            (Some(submit), Some(produce)) => {
                format!("\"submit_secs\":{submit:.6},\"produce_secs\":{produce:.6},")
            }
            _ => String::new(),
        };
        format!(
            concat!(
                "{{",
                "\"requested_txs\":{},",
                "\"mode\":\"{}\",",
                "\"executed\":{},",
                "\"failed\":{},",
                "\"tx_count\":{},",
                "\"duration_secs\":{:.6},",
                "{}",
                "\"tps\":{:.6}",
                "}}"
            ),
            self.requested_txs,
            self.mode.as_str(),
            self.block_info.executed,
            self.block_info.failed,
            self.block_info.tx_count,
            self.duration_secs,
            timing_fields,
            self.tps
        )
    }
}

impl fmt::Display for HarnessReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render_text())
    }
}

fn parse_args<I>(args: I) -> Result<HarnessConfig>
where
    I: IntoIterator<Item = String>,
{
    let mut config = HarnessConfig::default();
    let mut args = args.into_iter();
    let _ = args.next();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--txs" => {
                let value = args
                    .next()
                    .context("missing value after --txs")?
                    .parse::<usize>()
                    .context("failed to parse --txs as usize")?;
                config.tx_count = value;
            }
            "--json" => config.json = true,
            "--mode" => {
                let value = args.next().context("missing value after --mode")?;
                config.mode = match value.as_str() {
                    "production" => HarnessMode::Production,
                    "immediate" => HarnessMode::Immediate,
                    "parallel" => HarnessMode::Parallel,
                    "parallel-exec-only" => HarnessMode::ParallelExecOnly,
                    _ => bail!("unknown mode: {value}\n\n{}", usage()),
                };
            }
            "--help" | "-h" => {
                eprintln!("{}", usage());
                std::process::exit(0);
            }
            value if !value.starts_with('-') => {
                config.tx_count = value
                    .parse::<usize>()
                    .context("failed to parse positional tx count as usize")?;
            }
            _ => bail!("unknown argument: {arg}\n\n{}", usage()),
        }
    }

    if config.tx_count == 0 {
        bail!("tx count must be greater than zero");
    }

    Ok(config)
}

fn usage() -> &'static str {
    "Usage: cargo run --release -p kanari-benchmarks -- [--txs N] [--mode production|immediate|parallel|parallel-exec-only] [--json]\n       cargo run --release -p kanari-benchmarks -- [N]\n\nDefault mode is production: submit_transaction + produce_block."
}

fn prepare_engine(temp_dir: &TempDir) -> Result<BlockchainEngine> {
    let mut engine = BlockchainEngine::new_dir(
        temp_dir
            .path()
            .to_str()
            .context("temp dir path is not valid UTF-8")?,
    )?;

    // Set up a demo consensus signing key for benchmarks
    use kanari_crypto::keys::{CurveType, generate_keypair};
    let keypair = generate_keypair(CurveType::Ed25519)?;

    // Convert the private key from hex string to bytes
    // Private key format is "kanari" + hex_encoded_bytes
    let private_key_hex = keypair.private_key.as_str();
    let hex_part = private_key_hex.trim_start_matches("kanari");
    let signing_key_bytes_vec =
        hex::decode(hex_part).context("Failed to decode private key hex")?;
    let signing_key_bytes: [u8; 32] = signing_key_bytes_vec
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid private key length: expected 32 bytes"))?;
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&signing_key_bytes);

    // Build authority public keys map
    let mut authority_public_keys = std::collections::BTreeMap::new();
    let verifying_key = signing_key.verifying_key();
    authority_public_keys.insert(
        engine.authority_id().to_string(),
        verifying_key.to_bytes().to_vec(),
    );

    // Set the consensus signing key
    engine.set_consensus_signing_key(signing_key, authority_public_keys)?;

    Ok(engine)
}

fn fund_senders(engine: &BlockchainEngine, sender_addresses: &[String]) {
    let mut state = engine.state.write().unwrap_or_else(|e| e.into_inner());
    for address in sender_addresses {
        if let Ok(addr) = AccountAddress::from_hex_literal(address) {
            let mut acc = state
                .get_account(&addr)
                .unwrap_or_else(|| kanari_core::kanari_move_runtime_v1::state::Account::new(addr));
            acc.set_token_balance(
                KANARI_TOKEN_TYPE.to_string(),
                BalanceRecord::new(1_000_000_000_000),
            );
            state
                .save_account(&acc)
                .expect("failed to save funded account");
        }
    }
}

fn execute_production_path(
    engine: &BlockchainEngine,
    signed_txs: Vec<SignedTransaction>,
) -> Result<(BlockInfo, f64, f64)> {
    let submit_start = Instant::now();
    engine.submit_transactions_batch(signed_txs)?;
    let submit_secs = submit_start.elapsed().as_secs_f64();

    let produce_start = Instant::now();
    let block_info = engine.produce_block()?;
    let produce_secs = produce_start.elapsed().as_secs_f64();

    Ok((block_info, submit_secs, produce_secs))
}

fn execute_immediate(
    engine: &BlockchainEngine,
    signed_txs: Vec<SignedTransaction>,
) -> Result<BlockInfo> {
    let mut executed = 0usize;
    let mut failed = 0usize;
    let mut failure_samples = Vec::new();

    for signed_tx in signed_txs {
        match engine.execute_transaction_immediate(signed_tx) {
            Ok((_tx_hash, changeset)) => {
                if changeset.success {
                    let mut state = engine.state.write().unwrap_or_else(|e| e.into_inner());
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

    Ok(BlockInfo {
        vertex_id: "immediate-mode".to_string(),
        round: 0,
        tx_count: executed + failed,
        executed,
        failed,
        events: vec![],
        checkpoint: None,
        vertex: None,
    })
}

fn execute_parallel(
    engine: &BlockchainEngine,
    signed_txs: Vec<SignedTransaction>,
    apply_results: bool,
) -> Result<BlockInfo> {
    let results = engine.execute_transactions_parallel(signed_txs);
    let mut executed = 0usize;
    let mut failed = 0usize;
    let mut state = if apply_results {
        Some(engine.state.write().unwrap_or_else(|e| e.into_inner()))
    } else {
        None
    };

    for (_signed_tx, result) in results {
        match result {
            Ok((_tx_hash, changeset)) if changeset.success => {
                if let Some(state) = state.as_mut() {
                    state.apply_changeset(&changeset)?;
                }
                executed += 1;
            }
            Ok(_) | Err(_) => {
                failed += 1;
            }
        }
    }

    if let Some(state) = state.as_mut() {
        state.commit()?;
    }

    Ok(BlockInfo {
        vertex_id: if apply_results {
            "parallel-mode".to_string()
        } else {
            "parallel-exec-only-mode".to_string()
        },
        round: 0,
        tx_count: executed + failed,
        executed,
        failed,
        events: vec![],
        checkpoint: None,
        vertex: None,
    })
}

fn run_harness(config: &HarnessConfig) -> Result<HarnessReport> {
    eprintln!(
        "Kanari benchmarks: creating engine and preparing {} txs in {} mode",
        config.tx_count,
        config.mode.as_str()
    );

    let temp_dir = tempfile::Builder::new().prefix("kanari_tps").tempdir()?;
    let engine = prepare_engine(&temp_dir)?;

    eprintln!("Generating keypairs and transactions...");
    let senders: Vec<_> = (0..config.tx_count)
        .map(|_| generate_keypair(CurveType::Ed25519).expect("key generation should succeed"))
        .collect();
    let recipients: Vec<_> = (0..config.tx_count)
        .map(|_| generate_keypair(CurveType::Ed25519).expect("key generation should succeed"))
        .collect();
    let sender_addresses: Vec<_> = senders.iter().map(|kp| kp.address.clone()).collect();

    eprintln!("Funding accounts...");
    fund_senders(&engine, &sender_addresses);

    eprintln!("Signing transactions...");
    let signed_txs: Vec<_> = senders
        .iter()
        .zip(recipients.iter())
        .map(|(sender, recipient)| {
            let tx =
                Transaction::new_transfer(sender.tagged_address(), recipient.address.clone(), 1, 0);
            let mut signed_tx = SignedTransaction::new(tx);
            signed_tx
                .sign(&sender.private_key, sender.curve_type)
                .expect("transaction signing should succeed");
            signed_tx
        })
        .collect();
    eprintln!("Starting benchmark...");
    let start = Instant::now();
    let mut submit_secs = None;
    let mut produce_secs = None;
    let block_info = match config.mode {
        HarnessMode::Production => {
            let (block_info, submit, produce) = execute_production_path(&engine, signed_txs)?;
            submit_secs = Some(submit);
            produce_secs = Some(produce);
            block_info
        }
        HarnessMode::Immediate => execute_immediate(&engine, signed_txs)?,
        HarnessMode::Parallel => execute_parallel(&engine, signed_txs, true)?,
        HarnessMode::ParallelExecOnly => execute_parallel(&engine, signed_txs, false)?,
    };
    let duration_secs = start.elapsed().as_secs_f64();
    let tps = if duration_secs > 0.0 {
        block_info.tx_count as f64 / duration_secs
    } else {
        block_info.tx_count as f64
    };

    Ok(HarnessReport {
        requested_txs: config.tx_count,
        mode: config.mode.clone(),
        block_info,
        duration_secs,
        submit_secs,
        produce_secs,
        tps,
    })
}

fn main() -> Result<()> {
    let config = parse_args(std::env::args())?;
    let report = run_harness(&config)?;

    if config.json {
        eprintln!("{}", report.render_json());
    } else {
        eprintln!("{report}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parse_args_accepts_positional_tx_count() {
        let config = parse_args(args(&["kanari-benchmarks", "128"])).unwrap();
        assert_eq!(
            config,
            HarnessConfig {
                tx_count: 128,
                json: false,
                mode: HarnessMode::Production,
            }
        );
    }

    #[test]
    fn parse_args_accepts_flag_form_and_json() {
        let config = parse_args(args(&[
            "kanari-benchmarks",
            "--txs",
            "64",
            "--mode",
            "parallel-exec-only",
            "--json",
        ]))
        .unwrap();
        assert_eq!(
            config,
            HarnessConfig {
                tx_count: 64,
                json: true,
                mode: HarnessMode::ParallelExecOnly,
            }
        );
    }

    #[test]
    fn parse_args_rejects_zero_tx_count() {
        let err = parse_args(args(&["kanari-benchmarks", "--txs", "0"])).unwrap_err();
        assert!(err.to_string().contains("greater than zero"));
    }

    #[test]
    fn report_renders_json_with_core_fields() {
        let report = HarnessReport {
            requested_txs: 5,
            mode: HarnessMode::Immediate,
            block_info: BlockInfo {
                vertex_id: "vertex".to_string(),
                round: 1,
                tx_count: 5,
                executed: 5,
                failed: 0,
                events: vec![],
                checkpoint: None,
                vertex: None,
            },
            duration_secs: 0.5,
            submit_secs: None,
            produce_secs: None,
            tps: 10.0,
        };

        let json = report.render_json();
        assert!(json.contains("\"requested_txs\":5"));
        assert!(json.contains("\"mode\":\"immediate\""));
        assert!(json.contains("\"executed\":5"));
        assert!(json.contains("\"tps\":10.000000"));
    }
}
