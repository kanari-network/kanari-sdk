package com.jamesatomc.kanariapp.network.models

import kotlinx.serialization.Serializable
import kotlinx.serialization.SerialName

/** Parsed BCS payload of a standard framework NFT (`kanari_getOwnedNfts`). */
@Serializable
data class NftParsedData(
    val name: String? = null,
    @SerialName("image_url") val imageUrl: String? = null,
    val description: String? = null,
    val number: String? = null,
    val creator: String? = null,
    val attributes: NftAttributes? = null,
)

@Serializable
data class NftAttributes(
    val keys: List<String> = emptyList(),
    val values: List<String> = emptyList(),
)

@Serializable
data class Krc1155BalanceEntry(
    @SerialName("object_id") val objectId: String,
    val amount: Long? = null,
)

/** One owned KRC-1155 balance (aggregated by token type) as returned by `kanari_getOwnedKrc1155Balances`. */
@Serializable
data class OwnedKrc1155Balance(
    @SerialName("object_id") val objectId: String? = null,
    val type: String? = null,
    val owner: String? = null,
    val version: Long? = null,
    @SerialName("token_type") val tokenType: String? = null,
    val amount: Long? = null,
    val name: String? = null,
    val description: String? = null,
    @SerialName("image_url") val imageUrl: String? = null,
    @SerialName("max_supply") val maxSupply: Long? = null,
    @SerialName("total_supply") val totalSupply: Long? = null,
    val balances: List<Krc1155BalanceEntry>? = null,
    val data: List<Int>? = null,
)

/** One owned NFT object as returned by `kanari_getOwnedNfts`. */
@Serializable
data class OwnedNft(
    @SerialName("object_id") val objectId: String,
    val type: String,
    val owner: String,
    val version: Long? = null,
    val parsed: NftParsedData? = null,
    val data: List<Int>? = null,
) {
    /** Short `module::Struct` display of the full type tag. */
    val shortType: String
        get() = type.split("::").takeLast(2).joinToString("::")

    val displayName: String
        get() = parsed?.name?.takeIf { it.isNotBlank() }
        // Hex id can be long; show the middle so a card still fits.
            ?: "NFT ${objectId.takeLast(6)}"

    val imageUrl: String? get() = parsed?.imageUrl?.takeIf { it.isNotBlank() }

    val attributes: List<Pair<String, String>>
        get() {
            val attr = parsed?.attributes ?: return emptyList()
            return attr.keys.zip(attr.values)
        }

    /**
     * This object with a client-side BCS decode applied when the RPC left
     * `parsed` empty (framework KRC-721 layouts need this local fallback).
     */
    fun withLocalParse(): OwnedNft {
        if (parsed != null) return this
        val bytes = data ?: return this
        if (bytes.size < 33) return this
        val byteArray = ByteArray(bytes.size) { i -> bytes[i].toByte() }
        val local = NftBcsParser.parse(byteArray) ?: return this
        return copy(parsed = local)
    }
}
