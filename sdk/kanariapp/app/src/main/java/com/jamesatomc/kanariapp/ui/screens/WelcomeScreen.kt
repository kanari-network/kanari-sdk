package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Login
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.FileDownload
import androidx.compose.material.icons.filled.LockOpen
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.jamesatomc.kanariapp.wallet.WalletStorage

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WelcomeScreen(
    onNavigateToLogin: () -> Unit,
    onNavigateToRegister: () -> Unit,
    onNavigateToWalletGen: () -> Unit,
    onNavigateToUnlock: () -> Unit
) {
    val context = LocalContext.current
    val walletStorage = remember { WalletStorage(context) }
    var hasWallet by remember { mutableStateOf(walletStorage.loadWallets().isNotEmpty()) }

    LaunchedEffect(Unit) { hasWallet = walletStorage.loadWallets().isNotEmpty() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Kanari Wallet", fontWeight = FontWeight.ExtraBold) },
                actions = {
                    TextButton(onClick = onNavigateToRegister) {
                        Text(
                            "Register",
                            fontWeight = FontWeight.Bold
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surface)
            )
        },
        containerColor = MaterialTheme.colorScheme.surface
    ) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Spacer(modifier = Modifier.height(48.dp))

            Text("Secure & Simple", style = MaterialTheme.typography.headlineLarge, textAlign = TextAlign.Center)
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                "Manage your Kanari Network assets with ease.",
                style = MaterialTheme.typography.bodyLarge,
                textAlign = TextAlign.Center,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(modifier = Modifier.height(32.dp))

            Card(
                modifier = Modifier.fillMaxWidth(), shape = RoundedCornerShape(18.dp),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow),
                elevation = CardDefaults.cardElevation(defaultElevation = 0.dp)
            ) {
                Column(modifier = Modifier.padding(24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text("Wallet Access", style = MaterialTheme.typography.titleMedium)

                    if (hasWallet) {
                        Button(
                            onClick = onNavigateToUnlock,
                            modifier = Modifier.fillMaxWidth().height(56.dp),
                            shape = RoundedCornerShape(12.dp)
                        ) {
                            Icon(Icons.Default.LockOpen, contentDescription = null)
                            Spacer(Modifier.width(8.dp))
                            Text("Unlock Saved Wallet", fontWeight = FontWeight.Bold)
                        }
                    }
                    FilledTonalButton(
                        onClick = onNavigateToWalletGen,
                        modifier = Modifier.fillMaxWidth().height(56.dp),
                        shape = RoundedCornerShape(12.dp)
                    ) {
                        Icon(
                            Icons.Default.Add,
                            contentDescription = null
                        ); Spacer(Modifier.width(8.dp)); Text("Create New Wallet", fontWeight = FontWeight.Bold)
                    }
                    OutlinedButton(
                        onClick = onNavigateToWalletGen,
                        modifier = Modifier.fillMaxWidth().height(56.dp),
                        shape = RoundedCornerShape(12.dp)
                    ) {
                        Icon(Icons.Default.FileDownload, contentDescription = null); Spacer(Modifier.width(8.dp)); Text(
                        "Import Existing Wallet",
                        fontWeight = FontWeight.Bold
                    )
                    }
                }
            }

            Spacer(modifier = Modifier.height(24.dp))
            HorizontalDivider(
                modifier = Modifier.padding(vertical = 8.dp),
                color = MaterialTheme.colorScheme.outlineVariant
            )
            TextButton(onClick = onNavigateToLogin, modifier = Modifier.fillMaxWidth().height(56.dp)) {
                Icon(
                    Icons.AutoMirrored.Filled.Login,
                    contentDescription = null
                ); Spacer(Modifier.width(8.dp)); Text("Login to Kanari Account", fontWeight = FontWeight.Bold)
            }
            Spacer(modifier = Modifier.height(16.dp))
        }
    }
}
