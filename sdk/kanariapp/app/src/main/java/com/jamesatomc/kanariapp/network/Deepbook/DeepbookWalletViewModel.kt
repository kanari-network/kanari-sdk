// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

package com.jamesatomc.kanariapp.wallet

import android.app.Application
import com.jamesatomc.kanariapp.network.KanariClient
import com.jamesatomc.kanariapp.network.buildDeepbookAsk
import com.jamesatomc.kanariapp.network.buildDeepbookBid
import com.jamesatomc.kanariapp.network.buildDeepbookCancel
import com.jamesatomc.kanariapp.network.callViewFunction
import com.jamesatomc.kanariapp.network.extractSymbol
import com.jamesatomc.kanariapp.network.isCoinOfType
import com.jamesatomc.kanariapp.network.models.DEEPBOOK_MODULE
import com.jamesatomc.kanariapp.network.models.DEEPBOOK_PACKAGE_ID
import com.jamesatomc.kanariapp.network.models.DEEPBOOK_REGISTRY_ID
import com.jamesatomc.kanariapp.network.models.DEEPBOOK_REGISTRY_MODULE
import com.jamesatomc.kanariapp.network.models.DeepbookBookInfo
import com.jamesatomc.kanariapp.network.models.DeepbookOrderLevel
import com.jamesatomc.kanariapp.network.models.TOKEN_KANARI
import com.jamesatomc.kanariapp.network.models.TOKEN_USD
import com.jamesatomc.kanariapp.network.submitDeepbookTx
import com.jamesatomc.kanariapp.network.submitDeepbookTxZkLogin
import com.jamesatomc.kanariapp.network.toViewAddress
import com.jamesatomc.kanariapp.network.toViewULong
import com.jamesatomc.kanariapp.wallet.zklogin.ZkLoginAuth
import com.jamesatomc.kanariapp.wallet.zklogin.ZkLoginTxSigner
import kotlinx.serialization.json.JsonPrimitive
import java.util.Locale

/**
 * Decrypt the active wallet's private key with the unlocked PIN and build a
 * signing wallet. Returns `null` (with `_error` set) when the wallet is
 * locked or the key cannot be decrypted.
 */
