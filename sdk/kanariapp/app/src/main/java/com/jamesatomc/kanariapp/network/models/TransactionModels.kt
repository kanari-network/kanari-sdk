package com.jamesatomc.kanariapp.network.models

import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.SerializationException
import kotlinx.serialization.descriptors.PrimitiveKind
import kotlinx.serialization.descriptors.PrimitiveSerialDescriptor
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder
import kotlinx.serialization.json.*

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
    @SerialName("gas_limit") val gasLimit: ULong,
    @SerialName("gas_price") val gasPrice: ULong,
    val module: String? = null,
    val function: String? = null,
    @SerialName("module_functions") val moduleFunctions: List<String>? = null,
    val effects: TransactionEffectsInfo? = null
)

@Serializable
data class ObjectChangeInfo(
    @SerialName("change_type") val changeType: String,
    @SerialName("object_ref") val objectRef: ObjectRef,
    val type: String? = null,
    @SerialName("type_") val typeAlternative: String? = null,
    val owner: ObjectOwnerKind? = null
)

@Serializable(with = ObjectOwnerKindSerializer::class)
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

object ObjectOwnerKindSerializer : KSerializer<ObjectOwnerKind> {
    @Serializable
    @SerialName("ObjectOwnerKind")
    private sealed class Surrogate {
        @Serializable
        @SerialName("AddressOwner")
        data class AddressOwner(val address: String) : Surrogate()
        @Serializable
        @SerialName("Shared")
        object Shared : Surrogate()
        @Serializable
        @SerialName("Immutable")
        object Immutable : Surrogate()
    }

    override val descriptor: SerialDescriptor = Surrogate.serializer().descriptor

    override fun deserialize(decoder: Decoder): ObjectOwnerKind {
        return if (decoder is JsonDecoder) {
            val element = decoder.decodeJsonElement()
            if (element is JsonPrimitive) {
                when (element.content) {
                    "Shared" -> ObjectOwnerKind.Shared
                    "Immutable" -> ObjectOwnerKind.Immutable
                    else -> throw SerializationException("Unknown ObjectOwnerKind variant: ${element.content}")
                }
            } else {
                val obj = element.jsonObject
                when {
                    "AddressOwner" in obj -> ObjectOwnerKind.AddressOwner(obj["AddressOwner"]!!.jsonPrimitive.content)
                    "Shared" in obj -> ObjectOwnerKind.Shared
                    "Immutable" in obj -> ObjectOwnerKind.Immutable
                    else -> throw SerializationException("Unknown ObjectOwnerKind object: $obj")
                }
            }
        } else {
            when (val surrogate = decoder.decodeSerializableValue(Surrogate.serializer())) {
                is Surrogate.AddressOwner -> ObjectOwnerKind.AddressOwner(surrogate.address)
                Surrogate.Shared -> ObjectOwnerKind.Shared
                Surrogate.Immutable -> ObjectOwnerKind.Immutable
            }
        }
    }

    override fun serialize(encoder: Encoder, value: ObjectOwnerKind) {
        if (encoder is JsonEncoder) {
            val element = when (value) {
                is ObjectOwnerKind.AddressOwner -> buildJsonObject { put("AddressOwner", value.address) }
                ObjectOwnerKind.Shared -> JsonPrimitive("Shared")
                ObjectOwnerKind.Immutable -> JsonPrimitive("Immutable")
            }
            encoder.encodeJsonElement(element)
        } else {
            val surrogate = when (value) {
                is ObjectOwnerKind.AddressOwner -> Surrogate.AddressOwner(value.address)
                ObjectOwnerKind.Shared -> Surrogate.Shared
                ObjectOwnerKind.Immutable -> Surrogate.Immutable
            }
            encoder.encodeSerializableValue(Surrogate.serializer(), surrogate)
        }
    }
}

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
    val amount: ULong,
    @SerialName("gas_limit") val gasLimit: ULong,
    @SerialName("gas_price") val gasPrice: ULong,
    @SerialName("excluded_object_ids") val excludedObjectIds: List<String> = emptyList(),
    val nonce: ULong? = null,
    @SerialName("execute_immediate") val executeImmediate: Boolean? = true
)

/**
 * Lenient ULong serializer that accepts JSON numbers (including > Long.MAX_VALUE)
 * and JSON strings. Needed because `fresh_nonce` is a random u64 (0..2^64-1) which
 * may be serialized as a number beyond signed 64-bit. Serializes as unquoted
 * JSON number via JsonUnquotedLiteral to preserve full precision.
 */
@OptIn(ExperimentalSerializationApi::class)
object LenientULongSerializer : KSerializer<ULong> {
    override val descriptor: SerialDescriptor = PrimitiveSerialDescriptor("LenientULong", PrimitiveKind.LONG)
    override fun deserialize(decoder: Decoder): ULong {
        return if (decoder is JsonDecoder) {
            val element = decoder.decodeJsonElement()
            if (element is JsonNull) throw SerializationException("Expected ULong but got null")
            val prim = element.jsonPrimitive
            val content = prim.content
            content.toULongOrNull()
                ?: prim.longOrNull?.toULong()
                ?: prim.doubleOrNull?.toLong()?.toULong()
                ?: throw SerializationException("Invalid ULong value: $content")
        } else {
            decoder.decodeLong().toULong()
        }
    }

