package com.jamesatomc.kanariapp.wallet.zklogin

import android.annotation.SuppressLint
import android.content.Context
import androidx.credentials.CredentialManager
import androidx.credentials.GetCredentialRequest
import androidx.credentials.exceptions.GetCredentialCancellationException
import androidx.credentials.exceptions.NoCredentialException
import com.google.android.libraries.identity.googleid.GetGoogleIdOption
import com.google.android.libraries.identity.googleid.GoogleIdTokenCredential
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.io.File
import java.net.HttpURLConnection
import java.net.URL

/**
 * Google OIDC login bound to an ephemeral key — native Android flow.
 *
 * The old browser + loopback OAuth flow is DELETED: Google rejects it on
 * Android with `400 invalid_request` (Web clients cannot use loopback
 * redirects from an app). This uses Credential Manager + Google Sign-In:
 * the OS returns an `id_token` directly, no browser, no redirect server,
 * no client secret. The `nonce` we pass in comes back inside the JWT, so
 * the ephemeral binding is preserved.
 *
 * Crypto (nonce, JWT verify, address) runs in Rust via the local
 * `kanari-kotlin` FFI (`KanariCrypto.zkLogin*`) — same vectors as the
 * chain, so app and node can never drift. This file owns only the
 * Android glue: credential request, JWKS fetch, session files.
 *
 * 1. Ephemeral key + randomness + nonce (Rust FFI; the address-salt is
 *    the kanari-crypto standard, derived after the JWT is known).
 * 2. Credential Manager Google Sign-In with our nonce (OS bottom sheet).
 * 3. Google JWKS fetch + JWT verify + nonce binding (fail-closed, Rust).
 * 4. Address derivation (Rust) + private session file.
 *
 * Requires an **Android-type** OAuth client ID (package + SHA-1 bound) as
 * the "serverClientId": that is the **Web** client ID of the same project.
 * Google mints the JWT with `aud = serverClientId`.
 */
object ZkLoginAuth {

    const val GOOGLE_JWKS_URL = "https://www.googleapis.com/oauth2/v3/certs"

    // WEB client ID of this project (becomes the JWT `aud`). Do NOT put the
    // Android-type client ID here: Credential Manager needs the Web ID as
    // `serverClientId`, while the OS authorizes the caller against the
    // sibling Android-type client (package + SHA-1) in the SAME project.
    const val DEFAULT_CLIENT_ID =
        "1041694414791-4e9tgsn3ai7n0bu7296jbm68hsfftjee.apps.googleusercontent.com"

    @Serializable
    data class Session(
        val version: Int = 1,
        val provider: String = "google",
        val iss: String,
        val aud: String,
        val sub: String,
        val address: String,
        val saltHex: String,
        val ephemeralPubkeyHex: String,
        // 32-byte ephemeral Ed25519 SECRET. Needed only to sign transaction
        // hashes; never leaves this session file (app-private storage).
        val ephemeralSecretHex: String = "",
        val maxEpoch: Long,
        val randomnessHex: String,
        val idToken: String,
        // Provider JWKS at login time, embedded in every zkLogin bundle so
        // the chain can verify the JWT without dialing out.
        val jwksJson: String = "",
        val obtainedAtSecs: Long,
        val idTokenExpSecs: Long?,
    )

    private val json = Json { ignoreUnknownKeys = true }

    class AuthException(message: String) : Exception(message)

    /** User dismissed the Google sheet. Callers map this to "canceled", not an error. */
    class CancelledException : Exception("sign-in cancelled")

    data class LoginResult(val session: Session, val sessionFile: File)

