package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.fragment.app.FragmentActivity
import androidx.biometric.BiometricPrompt
import androidx.compose.ui.platform.LocalContext
import com.jamesatomc.kanariapp.wallet.WalletViewModel
import java.util.concurrent.Executors

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun UnlockScreen(
    viewModel: WalletViewModel,
    onUnlockSuccess: () -> Unit,
    onBack: () -> Unit
) {
    val androidContext = LocalContext.current
    var pin by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Unlock Wallet") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.background,
                    titleContentColor = MaterialTheme.colorScheme.onBackground
                )
            )
        },
        containerColor = MaterialTheme.colorScheme.background
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            Text(
                "Welcome Back",
                style = MaterialTheme.typography.headlineMedium,
                fontWeight = FontWeight.Bold
            )
            Spacer(Modifier.height(8.dp))
            Text(
                "Enter your 6-digit PIN to unlock your wallet",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            
            Spacer(Modifier.height(32.dp))
            
            OutlinedTextField(
                value = pin,
                onValueChange = { if (it.length <= 6) pin = it },
                label = { Text("6-Digit PIN") },
                visualTransformation = PasswordVisualTransformation(),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.fillMaxWidth(),
                isError = error != null,
                shape = RoundedCornerShape(12.dp)
            )
            
            if (error != null) {
                Text(
                    error!!,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.padding(top = 8.dp)
                )
            }
            
            Spacer(Modifier.height(24.dp))
            
            Button(
                onClick = {
                    if (viewModel.unlock(pin)) {
                        onUnlockSuccess()
                    } else {
                        error = "Invalid PIN. Please try again."
                    }
                },
                modifier = Modifier.fillMaxWidth().height(56.dp),
                enabled = pin.length == 6,
                shape = RoundedCornerShape(12.dp)
            ) {
                Text("Unlock")
            }
            
            Spacer(Modifier.height(16.dp))
            
            TextButton(
                onClick = {
                    val executor = Executors.newSingleThreadExecutor()
                    val activity = androidContext as FragmentActivity
                    val biometricPrompt = BiometricPrompt(activity, executor,
                        object : BiometricPrompt.AuthenticationCallback() {
                            override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                                super.onAuthenticationSucceeded(result)
                                activity.runOnUiThread {
                                    // In a real app, you'd retrieve the PIN from secure storage using Biometrics
                                    // For now, we simulate success if the user has wallets
                                    onUnlockSuccess()
                                }
                            }
                        })

                    val promptInfo = BiometricPrompt.PromptInfo.Builder()
                        .setTitle("Biometric Unlock")
                        .setSubtitle("Unlock your Kanari wallet using biometrics")
                        .setNegativeButtonText("Use PIN")
                        .build()

                    biometricPrompt.authenticate(promptInfo)
                }
            ) {
                Text("Unlock with Biometrics")
            }
        }
    }
}
