// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! In-node USD faucet: web claim without any custodial transfer wallet.
//!
//! Flow: the visitor enters an address on the web page → the node builds each
//! 5 USD mint through `kanari_buildCallFunction`, signs it with the sponsor
//! key, then submits it through `kanari_callFunction`. The two legs mint
//! exactly 10 USD as **two** `Coin<USD>` objects (5 + 5) straight to that
//! address through the permissionless `0x2::usd::mint` entry (shared
//! `TreasuryCap`, no allowlist).
//!
//! Gas reality check: every transaction needs gas, so a node-side sponsor
//! pays execution gas from its own KANARI (funded once by the operator).
//! That is the ONLY key involved — claimants hold no key and enter no
//! password. This module never touches `kanari-faucet` (no dev wallet, no
//! transfer/consolidation logic).
//!
//! Environment (on the node process):
//! - `FAUCET_SPONSOR_KEY` (required to enable): K256 private key that pays
//!   gas. Fund it once with a little KANARI.
//! - `FAUCET_COOLDOWN_SECS` (optional): per-address cooldown, default `86400` (24h).
//! - `FAUCET_IP_COOLDOWN_SECS` (optional): per-IP cooldown, defaults to the
//!   per-address cooldown. Both must expire before the same client can claim
//!   again, so one wallet per address cannot be farmed from a single IP.
//!
//! Devnet only: never configure a funded mainnet key here.

use super::{build_object_input, parse_hex_address};
use crate::{RpcRequest, RpcResponse};
use crate::{
    RpcServerState, internal_error_response, invalid_params_response, respond_with_serialize,
};
use anyhow::{Context, Result};
use kanari_crypto::keys::{CurveType, KeyPair, keypair_from_private_key};
use kanari_rpc_api::{
    BuildCallFunctionRequest, CallFunctionRequest, RequestUsdFaucetRequest,
    RequestUsdFaucetResponse, methods,
};
use kanari_types::address::Address;
use kanari_types::transaction::ObjectInput;
use kanari_types::usd_coin::{USD_COIN, UsdModule};
use move_core_types::account_address::AccountAddress;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Fixed claim: 10 USD total, minted as two 5 USD coin objects so the
/// recipient can immediately transfer again (transfers need a transfer coin
/// plus a SEPARATE gas coin).
const CLAIM_HALVES: [u64; 2] = [5_000_000, 5_000_000];
const FAUCET_GAS_LIMIT: u64 = 100_000;
const FAUCET_GAS_PRICE: u64 = 1_000;
const COMMIT_POLL_INTERVAL: Duration = Duration::from_millis(500);
const COMMIT_TIMEOUT: Duration = Duration::from_secs(30);

/// Canonical shared-cap object type gating the permissionless mint.
fn treasury_cap_type() -> String {
    format!("0x2::coin::TreasuryCap<{USD_COIN}>")
}

#[derive(Debug, Clone)]
struct FaucetConfig {
    sponsor_key: String,
    cooldown: Duration,
    ip_cooldown: Duration,
}

