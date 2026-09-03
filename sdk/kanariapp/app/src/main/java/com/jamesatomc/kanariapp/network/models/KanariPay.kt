package com.jamesatomc.kanariapp.network.models

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
sealed class KanariTransaction {
    @Serializable
    @SerialName("PublishModule")
    data class PublishModule(
        val sender: String,
        @SerialName("module_bytes") val moduleBytes: List<Byte>,
        @SerialName("module_name") val moduleName: String,
        @SerialName("gas_payment") val gasPayment: GasPayment? = null,
        @SerialName("gas_limit") val gasLimit: Long,
        @SerialName("gas_price") val gasPrice: Long,
        val nonce: ULong
    ) : KanariTransaction()

    @Serializable
    @SerialName("PublishPackage")
    data class PublishPackage(
        val sender: String,
        val modules: List<PublishedModule>,
        @SerialName("gas_payment") val gasPayment: GasPayment? = null,
        @SerialName("gas_limit") val gasLimit: Long,
        @SerialName("gas_price") val gasPrice: Long,
        val nonce: ULong
    ) : KanariTransaction()

    @Serializable
    @SerialName("UpgradeModule")
    data class UpgradeModule(
        val sender: String,
        @SerialName("module_bytes") val moduleBytes: List<Byte>,
        @SerialName("module_name") val moduleName: String,
        @SerialName("gas_payment") val gasPayment: GasPayment? = null,
        @SerialName("gas_limit") val gasLimit: Long,
        @SerialName("gas_price") val gasPrice: Long,
        val nonce: ULong
    ) : KanariTransaction()

    @Serializable
    @SerialName("UpgradePackage")
    data class UpgradePackage(
        val sender: String,
        val modules: List<PublishedModule>,
        @SerialName("gas_payment") val gasPayment: GasPayment? = null,
        @SerialName("gas_limit") val gasLimit: Long,
        @SerialName("gas_price") val gasPrice: Long,
        val nonce: ULong
    ) : KanariTransaction()

    @Serializable
    @SerialName("ExecuteFunction")
    data class ExecuteFunction(
        val sender: String,
        val module: String,
        val function: String,
        @SerialName("type_args") val typeArgs: List<String> = emptyList(),
        val args: List<List<Byte>> = emptyList(),
        @SerialName("object_inputs") val objectInputs: List<ObjectInput> = emptyList(),
        @SerialName("gas_payment") val gasPayment: GasPayment? = null,
        @SerialName("gas_limit") val gasLimit: Long,
        @SerialName("gas_price") val gasPrice: Long,
        val nonce: ULong
    ) : KanariTransaction()
}

@Serializable
data class PublishedModule(
    @SerialName("module_name") val moduleName: String,
    @SerialName("module_bytes") val moduleBytes: List<Byte>
)

@Serializable
sealed class ObjectOwnerKind {
    @Serializable
    @SerialName("AddressOwner")
    data class AddressOwner(val address: String) : ObjectOwnerKind()

    @Serializable
    @SerialName("Shared")
    object Shared : ObjectOwnerKind()

    @Serializable
    @SerialName("Immutable")
    object Immutable : ObjectOwnerKind()
}

@Serializable
data class ObjectInput(
    @SerialName("object_ref") val objectRef: ObjectRef,
    val owner: ObjectOwnerKind? = null,
    val mutable: Byte // Using Byte for u8
)

@Serializable
data class KanariPay(
    val sender: String,
    val recipient: String,
    val amount: Long,
    val tokenType: String = "0x2::kanari::KANARI",
    val nonce: ULong = System.currentTimeMillis().toULong()
)
