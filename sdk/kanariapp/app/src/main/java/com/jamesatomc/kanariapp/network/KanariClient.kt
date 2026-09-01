package com.jamesatomc.kanariapp.network

import com.jamesatomc.kanariapp.network.models.*
import retrofit2.converter.kotlinx.serialization.asConverterFactory
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.add
import kotlinx.serialization.json.buildJsonArray
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.logging.HttpLoggingInterceptor
import retrofit2.Retrofit

class KanariClient(private val environment: KanariEnvironment) {
    private val json = Json { 
        ignoreUnknownKeys = true 
        coerceInputValues = true
    }
    
    private val okHttpClient = OkHttpClient.Builder()
        .addInterceptor(HttpLoggingInterceptor().apply {
            level = HttpLoggingInterceptor.Level.BODY
        })
        .build()

    val authService: AuthService = Retrofit.Builder()
        .baseUrl(environment.authUrl)
        .client(okHttpClient)
        .addConverterFactory(json.asConverterFactory("application/json".toMediaType()))
        .build()
        .create(AuthService::class.java)

    val rpcService: KanariRpcService = Retrofit.Builder()
        .baseUrl(environment.baseUrl)
        .client(okHttpClient)
        .addConverterFactory(json.asConverterFactory("application/json".toMediaType()))
        .build()
        .create(KanariRpcService::class.java)

    val escrowService: EscrowService = Retrofit.Builder()
        .baseUrl(environment.baseUrl)
        .client(okHttpClient)
        .addConverterFactory(json.asConverterFactory("application/json".toMediaType()))
        .build()
        .create(EscrowService::class.java)

    // Auth Helper Methods
    suspend fun login(email: String, password: String): ApiResponse<LoginResponse>? {
        return try {
            val response = authService.login(LoginRequest(email, password))
            if (response.isSuccessful) response.body()
            else ApiResponse(false, error = response.errorBody()?.string() ?: "Login failed")
        } catch (e: Exception) {
            ApiResponse(false, error = e.message)
        }
    }

    // RPC Helper Methods
    suspend fun getAccount(address: String): AccountInfo? {
        val request = RpcRequest(
            method = "kanari_getOwner",
            params = buildJsonArray { add(address) }.toList()
        )
        return try {
            val response = rpcService.getAccount(request)
            response.result
        } catch (e: Exception) {
            null
        }
    }

    suspend fun getAllBalances(address: String): List<TokenBalance> {
        val request = RpcRequest(
            method = "kanari_getAllBalances",
            params = buildJsonArray { add(address) }.toList()
        )
        return try {
            val response = rpcService.getAllBalances(request)
            response.result ?: emptyList()
        } catch (e: Exception) {
            emptyList()
        }
    }

    suspend fun transfer(
        address: String,
        recipient: String,
        amount: Long
    ): TransactionResult? {
        val request = RpcRequest(
            method = "kanari_transfer",
            params = buildJsonArray {
                add(address)
                add(recipient)
                add(amount)
            }.toList()
        )
        return try {
            val response = rpcService.executeTransaction(request)
            response.result
        } catch (e: Exception) {
            null
        }
    }
    
    // Additional methods for transactions etc. can be added here
}
