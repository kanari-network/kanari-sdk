// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

package com.jamesatomc.kanariapp.network

import android.util.Log
import com.jamesatomc.kanariapp.network.models.CallFunctionData
import com.jamesatomc.kanariapp.network.models.DEEPBOOK_MODULE
import com.jamesatomc.kanariapp.network.models.DEEPBOOK_PACKAGE_ID
import com.jamesatomc.kanariapp.network.models.RpcRequest
import com.jamesatomc.kanariapp.network.models.TransactionResult
import com.jamesatomc.kanariapp.wallet.KanariWallet
import com.jamesatomc.kanariapp.wallet.zklogin.ZkLoginAuth
import com.jamesatomc.kanariapp.wallet.zklogin.ZkLoginTxSigner
import com.kanari.kanari_crypto.KanariCrypto
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.add
import kotlinx.serialization.json.addAll
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.longOrNull
import kotlinx.serialization.json.put
import java.nio.ByteBuffer
import java.nio.ByteOrder

// ==================== DeepBook ====================
//
// All functions are extensions on [KanariClient] so they reuse its RPC
// service, address normalization, transaction prep and submission flow —
// the same path token transfers and NFT transfers go through.

/** BCS-encode a u64 as 8 little-endian bytes. */
private fun bcsEncodeU64(value: Long): List<Int> {
    val buf = ByteBuffer.allocate(8).order(ByteOrder.LITTLE_ENDIAN).putLong(value)
    return buf.array().map { it.toInt() and 0xFF }
}

/** BCS-encode a u8 as a single byte. */
private fun bcsEncodeU8(value: Int): List<Int> = listOf(value and 0xFF)

/**
 * Build a `place_bid<Base, Quote>` call for the deepbook module.
 * Returns normalized [CallFunctionData] ready for signing via
 * [submitDeepbookTx] + [submitCallFunction].
 *
 * No `object_inputs` block is sent: the node infers owner, version, digest
 * and mutability for every 32-byte arg at an object-parameter position from
 * the module bytecode (`infer_object_inputs`).
 */
suspend fun KanariClient.buildDeepbookBid(
    sender: String,
    bookId: String,
    price: ULong,
    quantity: ULong,
    paymentCoinId: String,
    orderType: Int,
    baseType: String,
    quoteType: String,
    gasLimit: ULong = 100000uL,
    gasPrice: ULong = DEFAULT_GAS_PRICE
): CallFunctionData {
    val normalizedSender = normalizeAddress(sender)
    val normalizedBookId = normalizeAddress(bookId)
    val normalizedPaymentCoin = normalizeAddress(paymentCoinId)
    val normalizedBase = normalizeTokenType(baseType)
    val normalizedQuote = normalizeTokenType(quoteType)

    val request = RpcRequest(
        method = "kanari_buildCallFunction",
        params = buildJsonObject {
            put("sender", normalizedSender)
            put("package", DEEPBOOK_PACKAGE_ID)
            put("module", DEEPBOOK_MODULE)
            put("function", "place_bid")
            put("type_args", buildJsonArray {
                add(normalizedBase)
                add(normalizedQuote)
            })
            put("args", buildJsonArray {
                add(buildJsonArray { addAll(hexToBytes(normalizedBookId).map { it.toInt() and 0xFF }) })
                add(buildJsonArray { addAll(bcsEncodeU64(price.toLong())) })
                add(buildJsonArray { addAll(bcsEncodeU64(quantity.toLong())) })
                add(buildJsonArray { addAll(hexToBytes(normalizedPaymentCoin).map { it.toInt() and 0xFF }) })
                add(buildJsonArray { addAll(bcsEncodeU8(orderType)) })
            })
            put("gas_limit", gasLimit.toLong())
            put("gas_price", gasPrice.toLong())
            put("execute_immediate", true)
        }
    )
    val response = rpcService.buildCallFunction(request)
    if (response.error != null) throw Exception(response.error.message)
    return response.result ?: throw Exception("Empty build response for place_bid")
}

