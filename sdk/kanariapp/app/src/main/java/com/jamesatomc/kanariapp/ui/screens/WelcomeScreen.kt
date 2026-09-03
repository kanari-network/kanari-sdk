package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
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
    
    LaunchedEffect(Unit) {
        hasWallet = walletStorage.loadWallets().isNotEmpty()
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Kanari Wallet") },
                actions = {
                    TextButton(onClick = onNavigateToRegister) {
                        Text("Register")
                    }
                }
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            Spacer(modifier = Modifier.height(32.dp))
            
            Text(
                text = "Secure & Simple",
                style = MaterialTheme.typography.headlineMedium,
                textAlign = TextAlign.Center
            )
            
            Text(
                text = "Manage your Kanari Network assets with ease.",
                style = MaterialTheme.typography.bodyLarge,
                textAlign = TextAlign.Center,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            
            Spacer(modifier = Modifier.height(32.dp))
            
            Card(
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(
                    modifier = Modifier.padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    Text(
                        text = "Wallet Access",
                        style = MaterialTheme.typography.titleMedium
                    )
                    
                    if (hasWallet) {
                        Button(
                            onClick = onNavigateToUnlock,
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Icon(Icons.Default.LockOpen, contentDescription = null)
                            Spacer(Modifier.width(8.dp))
                            Text("Unlock Saved Wallet")
                        }
                    }
                    
                    FilledTonalButton(
                        onClick = onNavigateToWalletGen,
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Icon(Icons.Default.Add, contentDescription = null)
                        Spacer(Modifier.width(8.dp))
                        Text("Create New Wallet")
                    }
                    
                    OutlinedButton(
                        onClick = onNavigateToWalletGen,
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Icon(Icons.Default.FileDownload, contentDescription = null)
                        Spacer(Modifier.width(8.dp))
                        Text("Import Existing Wallet")
                    }
                }
            }
            
            HorizontalDivider(modifier = Modifier.padding(vertical = 16.dp))
            
            TextButton(
                onClick = onNavigateToLogin,
                modifier = Modifier.fillMaxWidth()
            ) {
                Icon(Icons.AutoMirrored.Filled.Login, contentDescription = null)
                Spacer(Modifier.width(8.dp))
                Text("Login to Kanari Account")
            }
        }
    }
}
