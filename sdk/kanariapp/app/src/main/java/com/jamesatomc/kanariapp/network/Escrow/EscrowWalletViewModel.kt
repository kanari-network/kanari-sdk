// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

package com.jamesatomc.kanariapp.wallet

import com.jamesatomc.kanariapp.network.models.ObjectInfo
import com.jamesatomc.kanariapp.network.models.RpcRequest

import android.app.Application
import com.jamesatomc.kanariapp.network.KanariClient
import com.jamesatomc.kanariapp.network.buildEscrowCreateDeal
import com.jamesatomc.kanariapp.network.callViewFunction
import com.jamesatomc.kanariapp.network.toViewULong
import com.jamesatomc.kanariapp.network.buildEscrowConfirmDelivery
import com.jamesatomc.kanariapp.network.buildEscrowReleaseFunds
import com.jamesatomc.kanariapp.network.buildEscrowRaiseDispute
import com.jamesatomc.kanariapp.network.submitEscrowTx
import com.jamesatomc.kanariapp.network.submitEscrowTxZkLogin
import com.jamesatomc.kanariapp.network.models.TransactionResult
import com.jamesatomc.kanariapp.network.models.ObjectOwnerKind
import com.jamesatomc.kanariapp.wallet.zklogin.ZkLoginAuth
import com.jamesatomc.kanariapp.wallet.zklogin.ZkLoginTxSigner
import com.jamesatomc.kanariapp.network.ESCROW_MODULE
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.coroutines.withContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.Dispatchers
import java.util.Locale

private suspend fun WalletViewModel.unlockedEscrowWallet(record: com.jamesatomc.kanariapp.wallet.WalletRecord): KanariWallet? {
    val pin = unlockedPin ?: run {
        _error.value = "Wallet is locked. Please unlock again."
        return null
    }
    val privateKeyEncrypted = record.privateKeyEncrypted ?: run {
        _error.value = "Wallet missing private key"
        return null
    }
    return try {
        KanariWallet.fromPrivateKey(walletStorage.decrypt(privateKeyEncrypted, pin), record.curveType)
    } catch (e: Exception) {
        _error.value = "Failed to unlock wallet: ${e.message}"
        null
    }
}

private fun WalletViewModel.settleEscrowResult(result: TransactionResult?): Boolean {
    val status = result?.status?.lowercase(Locale.US)
    val isSuccess = result?.success == true ||
            status == "success" || status == "executed" ||
            status == "committed" || status == "pending" ||
            status == "simulated_pending"
    return if (isSuccess) {
        _error.value = null
        refreshBalanceAfterSend()
        true
    } else {
        val msg = result?.errorMessage ?: "Transaction failed (Status: ${result?.status})"
        _error.value = mapTransferError(msg)
        false
    }
}

suspend fun WalletViewModel.createEscrowDeal(
    dealId: String,
    sellerAddress: String,
    amount: Long,
    description: String,
    tokenType: String,
    coinObjectId: String = ""
): Boolean {
    val record = activeWallet.value ?: run {
        _error.value = "No active wallet"
        return false
    }
    val effectiveCoin = if (coinObjectId.isNotBlank()) coinObjectId else {
        accountInfo.value?.ownedObjects.orEmpty()
            .firstOrNull { it.getEffectiveType().contains(tokenType) }?.id ?: run {
            _error.value = "No coin found for $tokenType"
            return false
        }
    }
    return try {
        val result = if (record.curveType == "ZkLogin") {
            val session = try {
                ZkLoginAuth.loadSession(getApplication<Application>(), record.address)
            } catch (_: Exception) {
                _error.value = "No Google session - sign in with Google first."
                return false
            }
            val prepared = client.buildEscrowCreateDeal(
                "ZkLogin:${session.address}",
                environment.value.escrow.packageId,
                dealId,
                sellerAddress,
                amount,
                description,
                effectiveCoin,
                tokenType
            )
            client.submitEscrowTxZkLogin(session, prepared)
        } else {
            val wallet = unlockedEscrowWallet(record) ?: return false
            val prepared = client.buildEscrowCreateDeal(
                wallet.taggedAddress,
                environment.value.escrow.packageId,
                dealId,
                sellerAddress,
                amount,
                description,
                effectiveCoin,
                tokenType
            )
            client.submitEscrowTx(wallet, prepared)
        }
        settleEscrowResult(result)
    } catch (_: ZkLoginTxSigner.SessionExpiredException) {
        _error.value = "Google session expired - sign in again."
        false
    } catch (_: ZkLoginTxSigner.SessionIncompleteException) {
        _error.value = "Google session file is from an old app version - sign in again."
        false
    } catch (e: Exception) {
        _error.value = "Escrow create failed: ${e.message}"
        false
    }
}

