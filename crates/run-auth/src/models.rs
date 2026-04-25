use serde::{Deserialize, Serialize};

/// Registration request
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    #[serde(rename = "curveType")]
    pub curve_type: Option<String>, // "ed25519", "secp256k1", "bls12_381"
}

/// Registration response
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub success: bool,
    #[serde(rename = "walletAddress")]
    pub wallet_address: String,
    pub message: String,
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    #[serde(rename = "sessionTimeoutHours")]
    pub session_timeout_hours: Option<u64>,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "userEmail")]
    pub user_email: String,
    #[serde(rename = "walletAddress")]
    pub wallet_address: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: String,
}

/// Logout request
#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

/// Logout all request
#[derive(Debug, Deserialize)]
pub struct LogoutAllRequest {
    pub email: String,
}

/// Change password request
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub email: String,
    #[serde(rename = "oldPassword")]
    pub old_password: String,
    #[serde(rename = "newPassword")]
    pub new_password: String,
}

/// Delete account request
#[derive(Debug, Deserialize)]
pub struct DeleteAccountRequest {
    pub email: String,
    pub password: String,
}

/// Transfer signing request
#[derive(Debug, Deserialize)]
pub struct SignTransferRequest {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub recipient: String,
    pub amount: u64,
    #[serde(rename = "gasLimit")]
    pub gas_limit: Option<u64>,
    #[serde(rename = "gasPrice")]
    pub gas_price: Option<u64>,
}

/// Generic transaction signing request
#[derive(Debug, Deserialize)]
pub struct SignTransactionRequest {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "transactionJson")]
    pub transaction_json: String, // JSON-encoded transaction
}

/// User info response
#[derive(Debug, Serialize)]
pub struct UserInfoResponse {
    pub email: String,
    #[serde(rename = "walletAddress")]
    pub wallet_address: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "lastLogin")]
    pub last_login: Option<String>,
}

/// List users response
#[derive(Debug, Serialize)]
pub struct ListUsersResponse {
    pub users: Vec<String>,
    pub count: usize,
}

/// Session validation response
#[derive(Debug, Serialize)]
pub struct ValidateSessionResponse {
    pub valid: bool,
    pub session_id: String,
}

/// Generic API response
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}
