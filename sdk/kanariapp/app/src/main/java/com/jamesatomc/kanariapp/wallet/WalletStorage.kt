package com.jamesatomc.kanariapp.wallet

import android.content.Context
import android.util.Base64
import android.util.Log
import androidx.datastore.core.DataStore
import androidx.datastore.core.handlers.ReplaceFileCorruptionHandler
import androidx.datastore.preferences.core.*
import androidx.datastore.preferences.preferencesDataStore
import com.google.crypto.tink.Aead
import com.google.crypto.tink.KeyTemplates
import com.google.crypto.tink.RegistryConfiguration
import com.google.crypto.tink.aead.AeadConfig
import com.google.crypto.tink.integration.android.AndroidKeysetManager
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.SecretKeyFactory
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.PBEKeySpec
import javax.crypto.spec.SecretKeySpec
import androidx.core.content.edit

@Serializable
data class EncryptedData(
    val salt: String,
    val nonce: String,
    val cipherText: String,
    val mac: String,
    val iterations: Int,
    val algorithm: String = "aes_gcm_256_pbkdf2_sha256"
)

@Serializable
data class WalletRecord(
    val id: String,
    val name: String,
    val address: String,
    val curveType: String,
    val privateKeyEncrypted: EncryptedData? = null,
    val mnemonicEncrypted: EncryptedData? = null,
    val encryption: String = "pin_aes_gcm_pbkdf2_v1"
)

// A torn write (process kill, full disk, restore) leaves preferences_pb
// unparseable protobuf. Without a handler every read/edit throws
// "Protocol message end-group tag did not match expected tag" and bricks
// wallet save/load. Reset to empty prefs instead (same as a data clear).
private val Context.dataStore: DataStore<Preferences> by preferencesDataStore(
    name = "kanari_storage",
    corruptionHandler = ReplaceFileCorruptionHandler { emptyPreferences() },
)

class WalletStorage(private val context: Context) {

    private val json = Json { ignoreUnknownKeys = true }
    private val biometricPrefs = context.getSharedPreferences(
        "kanari_biometric",
        Context.MODE_PRIVATE,
    )

    private val aead: Aead by lazy {
        AeadConfig.register()
        buildAeadWithRecovery()
    }

    /**
     * Durably deletes the Tink keyset file. MUST use synchronous `commit()`:
     * `edit { clear() }` uses async `apply()`, which loses the race with the
     * immediate rebuild below — the corrupt bytes are still on disk when the
     * rebuild reads them, so recovery never takes effect (and `lazy`
     * memoizes the failure for the rest of the process).
     */
    private fun durableClearKeyset() {
        try {
            val cleared = context
                .getSharedPreferences("kanari_keyset", Context.MODE_PRIVATE)
                .edit().clear().commit()
            if (!cleared) Log.e("WalletStorage", "Keyset clear commit returned false")
        } catch (e: Exception) {
            Log.e("WalletStorage", "Keyset clear failed: ${e.message}")
        }
    }

    private fun buildAeadWithRecovery(): Aead {
        try {
            return buildAead()
        } catch (e: Exception) {
            Log.e("WalletStorage", "Tink init failed: ${e.message}. Clearing keyset and rebuilding.")
        }
        durableClearKeyset()
        try {
            return buildAead()
        } catch (e2: Exception) {
            Log.e("WalletStorage", "Rebuild failed: ${e2.message}. Falling back to non-keystore backed AEAD.")
        }
        // Final fallback: use a fresh keyset namespace if the old keyset is
        // unreadable. This keeps biometric PINs encrypted without reusing the
        // corrupted protobuf entry.
        return AndroidKeysetManager.Builder()
            .withSharedPref(context, "kanari_keyset_recovery", "kanari_master_key")
            .withKeyTemplate(KeyTemplates.get("AES256_GCM"))
            .build()
            .keysetHandle
            .getPrimitive(RegistryConfiguration.get(), Aead::class.java)
    }

    private fun buildAead(): Aead {
        return AndroidKeysetManager.Builder()
            .withSharedPref(context, "kanari_keyset", "kanari_master_key")
            .withKeyTemplate(KeyTemplates.get("AES256_GCM"))
            .withMasterKeyUri("android-keystore://kanari_master_key_alias")
            .build()
            .keysetHandle
            .getPrimitive(RegistryConfiguration.get(), Aead::class.java)
    }