suspend fun WalletViewModel.confirmEscrowDelivery(
    dealObjectId: String,
    proofObjectId: String,
    coinType: String
): Boolean {
    val record = activeWallet.value ?: run {
        _error.value = "No active wallet"
        return false
    }
    return try {
        val result = if (record.curveType == "ZkLogin") {
            val session = try {
                ZkLoginAuth.loadSession(getApplication<Application>(), record.address)
            } catch (_: Exception) {
                _error.value = "No Google session - sign in with Google first."
                return false
            }
            val prepared = client.buildEscrowConfirmDelivery(
                "ZkLogin:${session.address}", environment.value.escrow.packageId, dealObjectId, proofObjectId, coinType
            )
            client.submitEscrowTxZkLogin(session, prepared)
        } else {
            val wallet = unlockedEscrowWallet(record) ?: return false
            val prepared = client.buildEscrowConfirmDelivery(
                wallet.taggedAddress, environment.value.escrow.packageId, dealObjectId, proofObjectId, coinType
            )
            client.submitEscrowTx(wallet, prepared)
        }
        settleEscrowResult(result)
    } catch (_: ZkLoginTxSigner.SessionExpiredException) {
        _error.value = "Google session expired - sign in again."
        false
    } catch (_: ZkLoginTxSigner.SessionIncompleteException) {
        _error.value = "Google session file is from an old app version - sign in again."
        false
    } catch (e: Exception) {
        _error.value = "Escrow confirm failed: ${e.message}"
        false
    }
}

suspend fun WalletViewModel.releaseEscrowFunds(
    dealObjectId: String,
    proofObjectId: String,
    coinType: String
): Boolean {
    val record = activeWallet.value ?: run {
        _error.value = "No active wallet"
        return false
    }
    return try {
        val result = if (record.curveType == "ZkLogin") {
            val session = try {
                ZkLoginAuth.loadSession(getApplication<Application>(), record.address)
            } catch (_: Exception) {
                _error.value = "No Google session - sign in with Google first."
                return false
            }
            val prepared = client.buildEscrowReleaseFunds(
                "ZkLogin:${session.address}", environment.value.escrow.packageId, dealObjectId, proofObjectId, coinType
            )
            client.submitEscrowTxZkLogin(session, prepared)
        } else {
            val wallet = unlockedEscrowWallet(record) ?: return false
            val prepared = client.buildEscrowReleaseFunds(
                wallet.taggedAddress, environment.value.escrow.packageId, dealObjectId, proofObjectId, coinType
            )
            client.submitEscrowTx(wallet, prepared)
        }
        settleEscrowResult(result)
    } catch (_: ZkLoginTxSigner.SessionExpiredException) {
        _error.value = "Google session expired - sign in again."
        false
    } catch (_: ZkLoginTxSigner.SessionIncompleteException) {
        _error.value = "Google session file is from an old app version - sign in again."
        false
    } catch (e: Exception) {
        _error.value = "Escrow release failed: ${e.message}"
        false
    }
}

