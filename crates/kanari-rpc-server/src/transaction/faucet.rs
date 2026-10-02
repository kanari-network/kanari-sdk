// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! In-node USD faucet: web claim without any custodial transfer wallet.
//!
//! Flow: the visitor enters an address on the web page → the node mints
//! exactly 100 USD as **two** `Coin<USD>` objects (50 + 50) straight to that
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
//! - `FAUCET_COOLDOWN_SECS` (optional): per-address cooldown, default `3600`.
//!
//! Devnet only: never configure a funded mainnet key here.

use super::{
    build_object_input, fresh_nonce, normalize_addr, parse_hex_address, select_gas_payment_auto,
};
use crate::{RpcRequest, RpcResponse};
use crate::{
    RpcServerState, internal_error_response, invalid_params_response, respond_with_serialize,
};
use anyhow::{Context, Result};
use kanari_crypto::keys::{CurveType, KeyPair, keypair_from_private_key};
use kanari_rpc_api::{
    CallFunctionRequest, RequestUsdFaucetRequest, RequestUsdFaucetResponse, methods,
};
use kanari_types::address::Address;
use kanari_types::transaction::ObjectInput;
use kanari_types::usd_coin::{USD_COIN, UsdModule};
use move_core_types::account_address::AccountAddress;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Fixed claim: 100 USD total, minted as two 50 USD coin objects so the
/// recipient can immediately transfer again (transfers need a transfer coin
/// plus a SEPARATE gas coin).
const CLAIM_HALVES: [u64; 2] = [50_000_000, 50_000_000];
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
            .unwrap_or(3600);
        Ok(Self {
            sponsor_key,
            cooldown: Duration::from_secs(cooldown_secs),
        })
    }
}

struct FaucetService {
    config: FaucetConfig,
    keypair: KeyPair,
    sender_tagged: String,
    sender_hex: String,
    last_claim: Mutex<HashMap<String, Instant>>,
    /// Serializes claims across awaits (must be an async mutex: the guard is
    /// held while waiting for checkpoint commits, and a `std` guard there
    /// would make the handler future `!Send`).
    claim_lock: tokio::sync::Mutex<()>,
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
                "USD web faucet enabled: 100 USD (2x50) per claim, cooldown {}s",
                config.cooldown.as_secs()
            );
            let sender_tagged = keypair.tagged_address();
            let sender_hex = keypair.address.clone();
            let service = Arc::new(FaucetService {
                config,
                keypair,
                sender_tagged,
                sender_hex,
                last_claim: Mutex::new(HashMap::new()),
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

/// Build, sign (sponsor), and submit one 50 USD mint leg directly against
/// the engine.
async fn mint_leg(
    state: &RpcServerState,
    service: &FaucetService,
    cap: &kanari_rpc_api::ObjectInfo,
    recipient: &str,
    amount: u64,
    used_gas_ids: &mut HashSet<String>,
) -> Result<Vec<u8>> {
    let owner_info = state
        .engine
        .get_owner_info(&service.sender_hex)
        .context("Faucet sponsor not found in node state (fund it with KANARI once)")?;
    let owned_objects = owner_info
        .owned_objects
        .context("Faucet sponsor has no objects")?;
    let pending = state.engine.pending_access_keys_snapshot();

    let mut cap_input = build_object_input(cap, &service.sender_tagged)?;
    cap_input.mutable = true;

    let needs: BTreeMap<String, u64> = BTreeMap::new();
    let (gas_payment, effective_price) = select_gas_payment_auto(
        &owned_objects,
        &service.sender_tagged,
        FAUCET_GAS_LIMIT,
        FAUCET_GAS_PRICE,
        &[],
        &pending,
        &needs,
    )
    .context("Faucet sponsor cannot cover execution gas (fund it with KANARI once)")?;
    for payment in &gas_payment.payment_objects {
        used_gas_ids.insert(normalize_addr(&payment.object_id));
    }

    let cap_id_bytes = AccountAddress::from_hex_literal(&cap.id)
        .context("Invalid treasury cap object id")?
        .to_vec();
    let recipient_bytes = AccountAddress::from_hex_literal(recipient)
        .context("Invalid claim recipient")?
        .to_vec();
    let inputs: Vec<ObjectInput> = vec![cap_input];

    let call_data = CallFunctionRequest {
        sender: service.sender_tagged.clone(),
        package: Address::KANARI_SYSTEM_ADDRESS.to_string(),
        module: UsdModule::USD_MODULE.to_string(),
        function: "mint".to_string(),
        type_args: vec![],
        args: vec![
            cap_id_bytes,
            bcs::to_bytes(&amount).context("Failed to encode mint amount")?,
            recipient_bytes,
        ],
        object_inputs: Some(inputs),
        gas_limit: FAUCET_GAS_LIMIT,
        gas_price: effective_price,
        nonce: Some(fresh_nonce(None, owner_info.nonce)?),
        gas_payment: Some(gas_payment),
        signature: None,
        execute_immediate: Some(false),
    };

    let mut signed_tx = super::build_call_signed_tx(call_data);
    signed_tx
        .sign(&service.keypair.private_key, service.keypair.curve_type)
        .context("Failed to sign faucet mint transaction")?;

    let hashes = state
        .engine
        .submit_transactions_batch(vec![signed_tx.clone()])
        .context("Failed to submit faucet mint transaction")?;
    state.broadcast_submitted_transaction(signed_tx);
    hashes
        .into_iter()
        .next()
        .context("Faucet submit returned no hash")
}

async fn execute_claim(
    state: &RpcServerState,
    service: &FaucetService,
    recipient: &str,
) -> Result<Vec<Vec<u8>>> {
    // Serialize claims: both legs touch the same shared cap, so leg 2 must
    // build on leg 1's committed version. Held across the commit wait
    // (devnet only).
    let _guard = service.claim_lock.lock().await;

    // Re-read the shared cap per leg: its version advances on every mint.
    let mut hashes = Vec::with_capacity(CLAIM_HALVES.len());
    let mut used_gas_ids = HashSet::new();
    for (leg, amount) in CLAIM_HALVES.iter().enumerate() {
        let cap = shared_cap_id(state)?;
        let hash = mint_leg(state, service, &cap, recipient, *amount, &mut used_gas_ids).await?;
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
        if let Some(last) = log.get(&recipient)
            && last.elapsed() < service.config.cooldown
        {
            let wait_secs = service
                .config
                .cooldown
                .saturating_sub(last.elapsed())
                .as_secs();
            return invalid_params_response(
                request.id,
                format!("Cooldown active for this address, try again in {wait_secs}s"),
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
    match execute_claim(state, &service, &recipient).await {
        Ok(hashes) => {
            service
                .last_claim
                .lock()
                .expect("faucet log poisoned")
                .insert(recipient.clone(), Instant::now());
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
    fn claim_is_exactly_two_fifty_unit_legs() {
        assert_eq!(CLAIM_HALVES, [50_000_000, 50_000_000]);
        assert_eq!(CLAIM_HALVES.iter().sum::<u64>(), 100_000_000);
        assert_eq!(
            UsdModule::format_units_to_usd(CLAIM_HALVES.iter().sum()),
            "100"
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
    fn usd_faucet_method_names_are_stable() {
        assert_eq!(methods::REQUEST_USD_FAUCET, "kanari_requestUsdFaucet");
        assert_eq!(methods::GET_USD_FAUCET_STATUS, "kanari_getUsdFaucetStatus");
        assert!(FAUCET_METHODS.contains(&methods::REQUEST_USD_FAUCET));
    }
}
