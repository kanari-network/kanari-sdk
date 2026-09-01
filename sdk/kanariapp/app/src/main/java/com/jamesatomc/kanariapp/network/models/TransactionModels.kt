package com.jamesatomc.kanariapp.network.models

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class TransactionResult(
    val hash: String,
    val status: String,
    @SerialName("gas_used") val gasUsed: Long? = null,
    val effects: TransactionEffectsInfo? = null,
    @SerialName("error_message") val errorMessage: String? = null,
    val action: String? = null
)

@Serializable
data class TransactionDetails(
    val hash: String,
    val status: String,
    @SerialName("block_height") val blockHeight: Long? = null,
    @SerialName("checkpoint_height") val checkpointHeight: Long? = null,
    @SerialName("gas_used") val gasUsed: Long? = null,
    @SerialName("tx_type") val txType: String,
    val sender: String,
    @SerialName("sender_address") val senderAddress: String? = null,
    val nonce: Long,
    @SerialName("gas_limit") val gasLimit: Long,
    @SerialName("gas_price") val gasPrice: Long,
    val module: String? = null,
    val function: String? = null,
    @SerialName("module_functions") val moduleFunctions: List<String>? = null,
    val effects: TransactionEffectsInfo? = null
)

@Serializable
data class ObjectRefInfo(
    @SerialName("object_id") val objectId: String,
    val version: Long? = null,
    val digest: String? = null
)

@Serializable
data class ObjectChangeInfo(
    @SerialName("change_type") val changeType: String,
    @SerialName("object_ref") val objectRef: ObjectRefInfo,
    val type: String? = null,
    @SerialName("type_") val typeAlternative: String? = null,
    val owner: String? = null
)

@Serializable
data class TransactionEffectsInfo(
    val status: String,
    @SerialName("gas_used") val gasUsed: Long,
    @SerialName("object_changes") val objectChanges: List<ObjectChangeInfo> = emptyList(),
    val created: List<ObjectChangeInfo>? = null,
    val mutated: List<ObjectChangeInfo>? = null,
    val transferred: List<ObjectChangeInfo>? = null
)
