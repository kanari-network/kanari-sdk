package com.jamesatomc.kanariapp.network

import com.jamesatomc.kanariapp.network.models.*
import retrofit2.Response
import retrofit2.http.*

interface AuthService {
    @POST("api/v1/register")
    suspend fun register(@Body request: RegisterRequest): Response<ApiResponse<RegisterResponse>>

    @POST("api/v1/login")
    suspend fun login(@Body request: LoginRequest): Response<ApiResponse<LoginResponse>>

    @POST("api/v1/2fa/setup")
    suspend fun setup2fa(@Header("Authorization") token: String, @Body request: TwoFactorSetupRequest): Response<ApiResponse<TwoFactorSetupResponse>>

    @POST("api/v1/2fa/enable")
    suspend fun enable2fa(@Header("Authorization") token: String, @Body request: Enable2faRequest): Response<ApiResponse<Map<String, String>>>

    @POST("api/v1/2fa/disable")
    suspend fun disable2fa(@Header("Authorization") token: String, @Body request: Disable2faRequest): Response<ApiResponse<Map<String, String>>>

    @POST("api/v1/logout")
    suspend fun logout(@Body request: LogoutRequest): Response<ApiResponse<Map<String, String>>>

    @POST("api/v1/logout-all")
    suspend fun logoutAll(@Body request: LogoutAllRequest): Response<ApiResponse<Map<String, String>>>

    @POST("api/v1/change-password")
    suspend fun changePassword(@Body request: ChangePasswordRequest): Response<ApiResponse<Map<String, String>>>

    @POST("api/v1/delete-account")
    suspend fun deleteAccount(@Body request: DeleteAccountRequest): Response<ApiResponse<Map<String, String>>>

    @GET("api/v1/session/validate")
    suspend fun validateSession(@Header("Authorization") token: String): Response<ApiResponse<ValidateSessionResponse>>
}