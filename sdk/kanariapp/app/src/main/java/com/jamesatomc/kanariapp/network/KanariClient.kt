package com.jamesatomc.kanariapp.network

import android.util.Log
import com.jamesatomc.kanariapp.network.models.*
import com.jamesatomc.kanariapp.wallet.KanariWallet
import com.kanari.kanari_crypto.KanariCrypto
import retrofit2.converter.kotlinx.serialization.asConverterFactory
import kotlinx.serialization.json.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.logging.HttpLoggingInterceptor
import retrofit2.Retrofit
import xyz.mcxross.bcs.Bcs

class KanariClient(private val environment: KanariEnvironment) {
    private val json = Json {
        ignoreUnknownKeys = true
        coerceInputValues = true
        encodeDefaults = true
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
            params = JsonPrimitive(normalizeAddress(address))
        )
        val response = rpcService.getAccount(request)
        if (response.error != null) {
            if (response.error.message.contains("Owner not found", ignoreCase = true)) {
                return null
            }
            throw Exception(response.error.message)
        }
        return response.result
    }

    suspend fun getAllBalances(address: String): List<TokenBalance> {
        val request = RpcRequest(
            method = "kanari_getOwnerBalances",
            params = buildJsonObject { put("owner", normalizeAddress(address)) }
        )
        val response = rpcService.getAllBalances(request)
        if (response.error != null) {
            if (response.error.message.contains("Owner not found", ignoreCase = true)) {
                return emptyList()
            }
            throw Exception(response.error.message)
        }

        val result = response.result
        val list = mutableListOf<TokenBalance>()

        if (result is JsonObject) {
            val rawBalances = result["balances"] ?: result["token_balances"]
            if (rawBalances is JsonArray) {
                rawBalances.forEach {
                    list.add(json.decodeFromJsonElement<TokenBalance>(it))
                }
            } else if (rawBalances is JsonObject) {
                rawBalances.forEach { (key, value) ->
                    val amount = value.jsonPrimitive.longOrNull ?: 0L
                    list.add(TokenBalance(tokenType = key, amount = amount, symbol = key.split("::").last()))
                }
            }
        }
        return list
    }

    suspend fun getAllTransactions(address: String, limit: Int = 50): List<TransactionDetails> {
        val request = RpcRequest(
            method = "kanari_getAllTransactions",
            params = buildJsonObject {
                put("account", normalizeAddress(address))
                put("limit", limit)
            }
        )
        val response = rpcService.getAllTransactions(request)
        if (response.error != null) {
            if (response.error.message.contains("Owner not found", ignoreCase = true)) {
                return emptyList()
            }
            throw Exception(response.error.message)
        }
        return response.result ?: emptyList()
    }

    suspend fun buildNativeTransfer(
        sender: String,
        recipient: String,
        amount: ULong,
        gasLimit: ULong = 1000000uL,
        gasPrice: ULong = 1uL
    ): ObjectTransferData? {
        val request = RpcRequest(
            method = "kanari_buildNativeTransfer",
            params = buildJsonObject {
                put("sender", sender)
                put("recipient", normalizeAddress(recipient))
                put("amount", amount.toLong())
                put("gas_limit", gasLimit.toLong())
                put("gas_price", gasPrice.toLong())
                put("execute_immediate", true)
            }
        )
        val response = rpcService.buildNativeTransfer(request)
        if (response.error != null) throw Exception(response.error.message)
        return response.result
    }

    suspend fun submitObjectTransfer(data: ObjectTransferData): TransactionResult? {
        val request = RpcRequest(
            method = "kanari_submitObjectTransfer",
            params = json.encodeToJsonElement(data)
        )
        val response = rpcService.submitObjectTransfer(request)
        if (response.error != null) throw Exception(response.error.message)
        return response.result
    }

    suspend fun transfer(
        wallet: KanariWallet,
        recipient: String,
        amount: ULong
    ): TransactionResult? {
        val prepared = buildNativeTransfer(wallet.taggedAddress, recipient, amount)
            ?: throw Exception("Failed to build transaction: RPC returned no data")

        val coinRef = prepared.coinObjectRef
            ?: throw Exception("Transaction building failed: No valid coin object found for balance")

        // 1. Reconstruct EXACT ExecuteFunction payload for binary hashing.
        // This variant is index 4 in the Rust 'Transaction' enum.
        val txData = KanariTransaction.ExecuteFunction(
            sender = prepared.sender,
            module = "0x2::kanari",
            function = "transfer",
            typeArgs = emptyList(),
            args = listOf(
                hexToBytes(normalizeAddress(coinRef.objectId)).map { it.toUByte() },
                u64ToBytes(prepared.amount).map { it.toUByte() },
                hexToBytes(normalizeAddress(prepared.recipient)).map { it.toUByte() }
            ),
            objectInputs = listOf(
                ObjectInput(
                    objectRef = coinRef.copy(objectId = normalizeAddress(coinRef.objectId)),
                    owner = ObjectOwnerKind.AddressOwner(prepared.sender),
                    mutable = true
                )
            ),
            gasPayment = prepared.gasPayment?.let { gp ->
                gp.copy(
                    paymentObjects = gp.paymentObjects.map { it.copy(objectId = normalizeAddress(it.objectId)) }
                )
            },
            gasLimit = prepared.gasLimit,
            gasPrice = prepared.gasPrice,
            nonce = prepared.nonce ?: 0uL
        )

        // 2. Encode to BCS with explicit serializer to ensure polymorphism (enum tags) are handled.
        val txBytes = Bcs.encodeToByteArray(KanariTransaction.serializer(), txData)
        
        // 3. Hash with Blake3
        val hash = KanariCrypto.blake3Hash(txBytes)
        
        // 4. Sign the hash
        val signature = wallet.sign(hash)

        // 5. [DEBUG] Verify locally before submission to isolate signing bugs
        val isLocalValid = KanariCrypto.verifySignature(wallet.taggedAddress, hash, signature, wallet.curveType)
        Log.d("KanariClient", "Local signature verification: $isLocalValid")
        if (!isLocalValid) {
            throw Exception("Local signature verification failed. Private key or curve mismatch.")
        }

        // 6. Finalize request with signature
        val finalPrepared = prepared.copy(
            signature = signature.map { it.toInt() and 0xFF }
        )

        return submitObjectTransfer(finalPrepared)
    }

    suspend fun createEscrowDeal(
        walletAddress: String,
        dealId: String,
        sellerAddress: String,
        amount: Long,
        tokenType: String,
        description: String
    ): TransactionResult? {
        val request = RpcRequest(
            method = "kanari_createDeal",
            params = buildJsonArray {
                add(normalizeAddress(walletAddress))
                add(dealId)
                add(normalizeAddress(sellerAddress))
                add(amount)
                add(tokenType)
                add(description)
            }
        )
        val response = rpcService.executeTransaction(request)
        if (response.error != null) throw Exception(response.error.message)
        return response.result
    }

    suspend fun confirmEscrowDelivery(
        walletAddress: String,
        objectId: String,
        coinType: String,
        proofId: String
    ): TransactionResult? {
        val request = RpcRequest(
            method = "kanari_confirmDelivery",
            params = buildJsonArray {
                add(normalizeAddress(walletAddress))
                add(normalizeAddress(objectId))
                add(coinType)
                add(normalizeAddress(proofId))
            }
        )
        val response = rpcService.executeTransaction(request)
        if (response.error != null) throw Exception(response.error.message)
        return response.result
    }

    private fun hexToBytes(hex: String): ByteArray {
        val s = hex.removePrefix("0x")
        val clean = if (s.length % 2 != 0) "0$s" else s
        val data = ByteArray(clean.length / 2)
        for (i in 0 until clean.length step 2) {
            data[i / 2] = ((Character.digit(clean[i], 16) shl 4) + Character.digit(clean[i + 1], 16)).toByte()
        }
        return data
    }

    private fun u64ToBytes(value: ULong): ByteArray {
        val buffer = java.nio.ByteBuffer.allocate(8).order(java.nio.ByteOrder.LITTLE_ENDIAN)
        buffer.putLong(value.toLong())
        return buffer.array()
    }

    private fun normalizeAddress(address: String): String {
        val clean = address.removePrefix("0x").lowercase()
        return "0x${clean.padStart(64, '0')}"
    }
}
