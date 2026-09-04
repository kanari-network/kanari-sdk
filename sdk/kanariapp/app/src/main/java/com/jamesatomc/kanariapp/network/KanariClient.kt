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
import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder

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
        gasLimit: ULong = 100000uL,
        gasPrice: ULong = 1000uL
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

    suspend fun buildTokenTransfer(
        sender: String,
        recipient: String,
        tokenType: String,
        amount: ULong,
        gasLimit: ULong = 100000uL,
        gasPrice: ULong = 1000uL
    ): CallFunctionData? {
        val normalizedTokenType = normalizeTokenType(tokenType)
        val request = RpcRequest(
            method = "kanari_buildTokenTransfer",
            params = buildJsonObject {
                put("sender", sender)
                put("recipient", normalizeAddress(recipient))
                put("token_type", normalizedTokenType)
                put("amount", amount.toLong())
                put("gas_limit", gasLimit.toLong())
                put("gas_price", gasPrice.toLong())
                put("execute_immediate", true)
            }
        )
        val response = rpcService.buildTokenTransfer(request)
        if (response.error != null) throw Exception(response.error.message)
        return response.result
    }

    suspend fun submitCallFunction(data: CallFunctionData): TransactionResult? {
        val request = RpcRequest(
            method = "kanari_callFunction",
            params = json.encodeToJsonElement(data)
        )
        val response = rpcService.callFunction(request)
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
            ?: throw Exception("Transaction building failed: No valid coin object found for balance. Native KANARI transfer needs two distinct Coin<0x2::kanari::KANARI> objects: one coin to send and another coin to pay gas. Receive/fund this wallet once more, then try again.")

        // Validate nonce - must be non-zero and present. Large u64 values may have been truncated if parsed as signed Long.
        val nonce = prepared.nonce
        if (nonce == null || nonce == 0uL) {
            throw Exception("Prepared transaction is missing a valid nonce (got: $nonce). Please try again.")
        }

        val normalizedRecipient = normalizeAddress(prepared.recipient)
        val normalizedCoinObjectId = normalizeAddress(coinRef.objectId)

        // Use gasPayment as returned by server, normalizing only object_ids to 32-byte form.
        val gasPayment = prepared.gasPayment
            ?: throw Exception("Prepared transaction missing gas_payment. Insufficient gas coin?")

        val normalizedGasPayment = gasPayment.copy(
            paymentObjects = gasPayment.paymentObjects.map { it.copy(objectId = normalizeAddress(it.objectId)) }
        )

        // Build BCS bytes manually to guarantee exact match with Rust `Transaction::hash()` (bcs::to_bytes + blake3)
        // Rust Transaction enum index: 0 PublishModule, 1 PublishPackage, 2 UpgradeModule, 3 UpgradePackage, 4 ExecuteFunction
        val txBytes = bcsEncodeNativeTransfer(
            sender = prepared.sender,
            coinObjectId = normalizedCoinObjectId,
            coinRef = coinRef.copy(objectId = normalizedCoinObjectId),
            recipient = normalizedRecipient,
            amount = prepared.amount,
            gasPayment = normalizedGasPayment,
            gasLimit = prepared.gasLimit,
            gasPrice = prepared.gasPrice,
            nonce = nonce
        )

        // Hash with Blake3 (same as kanari_crypto::hash_data_blake3 / Transaction::hash)
        val hash = KanariCrypto.blake3Hash(txBytes)

        // Sign the hash with the wallet's curve
        val signature = wallet.sign(hash)

        // Local verify is best-effort only - swallow CALL_ERROR to avoid masking real RPC error.
        // Previous implementation passed taggedAddress as raw hex which caused `Unexpected CALL_ERROR`.
        try {
            val isLocalValid = KanariCrypto.verifySignature(wallet.taggedAddress, hash, signature, wallet.curveType)
            Log.d("KanariClient", "Local signature verification: $isLocalValid")
            // Don't throw on false - let server be the source of truth; just log.
            if (!isLocalValid) {
                Log.w(
                    "KanariClient",
                    "Local signature verification returned false - proceeding anyway, server will validate"
                )
            }
        } catch (e: Exception) {
            Log.w("KanariClient", "Local signature verification threw (ignored): ${e.message}")
            // Do not throw - this was the source of `Unexpected CALL_ERROR` in UI.
        }

        // Finalize request with signature (RPC expects List<Int> 0-255)
        val finalPrepared = prepared.copy(
            signature = signature.map { it.toInt() and 0xFF },
            // Ensure we send the normalized IDs that were signed
            coinObjectId = normalizedCoinObjectId,
            recipient = normalizedRecipient,
            gasPayment = normalizedGasPayment,
            nonce = nonce
        )

        return submitObjectTransfer(finalPrepared)
    }

    suspend fun transferToken(
        wallet: KanariWallet,
        recipient: String,
        tokenType: String,
        amount: ULong
    ): TransactionResult? {
        val prepared = buildTokenTransfer(wallet.taggedAddress, recipient, tokenType, amount)
            ?: throw Exception("Failed to build token transfer: RPC returned no data")

        val nonce = prepared.nonce ?: throw Exception("Prepared token transfer missing nonce")
        if (nonce == 0uL) throw Exception("Prepared token transfer nonce must be non-zero")

        // Normalize gas payment objectIds for signing consistency
        val normalizedGasPayment = prepared.gasPayment?.let { gp ->
            gp.copy(paymentObjects = gp.paymentObjects.map { it.copy(objectId = normalizeAddress(it.objectId)) })
        }

        val normalizedObjectInputs = prepared.objectInputs?.map { input ->
            input.copy(objectRef = input.objectRef.copy(objectId = normalizeAddress(input.objectRef.objectId)))
        } ?: emptyList()

        // Convert args from List<List<Int>> to List<ByteArray>
        val argsBytes = prepared.args.map { list -> list.map { (it and 0xFF).toByte() }.toByteArray() }

        // Module is "package::module" e.g. "0xabc::usdt"
        val fullModule = "${prepared.packageAddr}::${prepared.module}"

        val txBytes = bcsEncodeExecuteFunction(
            sender = prepared.sender,
            module = fullModule,
            function = prepared.function,
            typeArgs = prepared.typeArgs,
            args = argsBytes,
            objectInputs = normalizedObjectInputs,
            gasPayment = normalizedGasPayment,
            gasLimit = prepared.gasLimit,
            gasPrice = prepared.gasPrice,
            nonce = nonce
        )

        val hash = KanariCrypto.blake3Hash(txBytes)
        val signature = wallet.sign(hash)

        try {
            val ok = KanariCrypto.verifySignature(wallet.taggedAddress, hash, signature, wallet.curveType)
            Log.d("KanariClient", "Token transfer local verify: $ok")
        } catch (e: Exception) {
            Log.w("KanariClient", "Token local verify ignored: ${e.message}")
        }

        val finalPrepared = prepared.copy(
            // Normalize object inputs/gas payment that were signed
            objectInputs = if (normalizedObjectInputs.isEmpty()) null else normalizedObjectInputs,
            gasPayment = normalizedGasPayment,
            nonce = nonce,
            signature = signature.map { it.toInt() and 0xFF }
        )
        return submitCallFunction(finalPrepared)
    }

    // ==================== BCS Manual Encoding (matches Rust bcs::to_bytes) ====================

    private fun bcsEncodeNativeTransfer(
        sender: String,
        coinObjectId: String,
        coinRef: ObjectRef,
        recipient: String,
        amount: ULong,
        gasPayment: GasPayment,
        gasLimit: ULong,
        gasPrice: ULong,
        nonce: ULong
    ): ByteArray {
        val out = ByteArrayOutputStream()

        // Transaction enum tag: ExecuteFunction = 4
        writeUleb128(out, 4)

        // sender: string
        writeString(out, sender)
        // module: string "0x2::kanari"
        writeString(out, "0x2::kanari")
        // function: string "transfer"
        writeString(out, "transfer")
        // type_args: vector<string> empty
        writeUleb128(out, 0)

        // args: vector<vector<u8>> with 3 elements
        writeUleb128(out, 3)
        // args[0]: coin object id as 32-byte address (raw bytes)
        writeVectorU8(out, hexToBytes(coinObjectId))
        // args[1]: amount as u64 little-endian 8 bytes -> then as vector<u8>
        writeVectorU8(out, u64ToBytes(amount))
        // args[2]: recipient address 32 bytes
        writeVectorU8(out, hexToBytes(recipient))

        // object_inputs: vector<ObjectInput> size 1
        writeUleb128(out, 1)
        writeObjectInput(out, coinRef, sender)

        // gas_payment: option<GasPayment> Some(1 + struct)
        out.write(1)
        writeGasPayment(out, gasPayment)

        // gas_limit, gas_price, nonce as u64 LE
        writeU64(out, gasLimit)
        writeU64(out, gasPrice)
        writeU64(out, nonce)

        return out.toByteArray()
    }

    private fun bcsEncodeExecuteFunction(
        sender: String,
        module: String,
        function: String,
        typeArgs: List<String>,
        args: List<ByteArray>,
        objectInputs: List<ObjectInput>,
        gasPayment: GasPayment?,
        gasLimit: ULong,
        gasPrice: ULong,
        nonce: ULong
    ): ByteArray {
        val out = ByteArrayOutputStream()
        writeUleb128(out, 4) // ExecuteFunction variant
        writeString(out, sender)
        writeString(out, module)
        writeString(out, function)
        // type_args
        writeUleb128(out, typeArgs.size)
        for (ta in typeArgs) writeString(out, ta)
        // args
        writeUleb128(out, args.size)
        for (a in args) writeVectorU8(out, a)
        // object_inputs
        writeUleb128(out, objectInputs.size)
        for (inp in objectInputs) {
            writeObjectRef(out, inp.objectRef)
            if (inp.owner != null) {
                out.write(1)
                when (val o = inp.owner) {
                    is ObjectOwnerKind.AddressOwner -> {
                        writeUleb128(out, 0)
                        writeString(out, o.address)
                    }
                    ObjectOwnerKind.Shared -> writeUleb128(out, 1)
                    ObjectOwnerKind.Immutable -> writeUleb128(out, 2)
                }
            } else {
                out.write(0)
            }
            out.write(if (inp.mutable) 1 else 0)
        }
        // gas_payment option
        if (gasPayment != null) {
            out.write(1)
            writeGasPayment(out, gasPayment)
        } else {
            out.write(0)
        }
        writeU64(out, gasLimit)
        writeU64(out, gasPrice)
        writeU64(out, nonce)
        return out.toByteArray()
    }

    fun normalizeTokenType(tokenType: String): String {
        val parts = tokenType.split("::")
        if (parts.size < 3) throw IllegalArgumentException("Invalid token type: $tokenType (expected address::module::struct)")
        var pkg = parts[0].trim()
        if (!pkg.startsWith("0x")) pkg = "0x$pkg"
        // Keep short addresses like 0x2 as-is, otherwise pad to 32 bytes for canonical form
        val hex = pkg.removePrefix("0x")
        val normalizedPkg = if (hex.length >= 64) {
            "0x${hex.lowercase().padStart(64, '0')}"
        } else {
            // short form is valid on-chain; keep lowercase without padding for RPC compatibility
            // but flutter's normalizeTokenType keeps short as-is
            "0x${hex.lowercase()}"
        }
        return "$normalizedPkg::${parts[1]}::${parts[2]}"
    }

    private fun writeObjectInput(out: ByteArrayOutputStream, coinRef: ObjectRef, sender: String) {
        writeObjectRef(out, coinRef)
        // owner: option<ObjectOwnerKind> Some(AddressOwner(sender)) -> 1 + (enum tag 0 + string)
        out.write(1)
        writeUleb128(out, 0) // AddressOwner
        writeString(out, sender)
        // mutable: bool true -> 1
        out.write(1)
    }

    private fun writeObjectRef(out: ByteArrayOutputStream, ref: ObjectRef) {
        writeString(out, ref.objectId)
        // version: option<u64>
        if (ref.version != null) {
            out.write(1)
            writeU64(out, ref.version)
        } else {
            out.write(0)
        }
        // digest: option<string>
        if (ref.digest != null) {
            out.write(1)
            writeString(out, ref.digest)
        } else {
            out.write(0)
        }
    }

    private fun writeGasPayment(out: ByteArrayOutputStream, gp: GasPayment) {
        // payment_objects: vector<ObjectRef>
        writeUleb128(out, gp.paymentObjects.size)
        for (obj in gp.paymentObjects) {
            writeObjectRef(out, obj)
        }
        writeString(out, gp.owner)
        writeU64(out, gp.budget)
        writeU64(out, gp.price)
    }

    private fun writeString(out: ByteArrayOutputStream, s: String) {
        val bytes = s.toByteArray(Charsets.UTF_8)
        writeUleb128(out, bytes.size)
        out.write(bytes)
    }

    private fun writeU64(out: ByteArrayOutputStream, v: ULong) {
        val buf = ByteBuffer.allocate(8).order(ByteOrder.LITTLE_ENDIAN).putLong(v.toLong())
        out.write(buf.array())
    }

    private fun writeVectorU8(out: ByteArrayOutputStream, bytes: ByteArray) {
        writeUleb128(out, bytes.size)
        out.write(bytes)
    }

    private fun writeUleb128(out: ByteArrayOutputStream, value: Int) {
        var v = value.toLong() and 0xFFFFFFFFL
        while (true) {
            var byte = (v and 0x7FL).toInt()
            v = v ushr 7
            if (v != 0L) {
                byte = byte or 0x80
                out.write(byte)
            } else {
                out.write(byte)
                break
            }
        }
    }

    private fun writeUleb128(out: ByteArrayOutputStream, value: ULong) {
        var v = value
        while (true) {
            var byte = (v and 0x7FuL).toInt()
            v = v shr 7
            if (v != 0uL) {
                byte = byte or 0x80
                out.write(byte)
            } else {
                out.write(byte)
                break
            }
        }
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
        val buffer = ByteBuffer.allocate(8).order(ByteOrder.LITTLE_ENDIAN)
        buffer.putLong(value.toLong())
        return buffer.array()
    }

    private fun normalizeAddress(address: String): String {
        // Do not normalize tagged addresses (contain ':') - they are Curve:hex strings.
        if (address.contains(":")) return address
        val clean = address.removePrefix("0x").lowercase()
        // If already hex with odd length, pad; then pad to 64 chars (32 bytes) for canonical addresses.
        // Short addresses like 0x2 stay short in some contexts but for transaction args we need 32-byte form.
        return "0x${clean.padStart(64, '0')}"
    }
}
