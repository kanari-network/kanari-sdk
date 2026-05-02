use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use axum::{
    Json, Router,
    http::Method,
    routing::{get, post},
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use kanari_auth::AuthManager;

mod audit_logger;
mod handlers;
mod models;
mod rate_limiter;
mod two_factor;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub auth_manager: Arc<Mutex<AuthManager>>,
    pub audit_logger: audit_logger::AuditLogger,
    pub rate_limiter: rate_limiter::RateLimiter,
    pub totp_manager: two_factor::TotpManager,
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

    // Create auth manager with persistent storage
    let db_path = std::env::var("AUTH_DB_PATH").unwrap_or_else(|_| "data/auth.db".to_string());

    // Ensure data directory exists
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let auth_manager = AuthManager::with_persistence(db_path.into())?;

    // Initialize audit logger
    let audit_log_dir = std::env::var("AUDIT_LOG_DIR").ok().map(PathBuf::from);
    let audit_logger = audit_logger::AuditLogger::new(audit_log_dir);
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

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any);

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
        .route("/api/v1/users", get(handlers::list_users))
        .route("/api/v1/users/count", get(handlers::user_count))
        .route("/api/v1/user/info", get(handlers::get_user_info))
        // SECURITY FIX #5: Changed from GET to POST for encrypted key retrieval (requires session validation)
        .route(
            "/api/v1/user/encrypted-key",
            post(handlers::get_user_encrypted_key),
        )
        // Transaction signing
        .route("/api/v1/sign/transfer", post(handlers::sign_transfer))
        .route("/api/v1/sign/transaction", post(handlers::sign_transaction))
        // Session validation
        .route(
            "/api/v1/session/validate/{session_id}",
            get(handlers::validate_session),
        )
        // Two-Factor Authentication routes
        .route("/api/v1/2fa/setup", post(handlers::setup_2fa))
        .route("/api/v1/2fa/enable", post(handlers::enable_2fa))
        .route("/api/v1/2fa/disable", post(handlers::disable_2fa))
        .route("/api/v1/2fa/verify", post(handlers::verify_2fa))
        // Apply middleware and state
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Get port from environment or default to 3000
    let port = std::env::var("AUTH_API_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()?;

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Kanari Auth API listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