impl FaucetConfig {
    fn from_env() -> Result<Self> {
        let sponsor_key = std::env::var("FAUCET_SPONSOR_KEY")
            .context("FAUCET_SPONSOR_KEY is not set (K256 private key that pays claim gas)")?;
        anyhow::ensure!(
            !sponsor_key.trim().is_empty(),
            "FAUCET_SPONSOR_KEY must not be empty"
        );
        let cooldown_secs: u64 = std::env::var("FAUCET_COOLDOWN_SECS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(86_400);
        // Per-IP cooldown defaults to the per-address cooldown. A farmer with
        // many addresses still hits this wall from a single network address.
        let ip_cooldown_secs: u64 = std::env::var("FAUCET_IP_COOLDOWN_SECS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(cooldown_secs);
        Ok(Self {
            sponsor_key,
            cooldown: Duration::from_secs(cooldown_secs),
            ip_cooldown: Duration::from_secs(ip_cooldown_secs),
        })
    }
}

struct FaucetService {
    config: FaucetConfig,
    keypair: KeyPair,
    sender_tagged: String,
    last_claim: Mutex<HashMap<String, Instant>>,
    last_claim_ip: Mutex<HashMap<String, Instant>>,
    /// Serializes claims across awaits (must be an async mutex: the guard is
    /// held while waiting for checkpoint commits, and a `std` guard there
    /// would make the handler future `!Send`).
    claim_lock: tokio::sync::Mutex<()>,
}

/// Hard bound on the per-IP claim log so rotating source addresses cannot
/// grow node memory without limit.
const MAX_IP_LOG_ENTRIES: usize = 100_000;

/// Remaining wait before `key` may claim again, if still cooling down.
fn cooldown_remaining(
    log: &HashMap<String, Instant>,
    key: &str,
    cooldown: Duration,
) -> Option<Duration> {
    log.get(key).and_then(|last| {
        let remaining = cooldown.saturating_sub(last.elapsed());
        if remaining.is_zero() {
            None
        } else {
            Some(remaining)
        }
    })
}

/// Record a claim, first dropping expired entries when the log is full.
fn record_claim(log: &mut HashMap<String, Instant>, key: String, now: Instant, cooldown: Duration) {
    if log.len() >= MAX_IP_LOG_ENTRIES {
        log.retain(|_, seen| now.saturating_duration_since(*seen) < cooldown);
    }
    log.insert(key, now);
}

fn faucet_service() -> Option<Arc<FaucetService>> {
    static SERVICE: OnceLock<Arc<FaucetService>> = OnceLock::new();
    static DISABLED: OnceLock<()> = OnceLock::new();
    if let Some(service) = SERVICE.get() {
        return Some(service.clone());
    }
    if DISABLED.get().is_some() {
        return None;
    }
    match FaucetConfig::from_env().and_then(|config| {
        let keypair = keypair_from_private_key(&config.sponsor_key, CurveType::K256)
            .context("Invalid FAUCET_SPONSOR_KEY")?;
        Ok::<_, anyhow::Error>((config, keypair))
    }) {
        Ok((config, keypair)) => {
            tracing::info!(
                "USD web faucet enabled: 10 USD (2x5) per claim, cooldown {}s per address / {}s per IP",
                config.cooldown.as_secs(),
                config.ip_cooldown.as_secs()
            );
            let sender_tagged = keypair.tagged_address();
            let service = Arc::new(FaucetService {
                config,
                keypair,
                sender_tagged,
                last_claim: Mutex::new(HashMap::new()),
                last_claim_ip: Mutex::new(HashMap::new()),
                claim_lock: tokio::sync::Mutex::new(()),
            });
            let _ = SERVICE.set(service.clone());
            Some(service)
        }
        Err(error) => {
            tracing::info!("USD web faucet disabled: {error:#}");
            let _ = DISABLED.set(());
            None
        }
    }
}

fn valid_address(address: &str) -> bool {
    address.starts_with("0x")
        && address.len() == 66
        && address[2..].chars().all(|c| c.is_ascii_hexdigit())
}

fn is_committed(state: &RpcServerState, tx_hash: &[u8]) -> bool {
    {
        let chain = state
            .engine
            .blockchain
            .read()
            .unwrap_or_else(|p| p.into_inner());
        if chain
            .get_transaction_location_with_effect(tx_hash)
            .is_some()
        {
            return true;
        }
    }
    state
        .engine
        .get_committed_transaction_from_history(tx_hash)
        .is_some()
}

async fn wait_for_commit(state: &RpcServerState, tx_hash: &[u8], timeout: Duration) -> Result<()> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if is_committed(state, tx_hash) {
            return Ok(());
        }
        tokio::time::sleep(COMMIT_POLL_INTERVAL).await;
    }
    anyhow::bail!("Timed out waiting for faucet claim commit")
}

/// Shared `TreasuryCap<USD>` object id (the permissionless mint authority).
fn shared_cap_id(state: &RpcServerState) -> Result<kanari_rpc_api::ObjectInfo> {
    let wanted = treasury_cap_type();
    let objects = state
        .engine
        .query_objects(None, None, Some(&wanted), None, None)
        .context("Failed to query USD treasury cap")?;
    objects.into_iter().next().with_context(|| {
        format!("No shared {wanted} object found (was usd::init run at genesis on a fresh devnet?)")
    })
}

fn handler_error_message(response: &RpcResponse) -> Option<String> {
    response.error.as_ref().map(|error| {
        format!(
            "faucet handler rejected request (code {}): {}",
            error.code, error.message
        )
    })
}