    override fun serialize(encoder: Encoder, value: ULong) {
        if (encoder is JsonEncoder) {
            encoder.encodeJsonElement(JsonUnquotedLiteral(value.toString()))
        } else {
            encoder.encodeLong(value.toLong())
        }
    }
}

@Serializable
data class ObjectRef(
    @SerialName("object_id") val objectId: String,
    @Serializable(with = LenientULongSerializer::class) val version: ULong? = null,
    val digest: String? = null
)

@Serializable
data class GasPayment(
    @SerialName("payment_objects") val paymentObjects: List<ObjectRef>,
    val owner: String,
    @Serializable(with = LenientULongSerializer::class) val budget: ULong,
    @Serializable(with = LenientULongSerializer::class) val price: ULong
)

@Serializable
data class ObjectTransferData(
    val sender: String,
    @SerialName("coin_object_id") val coinObjectId: String,
    @SerialName("coin_object_ref") val coinObjectRef: ObjectRef? = null,
    val recipient: String,
    @Serializable(with = LenientULongSerializer::class) val amount: ULong,
    @Serializable(with = LenientULongSerializer::class) @SerialName("gas_limit") val gasLimit: ULong,
    @Serializable(with = LenientULongSerializer::class) @SerialName("gas_price") val gasPrice: ULong,
    @Serializable(with = LenientULongSerializer::class) var nonce: ULong? = null,
    @SerialName("gas_payment") val gasPayment: GasPayment? = null,
    var signature: List<Int>? = null,
    @SerialName("execute_immediate") val executeImmediate: Boolean? = true
)

@Serializable
sealed class KanariTransaction {
    @Serializable
    @SerialName("PublishModule")
    data class PublishModule(
        val sender: String,
        @SerialName("module_bytes") val moduleBytes: List<UByte>,
        @SerialName("module_name") val moduleName: String,
        @SerialName("gas_payment") val gasPayment: GasPayment? = null,
        @SerialName("gas_limit") val gasLimit: ULong,
        @SerialName("gas_price") val gasPrice: ULong,
        val nonce: ULong
    ) : KanariTransaction()

    @Serializable
    @SerialName("PublishPackage")
    data class PublishPackage(
        val sender: String,
        val modules: List<PublishedModule>,
        @SerialName("gas_payment") val gasPayment: GasPayment? = null,
        @SerialName("gas_limit") val gasLimit: ULong,
        @SerialName("gas_price") val gasPrice: ULong,
        val nonce: ULong
    ) : KanariTransaction()

    @Serializable
    @SerialName("UpgradeModule")
    data class UpgradeModule(
        val sender: String,
        @SerialName("module_bytes") val moduleBytes: List<UByte>,
        @SerialName("module_name") val moduleName: String,
        @SerialName("gas_payment") val gasPayment: GasPayment? = null,
        @SerialName("gas_limit") val gasLimit: ULong,
        @SerialName("gas_price") val gasPrice: ULong,
        val nonce: ULong
    ) : KanariTransaction()

    @Serializable
    @SerialName("UpgradePackage")
    data class UpgradePackage(
        val sender: String,
        val modules: List<PublishedModule>,
        @SerialName("gas_payment") val gasPayment: GasPayment? = null,
        @SerialName("gas_limit") val gasLimit: ULong,
        @SerialName("gas_price") val gasPrice: ULong,
        val nonce: ULong
    ) : KanariTransaction()

    @Serializable
    @SerialName("ExecuteFunction")
    data class ExecuteFunction(
        val sender: String,
        val module: String,
        val function: String,
        @SerialName("type_args") val typeArgs: List<String>,
        val args: List<List<UByte>>,
        @SerialName("object_inputs") val objectInputs: List<ObjectInput>,
        @SerialName("gas_payment") val gasPayment: GasPayment?,
        @SerialName("gas_limit") val gasLimit: ULong,
        @SerialName("gas_price") val gasPrice: ULong,
        val nonce: ULong
    ) : KanariTransaction()
}

@Serializable
data class PublishedModule(
    @SerialName("module_name") val moduleName: String,
    @SerialName("module_bytes") val moduleBytes: List<UByte>
)

@Serializable
data class ObjectInput(
    @SerialName("object_ref") val objectRef: ObjectRef,
    val owner: ObjectOwnerKind?,
    val mutable: Boolean
)

@Serializable
data class CallFunctionData(
    val sender: String,
    @SerialName("package") val packageAddr: String,
    val module: String,
    val function: String,
    @SerialName("type_args") val typeArgs: List<String> = emptyList(),
    val args: List<List<Int>> = emptyList(),
    @SerialName("object_inputs") val objectInputs: List<ObjectInput>? = null,
    @SerialName("gas_payment") val gasPayment: GasPayment? = null,
    @Serializable(with = LenientULongSerializer::class) @SerialName("gas_limit") val gasLimit: ULong,
    @Serializable(with = LenientULongSerializer::class) @SerialName("gas_price") val gasPrice: ULong,
    @Serializable(with = LenientULongSerializer::class) val nonce: ULong? = null,
    var signature: List<Int>? = null,
    @SerialName("execute_immediate") val executeImmediate: Boolean? = true
)