    companion object {
        // Marker for wallet lists saved WITHOUT the Tink envelope (see
        // saveWallets). Base64 (the Tink encoding) never contains ':', so
        // this cannot collide with an encrypted blob.
        private const val PLAIN_PREFIX = "plain1:"
        private val KEY_WALLETS = stringPreferencesKey("kanari_wallets")
        private val KEY_PIN_SALT = stringPreferencesKey("kanari_pin_salt")
        private val KEY_PIN_VERIFIER = stringPreferencesKey("kanari_pin_verifier")
        private val KEY_PIN_ITERATIONS = intPreferencesKey("kanari_pin_iterations")
        private const val KEY_BIOMETRIC_ENABLED_NAME = "kanari_biometric_enabled"
        private const val KEY_BIOMETRIC_PIN_NAME = "kanari_biometric_pin"
        private const val PIN_LENGTH = 6
        private const val KDF_ITERATIONS = 10000
    }

    private suspend fun encryptSecure(data: String): String = withContext(Dispatchers.Default) {
        try {
            Base64.encodeToString(aead.encrypt(data.toByteArray(), null), Base64.NO_WRAP)
        } catch (e: Exception) {
            // The lazy AEAD may hold a poisoned instance built while the
            // keyset file was corrupt. Durably clear + rebuild once and retry,
            // so recovery works without an app restart.
            Log.e("WalletStorage", "Encrypt failed, rebuilding AEAD once: ${e.message}")
            durableClearKeyset()
            try {
                Base64.encodeToString(
                    buildAeadWithRecovery().encrypt(data.toByteArray(), null),
                    Base64.NO_WRAP,
                )
            } catch (e2: Exception) {
                throw Exception("Secure storage encrypt failed: ${e2.message ?: e2.javaClass.simpleName}")
            }
        }
    }

    private suspend fun decryptSecure(encrypted: String): String = withContext(Dispatchers.Default) {
        String(aead.decrypt(Base64.decode(encrypted, Base64.NO_WRAP), null))
    }

    suspend fun hasPin(): Boolean = withContext(Dispatchers.IO) {
        context.dataStore.data.map { it.contains(KEY_PIN_VERIFIER) }.first()
    }

    /** True when any wallet holds PIN-encrypted secrets (private key / mnemonic). */
    suspend fun hasSecrets(): Boolean = withContext(Dispatchers.IO) {
        loadWallets().any { it.privateKeyEncrypted != null || it.mnemonicEncrypted != null }
    }

    suspend fun savePin(pin: String) = withContext(Dispatchers.Default) {
        require(pin.length == PIN_LENGTH) { "PIN must be $PIN_LENGTH digits" }
        val salt = ByteArray(16).apply { SecureRandom().nextBytes(this) }
        val iterations = KDF_ITERATIONS
        val verifier = deriveKey(pin, salt, iterations)

        context.dataStore.edit { prefs ->
            prefs[KEY_PIN_SALT] = Base64.encodeToString(salt, Base64.NO_WRAP)
            prefs[KEY_PIN_VERIFIER] = Base64.encodeToString(verifier, Base64.NO_WRAP)
            prefs[KEY_PIN_ITERATIONS] = iterations
        }
    }

    suspend fun verifyPin(pin: String): Boolean = withContext(Dispatchers.Default) {
        val prefs = context.dataStore.data.first()
        val saltBase64 = prefs[KEY_PIN_SALT] ?: return@withContext false
        val verifierBase64 = prefs[KEY_PIN_VERIFIER] ?: return@withContext false
        // Default to 100,000 for legacy PINs created before migration
        val iterations = prefs[KEY_PIN_ITERATIONS] ?: 100000

        val salt = Base64.decode(saltBase64, Base64.NO_WRAP)
        val verifier = Base64.decode(verifierBase64, Base64.NO_WRAP)

        val candidate = deriveKey(pin, salt, iterations)
        candidate.contentEquals(verifier)
    }

    suspend fun isBiometricEnabled(): Boolean = withContext(Dispatchers.IO) {
        biometricPrefs.getBoolean(KEY_BIOMETRIC_ENABLED_NAME, false)
    }

    suspend fun setBiometricEnabled(enabled: Boolean) = withContext(Dispatchers.IO) {
        biometricPrefs.edit().apply {
            putBoolean(KEY_BIOMETRIC_ENABLED_NAME, enabled)
            if (!enabled) remove(KEY_BIOMETRIC_PIN_NAME)
        }.commit()
    }

    suspend fun saveBiometricPin(pin: String) = withContext(Dispatchers.Default) {
        require(pin.length == PIN_LENGTH) { "PIN must be $PIN_LENGTH digits" }
        val encrypted = encryptSecure(pin)
        biometricPrefs.edit()
            .putString(KEY_BIOMETRIC_PIN_NAME, encrypted)
            .putBoolean(KEY_BIOMETRIC_ENABLED_NAME, true)
            .commit()
    }