private suspend fun WalletViewModel.unlockedWallet(record: WalletRecord): KanariWallet? {
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

/**
 * Resolve the payment/base coin for an order from the wallet's owned
 * objects. An explicitly provided coin id wins; otherwise the first coin
 * whose `Coin<T>` type parameter matches [typeTag] is used.
 *
 * There is deliberately no "first object" fallback: handing a coin of the
 * wrong type to `place_bid`/`place_ask` only produces a node-side type error,
 * so a missing coin is reported as `null` and the caller explains what to do.
 *
 * Only the object id is needed — the node infers version, digest and owner
 * for every object-parameter arg at build time.
 */
private fun WalletViewModel.resolveTradeCoin(
    explicitCoinId: String,
    typeTag: String,
): String? {
    if (explicitCoinId.isNotBlank()) return explicitCoinId

    return accountInfo.value?.ownedObjects
        .orEmpty()
        .firstOrNull { isCoinOfType(it.getEffectiveType(), typeTag) }
        ?.id
}

/** Shared success/failure handling for the three order operations. */
private fun WalletViewModel.settleOrderResult(result: com.jamesatomc.kanariapp.network.models.TransactionResult?): Boolean {
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

/**
 * Place a bid on the deepbook order book. Follows the same signing flow as
 * token transfers: key wallets sign with the decrypted private key, zkLogin
 * wallets with the on-device Google session.
 */
suspend fun WalletViewModel.placeBid(
    bookId: String,
    price: ULong,
    quantity: ULong,
    paymentCoinId: String,
    orderType: Int,
    baseType: String = TOKEN_KANARI,
    quoteType: String = TOKEN_USD
): Boolean {
    val record = activeWallet.value ?: run {
        _error.value = "No active wallet"
        return false
    }

    val effectiveCoinId = resolveTradeCoin(paymentCoinId, quoteType) ?: run {
        _error.value = "No payment coin found in wallet"
        return false
    }

    return try {
        val result = if (record.curveType == "ZkLogin") {
            val session = try {
                ZkLoginAuth.loadSession(getApplication<Application>(), record.address)
            } catch (_: Exception) {
                _error.value =
                    "No Google session for this wallet — sign in with Google from the Login screen first."
                return false
            }
            val prepared = client.buildDeepbookBid(
                "ZkLogin:${session.address}",
                bookId, price, quantity, effectiveCoinId,
                orderType, baseType, quoteType
            )
            client.submitDeepbookTxZkLogin(session, prepared)
        } else {
            val wallet = unlockedWallet(record) ?: return false
            val prepared = client.buildDeepbookBid(
                wallet.taggedAddress,
                bookId, price, quantity, effectiveCoinId,
                orderType, baseType, quoteType
            )
            client.submitDeepbookTx(wallet, prepared)
        }
        settleOrderResult(result)
    } catch (_: ZkLoginTxSigner.SessionExpiredException) {
        _error.value = "Google session expired — open Login and sign in with Google again, then retry."
        false
    } catch (_: ZkLoginTxSigner.SessionIncompleteException) {
        _error.value = "Google session file is from an old app version — sign in with Google again."
        false
    } catch (e: Exception) {
        _error.value = "Deepbook bid failed: ${e.message}"
        false
    }
}

/**
 * Place an ask (sell order) on the deepbook order book.
 */
suspend fun WalletViewModel.placeAsk(
    bookId: String,
    price: ULong,
    quantity: ULong,
    baseCoinId: String,
    orderType: Int,
    baseType: String = TOKEN_KANARI,
    quoteType: String = TOKEN_USD
): Boolean {
    val record = activeWallet.value ?: run {
        _error.value = "No active wallet"
        return false
    }

    val effectiveCoinId = resolveTradeCoin(baseCoinId, baseType) ?: run {
        _error.value = "No base coin found in wallet"
        return false
    }

    return try {
        val result = if (record.curveType == "ZkLogin") {
            val session = try {
                ZkLoginAuth.loadSession(getApplication<Application>(), record.address)
            } catch (_: Exception) {
                _error.value =
                    "No Google session for this wallet — sign in with Google from the Login screen first."
                return false
            }
            val prepared = client.buildDeepbookAsk(
                "ZkLogin:${session.address}",
                bookId, price, quantity, effectiveCoinId,
                orderType, baseType, quoteType
            )
            client.submitDeepbookTxZkLogin(session, prepared)
        } else {
            val wallet = unlockedWallet(record) ?: return false
            val prepared = client.buildDeepbookAsk(
                wallet.taggedAddress,
                bookId, price, quantity, effectiveCoinId,
                orderType, baseType, quoteType
            )
            client.submitDeepbookTx(wallet, prepared)
        }
        settleOrderResult(result)
    } catch (_: ZkLoginTxSigner.SessionExpiredException) {
        _error.value = "Google session expired — open Login and sign in with Google again, then retry."
        false
    } catch (_: ZkLoginTxSigner.SessionIncompleteException) {
        _error.value = "Google session file is from an old app version — sign in with Google again."
        false
    } catch (e: Exception) {
        _error.value = "Deepbook ask failed: ${e.message}"
        false
    }
}

/**
 * Cancel an order on the deepbook order book.
 */
suspend fun WalletViewModel.cancelDeepbookOrder(
    bookId: String,
    orderId: ULong,
    baseType: String = TOKEN_KANARI,
    quoteType: String = TOKEN_USD
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
                _error.value =
                    "No Google session for this wallet — sign in with Google from the Login screen first."
                return false
            }
            val prepared = client.buildDeepbookCancel(
                "ZkLogin:${session.address}", bookId, orderId, baseType, quoteType
            )
            client.submitDeepbookTxZkLogin(session, prepared)
        } else {
            val wallet = unlockedWallet(record) ?: return false
            val prepared = client.buildDeepbookCancel(
                wallet.taggedAddress, bookId, orderId, baseType, quoteType
            )
            client.submitDeepbookTx(wallet, prepared)
        }
        settleOrderResult(result)
    } catch (_: ZkLoginTxSigner.SessionExpiredException) {
        _error.value = "Google session expired — open Login and sign in with Google again, then retry."
        false
    } catch (_: ZkLoginTxSigner.SessionIncompleteException) {
        _error.value = "Google session file is from an old app version — sign in with Google again."
        false
    } catch (e: Exception) {
        _error.value = "Deepbook cancel failed: ${e.message}"
        false
    }
}

// ---------------------------------------------------------------------------
// View functions (read-only)
// ---------------------------------------------------------------------------

suspend fun WalletViewModel.getBidCount(
    bookId: String,
    baseType: String = TOKEN_KANARI,
    quoteType: String = TOKEN_USD
): Int? = viewCount(bookId, baseType, quoteType, "get_bid_count")

suspend fun WalletViewModel.getAskCount(
    bookId: String,
    baseType: String = TOKEN_KANARI,
    quoteType: String = TOKEN_USD
): Int? = viewCount(bookId, baseType, quoteType, "get_ask_count")

private suspend fun WalletViewModel.viewCount(
    bookId: String,
    baseType: String,
    quoteType: String,
    function: String
): Int? = try {
    client.callViewFunction(
        DEEPBOOK_PACKAGE_ID, DEEPBOOK_MODULE, function,
        listOf(baseType, quoteType),
        listOf(JsonPrimitive(bookId))
    )?.toViewULong()?.toInt()
} catch (_: Exception) {
    null
}

/**
 * Look up the shared `OrderBook` for a token pair from the on-chain
 * `GlobalOrderBookRegistry`. The lookup lives in the `registry` module
 * (since the deepbook/registry split) and returns `Option<address>`.
 */