/**
 * Build a `place_ask<Base, Quote>` call for the deepbook module.
 */
suspend fun KanariClient.buildDeepbookAsk(
    sender: String,
    bookId: String,
    price: ULong,
    quantity: ULong,
    baseCoinId: String,
    orderType: Int,
    baseType: String,
    quoteType: String,
    gasLimit: ULong = 100000uL,
    gasPrice: ULong = DEFAULT_GAS_PRICE
): CallFunctionData {
    val normalizedSender = normalizeAddress(sender)
    val normalizedBookId = normalizeAddress(bookId)
    val normalizedBaseCoin = normalizeAddress(baseCoinId)
    val normalizedBase = normalizeTokenType(baseType)
    val normalizedQuote = normalizeTokenType(quoteType)

    val request = RpcRequest(
        method = "kanari_buildCallFunction",
        params = buildJsonObject {
            put("sender", normalizedSender)
            put("package", DEEPBOOK_PACKAGE_ID)
            put("module", DEEPBOOK_MODULE)
            put("function", "place_ask")
            put("type_args", buildJsonArray {
                add(normalizedBase)
                add(normalizedQuote)
            })
            put("args", buildJsonArray {
                add(buildJsonArray { addAll(hexToBytes(normalizedBookId).map { it.toInt() and 0xFF }) })
                add(buildJsonArray { addAll(bcsEncodeU64(price.toLong())) })
                add(buildJsonArray { addAll(bcsEncodeU64(quantity.toLong())) })
                add(buildJsonArray { addAll(hexToBytes(normalizedBaseCoin).map { it.toInt() and 0xFF }) })
                add(buildJsonArray { addAll(bcsEncodeU8(orderType)) })
            })
            put("gas_limit", gasLimit.toLong())
            put("gas_price", gasPrice.toLong())
            put("execute_immediate", true)
        }
    )
    val response = rpcService.buildCallFunction(request)
    if (response.error != null) throw Exception(response.error.message)
    return response.result ?: throw Exception("Empty build response for place_ask")
}

/**
 * Build a `cancel_order<Base, Quote>` call for the deepbook module.
 */
suspend fun KanariClient.buildDeepbookCancel(
    sender: String,
    bookId: String,
    orderId: ULong,
    baseType: String,
    quoteType: String,
    gasLimit: ULong = 100000uL,
    gasPrice: ULong = DEFAULT_GAS_PRICE
): CallFunctionData {
    val normalizedSender = normalizeAddress(sender)
    val normalizedBookId = normalizeAddress(bookId)
    val normalizedBase = normalizeTokenType(baseType)
    val normalizedQuote = normalizeTokenType(quoteType)

    val request = RpcRequest(
        method = "kanari_buildCallFunction",
        params = buildJsonObject {
            put("sender", normalizedSender)
            put("package", DEEPBOOK_PACKAGE_ID)
            put("module", DEEPBOOK_MODULE)
            put("function", "cancel_order")
            put("type_args", buildJsonArray {
                add(normalizedBase)
                add(normalizedQuote)
            })
            put("args", buildJsonArray {
                add(buildJsonArray { addAll(hexToBytes(normalizedBookId).map { it.toInt() and 0xFF }) })
                add(buildJsonArray { addAll(bcsEncodeU64(orderId.toLong())) })
            })
            put("gas_limit", gasLimit.toLong())
            put("gas_price", gasPrice.toLong())
            put("execute_immediate", true)
        }
    )
    val response = rpcService.buildCallFunction(request)
    if (response.error != null) throw Exception(response.error.message)
    return response.result ?: throw Exception("Empty build response for cancel_order")
}

/**
 * Sign + submit a deepbook call function transaction. Mirrors the token
 * transfer flow: normalize, sign the tx hash with the wallet's curve,
 * submit.
 */
