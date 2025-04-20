package io.kanari.wallet

import io.kanari.wallet.exceptions.WalletException
import io.kanari.wallet.models.*
import io.kanari.wallet.utils.*

import org.bouncycastle.asn1.x9.X9ECParameters
import org.bouncycastle.crypto.ec.CustomNamedCurves
import org.bouncycastle.crypto.params.ECDomainParameters
import org.bouncycastle.jce.provider.BouncyCastleProvider
import org.bouncycastle.jce.spec.ECParameterSpec
import org.bouncycastle.jce.spec.ECPrivateKeySpec
import org.bouncycastle.jce.spec.ECPublicKeySpec
import org.bouncycastle.math.ec.ECPoint
import org.bouncycastle.util.encoders.Hex

import org.bitcoinj.crypto.MnemonicCode
import org.bitcoinj.crypto.MnemonicException

import java.io.File
import java.security.*
import java.security.spec.PKCS8EncodedKeySpec
import java.security.spec.X509EncodedKeySpec
import javax.crypto.Cipher
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec
import kotlin.random.Random

import kotlinx.serialization.json.Json
import kotlinx.serialization.encodeToString
import kotlinx.serialization.decodeFromString

import de.mkammerer.argon2.Argon2Factory
import org.bouncycastle.crypto.digests.SHA3Digest
import java.nio.charset.StandardCharsets
import java.util.*

// Initialize Bouncy Castle provider
val provider = BouncyCastleProvider().also { Security.addProvider(it) }

/**
 * Main class for wallet key operations
 */
