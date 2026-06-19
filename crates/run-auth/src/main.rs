use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use axum::{
    Json, Router,
    body::Body,
    extract::{ConnectInfo, Request},
    http::{HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_client_ip::ClientIpSource;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use kanari_auth::AuthManager;

mod audit_logger;
mod handlers;
mod models;
mod rate_limiter;
mod security;
mod two_factor;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub auth_manager: Arc<Mutex<AuthManager>>,
    pub audit_logger: audit_logger::AuditLogger,
    pub rate_limiter: rate_limiter::RateLimiter,
    pub totp_manager: two_factor::TotpManager,
}

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

fn is_trusted_https_request(
    peer_ip: std::net::IpAddr,
    forwarded_proto: Option<&str>,
    trusted_proxy_ips: Option<&str>,
) -> bool {
    let peer_is_trusted = peer_ip.is_loopback()
        || trusted_proxy_ips.is_some_and(|trusted| {
            trusted.split(',').any(|candidate| {
                candidate
                    .trim()
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip == peer_ip)
            })
        });
    let forwarded_https = forwarded_proto
        .and_then(|value| value.split(',').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("https"));

    peer_is_trusted && forwarded_https
}
async fn enforce_https(request: Request<Body>, next: Next) -> Response {
    if env_flag("AUTH_ALLOW_INSECURE_HTTP") {
        return next.run(request).await;
    }

    let trusted_proxy_ips = std::env::var("AUTH_TRUSTED_PROXY_IPS").ok();
    let is_secure = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|peer| {
            is_trusted_https_request(
                peer.0.ip(),
                request
                    .headers()
                    .get("x-forwarded-proto")
                    .and_then(|value| value.to_str().ok()),
                trusted_proxy_ips.as_deref(),
            )
        })
        .unwrap_or(false);

    if !is_secure {
        return (
            StatusCode::UPGRADE_REQUIRED,
            "HTTPS is required; connect through the trusted local reverse proxy",
        )
            .into_response();
    }

    next.run(request).await
}

/// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "kanari-auth-api",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "run_auth=debug,tower_http=info,axum=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Kanari Auth API server...");

    // Create auth manager with persistent storage.
    let db_path =
        PathBuf::from(std::env::var("AUTH_DB_PATH").unwrap_or_else(|_| "data/auth.db".to_string()));
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
        security::secure_path(parent, true)?;
    }

    let auth_manager = AuthManager::with_persistence(db_path.clone())?;
    security::secure_path(&db_path, false)?;

    if auth_manager.has_legacy_two_factor_secrets()?
        && !env_flag("AUTH_ALLOW_LEGACY_TOTP_MIGRATION")
    {
        anyhow::bail!(
            "legacy plaintext TOTP secrets detected; run temporarily with AUTH_ALLOW_LEGACY_TOTP_MIGRATION=true and require affected users to log in"
        );
    }

    // Initialize audit logger. Failure to secure or open the log is fatal.
    let audit_log_dir = std::env::var("AUDIT_LOG_DIR").ok().map(PathBuf::from);
    let audit_logger = audit_logger::AuditLogger::new(audit_log_dir)?;
    tracing::info!("Audit logging enabled: {:?}", audit_logger.log_path());

    // Initialize rate limiter (strict for auth endpoints)
    let rate_limiter = rate_limiter::RateLimiter::new(rate_limiter::RateLimitConfig::strict());

    // Initialize TOTP manager
    let totp_manager = two_factor::TotpManager::new(None);

    let state = AppState {
        auth_manager: Arc::new(Mutex::new(auth_manager)),
        audit_logger,
        rate_limiter,
        totp_manager,
    };

    // Configure a single explicit browser origin. Native clients do not need CORS.
    let allow_insecure = env_flag("AUTH_ALLOW_INSECURE_HTTP");
    let allowed_origin = std::env::var("AUTH_ALLOWED_ORIGIN").ok();
    if !allow_insecure && allowed_origin.is_none() {
        anyhow::bail!("AUTH_ALLOWED_ORIGIN must be set in secure mode");
    }
    let cors = if let Some(origin) = allowed_origin {
        if origin == "*" {
            anyhow::bail!("AUTH_ALLOWED_ORIGIN cannot be '*'");
        }
        if !allow_insecure && !origin.starts_with("https://") {
            anyhow::bail!("AUTH_ALLOWED_ORIGIN must use https:// in secure mode");
        }
        CorsLayer::new()
            .allow_origin(HeaderValue::from_str(&origin)?)
            .allow_methods([Method::GET, Method::POST])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
    } else {
        CorsLayer::new()
    };

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(health_check))
        // Authentication routes
        .route("/api/v1/register", post(handlers::register))
        .route("/api/v1/login", post(handlers::login))
        .route("/api/v1/logout", post(handlers::logout))
        .route("/api/v1/logout-all", post(handlers::logout_all))
        .route("/api/v1/change-password", post(handlers::change_password))
        .route("/api/v1/delete-account", post(handlers::delete_account))
        // User management
        .route("/api/v1/user/info", get(handlers::get_user_info))
        // SECURITY FIX #5: Changed from GET to POST for encrypted key retrieval (requires session validation)
        .route(
            "/api/v1/user/encrypted-key",
            post(handlers::get_user_encrypted_key),
        )
        // Session validation
        .route("/api/v1/session/validate", get(handlers::validate_session))
        // Two-Factor Authentication routes
        .route("/api/v1/2fa/setup", post(handlers::setup_2fa))
        .route("/api/v1/2fa/enable", post(handlers::enable_2fa))
        .route("/api/v1/2fa/disable", post(handlers::disable_2fa))
        // Apply middleware and state
        .layer(ClientIpSource::ConnectInfo.into_extension())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(enforce_https))
        .with_state(state);

    // Get port from environment or default to 3000
    let port = std::env::var("AUTH_API_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()?;

    let bind_address =
        std::env::var("AUTH_BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string());
    let addr = format!("{}:{}", bind_address, port);
    tracing::info!("Kanari Auth API listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
#[cfg(test)]
mod security_tests {
    use super::is_trusted_https_request;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn direct_http_is_rejected() {
        assert!(!is_trusted_https_request(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            None,
            None,
        ));
    }

    #[test]
    fn loopback_https_proxy_is_trusted() {
        assert!(is_trusted_https_request(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            Some("https"),
            None,
        ));
    }

    #[test]
    fn unlisted_proxy_cannot_spoof_https() {
        assert!(!is_trusted_https_request(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 8)),
            Some("https"),
            Some("10.0.0.7"),
        ));
    }

    #[test]
    fn explicitly_listed_proxy_is_trusted() {
        assert!(is_trusted_https_request(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 8)),
            Some("https"),
            Some("10.0.0.7,10.0.0.8"),
        ));
    }
}
