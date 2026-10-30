package com.jamesatomc.kanariapp.network.models

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

// ==================== DeepBook Models ====================

@Serializable
data class DeepbookOrderType(
    val value: Int,
    val label: String
)

val DEEPBOOK_ORDER_TYPES = listOf(
    DeepbookOrderType(0, "Limit"),
    DeepbookOrderType(1, "IOC"),
    DeepbookOrderType(2, "FOK"),
    DeepbookOrderType(3, "Post-Only")
)

@Serializable
data class DeepbookPlaceBidRequest(
    val bookId: String,
    @Serializable(with = LenientULongSerializer::class) val price: ULong,
    @Serializable(with = LenientULongSerializer::class) val quantity: ULong,
    val paymentCoinId: String,
    val orderType: Int,
    @Serializable(with = LenientULongSerializer::class) val gasLimit: ULong,
    @Serializable(with = LenientULongSerializer::class) val gasPrice: ULong
)

@Serializable
data class DeepbookPlaceAskRequest(
    val bookId: String,
    @Serializable(with = LenientULongSerializer::class) val price: ULong,
    @Serializable(with = LenientULongSerializer::class) val quantity: ULong,
    val baseCoinId: String,
    val orderType: Int,
    @Serializable(with = LenientULongSerializer::class) val gasLimit: ULong,
    @Serializable(with = LenientULongSerializer::class) val gasPrice: ULong
)

@Serializable
data class DeepbookCancelRequest(
    val bookId: String,
    @Serializable(with = LenientULongSerializer::class) val orderId: ULong,
    @Serializable(with = LenientULongSerializer::class) val gasLimit: ULong,
    @Serializable(with = LenientULongSerializer::class) val gasPrice: ULong
)

/**
 * Order book info discovered on-chain. Includes bookId and token metadata.
 * The bookId is filled after on-chain discovery.
 */
@Serializable
data class DeepbookBookInfo(
    val bookId: String = "",
    @SerialName("base_type") val baseType: String,
    @SerialName("quote_type") val quoteType: String,
    @SerialName("fee_bps") val feeBps: Int? = null,
    @SerialName("max_depth") val maxDepth: Int? = null,
    @SerialName("base_decimals") val baseDecimals: Int,
    @SerialName("quote_decimals") val quoteDecimals: Int,
    val baseSymbol: String = "",
    val quoteSymbol: String = ""
) {
    val displayLabel: String
        get() = "$baseSymbol/$quoteSymbol (${bookId.take(6)}...${bookId.takeLast(4)})"
}

@Serializable
data class DeepbookTradeResult(
    val hash: String,
    val status: String,
    val success: Boolean,
    @SerialName("error_message") val errorMessage: String? = null
)

/**
 * One level of the on-chain order book, assembled field-by-field from the
 * per-index view getters (`get_bid_price_at` etc.). The whole `LimitOrder`
 * struct cannot be returned through a view call — struct results come back
 * as raw BCS bytes — so each field is fetched individually.
 */
data class DeepbookOrderLevel(
    val orderId: ULong,
    val price: ULong,
    val remaining: ULong,
    val maker: String
)

const val DEEPBOOK_MODULE = "deepbook"

// Registry functions (`get_book_address`, book creation, fee admin) live in
// the separate `registry` module since the deepbook/registry split.
const val DEEPBOOK_REGISTRY_MODULE = "registry"

