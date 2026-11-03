// Escrow data models for kanariapp

package com.jamesatomc.kanariapp.network.models

data class EscrowObjectRefs(
    val dealObjectId: String,
    val proofObjectId: String,
    val coinType: String
)

data class DealDetails(
    val dealId: String,
    val buyer: String,
    val seller: String,
    val amount: Long,
    val coinType: String,
    val state: Int
)
