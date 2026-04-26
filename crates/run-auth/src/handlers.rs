use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use kanari_crypto::keys::CurveType;
use tracing::{error, info, warn};

use crate::{
    AppState,
    models::{
        ApiResponse, ChangePasswordRequest, DeleteAccountRequest, EncryptedKeyResponse, ListUsersResponse, LoginRequest,
        LoginResponse, LogoutAllRequest, LogoutRequest, RegisterRequest, RegisterResponse,
        SignTransactionRequest, SignTransferRequest, UserInfoResponse, ValidateSessionResponse,
    },
};

/// Register a new user
///
/// Supported curve types:
/// - **Classical ECC**: `ed25519`, `k256` (or `secp256k1`), `p256` (or `secp256r1`, `nist`)
/// - **Post-Quantum**: `dilithium2`, `dilithium3`, `dilithium5`, `sphincsplus` (or `sphincs+sha256`, `sphincs`)
/// - **Hybrid**: `ed25519dilithium3` (or `ed25519+dilithium3`), `k256dilithium3` (or `k256+dilithium3`)
///
/// Default: `ed25519` (if curve_type not provided)
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> (StatusCode, Json<ApiResponse<RegisterResponse>>) {
    info!("Registration attempt for email: {}", payload.email);
    info!("Requested curve type: {:?}", payload.curve_type);

    // Parse curve type if provided
    let curve_type = if let Some(curve_str) = &payload.curve_type {
        info!("Parsing curve type string: '{}'", curve_str);
        match curve_str.as_str() {
            // Classical ECC
            "ed25519" => Some(CurveType::Ed25519),
            "k256" | "secp256k1" => Some(CurveType::K256),
            "p256" | "secp256r1" | "nist" => Some(CurveType::P256),

            // Post-Quantum Cryptography (PQC)
            "dilithium2" => Some(CurveType::Dilithium2),
            "dilithium3" => Some(CurveType::Dilithium3),
            "dilithium5" => Some(CurveType::Dilithium5),
            "sphincsplus" | "sphincs+sha256" | "sphincs" => {
                Some(CurveType::SphincsPlusSha256Robust)
            }

            // Hybrid Schemes
            "ed25519dilithium3" => Some(CurveType::Ed25519Dilithium3),
            "k256dilithium3" => Some(CurveType::K256Dilithium3),

            _ => {
                warn!("Invalid curve type: {}", curve_str);
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        "Invalid curve type. Supported: ed25519, k256, p256, dilithium2, dilithium3, dilithium5, sphincsplus, ed25519dilithium3, k256dilithium3",
                    )),
                );
            }
        }
    } else {
        info!("No curve type specified, will use default (Ed25519)");
        None
    };

    info!("Final curve type to use: {:?}", curve_type);

    let mut auth = state.auth_manager.lock().await;

    match auth.register_user(&payload.email, &payload.password, curve_type) {
        Ok(wallet) => {
            info!("User registered successfully: {}", payload.email);
            let response = RegisterResponse {
                success: true,
                wallet_address: wallet.address.to_hex_literal(),
                message: "User registered successfully".to_string(),
            };
            info!("Sending response: {:?}", serde_json::to_string(&response));
            (StatusCode::CREATED, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Registration failed: {:?}", e);
            let status_code = match e {
                kanari_auth::AuthError::UserAlreadyExists(_) => StatusCode::CONFLICT,
                kanari_auth::AuthError::InvalidEmail(_)
                | kanari_auth::AuthError::InvalidPassword(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status_code, Json(ApiResponse::error(format!("{:?}", e))))
        }
    }
}

/// Login user
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> (StatusCode, Json<ApiResponse<LoginResponse>>) {
    info!("Login attempt for email: {}", payload.email);

    let session_timeout = payload
        .session_timeout_hours
        .map(|hours| std::time::Duration::from_secs(hours * 3600));

    let mut auth = state.auth_manager.lock().await;

    match auth.login(&payload.email, &payload.password, session_timeout) {
        Ok(session) => {
            info!("Login successful: {}", payload.email);
            (
                StatusCode::OK,
                Json(ApiResponse::success(LoginResponse {
                    success: true,
                    session_id: session.session_id.clone(),
                    user_email: session.email.clone(),
                    wallet_address: session.wallet_address.clone(),
                    expires_at: session.expires_at.to_rfc3339(),
                })),
            )
        }
        Err(e) => {
            error!("Login failed: {:?}", e);
            let status_code = match e {
                kanari_auth::AuthError::AuthenticationFailed
                | kanari_auth::AuthError::UserNotFound(_) => StatusCode::UNAUTHORIZED,
                kanari_auth::AuthError::AccountLocked => StatusCode::FORBIDDEN,
                kanari_auth::AuthError::InvalidEmail(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status_code, Json(ApiResponse::error(format!("{:?}", e))))
        }
    }
}

/// Logout user
pub async fn logout(
    State(state): State<AppState>,
    Json(payload): Json<LogoutRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    info!("Logout attempt for session: {}", payload.session_id);

    let mut auth = state.auth_manager.lock().await;

    match auth.logout(&payload.session_id) {
        Ok(_) => {
            info!("Logout successful");
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "message": "Logged out successfully"
                }))),
            )
        }
        Err(e) => {
            error!("Logout failed: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("{:?}", e))),
            )
        }
    }
}

