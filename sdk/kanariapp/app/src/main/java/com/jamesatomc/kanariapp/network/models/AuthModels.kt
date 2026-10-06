package com.jamesatomc.kanariapp.network.models

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class RegisterRequest(
    val email: String,
    val password: String,
    @SerialName("curveType") val curveType: String? = null
)

@Serializable
data class RegisterResponse(
    val success: Boolean? = null,
    @SerialName("walletAddress") val walletAddress: String? = null,
    val message: String? = null
)

@Serializable
data class LoginRequest(
    val email: String,
    val password: String,
    @SerialName("totpCode") val totpCode: String? = null,
    @SerialName("backupCode") val backupCode: String? = null,
    @SerialName("sessionTimeoutHours") val sessionTimeoutHours: Int? = null
)

@Serializable
data class LoginResponse(
    val success: Boolean? = null,
    @SerialName("twoFactorEnabled") val twoFactorEnabled: Boolean? = null,
    @SerialName("sessionId") val sessionId: String? = null,
    @SerialName("userEmail") val userEmail: String? = null,
    @SerialName("walletAddress") val walletAddress: String? = null,
    @SerialName("curveType") val curveType: String? = null,
    @SerialName("encryptedPrivateKey") val encryptedPrivateKey: String? = null,
    @SerialName("expiresAt") val expiresAt: String? = null
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
    @SerialName("otpauthUrl") val otpauthUrl: String? = null,
    @SerialName("qrCodeSvg") val qrCodeSvg: String? = null,
    @SerialName("backupCodes") val backupCodes: List<String>? = null,
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
    @SerialName("sessionId") val sessionId: String
)

@Serializable
data class LogoutAllRequest(
    val email: String,
    @SerialName("sessionId") val sessionId: String
)

@Serializable
data class ChangePasswordRequest(
    val email: String,
    @SerialName("sessionId") val sessionId: String,
    @SerialName("oldPassword") val oldPassword: String,
    @SerialName("newPassword") val newPassword: String
)

@Serializable
data class DeleteAccountRequest(
    val email: String,
    @SerialName("sessionId") val sessionId: String,
    val password: String
)

@Serializable
data class ValidateSessionResponse(
    val valid: Boolean,
    @SerialName("sessionId") val sessionId: String
)

@Serializable
data class ApiResponse<T>(
    val success: Boolean,
    val data: T? = null,
    val error: String? = null
)