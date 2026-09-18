package com.jamesatomc.kanariapp.wallet

import android.app.Application
import android.content.Context
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.jamesatomc.kanariapp.network.KanariClient
import com.jamesatomc.kanariapp.network.models.AccountInfo
import com.jamesatomc.kanariapp.network.models.TokenBalance
import com.jamesatomc.kanariapp.network.models.TransactionDetails
import com.jamesatomc.kanariapp.network.models.KanariEnvironment
import com.jamesatomc.kanariapp.ui.theme.ThemeMode
import com.jamesatomc.kanariapp.wallet.zklogin.ZkLoginAuth
import com.jamesatomc.kanariapp.wallet.zklogin.ZkLoginTxSigner
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import androidx.core.content.edit

class WalletViewModel(application: Application) : AndroidViewModel(application) {
    private val walletStorage = WalletStorage(application)

    private val _wallets = MutableStateFlow<List<WalletRecord>>(emptyList())
    val wallets: StateFlow<List<WalletRecord>> = _wallets.asStateFlow()

    private val _activeWallet = MutableStateFlow<WalletRecord?>(null)
    val activeWallet: StateFlow<WalletRecord?> = _activeWallet.asStateFlow()

    private val _accountInfo = MutableStateFlow<AccountInfo?>(null)
    val accountInfo: StateFlow<AccountInfo?> = _accountInfo.asStateFlow()

    private val _tokenBalances = MutableStateFlow<List<TokenBalance>>(emptyList())
    val tokenBalances: StateFlow<List<TokenBalance>> = _tokenBalances.asStateFlow()

    private val _transactions = MutableStateFlow<List<TransactionDetails>>(emptyList())
    val transactions: StateFlow<List<TransactionDetails>> = _transactions.asStateFlow()

    private val _isLoading = MutableStateFlow(value = false)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private val _environment = MutableStateFlow(KanariEnvironment.dev)
    val environment: StateFlow<KanariEnvironment> = _environment.asStateFlow()

    private val _isUnlocked = MutableStateFlow(false)
    val isUnlocked: StateFlow<Boolean> = _isUnlocked.asStateFlow()

    private var _unlockedPin: String? = null
    val unlockedPin: String? get() = _unlockedPin

    private var client = KanariClient(_environment.value)

    private val prefs = application.getSharedPreferences("kanari_prefs", Context.MODE_PRIVATE)
    private val walletMutationMutex = Mutex()

    private val _themeMode = MutableStateFlow(loadThemeMode())
    val themeMode: StateFlow<ThemeMode> = _themeMode.asStateFlow()

    private val _biometricEnabled = MutableStateFlow(false)
    val biometricEnabled: StateFlow<Boolean> = _biometricEnabled.asStateFlow()

    private fun loadThemeMode(): ThemeMode {
        val saved = prefs.getString("theme_mode", ThemeMode.SYSTEM.name) ?: ThemeMode.SYSTEM.name
        return try {
            ThemeMode.valueOf(saved)
        } catch (_: Exception) {
            ThemeMode.SYSTEM
        }
    }

    fun setThemeMode(mode: ThemeMode) {
        _themeMode.value = mode
        prefs.edit { putString("theme_mode", mode.name) }
    }

    init {
        viewModelScope.launch {
            _biometricEnabled.value = walletStorage.isBiometricEnabled()
        }
        loadWallets()
    }

    companion object {
        private const val KEY_ACTIVE_WALLET_ID = "active_wallet_id"
    }

    fun loadWallets() {
        viewModelScope.launch {
            walletMutationMutex.withLock {
                _isLoading.value = true
                try {
                    val records = walletStorage.loadWallets()
                    _wallets.value = records
                    if (records.isNotEmpty() && (_activeWallet.value == null)) {
                        val savedId = prefs.getString(KEY_ACTIVE_WALLET_ID, null)
                        _activeWallet.value = records.firstOrNull { it.id == savedId } ?: records.first()
                    }
                } catch (e: Exception) {
                    _error.value = "Failed to load wallets: ${e.message}"
                }
                _isLoading.value = false
            }
        }
    }

