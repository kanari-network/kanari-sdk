package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.jamesatomc.kanariapp.ui.components.LoadingButton
import com.jamesatomc.kanariapp.ui.components.QrCodeImage
import com.jamesatomc.kanariapp.ui.components.ScaffoldWithBackBar
import com.jamesatomc.kanariapp.ui.components.copyToClipboard
import com.jamesatomc.kanariapp.wallet.WalletViewModel

@Composable
fun ReceiveScreen(viewModel: WalletViewModel, onBack: () -> Unit) {
    val activeWallet by viewModel.activeWallet.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val address = activeWallet?.address ?: ""

    ScaffoldWithBackBar(title = "Receive", onBack = onBack) { padding ->
        Column(
            Modifier.fillMaxSize().padding(padding).padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            Text("Your Wallet Address", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
            Spacer(Modifier.height(24.dp))
            QrCodeImage(address = address)
            Spacer(Modifier.height(24.dp))
            Surface(
                Modifier.fillMaxWidth(), shape = RoundedCornerShape(12.dp),
                color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f)
            ) {
                Text(
                    address.ifEmpty { "No wallet selected" },
                    style = MaterialTheme.typography.bodyMedium,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.padding(16.dp)
                )
            }
            Spacer(Modifier.height(32.dp))
            LoadingButton(
                onClick = { if (address.isNotEmpty()) copyToClipboard(context, address, toast = "Address copied") },
                text = "Copy Address", icon = Icons.Default.ContentCopy,
                modifier = Modifier.fillMaxWidth()
            )
        }
    }
}
