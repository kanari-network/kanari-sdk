package com.jamesatomc.kanariapp.network.models

import kotlinx.serialization.Serializable

@Serializable
data class KanariPay(
    val sender: String,
    val recipient: String,
    val amount: Long,
    val tokenType: String = "0x2::kanari::KANARI",
    val nonce: ULong = System.currentTimeMillis().toULong()
)
