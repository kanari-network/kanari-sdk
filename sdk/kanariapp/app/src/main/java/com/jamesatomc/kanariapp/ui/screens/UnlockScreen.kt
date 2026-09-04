package com.jamesatomc.kanariapp.ui.screens

import android.widget.Toast
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import com.jamesatomc.kanariapp.ui.components.PinCircles
import com.jamesatomc.kanariapp.ui.components.PinEntryHeader
import com.jamesatomc.kanariapp.ui.components.PinNumberPad
import com.jamesatomc.kanariapp.wallet.WalletViewModel
import java.util.concurrent.Executors

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun UnlockScreen(
    viewModel: WalletViewModel,
    onUnlockSuccess: () -> Unit,
    onBack: () -> Unit
) {
    val context = LocalContext.current
    val activity = context as? FragmentActivity
    var enteredPin by remember { mutableStateOf("") }
    var errorText by remember { mutableStateOf<String?>(null) }
    var isChecking by remember { mutableStateOf(false) }
    var canUseBiometric by remember { mutableStateOf(false) }

    // Check biometric availability - like kanari_pay's _loadBiometricAvailability
    LaunchedEffect(Unit) {
        try {
            val enabled = viewModel.isBiometricEnabled()
            if (!enabled) {
                canUseBiometric = false
                return@LaunchedEffect
            }
            val manager = BiometricManager.from(context)
            val canAuth = manager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG) == BiometricManager.BIOMETRIC_SUCCESS
            canUseBiometric = canAuth
        } catch (_: Exception) {
            canUseBiometric = false
        }
    }

    fun verifyPin(pin: String) {
        if (isChecking) return
        isChecking = true
        errorText = null
        // small delay to mimic kanari_pay's 200ms for smooth UI
        android.os.Handler(android.os.Looper.getMainLooper()).postDelayed({
            val ok = viewModel.unlock(pin)
            if (ok) {
                onUnlockSuccess()
            } else {
                enteredPin = ""
                errorText = "Invalid PIN"
                isChecking = false
            }
        }, 120)
    }

    fun onNumberPressed(num: String) {
        if (isChecking || enteredPin.length >= 6) return
        val next = enteredPin + num
        enteredPin = next
        errorText = null
        if (next.length == 6) {
            verifyPin(next)
        }
    }

    fun onBackspace() {
        if (isChecking || enteredPin.isEmpty()) return
        enteredPin = enteredPin.dropLast(1)
        errorText = null
    }

    fun onBiometric() {
        if (isChecking || activity == null) return
        val executor = ContextCompat.getMainExecutor(activity)
        val prompt = BiometricPrompt(activity, executor, object : BiometricPrompt.AuthenticationCallback() {
            override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                super.onAuthenticationSucceeded(result)
                activity.runOnUiThread {
                    isChecking = true
                    val ok = viewModel.unlockWithBiometric()
                    if (ok) {
                        onUnlockSuccess()
                    } else {
                        isChecking = false
                        errorText = "Biometric unlock failed"
                        Toast.makeText(context, "Biometric unlock failed", Toast.LENGTH_SHORT).show()
                    }
                }
            }
            override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                super.onAuthenticationError(errorCode, errString)
                activity.runOnUiThread {
                    isChecking = false
                    if (errorCode != BiometricPrompt.ERROR_USER_CANCELED && errorCode != BiometricPrompt.ERROR_NEGATIVE_BUTTON) {
                        errorText = "Biometric unavailable"
                    }
                }
            }
            override fun onAuthenticationFailed() {
                super.onAuthenticationFailed()
                activity.runOnUiThread { errorText = "Biometric not recognized" }
            }
        })
        val promptInfo = BiometricPrompt.PromptInfo.Builder()
            .setTitle("Unlock Kanari Wallet")
            .setSubtitle("Use biometrics to unlock your wallet")
            .setNegativeButtonText("Use PIN")
            .build()
        isChecking = true
        prompt.authenticate(promptInfo)
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Unlock Wallet") },
                navigationIcon = { IconButton(onClick = onBack) { Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back") } },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background)
            )
        },
        containerColor = MaterialTheme.colorScheme.background
    ) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Spacer(Modifier.height(16.dp))
            PinEntryHeader(
                title = "Welcome Back",
                subtitle = "Enter your 6-digit PIN to unlock your wallet",
                enteredLength = enteredPin.length,
                errorText = errorText,
                isChecking = isChecking
            )
            Spacer(Modifier.weight(1f))
            PinNumberPad(
                onNumberPressed = ::onNumberPressed,
                onBackspacePressed = ::onBackspace,
                biometricEnabled = canUseBiometric,
                onBiometricPressed = ::onBiometric
            )
            Spacer(Modifier.height(16.dp))
        }
    }
}
