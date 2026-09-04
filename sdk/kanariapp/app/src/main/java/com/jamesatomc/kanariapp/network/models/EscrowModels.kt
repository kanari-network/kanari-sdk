package com.jamesatomc.kanariapp.network.models

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class EscrowDeal(
    @SerialName("object_id") val objectId: String,
    @SerialName("deal_id") val dealId: String,
    val buyer: String,
    val seller: String,
    val amount: Long,
    @SerialName("coin_type") val coinType: String,
    val description: String = "",
    val state: Int,
    @SerialName("proof_id") val proofId: String? = null
)

object EscrowConstants {
    const val STATE_ACTIVE = 0
    const val STATE_DELIVERED = 1
    const val STATE_RELEASED = 2
    const val STATE_DISPUTED = 3
    const val STATE_REFUNDED = 4

    fun getStateName(state: Int): String = when (state) {
        STATE_ACTIVE -> "Active"
        STATE_DELIVERED -> "Delivered"
        STATE_RELEASED -> "Completed"
        STATE_DISPUTED -> "Disputed"
        STATE_REFUNDED -> "Refunded"
        else -> "Unknown"
    }
}