suspend fun WalletViewModel.raiseEscrowDispute(
    dealObjectId: String,
    proofObjectId: String,
    reason: String,
    coinType: String
): Boolean {
    val record = activeWallet.value ?: run {
        _error.value = "No active wallet"
        return false
    }
    return try {
        val result = if (record.curveType == "ZkLogin") {
            val session = try {
                ZkLoginAuth.loadSession(getApplication<Application>(), record.address)
            } catch (_: Exception) {
                _error.value = "No Google session - sign in with Google first."
                return false
            }
            val prepared = client.buildEscrowRaiseDispute(
                "ZkLogin:${session.address}",
                environment.value.escrow.packageId,
                dealObjectId,
                proofObjectId,
                reason,
                coinType
            )
            client.submitEscrowTxZkLogin(session, prepared)
        } else {
            val wallet = unlockedEscrowWallet(record) ?: return false
            val prepared = client.buildEscrowRaiseDispute(
                wallet.taggedAddress, environment.value.escrow.packageId, dealObjectId, proofObjectId, reason, coinType
            )
            client.submitEscrowTx(wallet, prepared)
        }
        settleEscrowResult(result)
    } catch (_: ZkLoginTxSigner.SessionExpiredException) {
        _error.value = "Google session expired - sign in again."
        false
    } catch (_: ZkLoginTxSigner.SessionIncompleteException) {
        _error.value = "Google session file is from an old app version - sign in again."
        false
    } catch (e: Exception) {
        _error.value = "Escrow dispute failed: ${e.message}"
        false
    }
}

// ---------------------------------------------------------------------------
// View helpers (read-only)
// ---------------------------------------------------------------------------

suspend fun WalletViewModel.getEscrowDealState(
    dealObjectId: String,
    coinType: String
): Int = withContext(Dispatchers.IO) {
    try {
        // Reference-based view call (kanari_pay parity): the address-based
        // `get_state` aborts in this runtime, so resolve the object first
        // and call `get_state_ref` through object_inputs.
        val obj = client.getObject(dealObjectId)
        val effectiveCoin = extractCoinType(obj.getEffectiveType()) ?: coinType
        val objectInput = kotlinx.serialization.json.buildJsonObject {
            put("object_ref", kotlinx.serialization.json.buildJsonObject {
                put("object_id", kotlinx.serialization.json.JsonPrimitive(client.normalizeAddress(obj.id)))
                put("version", kotlinx.serialization.json.JsonPrimitive(obj.version))
                put("digest", kotlinx.serialization.json.JsonPrimitive(obj.digest ?: ""))
            })
            when (val owner = obj.ownerKind) {
                ObjectOwnerKind.Shared -> put("owner", kotlinx.serialization.json.JsonPrimitive("Shared"))
                ObjectOwnerKind.Immutable -> put("owner", kotlinx.serialization.json.JsonPrimitive("Immutable"))
                is ObjectOwnerKind.AddressOwner -> put("owner", kotlinx.serialization.json.buildJsonObject {
                    put(
                        "AddressOwner",
                        kotlinx.serialization.json.JsonPrimitive(client.normalizeAddress(owner.address))
                    )
                })

                null -> put("owner", kotlinx.serialization.json.buildJsonObject {
                    put("AddressOwner", kotlinx.serialization.json.JsonPrimitive(client.normalizeAddress(obj.owner)))
                })
            }
            put("mutable", kotlinx.serialization.json.JsonPrimitive(false))
        }
        client.callViewFunction(
            environment.value.escrow.packageId, ESCROW_MODULE, "get_state_ref",
            listOf(effectiveCoin),
            emptyList(),
            listOf(objectInput)
        )?.toViewULong()?.toInt() ?: -1
    } catch (_: Exception) {
        -1
    }
}