    fun setEnvironment(env: KanariEnvironment) {
        _environment.value = env
        client = KanariClient(env)
        refreshBalance()
    }

    fun refreshBalance() {
        val address = _activeWallet.value?.address ?: return
        viewModelScope.launch {
            _isLoading.value = true
            _error.value = null

            try {
                val info = client.getAccount(address)
                _accountInfo.value = info

                val balances = client.getAllBalances(address)
                _tokenBalances.value = balances

                val txs = client.getAllTransactions(address)
                _transactions.value = txs
            } catch (e: Exception) {
                // Ignore "Owner not found" as a hard error for new wallets
                if (e.message?.contains("Owner not found", ignoreCase = true) == true) {
                    _accountInfo.value = null
                    _tokenBalances.value = emptyList()
                    _transactions.value = emptyList()
                } else {
                    _error.value = "Failed to refresh balance: ${e.message}"
                }
            }
            _isLoading.value = false
        }
    }

    fun switchWallet(record: WalletRecord) {
        _activeWallet.value = record
        prefs.edit { putString(KEY_ACTIVE_WALLET_ID, record.id) }
        refreshBalance()
    }

    suspend fun unlock(pin: String): Boolean {
        // Session-only wallets (zkLogin) hold no PIN-encrypted secrets, so any
        // PIN unlocks if no key-based wallet exists. Real key wallets always
        // require the exact PIN.
        val verified = walletStorage.verifyPin(pin) ||
                (!walletStorage.hasPin() && !walletStorage.hasSecrets())
        if (verified) {
            _unlockedPin = pin
            _isUnlocked.value = true
            // Sync biometric PIN if biometric is enabled (like kanari_pay)
            if (walletStorage.isBiometricEnabled()) {
                try {
                    walletStorage.saveBiometricPin(pin)
                } catch (_: Exception) {
                }
            }
            loadWallets()
            return true
        }
        return false
    }

    suspend fun unlockWithBiometric(): Boolean {
        val pin = walletStorage.getBiometricPin() ?: return false
        return unlock(pin)
    }

    suspend fun revealPrivateKeyWithBiometric(record: WalletRecord): String? {
        val pin = walletStorage.getBiometricPin() ?: return null
        return revealPrivateKey(record, pin)
    }

    suspend fun revealMnemonicWithBiometric(record: WalletRecord): String? {
        val pin = walletStorage.getBiometricPin() ?: return null
        return revealMnemonic(record, pin)
    }

    suspend fun verifyPin(pin: String): Boolean = walletStorage.verifyPin(pin)

    /** True when the user has set an app PIN (gate sensitive flows behind it). */
    suspend fun hasPin(): Boolean = try {
        walletStorage.hasPin()
    } catch (_: Exception) {
        false
    }

    suspend fun revealPrivateKey(record: WalletRecord, pin: String): String? {
        val enc = record.privateKeyEncrypted ?: return null
        return try {
            walletStorage.decrypt(enc, pin)
        } catch (_: Exception) {
            null
        }
    }

    suspend fun revealMnemonic(record: WalletRecord, pin: String): String? {
        val enc = record.mnemonicEncrypted ?: return null
        return try {
            walletStorage.decrypt(enc, pin)
        } catch (_: Exception) {
            null
        }
    }

    suspend fun isBiometricEnabled(): Boolean = walletStorage.isBiometricEnabled()

    suspend fun setBiometricEnabled(enabled: Boolean): Boolean {
        if (enabled) {
            val pin = unlockedPin ?: return false
            walletStorage.saveBiometricPin(pin)
        } else {
            walletStorage.clearBiometricPin()
            walletStorage.setBiometricEnabled(false)
        }
        _biometricEnabled.value = walletStorage.isBiometricEnabled()
        return true
    }