    suspend fun getBiometricPin(): String? = withContext(Dispatchers.Default) {
        if (!isBiometricEnabled()) return@withContext null
        val encrypted = biometricPrefs.getString(KEY_BIOMETRIC_PIN_NAME, null)
            ?: return@withContext null
        try {
            decryptSecure(encrypted)
        } catch (_: Exception) {
            null
        }
    }

    suspend fun clearBiometricPin() = withContext(Dispatchers.IO) {
        biometricPrefs.edit()
            .remove(KEY_BIOMETRIC_PIN_NAME)
            .remove(KEY_BIOMETRIC_ENABLED_NAME)
            .commit()
    }

    suspend fun saveWallets(wallets: List<WalletRecord>) = withContext(Dispatchers.Default) {
        val data = json.encodeToString(wallets)
        // The Tink envelope is privacy-only: private keys/mnemonics inside
        // the list stay PIN-encrypted (PBKDF2, independent of Tink), and
        // addresses are public on-chain by design. If Tink/Keystore is broken
        // on this device, persist with an explicit plaintext marker rather
        // than bricking the wallet: availability beats envelope privacy.
        val stored = try {
            encryptSecure(data)
        } catch (e: Exception) {
            Log.w("WalletStorage", "Tink envelope unavailable, saving with plaintext marker: ${e.message}")
            PLAIN_PREFIX + data
        }
        try {
            context.dataStore.edit { prefs ->
                prefs[KEY_WALLETS] = stored
            }
        } catch (e: Exception) {
            throw Exception("Cannot save wallet: ${e.message ?: e.javaClass.simpleName}")
        }
    }

    suspend fun loadWallets(): List<WalletRecord> = withContext(Dispatchers.Default) {
        val stored = context.dataStore.data.first()[KEY_WALLETS]
            ?: return@withContext emptyList()
        try {
            val data = if (stored.startsWith(PLAIN_PREFIX)) {
                stored.removePrefix(PLAIN_PREFIX)
            } else {
                decryptSecure(stored)
            }
            json.decodeFromString(data)
        } catch (e: Exception) {
            Log.e("WalletStorage", "Wallet list unreadable: ${e.message}")
            throw IllegalStateException("Wallet storage is unreadable", e)
        }
    }

    private fun deriveKey(pin: String, salt: ByteArray, iterations: Int = KDF_ITERATIONS): ByteArray {
        val spec = PBEKeySpec(pin.toCharArray(), salt, iterations, 256)
        val factory = SecretKeyFactory.getInstance("PBKDF2WithHmacSHA256")
        return factory.generateSecret(spec).encoded
    }

    suspend fun encrypt(data: String, pin: String): EncryptedData = withContext(Dispatchers.Default) {
        val salt = ByteArray(16).apply { SecureRandom().nextBytes(this) }
        val iterations = KDF_ITERATIONS
        val keyBytes = deriveKey(pin, salt, iterations)
        val key = SecretKeySpec(keyBytes, "AES")

        val nonce = ByteArray(12).apply { SecureRandom().nextBytes(this) }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key, GCMParameterSpec(128, nonce))

        val encryptedBytes = cipher.doFinal(data.toByteArray())

        // Split cipherText and MAC (Android's AES/GCM includes MAC at the end)
        val cipherText = encryptedBytes.copyOfRange(0, encryptedBytes.size - 16)
        val mac = encryptedBytes.copyOfRange(encryptedBytes.size - 16, encryptedBytes.size)

        EncryptedData(
            salt = Base64.encodeToString(salt, Base64.NO_WRAP),
            nonce = Base64.encodeToString(nonce, Base64.NO_WRAP),
            cipherText = Base64.encodeToString(cipherText, Base64.NO_WRAP),
            mac = Base64.encodeToString(mac, Base64.NO_WRAP),
            iterations = iterations
        )
    }

    suspend fun decrypt(encryptedData: EncryptedData, pin: String): String = withContext(Dispatchers.Default) {
        val salt = Base64.decode(encryptedData.salt, Base64.NO_WRAP)
        val keyBytes = deriveKey(pin, salt, encryptedData.iterations)
        val key = SecretKeySpec(keyBytes, "AES")

        val nonce = Base64.decode(encryptedData.nonce, Base64.NO_WRAP)
        val cipherText = Base64.decode(encryptedData.cipherText, Base64.NO_WRAP)
        val mac = Base64.decode(encryptedData.mac, Base64.NO_WRAP)

        val combined = cipherText + mac
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(128, nonce))

        String(cipher.doFinal(combined))
    }
}