fn faucet_build_request(
    sender_tagged: &str,
    cap_input: ObjectInput,
    cap_id_bytes: Vec<u8>,
    recipient_bytes: Vec<u8>,
    amount: u64,
) -> Result<BuildCallFunctionRequest> {
    Ok(BuildCallFunctionRequest {
        sender: sender_tagged.to_string(),
        package: Address::KANARI_SYSTEM_ADDRESS.to_string(),
        module: UsdModule::USD_MODULE.to_string(),
        function: "mint".to_string(),
        type_args: vec![],
        args: vec![
            cap_id_bytes,
            bcs::to_bytes(&amount).context("Failed to encode mint amount")?,
            recipient_bytes,
        ],
        object_inputs: Some(vec![cap_input]),
        gas_limit: FAUCET_GAS_LIMIT,
        gas_price: FAUCET_GAS_PRICE,
        nonce: None,
        execute_immediate: Some(false),
    })
}

/// Build, sign (sponsor), and submit one 5 USD mint leg through the standard
/// `kanari_buildCallFunction` / `kanari_callFunction` handlers.
async fn mint_leg(
    state: &RpcServerState,
    service: &FaucetService,
    request_id: u64,
    cap: &kanari_rpc_api::ObjectInfo,
    recipient: &str,
    amount: u64,
) -> Result<Vec<u8>> {
    let mut cap_input = build_object_input(cap, &service.sender_tagged)?;
    cap_input.mutable = true;

    let cap_id_bytes = AccountAddress::from_hex_literal(&cap.id)
        .context("Invalid treasury cap object id")?
        .to_vec();
    let recipient_bytes = AccountAddress::from_hex_literal(recipient)
        .context("Invalid claim recipient")?
        .to_vec();

    let build_request = faucet_build_request(
        &service.sender_tagged,
        cap_input,
        cap_id_bytes,
        recipient_bytes,
        amount,
    )?;
    let build_response = super::handle_build_call_function(
        state,
        &RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: methods::BUILD_CALL_FUNCTION.to_string(),
            params: serde_json::to_value(&build_request)
                .context("Failed to encode faucet build request")?,
            id: request_id,
        },
    )
    .await;
    if let Some(message) = handler_error_message(&build_response) {
        anyhow::bail!("{message}");
    }
    let mut prepared: CallFunctionRequest = serde_json::from_value(
        build_response
            .result
            .clone()
            .context("Faucet build returned no prepared call")?,
    )
    .context("Faucet build returned an invalid prepared call")?;

    let mut unsigned = super::build_call_signed_tx(prepared.clone());
    unsigned
        .sign(&service.keypair.private_key, service.keypair.curve_type)
        .context("Failed to sign faucet mint transaction")?;
    prepared.signature = Some(unsigned.signature.clone());

    let submit_response = super::handle_call_function(
        state,
        &RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: methods::CALL_FUNCTION.to_string(),
            params: serde_json::to_value(&prepared)
                .context("Failed to encode faucet submit request")?,
            id: request_id,
        },
    )
    .await;
    if let Some(message) = handler_error_message(&submit_response) {
        anyhow::bail!("{message}");
    }
    let hash_hex = submit_response
        .result
        .as_ref()
        .and_then(|result| result.get("hash"))
        .and_then(|hash| hash.as_str())
        .context("Faucet submit returned no hash")?;
    hex::decode(hash_hex).context("Faucet submit returned an invalid hash")
}

async fn execute_claim(
    state: &RpcServerState,
    service: &FaucetService,
    request_id: u64,
    recipient: &str,
) -> Result<Vec<Vec<u8>>> {
    // Serialize claims: both legs touch the same shared cap, so leg 2 must
    // build on leg 1's committed version. Held across the commit wait
    // (devnet only).
    let _guard = service.claim_lock.lock().await;

    // Re-read the shared cap per leg: its version advances on every mint.
    let mut hashes = Vec::with_capacity(CLAIM_HALVES.len());
    for (leg, amount) in CLAIM_HALVES.iter().enumerate() {
        let cap = shared_cap_id(state)?;
        let hash = mint_leg(state, service, request_id, &cap, recipient, *amount).await?;
        if leg + 1 < CLAIM_HALVES.len() {
            wait_for_commit(state, &hash, COMMIT_TIMEOUT).await?;
        }
        hashes.push(hash);
    }
    Ok(hashes)
}