    /**
     * Full login via the OS Google sheet. Must be called from a coroutine
     * (suspends on the credential request + network + crypto).
     *
     * The "nonce" binding the ephemeral key travels inside the Google
     * request and comes back as the JWT `nonce` claim; Rust re-checks it
     * fail-closed, so a token minted for another session can never pass.
     */
    suspend fun login(
        context: Context,
        clientId: String = DEFAULT_CLIENT_ID,
        maxEpoch: Long = 1000,
    ): LoginResult {
        // Drop stale non-standard salt storage from older versions: the
        // standard salt needs nothing stored, so this file must not exist.
        purgeLegacySaltFiles(context)
        val ffi = com.kanari.kanari_crypto.KanariCrypto
        val prep = ffi.zkLoginPrepareNonce(maxEpoch)
        val ephemeralPub = prep.ephemeralPubkey
        val randomness = prep.randomness
        val nonce = prep.nonce

        val idToken = requestGoogleIdToken(context, clientId, nonce)
        val jwksJson = httpGet(GOOGLE_JWKS_URL)
        val nowSecs = System.currentTimeMillis() / 1000
        // Nonce bound inside Rust (fail-closed); no separate check here.
        val claims =
            ffi.zkLoginVerifyJwt(idToken, jwksJson, ZkLoginCrypto.ISS_GOOGLE, clientId, nonce, nowSecs)
        // Address-salt: ONLY the kanari-crypto standard, deterministic per
        // (iss, aud, sub) — identical to `kanari zklogin` on desktop, stable
        // across reinstalls and devices. Nothing is stored on device (any
        // legacy `_salts.json` was purged above), so there is nothing to
        // lose and the wallet address can never rotate.
        val finalSalt = ffi.zkLoginDeterministicSalt(claims.iss, clientId, claims.sub)
        val finalSaltHex = ZkLoginCrypto.hex(finalSalt)
        val address = ffi.zkLoginDeriveAddress(claims.iss, clientId, claims.sub, finalSalt)
        val session = Session(
            iss = claims.iss,
            aud = clientId,
            sub = claims.sub,
            address = address,
            saltHex = finalSaltHex,
            ephemeralPubkeyHex = ZkLoginCrypto.hex(ephemeralPub),
            ephemeralSecretHex = ZkLoginCrypto.hex(prep.ephemeralSecret),
            maxEpoch = maxEpoch,
            randomnessHex = ZkLoginCrypto.hex(randomness),
            idToken = idToken,
            jwksJson = jwksJson,
            obtainedAtSecs = nowSecs,
            idTokenExpSecs = claims.exp,
        )
        return LoginResult(session, saveSession(context, session))
    }

    /**
     * OS Google sheet -> `id_token` with our [nonce] embedded.
     * Throws [CancelledException] on user dismiss, [AuthException] otherwise.
     */
    @SuppressLint("CredManMutableContext")
    private suspend fun requestGoogleIdToken(
        context: Context,
        serverClientId: String,
        nonce: String,
    ): String {
        val option = GetGoogleIdOption.Builder()
            .setServerClientId(serverClientId)
            // Always let the user choose the Google account. Reusing the
            // previously authorized account makes different email attempts
            // to appear to produce the same wallet address.
            .setFilterByAuthorizedAccounts(false)
            .setNonce(nonce)
            .build()
        val request = GetCredentialRequest.Builder()
            .addCredentialOption(option)
            .build()
        val manager = CredentialManager.create(context.applicationContext)
        val result = try {
            manager.getCredential(context, request)
        } catch (_: GetCredentialCancellationException) {
            throw CancelledException()
        } catch (_: NoCredentialException) {
            throw AuthException(
                "no Google account on this device (add one in Settings > Accounts)"
            )
        } catch (e: Exception) {
            throw AuthException(friendlyCredentialError(context, e))
        }
        val google = try {
            GoogleIdTokenCredential.createFrom(result.credential.data)
        } catch (e: Exception) {
            throw AuthException("bad credential response: ${e.message}")
        }
        if (google.idToken.isEmpty()) throw AuthException("empty id_token from Google")
        return google.idToken
    }

    /**
     * Translate Play-services status codes into the console fix that
     * actually unblocks the user. 28444 = the OS found no Android-type
     * OAuth client matching (package, SHA-1) of THIS app, so we print
     * both values for copy-paste into the console.
     */
    private fun friendlyCredentialError(context: Context, e: Exception): String {
        val msg = e.message ?: e.javaClass.simpleName
        if (msg.contains("28444")) {
            val (pkg, sha1) = appIdentity(context)
            return "Google console is not set up for this app. Create an " +
                    "Android-type OAuth client with package=$pkg SHA-1=$sha1 " +
                    "in the SAME project as the Web client, then retry. ($msg)"
        }
        return "credential request failed: $msg"
    }

    /** Package + SHA-1 of the actually-running app (what Google checks). */
    fun appIdentity(context: Context): Pair<String, String> {
        val pkg = context.packageName

        @Suppress("DEPRECATION")
        val sigs = if (android.os.Build.VERSION.SDK_INT >= 28) {
            context.packageManager
                .getPackageInfo(pkg, android.content.pm.PackageManager.GET_SIGNING_CERTIFICATES)
                .signingInfo?.apkContentsSigners
        } else {
            @Suppress("DEPRECATION")
            context.packageManager
                .getPackageInfo(pkg, android.content.pm.PackageManager.GET_SIGNATURES)
                .signatures
        } ?: emptyArray()
        val sha1 = sigs.firstOrNull()?.toByteArray()?.let { cert ->
            val digest =
                java.security.MessageDigest.getInstance("SHA-1").digest(cert)
            digest.joinToString(":") { "%02X".format(it) }
        } ?: "<unknown>"
        return pkg to sha1
    }

