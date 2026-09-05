package com.jamesatomc.kanariapp.wallet

import com.kanari.kanari_crypto.KanariCrypto
import com.kanari.kanari_crypto.model.KeyPairModel

class KanariWallet(
    val keyPair: KeyPairModel,
    val mnemonic: String? = null,
    val derivationPath: String? = null
) {
    val address: String get() = keyPair.address
    val taggedAddress: String get() = keyPair.taggedAddress
    val publicKey: String get() = keyPair.publicKey
    val privateKey: String get() = keyPair.privateKey
    val curveType: String get() = keyPair.curveType

    companion object {
        const val DEFAULT_DERIVATION_PATH = "m/44'/637'/0'/0/0"

        suspend fun generate(
            curveName: String = KanariCrypto.DEFAULT_CURVE,
            derivationPath: String = DEFAULT_DERIVATION_PATH
        ): KanariWallet {
            val isPostQuantum = curveName.contains("dilithium", ignoreCase = true)
            
            return if (isPostQuantum) {
                val pair = KanariCrypto.generateKeypair(curveName)
                KanariWallet(pair)
            } else {
                val words = KanariCrypto.generateMnemonic(12)
                val pair = KanariCrypto.deriveKeypairFromPath(words, derivationPath, curveName)
                KanariWallet(pair, mnemonic = words, derivationPath = derivationPath)
            }
        }

        suspend fun fromMnemonic(
            mnemonic: String,
            curveName: String = KanariCrypto.DEFAULT_CURVE,
            derivationPath: String = DEFAULT_DERIVATION_PATH
        ): KanariWallet {
            val pair = KanariCrypto.deriveKeypairFromPath(mnemonic, derivationPath, curveName)
            return KanariWallet(pair, mnemonic = mnemonic, derivationPath = derivationPath)
        }

        suspend fun fromPrivateKey(
            privateKey: String,
            curveName: String = KanariCrypto.DEFAULT_CURVE
        ): KanariWallet {
            val pair = KanariCrypto.importKeypairFromPrivateKey(privateKey, curveName)
            return KanariWallet(pair)
        }
    }

    suspend fun sign(message: ByteArray): ByteArray {
        return KanariCrypto.signMessage(privateKey, message, curveType)
    }
}