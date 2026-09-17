package com.kanari.kanari_crypto

import com.kanari.kanari_crypto.model.CurveInfoModel
import com.kanari.kanari_crypto.model.KeyPairModel
import com.kanari.kanari_crypto.model.ZkLoginClaimsModel
import com.kanari.kanari_crypto.model.ZkLoginNonceModel
import uniffi.kanari_kotlin.CurveInfo
import uniffi.kanari_kotlin.KeyPairData
import uniffi.kanari_kotlin.ZkLoginClaimsData
import uniffi.kanari_kotlin.ZkLoginNonceData
import uniffi.kanari_kotlin.blake3HashApi
import uniffi.kanari_kotlin.deriveKeypairFromMnemonic as ffiDeriveKeypairFromMnemonic
import uniffi.kanari_kotlin.deriveKeypairFromPathApi
import uniffi.kanari_kotlin.deriveKeypairFromSeedApi
import uniffi.kanari_kotlin.deriveMultipleAddressesApi
import uniffi.kanari_kotlin.generateKeypairApi
import uniffi.kanari_kotlin.generateMnemonicApi
import uniffi.kanari_kotlin.importKeypairFromPrivateKey as ffiImportKeypairFromPrivateKey
import uniffi.kanari_kotlin.listSupportedCurves as ffiListSupportedCurves
import uniffi.kanari_kotlin.signMessageApi
import uniffi.kanari_kotlin.verifySignatureApi
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

/**
 * High-level entry point for Kanari cryptographic operations on Android.
 *
 * Wraps the UniFFI-generated Rust bindings with idiomatic Kotlin types.
 */
object KanariCrypto {
    const val DEFAULT_CURVE = "Ed25519"

    suspend fun generateKeypair(curveName: String = DEFAULT_CURVE): KeyPairModel = calculateWithLargeStack {
        generateKeypairApi(curveName).toModel()
    }

    suspend fun generateMnemonic(wordCount: Int = 12): String = calculateWithLargeStack {
        require(wordCount == 12 || wordCount == 24) {
            "Only 12 or 24-word mnemonics are supported"
        }
        generateMnemonicApi(wordCount.toUInt())
    }

    suspend fun deriveKeypairFromMnemonic(
        mnemonic: String,
        curveName: String = DEFAULT_CURVE,
    ): KeyPairModel = calculateWithLargeStack {
        ffiDeriveKeypairFromMnemonic(mnemonic, curveName).toModel()
    }

    suspend fun deriveKeypairFromPath(
        mnemonic: String,
        derivationPath: String,
        curveName: String = DEFAULT_CURVE,
    ): KeyPairModel = calculateWithLargeStack {
        deriveKeypairFromPathApi(mnemonic, derivationPath, curveName).toModel()
    }

    /**
     * Derives a keypair deterministically from raw seed material.
     * [seed] must hold at least 64 bytes (e.g. a BIP39 seed). All curves
     * including post-quantum and hybrid types are supported.
     */
    suspend fun deriveKeypairFromSeed(
        seed: ByteArray,
        curveName: String = DEFAULT_CURVE,
    ): KeyPairModel = calculateWithLargeStack {
        require(seed.size >= 64) { "Seed must hold at least 64 bytes" }
        deriveKeypairFromSeedApi(seed.toUByteList(), curveName).toModel()
    }

    suspend fun deriveMultipleAddresses(
        mnemonic: String,
        pathTemplate: String,
        curveName: String = DEFAULT_CURVE,
        count: Int,
    ): List<KeyPairModel> = calculateWithLargeStack {
        deriveMultipleAddressesApi(mnemonic, pathTemplate, curveName, count.toUInt())
            .map { it.toModel() }
    }

    suspend fun importKeypairFromPrivateKey(
        privateKey: String,
        curveName: String = DEFAULT_CURVE,
    ): KeyPairModel = calculateWithLargeStack {
        ffiImportKeypairFromPrivateKey(privateKey, curveName).toModel()
    }

    suspend fun signMessage(
        privateKey: String,
        message: ByteArray,
        curveName: String = DEFAULT_CURVE,
    ): ByteArray = calculateWithLargeStack {
        signMessageApi(privateKey, message.toUByteList(), curveName).toByteArray()
    }

    suspend fun verifySignature(
        address: String,
        message: ByteArray,
        signature: ByteArray,
        curveName: String = DEFAULT_CURVE,
    ): Boolean = calculateWithLargeStack {
        verifySignatureApi(address, message.toUByteList(), signature.toUByteList(), curveName)
    }

    fun blake3Hash(data: ByteArray): ByteArray =
        blake3HashApi(data.toUByteList()).toByteArray()

    fun listSupportedCurves(): List<CurveInfoModel> =
        ffiListSupportedCurves().map { it.toModel() }

    // ---- zkLogin (Google OIDC bound to an ephemeral key) ----
    //
    // Crypto vectors live in Rust (`kanari_crypto::signatures::zklogin`)
    // so Kotlin and the chain can never drift. The OAuth browser dance
    // stays in the app layer; these functions own prepare/verify/derive.

    /** Fresh ephemeral key + randomness + salt bound into an OIDC nonce. */
    suspend fun zkLoginPrepareNonce(maxEpoch: Long): ZkLoginNonceModel =
        calculateWithLargeStack {
            uniffi.kanari_kotlin.zkloginPrepareNonce(maxEpoch.toULong()).toModel()
        }