pub async fn handle_faucet_usd_status(request: &crate::RpcRequest) -> RpcResponse {
    let Some(service) = faucet_service() else {
        return respond_with_serialize(
            request.id,
            kanari_rpc_api::UsdFaucetStatusResponse {
                enabled: false,
                usd_type: USD_COIN.to_string(),
                usd_decimals: UsdModule::USD_DECIMALS,
                default_drip_usd: String::new(),
                max_per_request_usd: String::new(),
                cooldown_secs: 0,
            },
        );
    };
    respond_with_serialize(
        request.id,
        kanari_rpc_api::UsdFaucetStatusResponse {
            enabled: true,
            usd_type: USD_COIN.to_string(),
            usd_decimals: UsdModule::USD_DECIMALS,
            default_drip_usd: UsdModule::format_units_to_usd(CLAIM_HALVES.iter().sum()),
            max_per_request_usd: UsdModule::format_units_to_usd(CLAIM_HALVES.iter().sum()),
            cooldown_secs: service.config.cooldown.as_secs(),
        },
    )
}

pub async fn handle_request_usd_faucet(
    state: &RpcServerState,
    request: &RpcRequest,
    client_ip: Option<&str>,
) -> RpcResponse {
    let params: RequestUsdFaucetRequest = match serde_json::from_value(request.params.clone()) {
        Ok(params) => params,
        Err(error) => return invalid_params_response(request.id, error.to_string()),
    };
    let recipient = params.address.trim().to_lowercase();
    if !valid_address(&recipient) {
        return invalid_params_response(
            request.id,
            "Invalid address: expected 0x-prefixed 64 hex characters",
        );
    }
    let Some(service) = faucet_service() else {
        return internal_error_response(
            request.id,
            "USD faucet is disabled on this node (FAUCET_SPONSOR_KEY is not set)",
        );
    };

    {
        let log = service.last_claim.lock().expect("faucet log poisoned");
        if let Some(wait) = cooldown_remaining(&log, &recipient, service.config.cooldown) {
            return invalid_params_response(
                request.id,
                format!(
                    "Cooldown active for this address, try again in {}s",
                    wait.as_secs()
                ),
            );
        }
    }
    if let Some(ip) = client_ip {
        let log = service.last_claim_ip.lock().expect("faucet log poisoned");
        if let Some(wait) = cooldown_remaining(&log, ip, service.config.ip_cooldown) {
            return invalid_params_response(
                request.id,
                format!(
                    "Cooldown active for this network address, try again in {}s",
                    wait.as_secs()
                ),
            );
        }
    }
    // Validate the recipient parses as an account before minting.
    if let Err(error) = parse_hex_address(request.id, &recipient, "recipient address") {
        return *error;
    }

    // NOTE: the router applies a global 30s timeout. A claim that outlives it
    // (slow checkpoints) is cancelled mid-flight: leg 1 may already be
    // committed while leg 2 never builds. Cooldown is recorded only on full
    // success, so the client can safely retry (worst case: extra devnet USD).
    match execute_claim(state, &service, request.id, &recipient).await {
        Ok(hashes) => {
            let now = Instant::now();
            service
                .last_claim
                .lock()
                .expect("faucet log poisoned")
                .insert(recipient.clone(), now);
            if let Some(ip) = client_ip {
                let mut log = service.last_claim_ip.lock().expect("faucet log poisoned");
                record_claim(&mut log, ip.to_string(), now, service.config.ip_cooldown);
            }
            let leg_hashes: Vec<String> = hashes.iter().map(hex::encode).collect();
            let total: u64 = CLAIM_HALVES.iter().sum();
            respond_with_serialize(
                request.id,
                RequestUsdFaucetResponse {
                    hash: leg_hashes.last().cloned().unwrap_or_default(),
                    status: "pending".to_string(),
                    leg_hashes,
                    amount_usd: UsdModule::format_units_to_usd(total),
                    amount_units: total,
                    usd_type: USD_COIN.to_string(),
                    recipient,
                },
            )
        }
        Err(error) => {
            internal_error_response(request.id, format!("Faucet claim failed: {error:#}"))
        }
    }
}

