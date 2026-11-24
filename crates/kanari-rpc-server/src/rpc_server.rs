// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::rpc_runtime::{VM_RPC_PERMIT, is_vm_heavy_rpc, run_rpc_dispatch};
use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Request, State, connect_info::ConnectInfo},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use kanari_types::{GAS_MODEL, GasConfig, effective_gas_price};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tower_http::cors::{Any, CorsLayer};
use tower_http::{limit::RequestBodyLimitLayer, timeout::TimeoutLayer};
use tracing::{debug, info};

const MAX_RPC_BODY_BYTES: usize = 2 * 1024 * 1024;

use crate::{
    balance::{
        handle_get_fungible_asset, handle_get_fungible_asset_holders, handle_get_owner,
        handle_get_owner_balances, handle_get_token_balance, handle_list_tokens,
    },
    block::{
        handle_compare_canonical_state_snapshot, handle_get_block, handle_get_block_height,
        handle_get_canonical_state_snapshot, handle_get_full_block, handle_get_smt_status,
        handle_get_stats,
    },
    krc_1155::{handle_get_krc1155_token_type, handle_get_owned_krc1155_balances},
    module::{
        handle_get_module, handle_get_object, handle_get_object_by_ref, handle_get_objects,
        handle_get_objects_by_type, handle_get_owned_objects, handle_list_modules,
        handle_verify_module,
    },
    nft::{handle_get_nfts_by_collection, handle_get_owned_nfts, handle_list_collections},
    transaction::{
        faucet::{handle_faucet_usd_status, handle_request_usd_faucet},
        handle_build_call_function, handle_build_krc1155_transfer,
        handle_build_native_coin_consolidation, handle_build_native_transfer,
        handle_build_nft_transfer, handle_build_publish_module, handle_build_publish_package,
        handle_build_token_transfer, handle_call_function, handle_get_fungible_asset_transactions,
        handle_get_transaction, handle_publish_module, handle_publish_package,
        handle_submit_object_transfer, handle_view_function,
    },
};

// Per-IP fixed-window request rate limiter. With a zero-fee gas model there is
// no economic spam barrier, so the RPC edge must bound how many requests a
// single source can issue per window independently of transaction economics.
#[derive(Clone, Default)]
pub(crate) struct RpcRateLimiter {
    pub(crate) windows: Arc<Mutex<HashMap<IpAddr, RateWindow>>>,
}

#[derive(Clone, Copy)]
pub(crate) struct RateWindow {
    pub(crate) started_at: Instant,
    pub(crate) request_count: u32,
}

pub(crate) const RPC_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(1);
pub(crate) const RPC_RATE_LIMIT_PER_WINDOW: u32 = 200;
pub(crate) const RPC_RATE_LIMITER_MAX_TRACKED_IPS: usize = 10_000;

impl RpcRateLimiter {
    pub(crate) fn allow(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut windows = self.windows.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if windows.len() >= RPC_RATE_LIMITER_MAX_TRACKED_IPS && !windows.contains_key(&ip) {
            // Opportunistically drop expired windows so a long-lived public
            // node does not accumulate one entry per source address forever.
            windows
                .retain(|_, window| now.duration_since(window.started_at) < RPC_RATE_LIMIT_WINDOW);
            // A distributed flood can keep every tracked window active within
            // the same second, so expired-window pruning alone cannot shrink
            // the map. Evict the oldest source so memory stays hard-bounded
            // regardless of how many distinct addresses arrive.
            if windows.len() >= RPC_RATE_LIMITER_MAX_TRACKED_IPS {
                let mut oldest: Vec<(IpAddr, Instant)> =
                    windows.iter().map(|(addr, w)| (*addr, w.started_at)).collect();
                oldest.sort_by_key(|(_, t)| *t);
                let evict_count = windows.len() - RPC_RATE_LIMITER_MAX_TRACKED_IPS + 1;
                for (addr, _) in oldest.into_iter().take(evict_count) {
                    windows.remove(&addr);
                }
            }
        }
        let window = windows.entry(ip).or_insert(RateWindow { started_at: now, request_count: 0 });
        if now.duration_since(window.started_at) >= RPC_RATE_LIMIT_WINDOW {
            *window = RateWindow { started_at: now, request_count: 0 };
        }
        if window.request_count >= RPC_RATE_LIMIT_PER_WINDOW {
            return false;
        }
        window.request_count += 1;
        true
    }
}