    /** Max JWKS payload we buffer (Google answers in KiBs; same cap as the CLI). */
    internal const val MAX_HTTP_BYTES: Int = 256 * 1024

    private suspend fun httpGet(url: String): String = withContext(Dispatchers.IO) {
        httpGetCapped(url, MAX_HTTP_BYTES)
    }

    /**
     * Capped GET: a lying Content-Length (or chunked flood) must not OOM
     * the app — the streaming cap below is the real guard, checked on
     * every chunk, not just the header.
     */
    internal fun httpGetCapped(url: String, maxBytes: Int): String {
        val conn = URL(url).openConnection() as HttpURLConnection
        try {
            conn.connectTimeout = 15000
            conn.readTimeout = 15000
            conn.requestMethod = "GET"
            val code = conn.responseCode
            if (code !in 200..299) throw AuthException("GET $url failed: HTTP $code")
            val declared = conn.getHeaderField("Content-Length")?.toLongOrNull()
            if (declared != null && declared > maxBytes) {
                throw AuthException("GET $url failed: response too large ($declared bytes)")
            }
            val out = java.io.ByteArrayOutputStream()
            val buf = ByteArray(8192)
            var total = 0
            try {
                while (true) {
                    val n = conn.inputStream.read(buf)
                    if (n < 0) break
                    total += n
                    if (total > maxBytes) {
                        throw AuthException("GET $url failed: response too large (>${maxBytes} bytes)")
                    }
                    out.write(buf, 0, n)
                }
            } catch (e: AuthException) {
                throw e
            } catch (e: java.io.IOException) {
                // Truncated stream, reset, timeout: never leak raw transport
                // errors (or half-bodies) to login logic — fail closed.
                throw AuthException("GET $url failed: ${e.message}")
            }
            return out.toString(Charsets.UTF_8)
        } finally {
            conn.disconnect()
        }
    }

    // --- session files (app-private storage = owner-only by sandbox) ---

    fun sessionDir(context: Context): File =
        File(context.filesDir, "zklogin").apply { mkdirs() }

    fun sessionFile(context: Context, address: String): File {
        val safe = address.filter { it.isDigit() || it in 'a'..'f' || it in 'A'..'F' }
        require(safe.isNotEmpty()) { "invalid session address" }
        return File(sessionDir(context), "$safe.json")
    }

    /**
     * Atomic write (tmp + rename): a kill mid-write must never leave a torn
     * session file behind, otherwise the next login fails on corrupt JSON.
     */
    private fun atomicWriteText(file: File, text: String) {
        val tmp = File(file.parent, "${file.name}.tmp")
        tmp.writeText(text)
        if (!tmp.renameTo(file)) {
            // Same-filesystem rename should not fail; fall back to direct
            // write rather than losing the session.
            file.writeText(text)
            try {
                tmp.delete()
            } catch (_: Exception) {
            }
        }
    }

    fun saveSession(context: Context, session: Session): File {
        val file = sessionFile(context, session.address)
        atomicWriteText(file, json.encodeToString(Session.serializer(), session))
        return file
    }

    fun loadSession(context: Context, address: String): Session {
        val file = sessionFile(context, address)
        if (!file.exists()) throw AuthException("no session for $address")
        try {
            return json.decodeFromString(Session.serializer(), file.readText())
        } catch (_: Exception) {
            throw AuthException("session file is corrupt — sign in with Google again")
        }
    }

    // NOTE: no salt is persisted on device. The address-salt is always the
    // kanari-crypto standard (see `login`). Session files hold only the
    // ephemeral key, the JWT, and the (re-derivable) saltHex the tx bundle
    // needs — never a non-standard salt.
    //
    // One-time cleanup: `_salts.json` (+ tmp leftovers) from older versions
    // is never read anymore, so delete it outright — no non-standard salt
    // may linger on disk. Best-effort: must never fail a login.
    private fun purgeLegacySaltFiles(context: Context) {
        try {
            sessionDir(context).listFiles()
                ?.filter { it.name == "_salts.json" || it.name.startsWith("_salts.json.") }
                ?.forEach {
                    try {
                        it.delete()
                    } catch (_: Exception) {
                    }
                }
        } catch (_: Exception) {
        }
    }
}
