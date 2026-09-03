package com.jamesatomc.kanariapp.network.models

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class AccountInfo(
    val address: String? = null,
    val owner: String? = null,
    val nonce: Int = 0,
    val modules: List<String> = emptyList(),
    @SerialName("token_balances") val tokenBalances: Map<String, Long> = emptyMap(),
    val balances: Map<String, Long> = emptyMap(),
    @SerialName("owned_objects") val ownedObjects: List<ObjectInfo>? = null
) {
    fun getEffectiveAddress(): String = address ?: owner ?: ""
    fun getEffectiveBalances(): Map<String, Long> = if (balances.isNotEmpty()) balances else tokenBalances
}

@Serializable
data class BalancesResponse(
    val owner: String? = null,
    val balances: List<TokenBalance> = emptyList()
)

@Serializable
data class TokenBalance(
    @SerialName("token_type") val tokenType: String,
    val amount: Long = 0,
    val balance: Long = 0,
    val decimals: Int = 9,
    val symbol: String = "",
    @SerialName("icon_url") val iconUrl: String? = null,
    val name: String? = null,
    val description: String? = null
) {
    fun getEffectiveAmount(): Long = if (amount != 0L) amount else balance
}

@Serializable
data class TokenInfo(
    @SerialName("token_type") val tokenType: String,
    @SerialName("total_supply") val totalSupply: Long,
    @SerialName("wallet_visible_supply") val walletVisibleSupply: Long? = null,
    @SerialName("circulating_supply") val circulatingSupply: Long? = null,
    @SerialName("object_locked_supply") val objectLockedSupply: Long? = null,
    @SerialName("accounted_supply") val accountedSupply: Long? = null,
    @SerialName("untracked_supply") val untrackedSupply: Long? = null,
    val decimals: Int = 9,
    val symbol: String = "",
    @SerialName("icon_url") val iconUrl: String? = null,
    val name: String? = null,
    val description: String? = null
)

@Serializable
data class ObjectInfo(
    val id: String,
    val owner: String,
    val type: String? = null,
    @SerialName("type_") val typeAlternative: String? = null,
    val data: List<Int> = emptyList(),
    val version: Int = 0,
    val digest: String? = null
) {
    fun getEffectiveType(): String = type ?: typeAlternative ?: ""
}
