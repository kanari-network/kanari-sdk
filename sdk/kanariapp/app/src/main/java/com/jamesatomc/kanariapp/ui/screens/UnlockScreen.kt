package com.jamesatomc.kanariapp.ui.screens

import androidx.biometric.BiometricManager
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.fragment.app.FragmentActivity
import com.jamesatomc.kanariapp.ui.components.PinVerificationContent
import com.jamesatomc.kanariapp.ui.components.showBiometricPrompt
import com.jamesatomc.kanariapp.wallet.WalletViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun UnlockScreen(viewModel: WalletViewModel, onUnlockSuccess: () -> Unit, onBack: () -> Unit) {
    val context = LocalContext.current
    val activity = context as? FragmentActivity
    var canUseBiometric by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) {
        try {
            val enabled = viewModel.isBiometricEnabled()
            if (!enabled) canUseBiometric = false
            else canUseBiometric = BiometricManager.from(context)
                .canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG) == BiometricManager.BIOMETRIC_SUCCESS
        } catch (_: Exception) {
            canUseBiometric = false
        }
    }
    fun onBiometric() {
        if (activity == null) return
        showBiometricPrompt(
            activity = activity,
            title = "Unlock Kanari Wallet",
            subtitle = "Use biometrics to unlock",
            onSuccess = { if (viewModel.unlockWithBiometric()) onUnlockSuccess() }
        )
    }
    Scaffold(topBar = {
        TopAppBar(
            title = { Text("Unlock Wallet") },
            navigationIcon = {
                IconButton(onClick = onBack) {
                    Icon(
                        Icons.AutoMirrored.Filled.ArrowBack,
                        contentDescription = "Back"
                    )
                }
            })
    }) { padding ->
        PinVerificationContent(
            title = "Welcome Back",
            subtitle = "Enter your 6-digit PIN to unlock your wallet",
            onVerify = { pin -> viewModel.unlock(pin) },
            onSuccess = { _: String -> onUnlockSuccess() },
            biometricEnabled = canUseBiometric,
            onBiometric = ::onBiometric,
            modifier = Modifier.fillMaxSize().padding(padding)
        )
    }
}