suspend fun WalletViewModel.getBookAddress(
    registryId: String = DEEPBOOK_REGISTRY_ID,
    baseType: String = TOKEN_KANARI,
    quoteType: String = TOKEN_USD
): String? = try {
    client.callViewFunction(
        DEEPBOOK_PACKAGE_ID, DEEPBOOK_REGISTRY_MODULE, "get_book_address",
        listOf(baseType, quoteType),
        listOf(JsonPrimitive(registryId))
    )?.toViewAddress()
} catch (_: Exception) {
    null
}

/**
 * Candidate pairs to check on-chain. Extend this list as new pairs are deployed.
 * Each entry: (baseType, quoteType, baseDecimals, quoteDecimals)
 * Symbols are derived automatically from type tags.
 */
private val DEEPBOOK_CANDIDATE_PAIRS = listOf(
    Triple(
        "0x2::kanari::KANARI", "0x2::usd::USD",
        DeepbookBookInfo(
            baseType = "0x2::kanari::KANARI",
            quoteType = "0x2::usd::USD",
            baseSymbol = extractSymbol("0x2::kanari::KANARI"),
            quoteSymbol = extractSymbol("0x2::usd::USD"),
            baseDecimals = 9,
            quoteDecimals = 6
        )
    ),
    Triple(
        "0x2::eth::ETH", "0x2::usd::USD",
        DeepbookBookInfo(
            baseType = "0x2::eth::ETH",
            quoteType = "0x2::usd::USD",
            baseSymbol = extractSymbol("0x2::eth::ETH"),
            quoteSymbol = extractSymbol("0x2::usd::USD"),
            baseDecimals = 18,
            quoteDecimals = 6
        )
    )
    // Add more candidate pairs here
)

/**
 * Discover which trading pairs have live order books by querying the registry.
 * Returns only pairs that exist on-chain (book address is not null).
 */
suspend fun WalletViewModel.discoverDeepbookBooks(): List<DeepbookBookInfo> =
    DEEPBOOK_CANDIDATE_PAIRS.mapNotNull { (baseType, quoteType, info) ->
        val bookId = getBookAddress(DEEPBOOK_REGISTRY_ID, baseType, quoteType)
        if (!bookId.isNullOrBlank()) info.copy(bookId = bookId) else null
    }

/**
 * Fetch one side of the order book level-by-level. Each level needs four
 * view calls (id, price, remaining, maker) because a whole `LimitOrder`
 * struct would come back as raw BCS bytes. Rows are fetched concurrently
 * and a row that disappears mid-refresh (book mutated between calls) is
 * skipped rather than failing the whole refresh.
 */
suspend fun WalletViewModel.getOrderBookLevels(
    bookId: String,
    isBid: Boolean,
    maxLevels: Int = 10,
    baseType: String = TOKEN_KANARI,
    quoteType: String = TOKEN_USD
): List<DeepbookOrderLevel> {
    if (bookId.isBlank()) return emptyList()
    val side = if (isBid) "bid" else "ask"
    val count = (if (isBid) getBidCount(bookId, baseType, quoteType)
    else getAskCount(bookId, baseType, quoteType)) ?: return emptyList()

    return (0 until minOf(count, maxLevels)).mapNotNull { index ->
        val indexHex = client.u64ToHex(index.toULong())
        try {
            val id = client.callViewFunction(
                DEEPBOOK_PACKAGE_ID, DEEPBOOK_MODULE, "get_${side}_id_at",
                listOf(baseType, quoteType),
                listOf(JsonPrimitive(bookId), JsonPrimitive(indexHex))
            )?.toViewULong() ?: return@mapNotNull null
            val price = client.callViewFunction(
                DEEPBOOK_PACKAGE_ID, DEEPBOOK_MODULE, "get_${side}_price_at",
                listOf(baseType, quoteType),
                listOf(JsonPrimitive(bookId), JsonPrimitive(indexHex))
            )?.toViewULong() ?: return@mapNotNull null
            val remaining = client.callViewFunction(
                DEEPBOOK_PACKAGE_ID, DEEPBOOK_MODULE, "get_${side}_remaining_at",
                listOf(baseType, quoteType),
                listOf(JsonPrimitive(bookId), JsonPrimitive(indexHex))
            )?.toViewULong() ?: return@mapNotNull null
            val maker = client.callViewFunction(
                DEEPBOOK_PACKAGE_ID, DEEPBOOK_MODULE, "get_${side}_maker_at",
                listOf(baseType, quoteType),
                listOf(JsonPrimitive(bookId), JsonPrimitive(indexHex))
            )?.toViewAddress() ?: return@mapNotNull null
            DeepbookOrderLevel(orderId = id, price = price, remaining = remaining, maker = maker)
        } catch (_: Exception) {
            // Index fell off the book between the count fetch and this row.
            null
        }
    }
}