    suspend fun changePin(oldPin: String, newPin: String): Boolean {
        if (!walletStorage.verifyPin(oldPin)) return false
        if (newPin.length != 6 || !newPin.all { it.isDigit() }) return false
        try {
            val wallets = walletStorage.loadWallets()
            val reEncrypted = wallets.map { record ->
                val newPrivate = record.privateKeyEncrypted?.let {
                    try {
                        walletStorage.decrypt(it, oldPin)
                    } catch (_: Exception) {
                        null
                    }
                }?.let { walletStorage.encrypt(it, newPin) } ?: record.privateKeyEncrypted
                val newMnemonic = record.mnemonicEncrypted?.let {
                    try {
                        walletStorage.decrypt(it, oldPin)
                    } catch (_: Exception) {
                        null
                    }
                }?.let { walletStorage.encrypt(it, newPin) } ?: record.mnemonicEncrypted
                record.copy(privateKeyEncrypted = newPrivate, mnemonicEncrypted = newMnemonic)
            }
            walletStorage.saveWallets(reEncrypted)
            walletStorage.savePin(newPin)
            _unlockedPin = newPin
            if (walletStorage.isBiometricEnabled()) {
                walletStorage.saveBiometricPin(newPin)
            }
            _wallets.value = reEncrypted
            _activeWallet.value = reEncrypted.find { it.id == _activeWallet.value?.id } ?: reEncrypted.firstOrNull()
            return true
        } catch (_: Exception) {
            return false
        }
    }

    fun logout() {
        _activeWallet.value = null
        _accountInfo.value = null
        _wallets.value = emptyList()
    }

    fun deleteWallet(record: WalletRecord) {
        viewModelScope.launch {
            val current = _wallets.value.toMutableList()
            current.remove(record)
            walletStorage.saveWallets(current)
            _wallets.value = current
            if (record.curveType == "ZkLogin") {
                try {
                    ZkLoginAuth.sessionFile(getApplication(), record.address).delete()
                } catch (_: Exception) {
                }
            }
            if (_activeWallet.value?.id == record.id) {
                val next = current.firstOrNull()
                _activeWallet.value = next
                if (next != null) prefs.edit { putString(KEY_ACTIVE_WALLET_ID, next.id) }
                else prefs.edit { remove(KEY_ACTIVE_WALLET_ID) }
                refreshBalance()
            }
        }
    }

    suspend fun transfer(recipient: String, amount: ULong, tokenType: String = "0x2::kanari::KANARI"): Boolean {
        val record = _activeWallet.value ?: run {
            _error.value = "No active wallet"
            return false
        }
        // zkLogin wallets hold no encrypted secrets, so no PIN is required.
        val pin = unlockedPin

        return try {
            val isKanari = tokenType == "0x2::kanari::KANARI" ||
                    tokenType.lowercase(java.util.Locale.US) == "0x2::kanari::kanari" ||
                    tokenType.endsWith("::kanari::KANARI", ignoreCase = true)

            val result: com.jamesatomc.kanariapp.network.models.TransactionResult? =
                if (record.curveType == "ZkLogin") {
                    // No private key: sign with the on-device Google session.
                    val session = try {
                        ZkLoginAuth.loadSession(getApplication(), record.address)
                    } catch (_: Exception) {
                        _error.value =
                            "No Google session for this wallet — sign in with Google from the Login screen first."
                        return false
                    }
                    if (isKanari) client.transferZkLogin(session, recipient, amount)
                    else client.transferTokenZkLogin(session, recipient, tokenType, amount)
                } else {
                    val pin = pin ?: run {
                        _error.value = "Wallet is locked. Please unlock again."
                        return false
                    }
                    val privateKeyEncrypted = record.privateKeyEncrypted ?: run {
                        _error.value = "Wallet missing private key"
                        return false
                    }
                    val privateKey = walletStorage.decrypt(privateKeyEncrypted, pin)
                    val wallet = KanariWallet.fromPrivateKey(privateKey, record.curveType)
                    if (isKanari) {
                        client.transfer(wallet, recipient, amount)
                    } else {
                        // Generic fungible token transfer via kanari_buildTokenTransfer / kanari_callFunction
                        // Amount is already in token's smallest units (as per parseAmountToMist)
                        client.transferToken(wallet, recipient, tokenType, amount)
                    }
                }

            val status = result?.status?.lowercase(java.util.Locale.US)
            val isSuccess = result?.success == true ||
                    status == "success" || status == "executed" ||
                    status == "committed" || status == "pending" ||
                    status == "simulated_pending"

            if (isSuccess) {
                _error.value = null
                refreshBalance()
                true
            } else {
                val msg = result?.errorMessage ?: "Transaction failed (Status: ${result?.status})"
                _error.value = mapTransferError(msg)
                false
            }
        } catch (_: ZkLoginTxSigner.SessionExpiredException) {
            _error.value = "Google session expired — open Login and sign in with Google again, then retry the transfer."
            false
        } catch (_: ZkLoginTxSigner.SessionIncompleteException) {
            _error.value = "Google session file is from an old app version — sign in with Google again."
            false
        } catch (e: Exception) {
            _error.value = mapTransferError(e.message ?: "Unknown error")
            false
        }
    }