/// Reject requests from a single source that exceed the per-window budget.
/// Falls back to a loopback identity when the server is not served with
/// [`SocketAddr`] connect info (e.g. in unit tests), which still rate-limits
/// but cannot attribute requests per source.
async fn rate_limit_by_ip(
    State(limiter): State<RpcRateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    let ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect_info| connect_info.0.ip())
        .unwrap_or_else(|| IpAddr::from([127, 0, 0, 1]));
    if !limiter.allow(ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32005,
                    "message": "Rate limit exceeded; retry later",
                },
                // RpcResponse.id is a non-optional u64 in the client schema, so
                // a JSON-RPC null here breaks every client that hits the limit.
                // Echoing a literal id is the pragmatic choice without buffering
                // and re-serializing the request body to read the real id.
                "id": 0,
            })),
        )
            .into_response();
    }
    next.run(request).await
}

/// Create RPC server router
pub fn create_router(state: RpcServerState) -> Router {
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    Router::new()
        .route("/", post(handle_rpc))
        .route("/rpc", post(handle_rpc))
        .route("/metrics", get(handle_metrics))
        .layer(RequestBodyLimitLayer::new(MAX_RPC_BODY_BYTES))
        .layer(tower::limit::ConcurrencyLimitLayer::new(128))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            std::time::Duration::from_secs(30),
        ))
        .layer(cors)
        .with_state(state)
}

/// Create the RPC router with per-IP rate limiting enabled. Production servers
/// should use this entry point: with a zero-fee gas model there is no economic
/// spam barrier, so request volume must be bounded at the network edge. Tests
/// keep using [`create_router`] so bursty unit tests are not throttled.
pub fn create_router_with_anti_spam(state: RpcServerState) -> Router {
    create_router(state)
        .layer(middleware::from_fn_with_state(RpcRateLimiter::default(), rate_limit_by_ip))
}

