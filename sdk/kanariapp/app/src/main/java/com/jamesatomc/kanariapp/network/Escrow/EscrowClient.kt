// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

package com.jamesatomc.kanariapp.network

import android.util.Log
import com.jamesatomc.kanariapp.network.models.CallFunctionData
import com.jamesatomc.kanariapp.network.models.ObjectOwnerKind
import com.jamesatomc.kanariapp.network.models.RpcRequest
import com.jamesatomc.kanariapp.network.models.TransactionResult
import com.jamesatomc.kanariapp.wallet.KanariWallet
import com.jamesatomc.kanariapp.wallet.zklogin.ZkLoginAuth
import com.jamesatomc.kanariapp.wallet.zklogin.ZkLoginTxSigner
import com.kanari.kanari_crypto.KanariCrypto
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.add
import kotlinx.serialization.json.addAll
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import java.nio.ByteBuffer
import java.nio.ByteOrder

internal const val ESCROW_PACKAGE =
    "0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146"
internal const val ESCROW_MODULE = "escrow"

private fun bcsEncodeU64(value: Long): List<Int> {
    val buf = ByteBuffer.allocate(8).order(ByteOrder.LITTLE_ENDIAN).putLong(value)
    return buf.array().map { it.toInt() and 0xFF }
}

private fun bcsEncodeString(value: String): List<Int> {
    val bytes = value.toByteArray(Charsets.UTF_8)
    val out = mutableListOf<Int>()
    var len = bytes.size
    while (true) {
        var b = (len and 0x7F)
        len = len shr 7
        if (len != 0) b = b or 0x80
        out.add(b)
        if (len == 0) break
    }
    bytes.forEach { out.add(it.toInt() and 0xFF) }
    return out
}

/**
 * Object input for a mutating action. The declared owner must be the object's
 * *actual* owner (not the tx sender) so cross-owner flows (e.g. a seller
 * acting on a buyer-owned deal) resolve correctly - kanari_pay parity.
 */
private suspend fun KanariClient.objectInputForAction(objectId: String): kotlinx.serialization.json.JsonObject {
    val obj = getObject(objectId)
    return buildJsonObject {
        put("object_ref", buildJsonObject {
            put("object_id", JsonPrimitive(normalizeAddress(obj.id)))
            put("version", JsonPrimitive(obj.version))
            if (!obj.digest.isNullOrBlank()) put("digest", JsonPrimitive(obj.digest))
        })
        when (val owner = obj.ownerKind) {
            ObjectOwnerKind.Shared -> put("owner", JsonPrimitive("Shared"))
            ObjectOwnerKind.Immutable -> put("owner", JsonPrimitive("Immutable"))
            is ObjectOwnerKind.AddressOwner -> put("owner", buildJsonObject {
                put("AddressOwner", JsonPrimitive(normalizeAddress(owner.address)))
            })

                        null -> put("owner", buildJsonObject {
                put("AddressOwner", JsonPrimitive(normalizeAddress(obj.owner)))
            })
        }
        put("mutable", JsonPrimitive(true))
    }
}

suspend fun KanariClient.buildEscrowCreateDeal(
    sender: String,
    packageId: String = ESCROW_PACKAGE,
    dealId: String,
    sellerAddress: String,
    amount: Long,
    description: String,
    coinObjectId: String,
    tokenType: String,
    gasLimit: Long = 100000,
    gasPrice: Long = 1
): CallFunctionData {
    val request = RpcRequest(
        method = "kanari_buildCallFunction",
        params = buildJsonObject {
            put("sender", normalizeAddress(sender))
            put("package", normalizeAddress(packageId))
            put("module", ESCROW_MODULE)
            // Use the reference-based entry point and declare the coin as an
            // explicit mutable input. The legacy address-based wrapper can
            // execute successfully while its coin writeback is only a raw
            // address dependency; the ref path makes the lock mutation part
            // of the transaction's object effects (kanari_pay parity).
            put("function", "create_deal_ref")
            put("type_args", buildJsonArray { add(normalizeTokenType(tokenType)) })
            put("args", buildJsonArray {
                add(buildJsonArray { addAll(bcsEncodeString(dealId)) })
                add(buildJsonArray { addAll(hexToBytes(normalizeAddress(sellerAddress)).map { it.toInt() and 0xFF }) })
                add(buildJsonArray { addAll(bcsEncodeU64(amount)) })
                add(buildJsonArray { addAll(bcsEncodeString(description)) })
                add(buildJsonArray { addAll(hexToBytes(normalizeAddress(coinObjectId)).map { it.toInt() and 0xFF }) })
            })
            put("object_inputs", buildJsonArray {
                add(objectInputForAction(coinObjectId))
            })
            put("gas_limit", gasLimit)
            put("gas_price", gasPrice)
            put("execute_immediate", true)
        }
    )
    val response = rpcService.buildCallFunction(request)
    if (response.error != null) throw Exception(response.error.message)
    return response.result ?: throw Exception("Empty build response for create_deal")
}