suspend fun KanariClient.submitDeepbookTx(
    wallet: KanariWallet,
    prepared: CallFunctionData
): TransactionResult? {
    val p = prepareCallTx(prepared)

    val signature = wallet.sign(p.hash)

    try {
        KanariCrypto.verifySignature(wallet.taggedAddress, p.hash, signature, wallet.curveType)
    } catch (e: Exception) {
        Log.w("KanariClient", "Deepbook local verify ignored: ${e.message}")
    }

    return submitCallFunction(p.normalized.copy(signature = signature.map { it.toInt() and 0xFF }))
}

/**
 * zkLogin flavor of [submitDeepbookTx]: signs with the on-device session.
 */
suspend fun KanariClient.submitDeepbookTxZkLogin(
    session: ZkLoginAuth.Session,
    prepared: CallFunctionData
): TransactionResult? {
    val p = prepareCallTx(prepared.copy(sender = "ZkLogin:${session.address}"))

    val signature = ZkLoginTxSigner.sign(session, p.hash, System.currentTimeMillis() / 1000)

    return submitCallFunction(
        p.normalized.copy(signature = signature.map { it.toInt() and 0xFF })
    )
}

/**
 * Read-only view call. Object ids in [args] may be plain hex strings — the
 * server decodes them and resolves them to objects at object-parameter
 * positions from the module bytecode.
 *
 * Unlike every other RPC method (which takes the labeled param object
 * directly), `kanari_viewFunction` requires `params` as a one-element
 * array — and wraps the value as `{"status","action":"view","result":…}`.
 * Both are normalized here so callers receive the bare Move result.
 */
internal suspend fun KanariClient.callViewFunction(
    packageName: String,
    moduleName: String,
    functionName: String,
    typeArgs: List<String> = emptyList(),
    args: List<JsonElement> = emptyList()
): JsonElement? {
    val request = RpcRequest(
        method = "kanari_viewFunction",
        params = buildJsonArray {
            add(buildJsonObject {
                put("package", packageName)
                put("module", moduleName)
                put("function", functionName)
                put("type_args", buildJsonArray { typeArgs.forEach { add(normalizeTokenType(it)) } })
                put("args", buildJsonArray { args.forEach { add(it) } })
            })
        }
    )
    val response = rpcService.call(request)
    if (response.error != null) throw Exception(response.error.message)
    val result = response.result ?: return null
    // Unwrap the view envelope: {"status":"success","action":"view","result":…}.
    if (result is JsonObject) {
        val action = result["action"]
        if (action is JsonPrimitive && action.content == "view") return result["result"]
    }
    return result
}

/** u64 view results arrive as a JSON number; tolerate string forms too. */
internal fun JsonElement?.toViewULong(): ULong? = when (this) {
    is JsonPrimitive -> longOrNull?.toULong() ?: content.trim().removePrefix("0x").toULongOrNull()
    is JsonArray -> firstOrNull()?.toViewULong()
    else -> null
}

/**
 * Decode an `Option<address>` / `address` view result. The fast result
 * serializer emits BCS bytes for `Option`: `Some(addr)` is 33 bytes → a JSON
 * array `[1, b0..b31]`, `None` is a single `0` byte → JSON number `0`
 * (never an address — numbers are rejected below). A plain `address` comes
 * back as a `"0x…"` hex string.
 */
internal fun JsonElement?.toViewAddress(): String? = when (this) {
    is JsonPrimitive -> {
        // Only string primitives can be addresses; a bare number is a BCS
        // `None` (or another primitive), not an address.
        if (isString) {
            val hex = content.removePrefix("0x").removePrefix("0X")
            if (hex.length in 1..64 && hex.all { it.isDigit() || it in "abcdefABCDEF" }) {
                "0x${hex.lowercase().padStart(64, '0')}"
            } else null
        } else null
    }

    is JsonArray -> {
        if (size == 33 && firstOrNull()?.let { it is JsonPrimitive && it.longOrNull == 1L } == true) {
            val hex = drop(1).joinToString("") { byte ->
                (byte as? JsonPrimitive)?.intOrNull
                    ?.let { it.toString(16).padStart(2, '0') }
                    ?: error("Bad address byte in view result")
            }
            "0x$hex"
        } else null
    }

    else -> null
}