/// Handle RPC request
async fn handle_rpc(State(state): State<RpcServerState>, req: Request) -> impl IntoResponse {
    // Read the client IP straight from the request extensions: `Option<T>`
    // extractors are not supported for `ConnectInfo` in axum 0.8, and a
    // required `ConnectInfo` extractor would 500 every router-level test that
    // posts without connect info.
    let client_ip =
        req.extensions().get::<ConnectInfo<SocketAddr>>().map(|info| info.0.ip().to_string());
    let is_json =
        req.headers().get(header::CONTENT_TYPE).and_then(|value| value.to_str().ok()).is_some_and(
            |value| {
                value.split(';').next().is_some_and(|media_type| {
                    media_type.trim().eq_ignore_ascii_case("application/json")
                })
            },
        );
    if !is_json {
        return (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Json(serde_json::json!({
                "jsonrpc": "2.0",
                "error": { "code": -32700, "message": "Content-Type must be application/json" },
                "id": 0
            })),
        )
            .into_response();
    }
    let body = match to_bytes(req.into_body(), MAX_RPC_BODY_BYTES).await {
        Ok(body) => body,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": { "code": -32700, "message": format!("Unable to read request body: {error}") },
                    "id": 0
                })),
            )
                .into_response();
        }
    };
    let request = match run_rpc_dispatch(async move { serde_json::from_slice::<RpcRequest>(&body) })
        .await
    {
        Ok(Ok(request)) => request,
        Ok(Err(error)) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": { "code": -32700, "message": format!("Invalid JSON-RPC request: {error}") },
                    "id": 0
                })),
            )
                .into_response();
        }
        Err(error) => {
            return (StatusCode::SERVICE_UNAVAILABLE, Json(internal_error_response(0, error)))
                .into_response();
        }
    };
    debug!("RPC request: method={}, id={}", request.method, request.id);

    let _vm_permit = if is_vm_heavy_rpc(&request.method) {
        match VM_RPC_PERMIT.try_acquire() {
            Ok(permit) => Some(permit),
            Err(_) => {
                return (
                    StatusCode::OK,
                    Json(invalid_params_response(
                        request.id,
                        "VM RPC capacity is temporarily exhausted; retry later",
                    )),
                )
                    .into_response();
            }
        }
    } else {
        None
    };

    let request_id = request.id;
    let dispatch = async move {
        // Dispatch the request to the appropriate handler based on the method name
        match request.method.as_str() {
            // Account & Balance
            methods::GET_OWNER => handle_get_owner(&state, &request).await,
            methods::GET_TOKEN_BALANCE => handle_get_token_balance(&state, &request).await,
            methods::LIST_TOKENS => handle_list_tokens(&state, &request).await,
            methods::GET_OWNER_BALANCES => handle_get_owner_balances(&state, &request).await,
            methods::GET_FUNGIBLE_ASSET => handle_get_fungible_asset(&state, &request).await,
            methods::GET_FUNGIBLE_ASSET_HOLDERS => {
                handle_get_fungible_asset_holders(&state, &request).await
            }
            methods::GET_FUNGIBLE_ASSET_TRANSACTIONS => {
                handle_get_fungible_asset_transactions(&state, &request).await
            }

            // Blocks & Transactions
            methods::GET_BLOCK => handle_get_block(&state, &request).await,
            methods::GET_FULL_BLOCK => handle_get_full_block(&state, &request).await,
            methods::GET_TRANSACTION => handle_get_transaction(&state, &request).await,
            methods::GET_ALL_TRANSACTIONS => {
                transaction::handle_get_all_transactions(&state, &request).await
            }
            methods::COUNT_TRANSACTIONS => {
                transaction::handle_count_transactions(&state, &request).await
            }
            methods::GET_BLOCK_HEIGHT => handle_get_block_height(&state, &request).await,
            methods::GET_STATS => handle_get_stats(&state, &request).await,
            methods::GET_SMT_STATUS => handle_get_smt_status(&state, &request).await,
            methods::GET_CANONICAL_STATE_SNAPSHOT => {
                handle_get_canonical_state_snapshot(&state, &request).await
            }
            methods::COMPARE_CANONICAL_STATE_SNAPSHOT => {
                handle_compare_canonical_state_snapshot(&state, &request).await
            }
            methods::BUILD_NATIVE_TRANSFER => handle_build_native_transfer(&state, &request).await,
            methods::BUILD_NATIVE_COIN_CONSOLIDATION => {
                handle_build_native_coin_consolidation(&state, &request).await
            }
            methods::SUBMIT_OBJECT_TRANSFER => {
                handle_submit_object_transfer(&state, &request).await
            }

            // Health
            methods::HEALTH => handle_health(&state, &request).await,
            methods::GET_GAS_INFO => handle_gas_info(&request).await,
            methods::GET_NETWORK_STATUS => handle_network_status(&state, &request).await,

            // Module operations
            methods::BUILD_PUBLISH_MODULE | methods::BUILD_UPGRADE_MODULE => {
                handle_build_publish_module(&state, &request).await
            }
            methods::PUBLISH_MODULE | methods::UPGRADE_MODULE => {
                handle_publish_module(&state, &request).await
            }
            methods::BUILD_PUBLISH_PACKAGE | methods::BUILD_UPGRADE_PACKAGE => {
                handle_build_publish_package(&state, &request).await
            }
            methods::PUBLISH_PACKAGE | methods::UPGRADE_PACKAGE => {
                handle_publish_package(&state, &request).await
            }
            methods::GET_MODULE => handle_get_module(&state, &request).await,
            methods::LIST_MODULES => handle_list_modules(&state, &request).await,
            methods::VERIFY_MODULE => handle_verify_module(&state, &request).await,

            // Function calls
            methods::BUILD_CALL_FUNCTION => handle_build_call_function(&state, &request).await,
            methods::BUILD_TOKEN_TRANSFER => handle_build_token_transfer(&state, &request).await,
            methods::BUILD_NFT_TRANSFER => handle_build_nft_transfer(&state, &request).await,
            methods::BUILD_KRC1155_TRANSFER => {
                handle_build_krc1155_transfer(&state, &request).await
            }
            methods::CALL_FUNCTION => handle_call_function(&state, &request).await,
            methods::VIEW_FUNCTION => handle_view_function(&state, &request).await,

            // Web USD faucet: fixed 100 USD claim as one coin object.
            methods::REQUEST_USD_FAUCET => {
                handle_request_usd_faucet(&state, &request, client_ip.as_deref()).await
            }
            methods::GET_USD_FAUCET_STATUS => handle_faucet_usd_status(&request).await,

            // Object queries
            methods::GET_OBJECT => handle_get_object(&state, &request).await,
            methods::GET_OBJECT_BY_REF => handle_get_object_by_ref(&state, &request).await,
            methods::GET_OBJECTS => handle_get_objects(&state, &request).await,
            methods::GET_OWNED_OBJECTS => handle_get_owned_objects(&state, &request).await,
            methods::GET_OBJECTS_BY_TYPE => handle_get_objects_by_type(&state, &request).await,

            // NFT queries
            methods::GET_OWNED_NFTS => handle_get_owned_nfts(&state, &request).await,

            // KRC-1155 queries
            methods::GET_OWNED_KRC1155_BALANCES => {
                handle_get_owned_krc1155_balances(&state, &request).await
            }
            methods::GET_KRC1155_TOKEN_TYPE => {
                handle_get_krc1155_token_type(&state, &request).await
            }

            // collection queries
            methods::LIST_COLLECTIONS => handle_list_collections(&state, &request).await,
            methods::GET_NFTS_BY_COLLECTION => {
                handle_get_nfts_by_collection(&state, &request).await
            }

            _ => error_response(request.id, RpcError::method_not_found(&request.method)),
        }
    };
    let response = match run_rpc_dispatch(dispatch).await {
        Ok(response) => response,
        Err(error) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(internal_error_response(request_id, error)),
            )
                .into_response();
        }
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// Handle Prometheus metrics
async fn handle_metrics(State(state): State<RpcServerState>) -> Response {
    let result =
        run_rpc_dispatch(async move { state.engine.export_consensus_metrics_prometheus() }).await;
    match result {
        Err(error) => (
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            error,
        )
            .into_response(),
        Ok(Err(error)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            format!("failed to export metrics: {error}"),
        )
            .into_response(),
        Ok(Ok(metrics)) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
            metrics,
        )
            .into_response(),
    }
}