suspend fun KanariClient.buildEscrowConfirmDelivery(
    sender: String,
    packageId: String = ESCROW_PACKAGE,
    dealObjectId: String,
    proofObjectId: String,
    coinType: String,
    gasLimit: Long = 100000,
    gasPrice: Long = 1
): CallFunctionData {
    val request = RpcRequest(
        method = "kanari_buildCallFunction",
        params = buildJsonObject {
            put("sender", normalizeAddress(sender))
            put("package", normalizeAddress(packageId))
            put("module", ESCROW_MODULE)
            put("function", "confirm_delivery")
            put("type_args", buildJsonArray { add(normalizeTokenType(coinType)) })
            put("args", buildJsonArray {
                add(buildJsonArray { addAll(hexToBytes(normalizeAddress(dealObjectId)).map { it.toInt() and 0xFF }) })
                add(buildJsonArray { addAll(hexToBytes(normalizeAddress(proofObjectId)).map { it.toInt() and 0xFF }) })
            })
            put("object_inputs", buildJsonArray {
                add(objectInputForAction(dealObjectId))
                add(objectInputForAction(proofObjectId))
            })
            put("gas_limit", gasLimit)
            put("gas_price", gasPrice)
            put("execute_immediate", true)
        }
    )
    val response = rpcService.buildCallFunction(request)
    if (response.error != null) throw Exception(response.error.message)
    return response.result ?: throw Exception("Empty build response for confirm_delivery")
}

suspend fun KanariClient.buildEscrowReleaseFunds(
    sender: String,
    packageId: String = ESCROW_PACKAGE,
    dealObjectId: String,
    proofObjectId: String,
    coinType: String,
    gasLimit: Long = 100000,
    gasPrice: Long = 1
): CallFunctionData {
    val request = RpcRequest(
        method = "kanari_buildCallFunction",
        params = buildJsonObject {
            put("sender", normalizeAddress(sender))
            put("package", normalizeAddress(packageId))
            put("module", ESCROW_MODULE)
            put("function", "release_funds")
            put("type_args", buildJsonArray { add(normalizeTokenType(coinType)) })
            put("args", buildJsonArray {
                add(buildJsonArray { addAll(hexToBytes(normalizeAddress(dealObjectId)).map { it.toInt() and 0xFF }) })
                add(buildJsonArray { addAll(hexToBytes(normalizeAddress(proofObjectId)).map { it.toInt() and 0xFF }) })
            })
            put("object_inputs", buildJsonArray {
                add(objectInputForAction(dealObjectId))
                add(objectInputForAction(proofObjectId))
            })
            put("gas_limit", gasLimit)
            put("gas_price", gasPrice)
            put("execute_immediate", true)
        }
    )
    val response = rpcService.buildCallFunction(request)
    if (response.error != null) throw Exception(response.error.message)
    return response.result ?: throw Exception("Empty build response for release_funds")
}

suspend fun KanariClient.buildEscrowRaiseDispute(
    sender: String,
    packageId: String = ESCROW_PACKAGE,
    dealObjectId: String,
    proofObjectId: String,
    reason: String,
    coinType: String,
    gasLimit: Long = 100000,
    gasPrice: Long = 1
): CallFunctionData {
    val request = RpcRequest(
        method = "kanari_buildCallFunction",
        params = buildJsonObject {
            put("sender", normalizeAddress(sender))
            put("package", normalizeAddress(packageId))
            put("module", ESCROW_MODULE)
            put("function", "raise_dispute")
            put("type_args", buildJsonArray { add(normalizeTokenType(coinType)) })
            put("args", buildJsonArray {
                add(buildJsonArray { addAll(hexToBytes(normalizeAddress(dealObjectId)).map { it.toInt() and 0xFF }) })
                add(buildJsonArray { addAll(hexToBytes(normalizeAddress(proofObjectId)).map { it.toInt() and 0xFF }) })
                add(buildJsonArray { addAll(bcsEncodeString(reason)) })
            })
            put("object_inputs", buildJsonArray {
                add(objectInputForAction(dealObjectId))
                add(objectInputForAction(proofObjectId))
            })
            put("gas_limit", gasLimit)
            put("gas_price", gasPrice)
            put("execute_immediate", true)
        }
    )
    val response = rpcService.buildCallFunction(request)
    if (response.error != null) throw Exception(response.error.message)
    return response.result ?: throw Exception("Empty build response for raise_dispute")
}

suspend fun KanariClient.submitEscrowTx(
    wallet: KanariWallet,
    prepared: CallFunctionData
): TransactionResult? {
    val p = prepareCallTx(prepared)
    val signature = wallet.sign(p.hash)
    try {
        KanariCrypto.verifySignature(wallet.taggedAddress, p.hash, signature, wallet.curveType)
    } catch (e: Exception) {
        Log.w("KanariClient", "Escrow local verify ignored: ${e.message}")
    }
    return submitCallFunction(p.normalized.copy(signature = signature.map { it.toInt() and 0xFF }))
}

suspend fun KanariClient.submitEscrowTxZkLogin(
    session: ZkLoginAuth.Session,
    prepared: CallFunctionData
): TransactionResult? {
    val p = prepareCallTx(prepared.copy(sender = "ZkLogin:${session.address}"))
    val signature = ZkLoginTxSigner.sign(session, p.hash, System.currentTimeMillis() / 1000)
    return submitCallFunction(p.normalized.copy(signature = signature.map { it.toInt() and 0xFF }))
}
