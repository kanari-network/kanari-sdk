package io.kanari.wallet.models

import kotlinx.serialization.Serializable
import io.kanari.wallet.WalletKey

@Serializable
data class Wallet(
    val address: String,
    val privateKey: String,
    val seedPhrase: String,
    val curveType: CurveType
) {
    /**
     * Sign a message using this wallet
     */
    fun sign(message: ByteArray, password: String): ByteArray {
        return WalletKey.signMessage(this, message, password)
    }

    /**
     * Get wallet's curve type
     */
    fun getCurveType(): CurveType = curveType
}

@Serializable
enum class CurveType {
    K256,
    P256;
    
    companion object {
        val DEFAULT = K256
    }
}

@Serializable
data class EncryptedData(
    val ciphertext: ByteArray,
    val salt: String,
    val nonce: ByteArray
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (javaClass != other?.javaClass) return false

        other as EncryptedData

        if (!ciphertext.contentEquals(other.ciphertext)) return false
        if (salt != other.salt) return false
        if (!nonce.contentEquals(other.nonce)) return false

        return true
    }

    override fun hashCode(): Int {
        var result = ciphertext.contentHashCode()
        result = 31 * result + salt.hashCode()
        result = 31 * result + nonce.contentHashCode()
        return result
    }
}
