package com.jamesatomc.kanariapp.wallet.zklogin

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.security.KeyFactory
import java.security.MessageDigest
import java.security.SecureRandom
import java.security.Signature
import java.security.spec.RSAPublicKeySpec

/**
 * Pure zkLogin cryptography — no Android dependencies (unit-testable on JVM).
 *
 * Mirrors `kanari-crypto/src/signatures/zklogin.rs` exactly:
 * - nonce = hex(SHA256("zkLogin-nonce" || pubkey || max_epoch_le64 || randomness))
 *
 * NOTE: there is deliberately NO address-derivation helper here. The only
 * accepted address-salt is the kanari-crypto standard (`deterministic_salt`,
 * via `KanariCrypto.zkLoginDeterministicSalt` + `zkLoginDeriveAddress` FFI);
 * a Kotlin helper taking an arbitrary salt would reopen the door to
 * non-standard addresses, so it was removed.
 */
object ZkLoginCrypto {

    const val ISS_GOOGLE = "https://accounts.google.com"

    private val json = Json { ignoreUnknownKeys = true }

    fun sha256(data: ByteArray): ByteArray =
        MessageDigest.getInstance("SHA-256").digest(data)

    fun randomBytes(n: Int): ByteArray {
        val out = ByteArray(n)
        SecureRandom().nextBytes(out)
        return out
    }

    // --- base64url (manual: no API-26 java.util.Base64, no android.util in unit tests) ---

    private const val ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"

    fun b64UrlEncodeNoPad(bytes: ByteArray): String {
        val sb = StringBuilder((bytes.size * 4 + 2) / 3)
        var i = 0
        while (i < bytes.size) {
            val b0 = bytes[i].toInt() and 0xFF
            val b1 = if (i + 1 < bytes.size) bytes[i + 1].toInt() and 0xFF else 0
            val b2 = if (i + 2 < bytes.size) bytes[i + 2].toInt() and 0xFF else 0
            sb.append(ALPHABET[b0 shr 2])
            sb.append(ALPHABET[((b0 and 0x03) shl 4) or (b1 shr 4)])
            if (i + 1 < bytes.size) sb.append(ALPHABET[((b1 and 0x0F) shl 2) or (b2 shr 6)])
            if (i + 2 < bytes.size) sb.append(ALPHABET[b2 and 0x3F])
            i += 3
        }
        return sb.toString()
    }

    fun b64UrlDecode(input: String): ByteArray {
        val s = input.trim()
        require(s.all { it in ALPHABET || it == '=' }) { "not base64url" }
        val clean = s.trimEnd('=')
        val out = ByteArray((clean.length * 6) / 8 + 1)
        var bits = 0
        var acc = 0
        var pos = 0
        for (c in clean) {
            val v = ALPHABET.indexOf(c)
            require(v >= 0) { "not base64url" }
            acc = (acc shl 6) or v
            bits += 6
            if (bits >= 8) {
                bits -= 8
                out[pos++] = ((acc shr bits) and 0xFF).toByte()
            }
        }
        return out.copyOf(pos)
    }

    fun hex(bytes: ByteArray): String {
        val h = "0123456789abcdef"
        val sb = StringBuilder(bytes.size * 2)
        for (b in bytes) {
            sb.append(h[(b.toInt() shr 4) and 0xF])
            sb.append(h[b.toInt() and 0xF])
        }
        return sb.toString()
    }

    fun unhex(s: String): ByteArray {
        val t = s.removePrefix("0x")
        require(t.length % 2 == 0 && t.all { it in '0'..'9' || it in 'a'..'f' || it in 'A'..'F' }) {
            "not hex"
        }
        return ByteArray(t.length / 2) { i ->
            ((t[i * 2].digitToInt(16) shl 4) or t[i * 2 + 1].digitToInt(16)).toByte()
        }
    }

    // --- zkLogin primitives ---

    /** PKCE S256 challenge. RFC 7636 test vector compatible. */
    fun pkceChallenge(verifierB64: String): String =
        b64UrlEncodeNoPad(sha256(verifierB64.toByteArray(Charsets.US_ASCII)))

    fun computeNonce(ephemeralPubkey: ByteArray, maxEpoch: Long, randomness: ByteArray): String {
        require(ephemeralPubkey.size == 32) { "pubkey must be 32 bytes" }
        require(randomness.size == 32) { "randomness must be 32 bytes" }
        val preimage = "zkLogin-nonce".toByteArray(Charsets.US_ASCII) +
                ephemeralPubkey +
                maxEpoch.toLeBytes() +
                randomness
        return hex(sha256(preimage))
    }

    private fun Long.toLeBytes(): ByteArray {
        val out = ByteArray(8)
        for (i in 0 until 8) out[i] = ((this shr (8 * i)) and 0xFF).toByte()
        return out
    }

