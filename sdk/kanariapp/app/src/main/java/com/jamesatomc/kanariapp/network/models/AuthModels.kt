package com.jamesatomc.kanariapp.network.models

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class RegisterRequest(
    val email: String,
    val password: String,
    @SerialName("curve_type") val curveType: String? = null
)

@Serializable
data class RegisterResponse(
    val success: Boolean? = null,
    @SerialName("wallet_address") val walletAddress: String? = null,
    val message: String? = null
)

@Serializable
data class LoginRequest(
    val email: String,
    val password: String,
    @SerialName("totp_code") val totpCode: String? = null,
    @SerialName("backup_code") val backupCode: String? = null,
    @SerialName("session_timeout_hours") val sessionTimeoutHours: Int? = null
)

@Serializable
data class LoginResponse(
    val success: Boolean? = null,
    @SerialName("two_factor_enabled") val twoFactorEnabled: Boolean? = null,
    @SerialName("session_id") val sessionId: String? = null,
    @SerialName("user_email") val userEmail: String? = null,
    @SerialName("wallet_address") val walletAddress: String? = null,
    @SerialName("curve_type") val curveType: String? = null,
    @SerialName("encrypted_private_key") val encryptedPrivateKey: String? = null,
    @SerialName("expires_at") val expiresAt: String? = null
)

@Serializable
data class TwoFactorSetupRequest(
    val email: String,
    val password: String
)

@Serializable
data class TwoFactorSetupResponse(
    val success: Boolean? = null,
    val secret: String? = null,
    @SerialName("otpauth_url") val otpauthUrl: String? = null,
    @SerialName("qr_code_svg") val qrCodeSvg: String? = null,
    @SerialName("backup_codes") val backupCodes: List<String>? = null,
    val message: String? = null
)

@Serializable
data class Enable2faRequest(
    val email: String,
    val password: String,
    val code: String
)

@Serializable
data class Disable2faRequest(
    val email: String,
    val password: String,
    val code: String
)

@Serializable
data class LogoutRequest(
    @SerialName("session_id") val sessionId: String
)

@Serializable
data class LogoutAllRequest(
    val email: String,
    @SerialName("session_id") val sessionId: String
)

@Serializable
data class ChangePasswordRequest(
    val email: String,
    @SerialName("session_id") val sessionId: String,
    @SerialName("old_password") val oldPassword: String,
    @SerialName("new_password") val newPassword: String
)

@Serializable
data class DeleteAccountRequest(
    val email: String,
    @SerialName("session_id") val sessionId: String,
    val password: String
)

@Serializable
data class ValidateSessionResponse(
    val valid: Boolean,
    @SerialName("session_id") val sessionId: String
)

@Serializable
data class ApiResponse<T>(
    val success: Boolean,
    val data: T? = null,
    val error: String? = null
)
