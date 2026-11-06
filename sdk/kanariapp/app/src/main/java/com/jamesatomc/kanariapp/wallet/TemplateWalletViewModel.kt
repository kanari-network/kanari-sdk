// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

package com.jamesatomc.kanariapp.wallet

import android.app.Application
import com.jamesatomc.kanariapp.network.KanariClient
import com.jamesatomc.kanariapp.network.buildTemplateAction
import com.jamesatomc.kanariapp.network.submitTemplateTx
import com.jamesatomc.kanariapp.network.submitTemplateTxZkLogin
import com.jamesatomc.kanariapp.network.models.TransactionResult
import com.jamesatomc.kanariapp.wallet.zklogin.ZkLoginAuth
import com.jamesatomc.kanariapp.wallet.zklogin.ZkLoginTxSigner
import java.util.Locale

private suspend fun WalletViewModel.unlockedTemplateWallet(record: WalletRecord): KanariWallet? {
    val pin = unlockedPin ?: run {
        _error.value = "Wallet is locked. Please unlock again."
        return null
    }
    val privateKeyEncrypted = record.privateKeyEncrypted ?: run {
        _error.value = "Wallet missing private key"
        return null
    }
    return try {
        KanariWallet.fromPrivateKey(walletStorage.decrypt(privateKeyEncrypted, pin), record.curveType)
    } catch (e: Exception) {
        _error.value = "Failed to unlock wallet: ${e.message}"
        null
    }
}

private fun WalletViewModel.settleTemplateResult(result: TransactionResult?): Boolean {
    val status = result?.status?.lowercase(Locale.US)
    val isSuccess = result?.success == true ||
            status == "success" || status == "executed" ||
            status == "committed" || status == "pending" ||
            status == "simulated_pending"
    return if (isSuccess) {
        _error.value = null
        refreshBalanceAfterSend()
        true
    } else {
        val msg = result?.errorMessage ?: "Transaction failed (Status: ${result?.status})"
        _error.value = mapTransferError(msg)
        false
    }
}

suspend fun WalletViewModel.runTemplateAction(): Boolean {
    val record = activeWallet.value ?: run {
        _error.value = "No active wallet"
        return false
    }
    return try {
        val result = if (record.curveType == "ZkLogin") {
            val session = try {
                ZkLoginAuth.loadSession(getApplication<Application>(), record.address)
            } catch (_: Exception) {
                _error.value = "No Google session - sign in with Google first."
                return false
            }
            val prepared = client.buildTemplateAction("ZkLogin:${session.address}")
            client.submitTemplateTxZkLogin(session, prepared)
        } else {
            val wallet = unlockedTemplateWallet(record) ?: return false
            val prepared = client.buildTemplateAction(wallet.taggedAddress)
            client.submitTemplateTx(wallet, prepared)
        }
        settleTemplateResult(result)
    } catch (_: ZkLoginTxSigner.SessionExpiredException) {
        _error.value = "Google session expired - sign in again."
        false
    } catch (_: ZkLoginTxSigner.SessionIncompleteException) {
        _error.value = "Google session file is from an old app version - sign in again."
        false
    } catch (e: Exception) {
        _error.value = "Template action failed: ${e.message}"
        false
    }
}
