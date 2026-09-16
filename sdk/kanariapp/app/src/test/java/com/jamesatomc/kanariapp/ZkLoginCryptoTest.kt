package com.jamesatomc.kanariapp.wallet.zklogin

import org.junit.Assert.*
import org.junit.Test
import java.math.BigInteger
import java.security.KeyPairGenerator
import java.security.Signature
import java.security.interfaces.RSAPublicKey

class ZkLoginCryptoTest {

    // Cross-language vectors from kanari-crypto (must never drift).
    private val ephPub = ZkLoginCrypto.unhex("197f6b23e16c8532c6abc838facd5ea789be0c76b2920334039bfa8b3d368d61")
    private val rand32 = ZkLoginCrypto.unhex("0707070707070707070707070707070707070707070707070707070707070707")
    private val salt = ZkLoginCrypto.unhex("0909090909090909090909090909090909090909090909090909090909090909")

    @Test
    fun nonce_matches_rust_vector() {
        assertEquals(
            "1bb486c02deef0a4accf5f7c4ca2e7e54ce18cb13b7fa4e193398ebe1b8580fb",
            ZkLoginCrypto.computeNonce(ephPub, 1000L, rand32)
        )
    }

    @Test
    fun address_v2_matches_rust_vector() {
        assertEquals(
            "0x3bc27fa23ddd2177cb1914572052272ce060182745a576019095c05a70971181",
            ZkLoginCrypto.deriveAddressV2("https://accounts.google.com", "kanari-test-client", "1234", salt)
        )
    }

    @Test
    fun address_v2_rejects_oversize() {
        try {
            ZkLoginCrypto.deriveAddressV2("https://accounts.google.com", "x".repeat(97), "b", salt)
            fail("expected oversize aud to fail")
        } catch (_: IllegalArgumentException) {
        }
    }

    @Test
    fun pkce_matches_rfc7636_vector() {
        assertEquals(
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
            ZkLoginCrypto.pkceChallenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk")
        )
    }

    @Test
    fun base64url_roundtrip() {
        val raw = ByteArray(64) { it.toByte() }
        assertArrayEquals(raw, ZkLoginCrypto.b64UrlDecode(ZkLoginCrypto.b64UrlEncodeNoPad(raw)))
        assertEquals("Zg", ZkLoginCrypto.b64UrlEncodeNoPad("f".toByteArray()))
    }

    private fun rsaKey(): java.security.KeyPair {
        val gen = KeyPairGenerator.getInstance("RSA")
        gen.initialize(2048)
        return gen.generateKeyPair()
    }

    private fun mintJwt(private: java.security.PrivateKey, kid: String, payload: String): String {
        val header = ZkLoginCrypto.b64UrlEncodeNoPad(
            """{"alg":"RS256","kid":"$kid"}""".toByteArray()
        )
        val body = ZkLoginCrypto.b64UrlEncodeNoPad(payload.toByteArray())
        val signer = Signature.getInstance("SHA256withRSA")
        signer.initSign(private)
        signer.update("$header.$body".toByteArray(Charsets.US_ASCII))
        return "$header.$body." + ZkLoginCrypto.b64UrlEncodeNoPad(signer.sign())
    }

    private fun jwksJson(pub: RSAPublicKey, kid: String): String {
        fun b64(n: BigInteger): String {
            var b = n.toByteArray()
            if (b.isNotEmpty() && b[0] == 0.toByte()) b = b.copyOfRange(1, b.size)
            return ZkLoginCrypto.b64UrlEncodeNoPad(b)
        }
        return """{"keys":[{"kty":"RSA","kid":"$kid","alg":"RS256","n":"${b64(pub.modulus)}","e":"${b64(pub.publicExponent)}"}]}"""
    }

    @Test
    fun jwt_verify_accepts_and_rejects() {
        val kp = rsaKey()
        val pub = kp.public as RSAPublicKey
        val payload =
            """{"iss":"https://accounts.google.com","aud":"cid","sub":"u1","exp":2000000000,"nonce":"n1"}"""
        val jwt = mintJwt(kp.private, "k1", payload)
        val jwks = jwksJson(pub, "k1")

        val claims = ZkLoginCrypto.verifyJwt(jwt, jwks, "https://accounts.google.com", "cid", 1_700_000_000L)
        assertEquals("u1", claims.sub)
        assertEquals("n1", claims.nonce)

        // Wrong audience fails closed.
        try {
            ZkLoginCrypto.verifyJwt(jwt, jwks, "https://accounts.google.com", "other", 1_700_000_000L)
            fail("expected aud mismatch")
        } catch (e: ZkLoginCrypto.JwtError) {
            assertTrue(e.message!!.contains("aud"))
        }
        // Tampered payload fails closed.
        val tampered = jwt.split(".").let { "${it[0]}.${it[1].dropLast(2)}AA.${it[2]}" }
        try {
            ZkLoginCrypto.verifyJwt(tampered, jwks, "https://accounts.google.com", "cid", 1_700_000_000L)
            fail("expected sig failure")
        } catch (e: ZkLoginCrypto.JwtError) {
            assertTrue(e.message!!.contains("signature"))
        }
    }
}