suspend fun WalletViewModel.createEscrowDealAndGetId(
    dealId: String,
    sellerAddress: String,
    amount: Long,
    description: String,
    tokenType: String,
    coinObjectId: String = ""
): String? {
    val record = activeWallet.value ?: run {
        _error.value = "No active wallet"
        return null
    }
    val effectiveCoin = if (coinObjectId.isNotBlank()) coinObjectId else {
        accountInfo.value?.ownedObjects.orEmpty()
            .firstOrNull { it.getEffectiveType().contains(tokenType) }?.id ?: run {
            _error.value = "No coin found for $tokenType"
            return null
        }
    }
    return try {
        val result = if (record.curveType == "ZkLogin") {
            val session = try {
                ZkLoginAuth.loadSession(getApplication<Application>(), record.address)
            } catch (_: Exception) {
                _error.value = "No Google session - sign in with Google first."
                return null
            }
            val prepared = client.buildEscrowCreateDeal(
                "ZkLogin:${session.address}",
                environment.value.escrow.packageId,
                dealId,
                sellerAddress,
                amount,
                description,
                effectiveCoin,
                tokenType
            )
            client.submitEscrowTxZkLogin(session, prepared)
        } else {
            val wallet = unlockedEscrowWallet(record) ?: return null
            val prepared = client.buildEscrowCreateDeal(
                wallet.taggedAddress,
                environment.value.escrow.packageId,
                dealId,
                sellerAddress,
                amount,
                description,
                effectiveCoin,
                tokenType
            )
            client.submitEscrowTx(wallet, prepared)
        }
        val status = result?.status?.lowercase(java.util.Locale.US)
        val isSuccess = result?.success == true ||
                status == "success" || status == "executed" ||
                status == "committed" || status == "pending" ||
                status == "simulated_pending"
        if (isSuccess) {
            _error.value = null
            refreshBalanceAfterSend()
            // Effects may not be populated on the first (pending) response;
            // retry fetching the transaction until they arrive (like kanari_pay).
            var effects = result?.effects
            if (effects?.objectChanges.isNullOrEmpty()) {
                for (i in 0 until 10) {
                    kotlinx.coroutines.delay(500)
                    try {
                        val txResp = client.rpcService.getTransaction(
                            com.jamesatomc.kanariapp.network.models.RpcRequest(
                                method = "kanari_getTransaction",
                                params = kotlinx.serialization.json.buildJsonObject {
                                    put("hash", kotlinx.serialization.json.JsonPrimitive(result?.hash ?: ""))
                                }
                            )
                        )
                        if (txResp.result?.effects != null) {
                            effects = txResp.result.effects
                            if (!effects?.objectChanges.isNullOrEmpty()) break
                        }
                    } catch (_: Exception) {
                    }
                }
            }
            val dealChange =
                effects?.objectChanges?.firstOrNull { ((it.type ?: it.typeAlternative) ?: "").contains("EscrowDeal") }
            dealChange?.objectRef?.objectId
        } else {
            val msg = result?.errorMessage ?: "Transaction failed (Status: ${result?.status})"
            _error.value = mapTransferError(msg)
            null
        }
    } catch (_: ZkLoginTxSigner.SessionExpiredException) {
        _error.value = "Google session expired - sign in again."
        null
    } catch (_: ZkLoginTxSigner.SessionIncompleteException) {
        _error.value = "Google session file is from an old app version - sign in again."
        null
    } catch (e: Exception) {
        _error.value = "Escrow create failed: ${e.message}"
        null
    }
}
// ---------------------------------------------------------------------------
// Auto-discovery of deals owned by a buyer (like kanari_pay getAllDeals)
// ---------------------------------------------------------------------------