/// Method names served by this module (re-exported for dispatch).
pub const FAUCET_METHODS: &[&str] = &[methods::REQUEST_USD_FAUCET, methods::GET_USD_FAUCET_STATUS];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claim_is_exactly_two_five_unit_legs() {
        assert_eq!(CLAIM_HALVES, [5_000_000, 5_000_000]);
        assert_eq!(CLAIM_HALVES.iter().sum::<u64>(), 10_000_000);
        assert_eq!(
            UsdModule::format_units_to_usd(CLAIM_HALVES.iter().sum()),
            "10"
        );
    }

    #[test]
    fn recipient_address_validation_is_strict() {
        assert!(valid_address(&format!("0x{}", "ab".repeat(32))));
        assert!(!valid_address("ab"));
        assert!(!valid_address("0x1234"));
        assert!(!valid_address(&format!("0x{}", "ab".repeat(33))));
        assert!(!valid_address(&format!("0x{}", "zz".repeat(32))));
        assert!(!valid_address(""));
    }

    #[test]
    fn ip_and_address_cooldowns_share_one_helper() {
        let mut log = HashMap::new();
        let cooldown = Duration::from_secs(60);
        assert_eq!(cooldown_remaining(&log, "1.2.3.4", cooldown), None);

        let now = Instant::now();
        log.insert("1.2.3.4".to_string(), now);
        let wait = cooldown_remaining(&log, "1.2.3.4", cooldown).unwrap();
        assert!(wait <= cooldown && !wait.is_zero());
        assert_eq!(cooldown_remaining(&log, "5.6.7.8", cooldown), None);

        log.insert("old".to_string(), now - cooldown - Duration::from_secs(1));
        assert_eq!(cooldown_remaining(&log, "old", cooldown), None);
    }

    #[test]
    fn ip_claim_log_is_memory_bounded() {
        let mut log = HashMap::new();
        let cooldown = Duration::from_secs(60);
        let now = Instant::now();
        for n in 0..MAX_IP_LOG_ENTRIES {
            log.insert(format!("10.0.0.{n}"), now);
        }
        // Full log of live entries keeps its size (no silent drops); expired
        // entries are pruned instead.
        record_claim(&mut log, "10.9.9.9".to_string(), now, cooldown);
        assert_eq!(log.len(), MAX_IP_LOG_ENTRIES + 1);

        let mut stale = HashMap::new();
        for n in 0..MAX_IP_LOG_ENTRIES {
            stale.insert(
                format!("10.1.0.{n}"),
                now - cooldown - Duration::from_secs(1),
            );
        }
        record_claim(&mut stale, "10.9.9.9".to_string(), now, cooldown);
        assert_eq!(stale.len(), 1);
    }

    #[test]
    fn usd_faucet_method_names_are_stable() {
        assert_eq!(methods::REQUEST_USD_FAUCET, "kanari_requestUsdFaucet");
        assert_eq!(methods::GET_USD_FAUCET_STATUS, "kanari_getUsdFaucetStatus");
        assert!(FAUCET_METHODS.contains(&methods::REQUEST_USD_FAUCET));
    }

    #[test]
    fn faucet_build_request_uses_standard_call_shape() {
        use kanari_types::transaction::{ObjectOwnerKind, ObjectRef};

        let cap_input = ObjectInput {
            object_ref: ObjectRef::new("0x1234".to_string(), Some(7), Some("0xabcd".to_string())),
            owner: Some(ObjectOwnerKind::Shared),
            mutable: true,
        };
        let request = faucet_build_request(
            "0x1::test::sponsor",
            cap_input,
            vec![0x12, 0x34],
            vec![0xab, 0xcd],
            5_000_000,
        )
        .unwrap();

        assert_eq!(request.package, Address::KANARI_SYSTEM_ADDRESS);
        assert_eq!(request.module, UsdModule::USD_MODULE);
        assert_eq!(request.function, "mint");
        assert_eq!(request.args.len(), 3);
        assert_eq!(bcs::from_bytes::<u64>(&request.args[1]).unwrap(), 5_000_000);
        let inputs = request.object_inputs.unwrap();
        assert_eq!(inputs.len(), 1);
        assert!(inputs[0].mutable);
        assert_eq!(request.nonce, None);
        assert_eq!(request.execute_immediate, Some(false));
    }
}
