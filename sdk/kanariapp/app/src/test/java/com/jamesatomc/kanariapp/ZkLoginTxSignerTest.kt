package com.jamesatomc.kanariapp.wallet.zklogin

import org.junit.Assert.*
import org.junit.Test
import kotlinx.serialization.json.*

/**
 * Wire-shape tests for the zkLogin transaction bundle — mirrors
 * `KanariTxSignature`/`ZkLoginTxSignature` serde in
 * `kanari-crypto/src/signatures/zk_authenticator.rs`. Pure JVM (no FFI).
 */
class ZkLoginTxSignerTest {

    // Deterministic session mirroring the engine test vectors in kanari-crypto.
    private fun session(
        secretHex: String = "0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f",
        idTokenExpSecs: Long? = 2_000_000_000L,
    ) = ZkLoginAuth.Session(
        iss = "https://accounts.google.com",
        aud = "kanari-test-client",
        sub = "1234",
        address = "0x3bc27fa23ddd2177cb1914572052272ce060182745a576019095c05a70971181",
        saltHex = "0909090909090909090909090909090909090909090909090909090909090909",
        ephemeralPubkeyHex = "197f6b23e16c8532c6abc838facd5ea789be0c76b2920334039bfa8b3d368d61",
        ephemeralSecretHex = secretHex,
        maxEpoch = 1000,
        randomnessHex = "0707070707070707070707070707070707070707070707070707070707070707",
        idToken = "eyJhbGciOiJSUzI1NiIsImtpZCI6ImsxIn0.eyJpc3MiOiJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20iLCJhdWQiOiJjYW5hcmktdGVzdC1jbGllbnQiLCJzdWIiOiIxMjM0Iiwibm9uY2UiOiIxM2FmNDZmYmJmZmY3NWZjMTc4YWFhMzc3ZDU4NTJkNCIsImV4cCI6MjAwMDAwMDAwMH0.placeholder",
        jwksJson = """{"keys":[{"kty":"RSA","kid":"k1","alg":"RS256","n":"abc","e":"AQAB"}]}""",
        obtainedAtSecs = 1_700_000_000L,
        idTokenExpSecs = idTokenExpSecs,
    )

    private val sigHex = "a".repeat(128)

    private fun parseBundle(session: ZkLoginAuth.Session, sig: String = sigHex): JsonObject =
        Json.parseToJsonElement(ZkLoginTxSigner.bundleJson(session, sig)).jsonObject

    @Test
    fun bundle_matches_rust_wire_shape() {
        val root = parseBundle(session())
        assertEquals(1, (root["v"] as JsonPrimitive).int)
        val auth = root["auth"]!!.jsonObject
        assertEquals("197f6b23e16c8532c6abc838facd5ea789be0c76b2920334039bfa8b3d368d61",
            auth["ephemeral_pubkey"]!!.jsonPrimitive.content)
        assertEquals(sigHex, auth["ephemeral_sig"]!!.jsonPrimitive.content)
        assertEquals(1000L, auth["max_epoch"]!!.jsonPrimitive.long)
        // Externally-tagged enum kind, exactly like serde: {"Jwt": {...}}
        val kind = auth["kind"]!!.jsonObject
        val jwt = kind.keys.single()
        assertEquals("Jwt", jwt)
        val payload = kind[jwt]!!.jsonObject
        assertEquals("kanari-test-client", payload["aud"]!!.jsonPrimitive.content)
        assertEquals("https://accounts.google.com", payload["iss"]!!.jsonPrimitive.content)
        assertEquals("0707070707070707070707070707070707070707070707070707070707070707",
            payload["randomness"]!!.jsonPrimitive.content)
        assertEquals("0909090909090909090909090909090909090909090909090909090909090909",
            payload["salt"]!!.jsonPrimitive.content)
        val jwksKeys = payload["jwks"]!!.jsonObject["keys"]!!.jsonArray
        assertEquals(1, jwksKeys.size)
        assertEquals("k1", jwksKeys[0].jsonObject["kid"]!!.jsonPrimitive.content)
    }

    @Test
    fun bundle_hex_is_lowercase_no_prefix() {
        val bundle = parseBundle(session())
        val auth = bundle["auth"]!!.jsonObject
        val signature = auth["ephemeral_sig"]!!.jsonPrimitive.content
        assertEquals(128, signature.length)
        assertFalse(signature.startsWith("0x"))
        assertTrue(signature.all { it in '0'..'9' || it in 'a'..'f' })
    }

    @Test
    fun fresh_session_passes() {
        ZkLoginTxSigner.ensureFresh(session(), nowSecs = 1_700_000_000L)
    }

    @Test
    fun expired_id_token_rejected_with_leeway() {
        // exp = 2_000_000_000, leeway 60 -> rejects after 2_000_000_060.
        try {
            ZkLoginTxSigner.ensureFresh(session(), nowSecs = 2_000_000_100L)
            fail("expected SessionExpiredException")
        } catch (e: ZkLoginTxSigner.SessionExpiredException) {
        }
    }

    @Test
    fun missing_ephemeral_secret_rejected() {
        try {
            ZkLoginTxSigner.ensureFresh(session(secretHex = ""), nowSecs = 1_700_000_000L)
            fail("expected SessionIncompleteException")
        } catch (e: ZkLoginTxSigner.SessionIncompleteException) {
        }
    }

    @Test
    fun expected_nonce_uses_computeNonce_vectors() {
        val s = session()
        assertEquals(
            "1bb486c02deef0a4accf5f7c4ca2e7e54ce18cb13b7fa4e193398ebe1b8580fb",
            ZkLoginTxSigner.expectedNonce(s)
        )
    }
}