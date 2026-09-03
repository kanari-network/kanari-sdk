package com.jamesatomc.kanariapp.network.models

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class TransactionResult(
    val hash: String,
    val status: String,
    val success: Boolean = false,
    val previewed: Boolean = false,
    val submitted: Boolean = false,
    val committed: Boolean = false,
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
    val nonce: ULong,
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

@Serializable
data class BuildNativeTransferRequest(
    val sender: String,
    val recipient: String,
    val amount: Long,
    @SerialName("gas_limit") val gasLimit: Long,
    @SerialName("gas_price") val gasPrice: Long,
    @SerialName("excluded_object_ids") val excludedObjectIds: List<String> = emptyList(),
    val nonce: ULong? = null,
    @SerialName("execute_immediate") val executeImmediate: Boolean? = true
)

@Serializable
data class ObjectRef(
    @SerialName("object_id") val objectId: String,
    val version: Long? = null,
    val digest: String? = null
)

@Serializable
data class GasPayment(
    @SerialName("payment_objects") val paymentObjects: List<ObjectRef>,
    val owner: String,
    val budget: Long,
    val price: Long
)

@Serializable
data class ObjectTransferData(
    val sender: String,
    @SerialName("coin_object_id") val coinObjectId: String,
    @SerialName("coin_object_ref") val coinObjectRef: ObjectRef? = null,
    val recipient: String,
    val amount: Long,
    @SerialName("gas_limit") val gasLimit: Long,
    @SerialName("gas_price") val gasPrice: Long,
    val nonce: ULong? = null,
    @SerialName("gas_payment") val gasPayment: GasPayment? = null,
    var signature: List<Int>? = null, // Using List<Int> for byte array compatibility in serialization
    @SerialName("execute_immediate") val executeImmediate: Boolean? = true
)