/// Logout all sessions for a user
pub async fn logout_all(
    State(state): State<AppState>,
    Json(payload): Json<LogoutAllRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    info!("Logout all sessions for email: {}", payload.email);

    let mut auth = state.auth_manager.lock().await;

    match auth.logout_all(&payload.email) {
        Ok(_) => {
            info!("All sessions logged out for: {}", payload.email);
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "message": "All sessions logged out successfully"
                }))),
            )
        }
        Err(e) => {
            error!("Logout all failed: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("{:?}", e))),
            )
        }
    }
}

/// Change password
pub async fn change_password(
    State(state): State<AppState>,
    Json(payload): Json<ChangePasswordRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    info!("Password change attempt for email: {}", payload.email);

    let mut auth = state.auth_manager.lock().await;

    match auth.change_password(&payload.email, &payload.old_password, &payload.new_password) {
        Ok(_) => {
            info!("Password changed successfully: {}", payload.email);
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "message": "Password changed successfully. All sessions invalidated."
                }))),
            )
        }
        Err(e) => {
            error!("Password change failed: {:?}", e);
            let status_code = match e {
                kanari_auth::AuthError::AuthenticationFailed => StatusCode::UNAUTHORIZED,
                kanari_auth::AuthError::UserNotFound(_) => StatusCode::NOT_FOUND,
                kanari_auth::AuthError::InvalidPassword(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status_code, Json(ApiResponse::error(format!("{:?}", e))))
        }
    }
}

/// Delete account
pub async fn delete_account(
    State(state): State<AppState>,
    Json(payload): Json<DeleteAccountRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    info!("Account deletion attempt for email: {}", payload.email);

    let mut auth = state.auth_manager.lock().await;

    match auth.delete_account(&payload.email, &payload.password) {
        Ok(_) => {
            info!("Account deleted successfully: {}", payload.email);
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "message": "Account deleted successfully"
                }))),
            )
        }
        Err(e) => {
            error!("Account deletion failed: {:?}", e);
            let status_code = match e {
                kanari_auth::AuthError::AuthenticationFailed => StatusCode::UNAUTHORIZED,
                kanari_auth::AuthError::UserNotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status_code, Json(ApiResponse::error(format!("{:?}", e))))
        }
    }
}

/// List all users
pub async fn list_users(
    State(state): State<AppState>,
) -> (StatusCode, Json<ApiResponse<ListUsersResponse>>) {
    info!("Listing all users");

    let auth = state.auth_manager.lock().await;

    let users = auth.list_users();
    let count = users.len();

    (
        StatusCode::OK,
        Json(ApiResponse::success(ListUsersResponse { users, count })),
    )
}

/// Get user count
pub async fn user_count(
    State(state): State<AppState>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    info!("Getting user count");

    let auth = state.auth_manager.lock().await;

    let count = auth.user_count();

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "count": count
        }))),
    )
}

/// Get user info from session
pub async fn get_user_info(
    State(_state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<ApiResponse<UserInfoResponse>>) {
    let _session_id = params.get("session_id");

    info!("Getting user info endpoint called");

    // This endpoint needs proper implementation with session validation
    // For now, return not implemented
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("This endpoint is not yet implemented")),
    )
}

/// Get user's encrypted private key for wallet restoration
pub async fn get_user_encrypted_key(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<ApiResponse<EncryptedKeyResponse>>) {
    let email = match params.get("email") {
        Some(email) => email.clone(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("Email parameter is required")),
            );
        }
    };

    info!("Encrypted key retrieval attempt for email: {}", email);

    let auth = state.auth_manager.lock().await;

    match auth.get_user_encrypted_key(&email) {
        Ok((email, wallet_address, encrypted_key)) => {
            info!("Encrypted key retrieved successfully for: {}", email);
            (
                StatusCode::OK,
                Json(ApiResponse::success(EncryptedKeyResponse {
                    success: true,
                    email,
                    wallet_address,
                    encrypted_private_key: encrypted_key,
                })),
            )
        }
        Err(e) => {
            error!("Encrypted key retrieval failed: {:?}", e);
            let status_code = match e {
                kanari_auth::AuthError::UserNotFound(_) => StatusCode::NOT_FOUND,
                kanari_auth::AuthError::AccountLocked => StatusCode::FORBIDDEN,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status_code, Json(ApiResponse::error(format!("{:?}", e))))
        }
    }
}

