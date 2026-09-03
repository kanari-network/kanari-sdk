package com.jamesatomc.kanariapp.network

import com.jamesatomc.kanariapp.network.models.*
import retrofit2.converter.kotlinx.serialization.asConverterFactory
import kotlinx.serialization.json.*
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
            params = buildJsonArray { add(address) }
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
            method = "kanari_getOwnerBalances",
            params = buildJsonArray { add(address) }
        )
        return try {
            val response = rpcService.getAllBalances(request)
            response.result ?: emptyList()
        } catch (e: Exception) {
            emptyList()
        }
    }

    suspend fun getAllTransactions(address: String, limit: Int = 50): List<TransactionDetails> {
        val request = RpcRequest(
            method = "kanari_getAllTransactions",
            params = buildJsonArray {
                add(address)
                add(limit)
            }
        )
        return try {
            val response = rpcService.getAllTransactions(request)
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
            }
        )
        return try {
            val response = rpcService.executeTransaction(request)
            response.result
        } catch (e: Exception) {
            null
        }
    }

    suspend fun transferToken(
        address: String,
        recipient: String,
        tokenType: String,
        amount: Long
    ): TransactionResult? {
        val request = RpcRequest(
            method = "kanari_transferToken",
            params = buildJsonArray {
                add(address)
                add(recipient)
                add(tokenType)
                add(amount)
            }
        )
        return try {
            val response = rpcService.executeTransaction(request)
            response.result
        } catch (e: Exception) {
            null
        }
    }

    // Escrow Operations
    suspend fun createEscrowDeal(
        address: String,
        dealId: String,
        seller: String,
        amount: Long,
        tokenType: String,
        description: String
    ): TransactionResult? {
        val request = RpcRequest(
            method = "kanari_createDeal",
            params = buildJsonArray {
                add(address)
                add(dealId)
                add(seller)
                add(amount)
                add(description)
                add(tokenType)
            }
        )
        return try {
            val response = rpcService.executeTransaction(request)
            response.result
        } catch (e: Exception) {
            null
        }
    }

    suspend fun confirmEscrowDelivery(
        address: String,
        dealObjectId: String,
        coinType: String,
        proofObjectId: String
    ): TransactionResult? {
        val request = RpcRequest(
            method = "kanari_confirmDelivery",
            params = buildJsonArray {
                add(address)
                add(dealObjectId)
                add(coinType)
                add(proofObjectId)
            }
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
