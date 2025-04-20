package io.kanari.wallet.exceptions

/**
 * Exception thrown for wallet-related errors
 */
class WalletException(message: String, cause: Throwable? = null) : Exception(message, cause)

/**
 * Exception thrown when encryption/decryption fails
 */
class CryptoException(message: String, cause: Throwable? = null) : WalletException(message, cause)

/**
 * Exception thrown when configuration operations fail
 */
class ConfigException(message: String, cause: Throwable? = null) : WalletException(message, cause)