/// Sign a transfer transaction
pub async fn sign_transfer(
    State(state): State<AppState>,
    Json(payload): Json<SignTransferRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    info!(
        "Transfer signing attempt for session: {}",
        payload.session_id
    );

    let auth = state.auth_manager.lock().await;

    // Create a mock session - in production, validate session first
    let session = kanari_auth::Session {
        session_id: payload.session_id.clone(),
        email: String::new(), // Would be retrieved from session store
        wallet_address: String::new(),
        created_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
        last_activity: chrono::Utc::now(),
        is_valid: true,
    };

    match auth.sign_transfer(
        &session,
        &payload.recipient,
        payload.amount,
        payload.gas_limit,
        payload.gas_price,
    ) {
        Ok(signed_tx) => {
            info!("Transfer signed successfully");
            // Serialize the signed transaction
            match serde_json::to_string(&signed_tx) {
                Ok(tx_json) => (
                    StatusCode::OK,
                    Json(ApiResponse::success(serde_json::json!({
                        "signed_transaction": tx_json
                    }))),
                ),
                Err(e) => {
                    error!("Failed to serialize transaction: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error("Failed to serialize transaction")),
                    )
                }
            }
        }
        Err(e) => {
            error!("Transfer signing failed: {:?}", e);
            let status_code = match e {
                kanari_auth::AuthError::SessionExpired | kanari_auth::AuthError::InvalidSession => {
                    StatusCode::UNAUTHORIZED
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status_code, Json(ApiResponse::error(format!("{:?}", e))))
        }
    }
}

/// Sign a generic transaction
pub async fn sign_transaction(
    State(state): State<AppState>,
    Json(payload): Json<SignTransactionRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    info!(
        "Transaction signing attempt for session: {}",
        payload.session_id
    );

    // Parse the transaction JSON
    let transaction: kanari_types::transaction::Transaction =
        match serde_json::from_str(&payload.transaction_json) {
            Ok(tx) => tx,
            Err(e) => {
                error!("Failed to parse transaction JSON: {:?}", e);
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(format!(
                        "Invalid transaction JSON: {}",
                        e
                    ))),
                );
            }
        };

    let auth = state.auth_manager.lock().await;

    // Create a mock session - in production, validate session first
    let session = kanari_auth::Session {
        session_id: payload.session_id.clone(),
        email: String::new(),
        wallet_address: String::new(),
        created_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
        last_activity: chrono::Utc::now(),
        is_valid: true,
    };

    match auth.sign_transaction(&session, transaction) {
        Ok(signed_tx) => {
            info!("Transaction signed successfully");
            match serde_json::to_string(&signed_tx) {
                Ok(tx_json) => (
                    StatusCode::OK,
                    Json(ApiResponse::success(serde_json::json!({
                        "signed_transaction": tx_json
                    }))),
                ),
                Err(e) => {
                    error!("Failed to serialize transaction: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error("Failed to serialize transaction")),
                    )
                }
            }
        }
        Err(e) => {
            error!("Transaction signing failed: {:?}", e);
            let status_code = match e {
                kanari_auth::AuthError::SessionExpired | kanari_auth::AuthError::InvalidSession => {
                    StatusCode::UNAUTHORIZED
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status_code, Json(ApiResponse::error(format!("{:?}", e))))
        }
    }
}

/// Validate session
pub async fn validate_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<ValidateSessionResponse>>) {
    info!("Validating session: {}", session_id);

    let mut auth = state.auth_manager.lock().await;

    match auth.validate_session(&session_id) {
        Ok(_session) => (
            StatusCode::OK,
            Json(ApiResponse::success(ValidateSessionResponse {
                valid: true,
                session_id,
            })),
        ),
        Err(e) => {
            let is_expired = matches!(e, kanari_auth::AuthError::SessionExpired);
            let is_invalid = matches!(e, kanari_auth::AuthError::InvalidSession);

            if is_expired || is_invalid {
                (
                    StatusCode::OK,
                    Json(ApiResponse::success(ValidateSessionResponse {
                        valid: false,
                        session_id,
                    })),
                )
            } else {
                error!("Session validation failed: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(format!("{:?}", e))),
                )
            }
        }
    }
}
