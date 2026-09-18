package com.jamesatomc.kanariapp.wallet.zklogin

import com.kanari.kanari_crypto.KanariCrypto
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.encodeToJsonElement
import kotlinx.serialization.json.jsonObject

/**
 * Builds the opaque `signature` bytes a `ZkLogin:` sender attaches to a
 * transaction — the exact wire shape of `KanariTxSignature` /
 * `ZkLoginTxSignature` in `kanari-crypto/src/signatures/zk_authenticator.rs`:
 *
 * ```json
 * {"v":1,"auth":{
 *    "ephemeral_pubkey":"<64 hex, lowercase, no 0x>",
 *    "ephemeral_sig":"<128 hex>",
 *    "max_epoch":<u64>,
 *    "kind":{"Jwt":{
 *       "jwt":"<raw id_token>",
 *       "jwks":{"keys":[...]},
 *       "iss":"https://accounts.google.com",
 *       "aud":"<web client id>",
 *       "randomness":"<64 hex>",
 *       "salt":"<64 hex>"
 *    }}
 * }}
 * ```
 *
 * Crypto stays in Rust via `KanariCrypto.zkLogin*` (same vectors as the
 * chain), including the final bundle assembly (`zkLoginBuildBundle`); this
 * file does a full fail-closed re-verify of the session (JWT RS256, nonce
 * binding, address match) BEFORE anything is sent, mirroring what the RPC
 * node will check. The node re-verifies independently, so a malformed bundle
 * can never execute. (`bundleJson` remains only as a JVM-testable record of
 * the wire shape.)
 */
object ZkLoginTxSigner {

    /** 60s leeway, must match the `verify_claims_timing` leeway on the node. */
    const val EXP_LEEWAY_SECS = 60L

    /** Google id_token is valid ~1h: require a fresh sign-in before sending. */
    class SessionExpiredException :
        Exception("Google session expired (id_token old) — reopen and sign in again")

    /** Old session files written before ephemeral secret was persisted. */
    class SessionIncompleteException :
        Exception("session file too old — sign in with Google again")

    @Serializable
    data class BundleEnvelope(val v: Int = 1, val auth: Auth)

    @Serializable
    data class Auth(
        @SerialName("ephemeral_pubkey") val ephemeralPubkey: String,
        @SerialName("ephemeral_sig") val ephemeralSig: String,
        @SerialName("max_epoch") val maxEpoch: Long,
        val kind: JwtKind,
    )

    /** Externally-tagged enum: `{"Jwt": {...}}`. */
    @Serializable
    data class JwtKind(val Jwt: JwtAuth)

    @Serializable
    data class JwtAuth(
        val jwt: String,
        val jwks: JsonElement,
        val iss: String,
        val aud: String,
        val randomness: String,
        val salt: String,
    )

    private val json = Json { encodeDefaults = true }

    fun expectedNonce(session: ZkLoginAuth.Session): String = ZkLoginCrypto.computeNonce(
        ZkLoginCrypto.unhex(session.ephemeralPubkeyHex),
        session.maxEpoch,
        ZkLoginCrypto.unhex(session.randomnessHex),
    )

    /**
     * Pure assembler (JVM-testable). [ephemeralSigHex] is the 64-byte
     * Ed25519 signature over the transaction hash as lowercase hex.
     */
    fun bundleJson(session: ZkLoginAuth.Session, ephemeralSigHex: String): String {
        val jwks = try {
            Json.parseToJsonElement(session.jwksJson.ifBlank { "{\"keys\":[]}" }).jsonObject
        } catch (_: Exception) {
            throw SessionIncompleteException()
        }
        val envelope = BundleEnvelope(
            auth = Auth(
                ephemeralPubkey = session.ephemeralPubkeyHex,
                ephemeralSig = ephemeralSigHex,
                maxEpoch = session.maxEpoch,
                kind = JwtKind(
                    Jwt = JwtAuth(
                        jwt = session.idToken,
                        jwks = jwks,
                        iss = session.iss,
                        aud = session.aud,
                        randomness = session.randomnessHex,
                        salt = session.saltHex,
                    ),
                ),
            ),
        )
        return json.encodeToJsonElement(envelope).toString()
    }

    fun ensureFresh(session: ZkLoginAuth.Session, nowSecs: Long) {
        if (session.ephemeralSecretHex.isBlank()) throw SessionIncompleteException()
        if (session.idTokenExpSecs == null) throw SessionExpiredException()
        if (nowSecs > session.idTokenExpSecs + EXP_LEEWAY_SECS) throw SessionExpiredException()
    }

    /**
     * Sign [txHash] with the session's ephemeral secret, assemble the JWT
     * bundle, and fail-closed re-verify the WHOLE bundle locally (JWT RS256
     * against the embedded JWKS, nonce binding, address match) before
     * returning the bytes to submit as the transaction `signature`.
     */
    suspend fun sign(
        session: ZkLoginAuth.Session,
        txHash: ByteArray,
        nowSecs: Long,
    ): ByteArray {
        ensureFresh(session, nowSecs)
        val secret = ZkLoginCrypto.unhex(session.ephemeralSecretHex)
        val pub = ZkLoginCrypto.unhex(session.ephemeralPubkeyHex)
        val sig = KanariCrypto.zkLoginSignEphemeral(secret, txHash)
        val sigOk = KanariCrypto.zkLoginVerifyEphemeral(pub, txHash, sig)
        if (!sigOk) throw IllegalStateException("ephemeral signature failed local check")

        // Mirror node-side verify_zklogin_authenticator fail-closed before emitting.
        val nonce = expectedNonce(session)
        val claims = KanariCrypto.zkLoginVerifyJwt(
            session.idToken, session.jwksJson, session.iss, session.aud, nonce, nowSecs,
        )
        val derived = KanariCrypto.zkLoginDeriveAddress(
            claims.iss, session.aud, claims.sub, ZkLoginCrypto.unhex(session.saltHex),
        )
        require(derived.equals(session.address, ignoreCase = true)) {
            "zkLogin address mismatch: derived $derived != session ${session.address}"
        }
        // Rust owns the bundle JSON (no Kotlin manual serde) — `lib.rs:275`
        return KanariCrypto.zkLoginBuildBundle(
            jwt = session.idToken,
            jwksJson = session.jwksJson,
            iss = session.iss,
            aud = session.aud,
            salt = ZkLoginCrypto.unhex(session.saltHex),
            randomness = ZkLoginCrypto.unhex(session.randomnessHex),
            ephemeralPubkey = pub,
            ephemeralSig = sig,
            maxEpoch = session.maxEpoch,
        )
    }
}