    /** Verify an id_token against a provider JWKS (RS256 + iss/aud/exp + nonce). */
    suspend fun zkLoginVerifyJwt(
        jwt: String,
        jwksJson: String,
        expectedIss: String,
        expectedAud: String,
        expectedNonce: String?,
        nowSecs: Long,
    ): ZkLoginClaimsModel = calculateWithLargeStack {
        uniffi.kanari_kotlin.zkloginVerifyJwt(
            jwt, jwksJson, expectedIss, expectedAud, expectedNonce, nowSecs.toULong()
        ).toModel()
    }

    /** Derive the canonical v2 zkLogin address (matches chain + CLI). */
    suspend fun zkLoginDeriveAddress(
        iss: String,
        aud: String,
        sub: String,
        salt: ByteArray,
    ): String = calculateWithLargeStack {
        require(salt.size == 32) { "salt must be 32 bytes" }
        uniffi.kanari_kotlin.zkloginDeriveAddress(iss, aud, sub, salt.toUByteList())
    }

    /** Sign bytes with a 32-byte ephemeral secret from `zkLoginPrepareNonce`. */
    suspend fun zkLoginSignEphemeral(secret: ByteArray, message: ByteArray): ByteArray =
        calculateWithLargeStack {
            require(secret.size == 32) { "ephemeral secret must be 32 bytes" }
            uniffi.kanari_kotlin.zkloginSignEphemeral(
                secret.toUByteList(), message.toUByteList()
            ).toByteArray()
        }

    /** Verify an ephemeral Ed25519 signature (false = wrong, throw = malformed). */
    suspend fun zkLoginVerifyEphemeral(
        pubkey: ByteArray,
        message: ByteArray,
        signature: ByteArray,
    ): Boolean = calculateWithLargeStack {
        uniffi.kanari_kotlin.zkloginVerifyEphemeral(
            pubkey.toUByteList(), message.toUByteList(), signature.toUByteList()
        )
    }

    /** Build the opaque zkLogin bundle bytes (Rust owns JSON, Kotlin just passes fields). */
    suspend fun zkLoginBuildBundle(
        jwt: String,
        jwksJson: String,
        iss: String,
        aud: String,
        salt: ByteArray,
        randomness: ByteArray,
        ephemeralPubkey: ByteArray,
        ephemeralSig: ByteArray,
        maxEpoch: Long,
    ): ByteArray = calculateWithLargeStack {
        require(salt.size == 32) { "salt must be 32 bytes" }
        require(randomness.size == 32) { "randomness must be 32 bytes" }
        require(ephemeralPubkey.size == 32) { "ephemeral pubkey must be 32 bytes" }
        require(ephemeralSig.size == 64) { "ephemeral sig must be 64 bytes" }
        uniffi.kanari_kotlin.zkloginBuildBundle(
            jwt, jwksJson, iss, aud,
            salt.toUByteList(), randomness.toUByteList(),
            ephemeralPubkey.toUByteList(), ephemeralSig.toUByteList(),
            maxEpoch.toULong()
        ).toByteArray()
    }

    /**
     * Executes crypto operations on a new thread with a larger stack size.
     * Required for Post-Quantum (PQ) and Hybrid curves (e.g. Dilithium) 
     * which can exceed the default Android stack limit.
     */
    private suspend fun <T> calculateWithLargeStack(block: () -> T): T = suspendCancellableCoroutine { cont ->
        val thread = Thread(null, {
            try {
                cont.resume(block())
            } catch (e: Throwable) {
                cont.resumeWithException(e)
            }
        }, "KanariCryptoThread", 16 * 1024 * 1024) // 16MB stack

        thread.start()
    }
}

private fun KeyPairData.toModel(): KeyPairModel =
    KeyPairModel(
        privateKey = privateKey,
        publicKey = publicKey,
        address = address,
        taggedAddress = taggedAddress,
        rawPublicKey = rawPublicKey.map { it.toByte() }.toByteArray(),
        curveType = curveType,
    )

private fun ZkLoginNonceData.toModel(): ZkLoginNonceModel =
    ZkLoginNonceModel(
        ephemeralPubkey = ephemeralPubkey.map { it.toByte() }.toByteArray(),
        ephemeralSecret = ephemeralSecret.map { it.toByte() }.toByteArray(),
        randomness = randomness.map { it.toByte() }.toByteArray(),
        salt = salt.map { it.toByte() }.toByteArray(),
        maxEpoch = maxEpoch.toLong(),
        nonce = nonce,
    )

private fun ZkLoginClaimsData.toModel(): ZkLoginClaimsModel =
    ZkLoginClaimsModel(
        iss = iss,
        aud = aud,
        sub = sub,
        exp = exp?.toLong(),
        nonce = nonce,
    )

private fun CurveInfo.toModel(): CurveInfoModel =
    CurveInfoModel(
        name = name,
        isPostQuantum = isPostQuantum,
        isHybrid = isHybrid,
        securityLevel = securityLevel.toInt(),
    )

private fun ByteArray.toUByteList(): List<UByte> = map { it.toUByte() }

private fun List<UByte>.toByteArray(): ByteArray = map { it.toByte() }.toByteArray()