class WalletKey {
    companion object {
        // Constants
        private const val GCM_IV_LENGTH = 12
        private const val GCM_TAG_LENGTH = 128
        private const val SALT_LENGTH = 16
        
        init {
            // Initialize security provider
            Security.addProvider(BouncyCastleProvider())
        }
        
        /**
         * Check if any wallet exists
         */
        fun checkWalletExists(): Boolean {
            return try {
                listWalletFiles().isNotEmpty()
            } catch (e: Exception) {
                false
            }
        }
        
        /**
         * Set the selected wallet address in configuration
         */
        fun setSelectedWallet(walletAddress: String): Boolean {
            return try {
                val config = ConfigManager.loadConfig() ?: mutableMapOf()
                val formattedAddress = walletAddress.removeSuffix(".enc")
                
                config["address"] = formattedAddress
                config["selected_wallet"] = formattedAddress
                
                ConfigManager.saveConfig(config)
                true
            } catch (e: Exception) {
                false
            }
        }
        
        /**
         * Save wallet with encryption
         */
        fun saveWallet(
            address: String,
            privateKey: String,
            seedPhrase: String,
            password: String,
            curveType: CurveType
        ): Boolean {
            return try {
                val wallet = Wallet(
                    address = address,
                    privateKey = privateKey,
                    seedPhrase = seedPhrase,
                    curveType = curveType
                )
                
                // Generate salt and nonce
                val salt = Random.nextBytes(SALT_LENGTH)
                val nonce = Random.nextBytes(GCM_IV_LENGTH)
                
                // Derive key using Argon2
                val key = deriveKey(password, salt)
                
                // Serialize wallet data
                val walletJson = Json.encodeToString(wallet)
                
                // Encrypt the wallet data
                val ciphertext = encryptAesGcm(walletJson.toByteArray(), key, nonce)
                
                val encryptedData = EncryptedData(
                    ciphertext = ciphertext,
                    salt = Hex.toHexString(salt),
                    nonce = nonce
                )
                
                // Prepare directory
                val walletDir = File(ConfigManager.getKariDir(), "wallets")
                if (!walletDir.exists()) {
                    walletDir.mkdirs()
                }
                
                // Save encrypted data
                val walletFile = File(walletDir, "$address.enc")
                val encryptedJson = Json.encodeToString(encryptedData)
                walletFile.writeText(encryptedJson)
                
                true
            } catch (e: Exception) {
                false
            }
        }
        
        /**
         * Load wallet with decryption
         */
        fun loadWallet(address: String, password: String): Wallet {
            val walletDir = File(ConfigManager.getKariDir(), "wallets")
            val walletFile = File(walletDir, "$address.enc")
            
            if (!walletFile.exists()) {
                throw WalletException("Wallet file not found")
            }
            
            val encryptedJson = walletFile.readText()
            val encryptedData = Json.decodeFromString<EncryptedData>(encryptedJson)
            
            val salt = Hex.decode(encryptedData.salt)
            val key = deriveKey(password, salt)
            
            val decrypted = decryptAesGcm(encryptedData.ciphertext, key, encryptedData.nonce)
            val walletJson = String(decrypted, StandardCharsets.UTF_8)
            
            return Json.decodeFromString(walletJson)
        }
        
        /**
         * Generate new wallet address
         */
        fun generateKarixAddress(wordCount: Int, curveType: CurveType): Triple<String, String, String> {
            return when (curveType) {
                CurveType.K256 -> generateK256Address(wordCount)
                CurveType.P256 -> generateP256Address(wordCount)
            }
        }
        
        /**
         * Generate K256 (secp256k1) wallet
         */
        fun generateK256Address(wordCount: Int): Triple<String, String, String> {
            // Setup curve parameters
            val curve = CustomNamedCurves.getByName("secp256k1")
            val domain = ECDomainParameters(curve.curve, curve.g, curve.n, curve.h)
            
            // Generate key pair
            val keyPair = generateECKeyPair("secp256k1")
            
            // Get private key bytes
            val privateKeyBytes = keyPair.private.encoded
            val privateKeyHex = Hex.toHexString(privateKeyBytes)
            
            // Get public key and format address
            val publicKeyPoint = curve.g.multiply(Hex.decode(privateKeyHex).let { 
                BigInteger(1, it) 
            })
            val publicKeyBytes = publicKeyPoint.getEncoded(false)
            val address = "0x" + Hex.toHexString(publicKeyBytes.sliceArray(1..32))
            
            // Generate mnemonic
            val mnemonicWords = generateMnemonic(wordCount)
            
            return Triple(privateKeyHex, address, mnemonicWords)
        }
        
        /**
         * Generate P256 (secp256r1) wallet
         */
        fun generateP256Address(wordCount: Int): Triple<String, String, String> {
            // Setup curve parameters
            val curve = CustomNamedCurves.getByName("secp256r1")
            val domain = ECDomainParameters(curve.curve, curve.g, curve.n, curve.h)
            
            // Generate key pair
            val keyPair = generateECKeyPair("secp256r1")
            
            // Get private key bytes
            val privateKeyBytes = keyPair.private.encoded
            val privateKeyHex = Hex.toHexString(privateKeyBytes)
            
            // Get public key and format address
            val publicKeyPoint = curve.g.multiply(Hex.decode(privateKeyHex).let { 
                BigInteger(1, it) 
            })
            val publicKeyBytes = publicKeyPoint.getEncoded(false)
            val address = "0x" + Hex.toHexString(publicKeyBytes.sliceArray(1..32))
            
            // Generate mnemonic
            val mnemonicWords = generateMnemonic(wordCount)
            
            return Triple(privateKeyHex, address, mnemonicWords)
        }
        
        /**
         * List wallet files with selection status
         */
        fun listWalletFiles(): List<Pair<String, Boolean>> {
            val walletDir = File(ConfigManager.getKariDir(), "wallets")
            if (!walletDir.exists()) {
                walletDir.mkdirs()
                return emptyList()
            }
            
            val selectedWallet = getSelectedWallet() ?: ""
            
            return walletDir.listFiles { file -> file.name.endsWith(".enc") }
                ?.map { file ->
                    val walletName = file.name
                    val isSelected = walletName.removeSuffix(".enc") == selectedWallet
                    Pair(walletName, isSelected)
                }
                ?.sortedBy { it.first }
                ?: emptyList()
        }
        
        /**
         * Import from seed phrase
         */
        fun importFromSeedPhrase(phrase: String, curveType: CurveType): Triple<String, String, String> {
            return when (curveType) {
                CurveType.K256 -> importFromSeedPhraseK256(phrase)
                CurveType.P256 -> importFromSeedPhraseP256(phrase)
            }
        }
        
        /**
         * Import from private key
         */
        fun importFromPrivateKey(privateKey: String, curveType: CurveType): Triple<String, String, String> {
            return when (curveType) {
                CurveType.K256 -> importFromPrivateKeyK256(privateKey)
                CurveType.P256 -> importFromPrivateKeyP256(privateKey)
            }
        }
        
        /**
         * Get selected wallet from config
         */
        fun getSelectedWallet(): String? {
            val config = ConfigManager.loadConfig() ?: return null
            
            return config["selected_wallet"] 
                ?: config["address"]
                ?: return null
        }
        
        /**
         * Sign a message using wallet
         */
        fun signMessage(wallet: Wallet, message: ByteArray, password: String): ByteArray {
            val loadedWallet = loadWallet(wallet.address, password)
            
            return when (loadedWallet.curveType) {
                CurveType.K256 -> signMessageK256(loadedWallet.privateKey, message)
                CurveType.P256 -> signMessageP256(loadedWallet.privateKey, message)
            }
        }
        
        /**
         * Verify a signature
         */
        fun verifySignature(address: String, message: ByteArray, signature: ByteArray): Boolean {
            val addressHex = address.removePrefix("0x")
            
            // Try both curve types
            val k256Result = try {
                verifySignatureK256(addressHex, message, signature)
            } catch (e: Exception) {
                false
            }
            
            val p256Result = try {
                verifySignatureP256(addressHex, message, signature)
            } catch (e: Exception) {
                false
            }
            
            return k256Result || p256Result
        }
        
        /**
         * Detect likely curve type for an address
         */
        fun detectCurveType(address: String): CurveType? {
            val addressHex = address.removePrefix("0x")
            val decoded = try {
                Hex.decode(addressHex)
            } catch (e: Exception) {
                return null
            }
            
            if (decoded.size != 64 && decoded.size != 32) {
                return null
            }
            
            // Try as K256 key
            val k256Valid = isValidK256Point(decoded)
            
            // Try as P256 key
            val p256Valid = isValidP256Point(decoded)
            
            return when {
                k256Valid && !p256Valid -> CurveType.K256
                !k256Valid && p256Valid -> CurveType.P256
                k256Valid && p256Valid -> CurveType.K256 // Default to K256 if both valid
                else -> null
            }
        }
        
        // Private helper methods
        
        private fun deriveKey(password: String, salt: ByteArray): ByteArray {
            val argon2 = Argon2Factory.create()
            val hash = argon2.hash(10, 65536, 1, password.toCharArray(), salt)
            val derivedKey = ByteArray(32)
            System.arraycopy(hash.toByteArray(), 0, derivedKey, 0, 32)
            return derivedKey
        }
        
        private fun encryptAesGcm(data: ByteArray, key: ByteArray, iv: ByteArray): ByteArray {
            val cipher = Cipher.getInstance("AES/GCM/NoPadding", "BC")
            val keySpec = SecretKeySpec(key, "AES")
            val paramSpec = GCMParameterSpec(GCM_TAG_LENGTH, iv)
            
            cipher.init(Cipher.ENCRYPT_MODE, keySpec, paramSpec)
            return cipher.doFinal(data)
        }
        
        private fun decryptAesGcm(ciphertext: ByteArray, key: ByteArray, iv: ByteArray): ByteArray {
            val cipher = Cipher.getInstance("AES/GCM/NoPadding", "BC")
            val keySpec = SecretKeySpec(key, "AES")
            val paramSpec = GCMParameterSpec(GCM_TAG_LENGTH, iv)
            
            cipher.init(Cipher.DECRYPT_MODE, keySpec, paramSpec)
            return cipher.doFinal(ciphertext)
        }
        
        private fun generateMnemonic(wordCount: Int): String {
            require(wordCount == 12 || wordCount == 24) { "Word count must be 12 or 24" }
            
            val entropy = ByteArray(if (wordCount == 12) 16 else 32)
            SecureRandom().nextBytes(entropy)
            
            return try {
                val words = MnemonicCode.INSTANCE.toMnemonic(entropy)
                words.joinToString(" ")
            } catch (e: MnemonicException) {
                throw WalletException("Failed to generate mnemonic: ${e.message}")
            }
        }
        
        private fun generateECKeyPair(curveName: String): KeyPair {
            val ecSpec = ECParameterSpec(
                CustomNamedCurves.getByName(curveName).curve,
                CustomNamedCurves.getByName(curveName).g,
                CustomNamedCurves.getByName(curveName).n,
                CustomNamedCurves.getByName(curveName).h
            )
            
            val keyGen = KeyPairGenerator.getInstance("EC", "BC")
            keyGen.initialize(ecSpec, SecureRandom())
            return keyGen.generateKeyPair()
        }
        
        private fun importFromSeedPhraseK256(phrase: String): Triple<String, String, String> {
            // Validate and process mnemonic
            val words = phrase.split(" ")
            if (words.size != 12 && words.size != 24) {
                throw WalletException("Invalid mnemonic length")
            }
            
            // Generate seed from mnemonic
            val seed = MnemonicCode.INSTANCE.toEntropy(words)
            
            // Use first 32 bytes as private key
            val privateKeyBytes = seed.sliceArray(0 until 32)
            val privateKeyHex = Hex.toHexString(privateKeyBytes)
            
            // Generate address from private key
            val curve = CustomNamedCurves.getByName("secp256k1")
            val publicKeyPoint = curve.g.multiply(BigInteger(1, privateKeyBytes))
            val publicKeyBytes = publicKeyPoint.getEncoded(false)
            val publicKeyHex = Hex.toHexString(publicKeyBytes.sliceArray(1..32))
            val address = "0x$publicKeyHex"
            
            return Triple(privateKeyHex, publicKeyHex, address)
        }
        
        private fun importFromSeedPhraseP256(phrase: String): Triple<String, String, String> {
            // Similar to K256 but with P256 curve
            val words = phrase.split(" ")
            if (words.size != 12 && words.size != 24) {
                throw WalletException("Invalid mnemonic length")
            }
            
            val seed = MnemonicCode.INSTANCE.toEntropy(words)
            val privateKeyBytes = seed.sliceArray(0 until 32)
            val privateKeyHex = Hex.toHexString(privateKeyBytes)
            
            val curve = CustomNamedCurves.getByName("secp256r1")
            val publicKeyPoint = curve.g.multiply(BigInteger(1, privateKeyBytes))
            val publicKeyBytes = publicKeyPoint.getEncoded(false)
            val publicKeyHex = Hex.toHexString(publicKeyBytes.sliceArray(1..32))
            val address = "0x$publicKeyHex"
            
            return Triple(privateKeyHex, publicKeyHex, address)
        }
        
        private fun importFromPrivateKeyK256(privateKey: String): Triple<String, String, String> {
            val privateKeyBytes = Hex.decode(privateKey)
            
            val curve = CustomNamedCurves.getByName("secp256k1")
            val publicKeyPoint = curve.g.multiply(BigInteger(1, privateKeyBytes))
            val publicKeyBytes = publicKeyPoint.getEncoded(false)
            val publicKeyHex = Hex.toHexString(publicKeyBytes.sliceArray(1..32))
            val address = "0x$publicKeyHex"
            
            return Triple(privateKey, publicKeyHex, address)
        }
        
        private fun importFromPrivateKeyP256(privateKey: String): Triple<String, String, String> {
            val privateKeyBytes = Hex.decode(privateKey)
            
            val curve = CustomNamedCurves.getByName("secp256r1")
            val publicKeyPoint = curve.g.multiply(BigInteger(1, privateKeyBytes))
            val publicKeyBytes = publicKeyPoint.getEncoded(false)
            val publicKeyHex = Hex.toHexString(publicKeyBytes.sliceArray(1..32))
            val address = "0x$publicKeyHex"
            
            return Triple(privateKey, publicKeyHex, address)
        }
        
        private fun signMessageK256(privateKeyHex: String, message: ByteArray): ByteArray {
            // Hash the message with SHA3
            val digest = SHA3Digest(256)
            val hash = ByteArray(digest.digestSize)
            digest.update(message, 0, message.size)
            digest.doFinal(hash, 0)
            
            // Sign with K256
            val privateKeyBytes = Hex.decode(privateKeyHex)
            val keySpec = ECPrivateKeySpec(
                BigInteger(1, privateKeyBytes),
                ECParameterSpec(
                    CustomNamedCurves.getByName("secp256k1").curve,
                    CustomNamedCurves.getByName("secp256k1").g,
                    CustomNamedCurves.getByName("secp256k1").n,
                    CustomNamedCurves.getByName("secp256k1").h
                )
            )
            
            val keyFactory = KeyFactory.getInstance("EC", "BC")
            val privateKey = keyFactory.generatePrivate(keySpec)
            
            val signature = Signature.getInstance("SHA3-256withECDSA", "BC")
            signature.initSign(privateKey)
            signature.update(message)
            
            return signature.sign()
        }
        
        private fun signMessageP256(privateKeyHex: String, message: ByteArray): ByteArray {
            // Hash with SHA3
            val digest = SHA3Digest(256)
            val hash = ByteArray(digest.digestSize)
            digest.update(message, 0, message.size)
            digest.doFinal(hash, 0)
            
            // Sign with P256
            val privateKeyBytes = Hex.decode(privateKeyHex)
            val keySpec = ECPrivateKeySpec(
                BigInteger(1, privateKeyBytes),
                ECParameterSpec(
                    CustomNamedCurves.getByName("secp256r1").curve,
                    CustomNamedCurves.getByName("secp256r1").g,
                    CustomNamedCurves.getByName("secp256r1").n,
                    CustomNamedCurves.getByName("secp256r1").h
                )
            )
            
            val keyFactory = KeyFactory.getInstance("EC", "BC")
            val privateKey = keyFactory.generatePrivate(keySpec)
            
            val signature = Signature.getInstance("SHA3-256withECDSA", "BC")
            signature.initSign(privateKey)
            signature.update(message)
            
            return signature.sign()
        }
        
        private fun verifySignatureK256(addressHex: String, message: ByteArray, signature: ByteArray): Boolean {
            // Reconstruct public key from address
            val publicKeyBytes = try {
                reconstructPublicKey(addressHex, "secp256k1")
            } catch (e: Exception) {
                return false
            }
            
            // Hash message with SHA3
            val digest = SHA3Digest(256)
            val hash = ByteArray(digest.digestSize)
            digest.update(message, 0, message.size)
            digest.doFinal(hash, 0)
            
            // Create public key and verify
            try {
                val keySpec = X509EncodedKeySpec(publicKeyBytes)
                val keyFactory = KeyFactory.getInstance("EC", "BC")
                val publicKey = keyFactory.generatePublic(keySpec)
                
                val verifier = Signature.getInstance("SHA3-256withECDSA", "BC")
                verifier.initVerify(publicKey)
                verifier.update(message)
                
                return verifier.verify(signature)
            } catch (e: Exception) {
                return false
            }
        }
        
        private fun verifySignatureP256(addressHex: String, message: ByteArray, signature: ByteArray): Boolean {
            // Similar to K256 but with P256 curve
            val publicKeyBytes = try {
                reconstructPublicKey(addressHex, "secp256r1")
            } catch (e: Exception) {
                return false
            }
            
            val digest = SHA3Digest(256)
            val hash = ByteArray(digest.digestSize)
            digest.update(message, 0, message.size)
            digest.doFinal(hash, 0)
            
            try {
                val keySpec = X509EncodedKeySpec(publicKeyBytes)
                val keyFactory = KeyFactory.getInstance("EC", "BC")
                val publicKey = keyFactory.generatePublic(keySpec)
                
                val verifier = Signature.getInstance("SHA3-256withECDSA", "BC")
                verifier.initVerify(publicKey)
                verifier.update(message)
                
                return verifier.verify(signature)
            } catch (e: Exception) {
                return false
            }
        }
        
        private fun reconstructPublicKey(addressHex: String, curveName: String): ByteArray {
            val decoded = Hex.decode(addressHex)
            val curve = CustomNamedCurves.getByName(curveName)
            
            // Try different formats to reconstruct the key
            if (decoded.size == 64) {
                // Full coordinates available
                val x = BigInteger(1, decoded.sliceArray(0 until 32))
                val y = BigInteger(1, decoded.sliceArray(32 until 64))
                val point = curve.curve.createPoint(x, y)
                return point.getEncoded(false)
            } else if (decoded.size == 32) {
                // Only X coordinate available, try both possible Y values
                val x = BigInteger(1, decoded)
                
                // Try even Y coordinate first
                try {
                    val pointEven = curve.curve.createPoint(x, BigInteger.ZERO)
                    return pointEven.getEncoded(false)
                } catch (e: Exception) {
                    // Try odd Y coordinate
                    try {
                        val pointOdd = curve.curve.createPoint(x, BigInteger.ONE)
                        return pointOdd.getEncoded(false)
                    } catch (e: Exception) {
                        throw IllegalArgumentException("Cannot reconstruct point from X coordinate")
                    }
                }
            }
            
            throw IllegalArgumentException("Invalid address format")
        }
        
        private fun isValidK256Point(decoded: ByteArray): Boolean {
            val curve = CustomNamedCurves.getByName("secp256k1")
            
            return try {
                if (decoded.size == 64) {
                    val x = BigInteger(1, decoded.sliceArray(0 until 32))
                    val y = BigInteger(1, decoded.sliceArray(32 until 64))
                    curve.curve.createPoint(x, y)
                    true
                } else if (decoded.size == 32) {
                    val x = BigInteger(1, decoded)
                    // Try both possible Y values
                    try {
                        curve.curve.createPoint(x, BigInteger.ZERO)
                        true
                    } catch (e: Exception) {
                        try {
                            curve.curve.createPoint(x, BigInteger.ONE)
                            true
                        } catch (e: Exception) {
                            false
                        }
                    }
                } else {
                    false
                }
            } catch (e: Exception) {
                false
            }
        }
        
        private fun isValidP256Point(decoded: ByteArray): Boolean {
            val curve = CustomNamedCurves.getByName("secp256r1")
            
            return try {
                if (decoded.size == 64) {
                    val x = BigInteger(1, decoded.sliceArray(0 until 32))
                    val y = BigInteger(1, decoded.sliceArray(32 until 64))
                    curve.curve.createPoint(x, y)
                    true
                } else if (decoded.size == 32) {
                    val x = BigInteger(1, decoded)
                    // Try both possible Y values
                    try {
                        curve.curve.createPoint(x, BigInteger.ZERO)
                        true
                    } catch (e: Exception) {
                        try {
                            curve.curve.createPoint(x, BigInteger.ONE)
                            true
                        } catch (e: Exception) {
                            false
                        }
                    }
                } else {
                    false
                }
            } catch (e: Exception) {
                false
            }
        }
    }
}
