// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

package com.jamesatomc.kanariapp.network

import android.util.Log
import com.jamesatomc.kanariapp.network.models.CallFunctionData
import com.jamesatomc.kanariapp.network.models.RpcRequest
import com.jamesatomc.kanariapp.network.models.TransactionResult
import com.jamesatomc.kanariapp.wallet.KanariWallet
import com.jamesatomc.kanariapp.wallet.zklogin.ZkLoginAuth
import com.jamesatomc.kanariapp.wallet.zklogin.ZkLoginTxSigner
import com.kanari.kanari_crypto.KanariCrypto
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

// Standard module constants - update per module.
internal const val TEMPLATE_PACKAGE = "0x0"
internal const val TEMPLATE_MODULE = "template"

suspend fun KanariClient.buildTemplateAction(
    sender: String,
    gasLimit: Long = 100000,
    gasPrice: Long = 1
): CallFunctionData {
    val request = RpcRequest(
        method = "kanari_buildCallFunction",
        params = buildJsonObject {
            put("sender", normalizeAddress(sender))
            put("package", normalizeAddress(TEMPLATE_PACKAGE))
            put("module", TEMPLATE_MODULE)
            put("function", "do_action")
            put("type_args", buildJsonArray { })
            put("args", buildJsonArray { })
            put("gas_limit", gasLimit)
            put("gas_price", gasPrice)
            put("execute_immediate", true)
        }
    )
    val response = rpcService.buildCallFunction(request)
    if (response.error != null) throw Exception(response.error.message)
    return response.result ?: throw Exception("Empty build response for template action")
}

suspend fun KanariClient.submitTemplateTx(
    wallet: KanariWallet,
    prepared: CallFunctionData
): TransactionResult? {
    val p = prepareCallTx(prepared)
    val signature = wallet.sign(p.hash)
    try {
        KanariCrypto.verifySignature(wallet.taggedAddress, p.hash, signature, wallet.curveType)
    } catch (e: Exception) {
        Log.w("KanariClient", "Template local verify ignored: ${e.message}")
    }
    return submitCallFunction(p.normalized.copy(signature = signature.map { it.toInt() and 0xFF }))
}

suspend fun KanariClient.submitTemplateTxZkLogin(
    session: ZkLoginAuth.Session,
    prepared: CallFunctionData
): TransactionResult? {
    val p = prepareCallTx(prepared.copy(sender = "ZkLogin:${session.address}"))
    val signature = ZkLoginTxSigner.sign(session, p.hash, System.currentTimeMillis() / 1000)
    return submitCallFunction(p.normalized.copy(signature = signature.map { it.toInt() and 0xFF }))
}