    /**
     * Registers the Google zkLogin wallet (no private key stored; signing
     * runs from the on-device session file) and switches to it.
     */
    suspend fun addZkLoginWallet(session: ZkLoginAuth.Session): WalletRecord {
        return walletMutationMutex.withLock {
            val record = WalletRecord(
                id = "zklogin-${session.address}",
                name = "Google zkLogin",
                address = session.address,
                curveType = "ZkLogin",
                privateKeyEncrypted = null,
                mnemonicEncrypted = null,
            )
            val persisted = try {
                walletStorage.loadWallets()
            } catch (e: Exception) {
                if (_wallets.value.isEmpty()) throw e
                _wallets.value
            }
            val current = (persisted + _wallets.value).distinctBy { it.id }
            val updated = if (current.any { it.id == record.id }) {
                current
            } else {
                current + record
            }
            if (updated != current) walletStorage.saveWallets(updated)
            _wallets.value = updated
            _activeWallet.value = record
            prefs.edit { putString(KEY_ACTIVE_WALLET_ID, record.id) }
            refreshBalance()
            record
        }
    }

    private fun mapTransferError(raw: String): String {
        val text = raw.lowercase(java.util.Locale.US)
        return when {
            text.contains("unexpected call_error") || text.contains("call_error") ->
                "Transfer failed: Local crypto error (Unexpected CALL_ERROR). This was a bug in BCS signing that is now fixed. Please retry. If it persists, re-import wallet and ensure curve type is correct."

            text.contains("invalid transaction signature") || text.contains("invalid_transaction_signature") ->
                "Transfer failed: Invalid transaction signature. BCS hash mismatch - please update app and retry."

            text.contains("requires two distinct") || text.contains("native_transfer_policy_not_satisfied") ||
                    text.contains("allows_single_coin_for_transfer_and_gas") ->
                "Native KANARI transfer needs two separate Coin<KANARI> objects: one coin to send and another coin to pay gas. Receive/fund this wallet once more, then try again."

            text.contains("no single coin") && text.contains("can cover requested amount") ->
                "Native KANARI transfer needs one Coin<KANARI> with enough balance to cover amount. Try a smaller amount or fund another coin."

            text.contains("no spendable native gas coin") ->
                "Insufficient gas coin. Need a separate Coin<KANARI> with enough Mist to pay gas (gas_limit * gas_price)."

            text.contains("missing a valid nonce") || text.contains("nonce must be non-zero") ->
                "Transfer failed: Transaction nonce error. Please try again."

            raw.startsWith("Transfer failed:") -> raw
            else -> "Transfer failed: $raw"
        }
    }
}