    // --- JWT ---

    @Serializable
    data class JwtHeader(
        val alg: String = "",
        val kid: String? = null,
    )

    @Serializable
    data class JwtClaims(
        val iss: String = "",
        // Google sends a string; some providers send an array.
        val aud: kotlinx.serialization.json.JsonElement? = null,
        val sub: String = "",
        val exp: Long? = null,
        val iat: Long? = null,
        val nbf: Long? = null,
        val nonce: String? = null,
    ) {
        fun audContains(expected: String): Boolean {
            val el = aud ?: return false
            return try {
                if (el is kotlinx.serialization.json.JsonPrimitive && el.isString) {
                    el.content == expected
                } else {
                    el as kotlinx.serialization.json.JsonArray
                    el.any { it is kotlinx.serialization.json.JsonPrimitive && it.content == expected }
                }
            } catch (_: Exception) {
                false
            }
        }
    }

    @Serializable
    data class Jwk(
        val kty: String = "",
        val kid: String? = null,
        val alg: String? = null,
        val n: String = "",
        val e: String = "",
    )

    @Serializable
    data class Jwks(val keys: List<Jwk> = emptyList())

    class JwtError(message: String) : Exception(message)

    fun decodeClaims(jwt: String): JwtClaims {
        val parts = jwt.split(".")
        if (parts.size != 3) throw JwtError("JWT must have 3 parts")
        val payload = try {
            b64UrlDecode(parts[1])
        } catch (_: Exception) {
            throw JwtError("JWT payload is not base64url")
        }
        return try {
            json.decodeFromString(JwtClaims.serializer(), payload.toString(Charsets.UTF_8))
        } catch (_: Exception) {
            throw JwtError("JWT payload is not valid claims")
        }
    }

    /**
     * Full RS256 verification against a JWKS document. Fail-closed, in order:
     * alg == RS256, kid selects a key, signature verifies, iss/aud match,
     * exp fresh (60s leeway), iat/nbf not in the future beyond the leeway
     * (mirrors `verify_claims_timing` in kanari-crypto). Returns claims.
     */
    fun verifyJwt(jwt: String, jwksJson: String, expectedIss: String, expectedAud: String, nowSecs: Long): JwtClaims {
        val parts = jwt.split(".")
        if (parts.size != 3) throw JwtError("JWT must have 3 parts")
        val header: JwtHeader = try {
            json.decodeFromString(
                JwtHeader.serializer(),
                b64UrlDecode(parts[0]).toString(Charsets.UTF_8)
            )
        } catch (_: Exception) {
            throw JwtError("bad JWT header")
        }
        if (header.alg != "RS256") throw JwtError("need RS256, got ${header.alg}")
        val kid = header.kid ?: throw JwtError("JWT header has no kid")
        val jwks = try {
            json.decodeFromString(Jwks.serializer(), jwksJson)
        } catch (_: Exception) {
            throw JwtError("JWKS is not valid JSON")
        }
        val jwk = jwks.keys.firstOrNull { it.kid == kid && it.kty == "RSA" }
            ?: throw JwtError("unknown JWK kid $kid")
        val signingInput = "${parts[0]}.${parts[1]}".toByteArray(Charsets.US_ASCII)
        val sig = try {
            b64UrlDecode(parts[2])
        } catch (_: Exception) {
            throw JwtError("JWT signature is not base64url")
        }
        val pubKey = try {
            val n = java.math.BigInteger(1, b64UrlDecode(jwk.n))
            val e = java.math.BigInteger(1, b64UrlDecode(jwk.e))
            KeyFactory.getInstance("RSA").generatePublic(RSAPublicKeySpec(n, e))
        } catch (_: Exception) {
            throw JwtError("bad RSA JWK components")
        }
        val ok = try {
            val verifier = Signature.getInstance("SHA256withRSA")
            verifier.initVerify(pubKey)
            verifier.update(signingInput)
            verifier.verify(sig)
        } catch (_: Exception) {
            throw JwtError("RSA verification failed")
        }
        if (!ok) throw JwtError("JWT signature invalid")
        val claims = decodeClaims(jwt)
        if (claims.iss != expectedIss) throw JwtError("iss mismatch")
        if (!claims.audContains(expectedAud)) throw JwtError("aud mismatch")
        val exp = claims.exp ?: throw JwtError("JWT has no exp")
        if (nowSecs > exp + 60) throw JwtError("JWT expired")
        // A token issued (or not yet valid) too far in the future is a
        // clock-integrity violation: fail closed, same leeway as Rust.
        val iat = claims.iat
        if (iat != null && iat > nowSecs + 60) throw JwtError("JWT issued in the future")
        val nbf = claims.nbf
        if (nbf != null && nowSecs + 60 < nbf) throw JwtError("JWT not yet valid")
        return claims
    }
}
