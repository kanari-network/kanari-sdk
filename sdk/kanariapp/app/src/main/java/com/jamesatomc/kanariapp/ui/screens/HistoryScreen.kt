package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.History
import androidx.compose.material.icons.filled.NorthEast
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.SouthWest
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.jamesatomc.kanariapp.network.models.TransactionDetails
import com.jamesatomc.kanariapp.wallet.WalletViewModel
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HistoryScreen(viewModel: WalletViewModel) {
    val transactions by viewModel.transactions.collectAsStateWithLifecycle()
    val isLoading by viewModel.isLoading.collectAsStateWithLifecycle()
    val activeWallet by viewModel.activeWallet.collectAsStateWithLifecycle()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("History") },
                actions = {
                    IconButton(onClick = { viewModel.refreshBalance() }) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                }
            )
        }
    ) { padding ->
        if (isLoading && transactions.isEmpty()) {
            Box(modifier = Modifier.fillMaxSize().padding(padding), contentAlignment = Alignment.Center) {
                CircularProgressIndicator()
            }
        } else if (transactions.isEmpty()) {
            Column(
                modifier = Modifier.fillMaxSize().padding(padding),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center
            ) {
                Icon(
                    Icons.Default.History, 
                    contentDescription = null, 
                    modifier = Modifier.size(64.dp),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Text("No Transactions Yet", style = MaterialTheme.typography.titleMedium)
            }
        } else {
            LazyColumn(
                modifier = Modifier.fillMaxSize().padding(padding)
            ) {
                items(transactions) { tx ->
                    val isIncoming = activeWallet?.let { 
                        it.address.lowercase() == tx.senderAddress?.lowercase() || 
                        it.address.lowercase() == tx.sender.lowercase() 
                    }.let { !(it ?: false) }
                    HistoryItem(tx, isIncoming)
                }
            }
        }
    }
}

@Composable
fun HistoryItem(tx: TransactionDetails, isIncoming: Boolean) {
    ListItem(
        headlineContent = { Text(tx.txType) },
        supportingContent = { Text(if (isIncoming) "From: ${tx.sender.take(8)}..." else "Sent", maxLines = 1) },
        leadingContent = {
            Icon(
                imageVector = if (isIncoming) Icons.Default.SouthWest else Icons.Default.NorthEast,
                contentDescription = null,
                tint = if (isIncoming) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error
            )
        },
        trailingContent = {
            val gasAmount = tx.effects?.gasUsed ?: 0L 
            Text(
                text = "${if (isIncoming) "+" else "-"}${String.format(Locale.US, "%.2f", gasAmount / 1_000_000_000.0)}",
                color = if (isIncoming) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface
            )
        }
    )
}