private suspend fun WalletViewModel.ownedEscrowObjects(owner: String): List<ObjectInfo> {
    val byId = mutableMapOf<String, ObjectInfo>()
    fun addAll(list: List<ObjectInfo>) {
        for (obj in list) {
            if (obj.id.isNotEmpty()) byId[obj.id.lowercase()] = obj
        }
    }
    // kanari_getOwnedObjects
    try {
        val request = com.jamesatomc.kanariapp.network.models.RpcRequest(
            method = "kanari_getOwnedObjects",
            params = kotlinx.serialization.json.buildJsonObject {
                put("owner", kotlinx.serialization.json.JsonPrimitive(client.normalizeAddress(owner)))
            }
        )
        val response = client.rpcService.getOwnedObjects(request)
        addAll(response.result?.objects ?: emptyList())
    } catch (_: Exception) {
    }
    // kanari_getObjects
    try {
        val request = com.jamesatomc.kanariapp.network.models.RpcRequest(
            method = "kanari_getObjects",
            params = kotlinx.serialization.json.buildJsonObject {
                put("owner", kotlinx.serialization.json.JsonPrimitive(client.normalizeAddress(owner)))
            }
        )
        val response = client.rpcService.getObjects(request)
        addAll(response.result?.objects ?: emptyList())
    } catch (_: Exception) {
    }
    // Shared EscrowDeal/EscrowProof objects have no address owner. Include
    // the global object index; the type filter below keeps only Escrow data.
    try {
        val request = com.jamesatomc.kanariapp.network.models.RpcRequest(
            method = "kanari_getObjects",
            params = kotlinx.serialization.json.buildJsonObject { }
        )
        val response = client.rpcService.getObjects(request)
        addAll(response.result?.objects ?: emptyList())
    } catch (_: Exception) {
    }
    return byId.values.toList()
}

private fun isEscrowDealType(type: String?): Boolean {
    return type != null && (type.contains("::escrow::EscrowDeal<") || type.contains("EscrowDeal<"))
}

private fun isEscrowProofType(type: String?): Boolean {
    return type != null && (type.contains("::escrow::EscrowProof") || type.contains("EscrowProof"))
}

private fun extractCoinType(objectType: String?): String? {
    if (objectType == null) return null
    val start = objectType.indexOf('<')
    val end = objectType.lastIndexOf('>')
    if (start == -1 || end == -1 || end <= start) return null
    val ct = objectType.substring(start + 1, end).trim()
    return if (ct.isEmpty()) null else ct
}

suspend fun WalletViewModel.getAllEscrowDeals(buyerAddress: String): List<Map<String, Any>> {
    val objects = ownedEscrowObjects(buyerAddress)
    val dealObjects = objects.filter { isEscrowDealType(it.getEffectiveType()) }
    val proofObjects = objects.filter { isEscrowProofType(it.getEffectiveType()) }
    val result = mutableListOf<Map<String, Any>>()
    for (obj in dealObjects) {
        val coinType = extractCoinType(obj.getEffectiveType()) ?: continue
        val state = try {
            getEscrowDealState(obj.id, coinType)
        } catch (_: Exception) {
            -1
        }
        // Match the proof by the deal label stored inside the object data (kanari_pay parity).
        val dealLabel = decodeObjectDealLabel(obj)
        val proofId = (proofObjects.firstOrNull { decodeObjectDealLabel(it) == dealLabel && dealLabel != null }
            ?: proofObjects.firstOrNull())?.id ?: ""
        result.add(
            mapOf(
                "object_id" to obj.id,
                "coin_type" to coinType,
                "proof_id" to proofId,
                "state" to state
            )
        )
    }
    return result
}

private fun decodeObjectDealLabel(obj: com.jamesatomc.kanariapp.network.models.ObjectInfo): String? {
    val data = obj.data
    if (data.size < 32) return null
    val bytes = data
    var offset = 32
    // uleb128 length prefix then the string
    var len = 0
    var shift = 0
    var ok = false
    while (offset < bytes.size) {
        val b = bytes[offset++]
        len = len or ((b and 0x7f) shl shift)
        if ((b and 0x80) == 0) {
            ok = true; break
        }
        shift += 7
        if (shift > 28) return null
    }
    if (!ok || len < 0 || offset + len > bytes.size) return null
    val lit = IntArray(len) { i -> bytes[offset + i] }
    return lit.map { it.toByte() }.toByteArray().toString(Charsets.UTF_8)
}