/// Start RPC server
pub async fn start_server(engine: Arc<BlockchainEngine>, addr: &str) -> Result<()> {
    let state = RpcServerState::new(engine);
    let app = create_router_with_anti_spam(state);

    info!("Starting RPC server on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;

    Ok(())
}

/// Start RPC server with transaction broadcaster
pub async fn start_server_with_transaction_broadcaster(
    engine: Arc<BlockchainEngine>,
    addr: &str,
    broadcaster: impl Fn(SignedTransaction) -> Result<()> + Send + Sync + 'static,
) -> Result<()> {
    let state = RpcServerState::with_transaction_broadcaster(engine, broadcaster);
    let app = create_router_with_anti_spam(state);

    info!("Starting RPC server on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;

    Ok(())
}

/// Handle health check
async fn handle_health(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let report = state.engine.runtime_health_report();

    let health = HealthStatus {
        status: report.status().to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.started_at.elapsed().as_secs(),
        sync_status: if report.guards.network == "local" {
            "local".to_string()
        } else {
            "unknown".to_string()
        },
        network: report.guards.network,
        supply_invariants_ok: report.supply_invariants_ok,
        supply_invariant_error: report.supply_invariant_error,
        fail_fast_enabled: report.guards.fail_fast_supply_enabled,
        strict_persistence_required: report.guards.strict_persistence_required,
        strict_checkpoint_roots: report.guards.strict_checkpoint_roots,
        persistent_storage_available: report.guards.persistent_storage_available,
    };

    respond_with_serialize(request.id, health)
}

async fn handle_gas_info(request: &RpcRequest) -> RpcResponse {
    let config = GasConfig::default();
    let requested = config.default_transaction_gas_price();
    respond_with_value(
        request.id,
        serde_json::json!({
            "model": GAS_MODEL,
            "requested_gas_price": requested,
            "effective_gas_price": effective_gas_price(requested),
            "minimum_gas_price": config.min_gas_price,
            "gas_limit": config.max_gas_per_tx,
            "storage_price_per_byte": config.storage_price_per_byte,
            "storage_rebate_rate": config.storage_rebate_rate,
        }),
    )
}

async fn handle_network_status(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let local_authority_id = state.engine.authority_id().to_string();
    let authorities = state
        .engine
        .authorities()
        .iter()
        .map(|authority_id| NetworkAuthorityStatus {
            authority_id: authority_id.clone(),
            local: authority_id == &local_authority_id,
        })
        .collect::<Vec<_>>();

    let status =
        NetworkStatus { local_authority_id, authority_count: authorities.len(), authorities };

    respond_with_serialize(request.id, status)
}
