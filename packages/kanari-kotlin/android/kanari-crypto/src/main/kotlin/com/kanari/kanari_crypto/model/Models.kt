package com.kanari.kanari_crypto.model

data class KeyPairModel(
    val privateKey: String,
    val publicKey: String,
    val address: String,
    val taggedAddress: String,
    val rawPublicKey: ByteArray,
    val curveType: String,
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is KeyPairModel) return false
        return privateKey == other.privateKey &&
            publicKey == other.publicKey &&
            address == other.address &&
            taggedAddress == other.taggedAddress &&
            rawPublicKey.contentEquals(other.rawPublicKey) &&
            curveType == other.curveType
    }

    override fun hashCode(): Int {
        var result = privateKey.hashCode()
        result = 31 * result + publicKey.hashCode()
        result = 31 * result + address.hashCode()
        result = 31 * result + taggedAddress.hashCode()
        result = 31 * result + rawPublicKey.contentHashCode()
        result = 31 * result + curveType.hashCode()
        return result
    }
}

data class CurveInfoModel(
    val name: String,
    val isPostQuantum: Boolean,
    val isHybrid: Boolean,
    val securityLevel: Int,
)

/**
 * Fresh login material from `zklogin_prepare_nonce` (Rust-owned crypto).
 * No salt here: the address-salt is the kanari-crypto standard
 * (`zkLoginDeterministicSalt`), derived after the JWT is known.
 */
data class ZkLoginNonceModel(
    val ephemeralPubkey: ByteArray,
    val ephemeralSecret: ByteArray,
    val randomness: ByteArray,
    val maxEpoch: Long,
    val nonce: String,
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is ZkLoginNonceModel) return false
        return ephemeralPubkey.contentEquals(other.ephemeralPubkey) &&
            ephemeralSecret.contentEquals(other.ephemeralSecret) &&
            randomness.contentEquals(other.randomness) &&
            maxEpoch == other.maxEpoch &&
            nonce == other.nonce
    }

    override fun hashCode(): Int {
        var result = ephemeralPubkey.contentHashCode()
        result = 31 * result + ephemeralSecret.contentHashCode()
        result = 31 * result + randomness.contentHashCode()
        result = 31 * result + maxEpoch.hashCode()
        result = 31 * result + nonce.hashCode()
        return result
    }
}

/** Verified JWT claims from `zklogin_verify_jwt`. */
data class ZkLoginClaimsModel(
    val iss: String,
    val aud: String,
    val sub: String,
    val exp: Long?,
    val nonce: String?,
)
