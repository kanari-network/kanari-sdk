package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.History
import androidx.compose.material.icons.filled.NorthEast
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.SouthWest
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.jamesatomc.kanariapp.network.models.TransactionDetails
import com.jamesatomc.kanariapp.ui.components.CopyableAddressRow
import com.jamesatomc.kanariapp.ui.components.copyToClipboard
import com.jamesatomc.kanariapp.wallet.WalletViewModel
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HistoryScreen(viewModel: WalletViewModel) {
    val transactions by viewModel.transactions.collectAsStateWithLifecycle()
    val isLoading by viewModel.isLoading.collectAsStateWithLifecycle()
    val activeWallet by viewModel.activeWallet.collectAsStateWithLifecycle()
    var selectedTx by remember { mutableStateOf<TransactionDetails?>(null) }

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
            LazyColumn(modifier = Modifier.fillMaxSize().padding(padding)) {
                items(transactions, key = { it.hash }) { tx ->
                    val myAddr = activeWallet?.address?.lowercase()
                    val isIncoming =
                        !(myAddr != null && (myAddr == tx.sender.lowercase() || myAddr == tx.senderAddress?.lowercase()))
                    HistoryItem(tx, isIncoming, onClick = { selectedTx = tx })
                }
            }
        }
    }

    selectedTx?.let { tx ->
        val myAddr = activeWallet?.address?.lowercase()
        val isIncoming =
            !(myAddr != null && (myAddr == tx.sender.lowercase() || myAddr == tx.senderAddress?.lowercase()))
        ModalBottomSheet(onDismissRequest = { selectedTx = null }) {
            TransactionDetailSheet(tx = tx, isIncoming = isIncoming, onDismiss = { selectedTx = null })
        }
    }
}

@Composable
fun HistoryItem(tx: TransactionDetails, isIncoming: Boolean, onClick: () -> Unit) {
    ListItem(
        headlineContent = { Text(tx.txType) },
        supportingContent = {
            Text(
                if (isIncoming) "From: ${tx.sender.take(8)}..." else "Sent • ${tx.status}",
                maxLines = 1
            )
        },
        leadingContent = {
            Icon(
                imageVector = if (isIncoming) Icons.Default.SouthWest else Icons.Default.NorthEast,
                contentDescription = null,
                tint = if (isIncoming) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error
            )
        },
        trailingContent = {
            val gasAmount = tx.effects?.gasUsed ?: tx.gasUsed ?: 0L
            Text(
                text = "${if (isIncoming) "+" else "-"}${
                    String.format(
                        Locale.US,
                        "%.2f",
                        gasAmount / 1_000_000_000.0
                    )
                }",
                color = if (isIncoming) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface
            )
        },
        modifier = Modifier.clickable(onClick = onClick)
    )
}

@Composable
fun TransactionDetailSheet(tx: TransactionDetails, isIncoming: Boolean, onDismiss: () -> Unit) {
    val context = LocalContext.current
    Column(
        modifier = Modifier.fillMaxWidth().padding(16.dp).verticalScroll(rememberScrollState()).navigationBarsPadding(),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Transaction Details", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
        HorizontalDivider()
        DetailRow(label = "Type", value = tx.txType, copyable = false)
        DetailRow(
            label = "Status", value = tx.status, valueColor = when (tx.status.lowercase()) {
                "committed", "success" -> MaterialTheme.colorScheme.primary; "failed" -> MaterialTheme.colorScheme.error; else -> MaterialTheme.colorScheme.onSurface
            }
        )
        DetailRow(label = "Direction", value = if (isIncoming) "Incoming" else "Outgoing")
        CopyableDetailRow(label = "Hash", value = tx.hash)
        CopyableDetailRow(label = "Sender", value = tx.sender)
        tx.senderAddress?.let { CopyableDetailRow(label = "Sender Address", value = it) }
        tx.module?.let { DetailRow(label = "Module", value = it) }
        tx.function?.let { DetailRow(label = "Function", value = it) }
        DetailRow(label = "Nonce", value = tx.nonce.toString())
        DetailRow(label = "Gas Limit", value = tx.gasLimit.toString())
        DetailRow(label = "Gas Price", value = tx.gasPrice.toString())
        tx.gasUsed?.let { DetailRow(label = "Gas Used", value = it.toString()) }
        tx.blockHeight?.let { DetailRow(label = "Block Height", value = it.toString()) }
        tx.checkpointHeight?.let { DetailRow(label = "Checkpoint", value = it.toString()) }
        tx.effects?.let { eff ->
            DetailRow(label = "Effects Status", value = eff.status)
            DetailRow(label = "Effects Gas", value = eff.gasUsed.toString())
            if (!eff.objectChanges.isNullOrEmpty()) {
                Text(
                    "Object Changes (${eff.objectChanges.size})",
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.padding(top = 8.dp)
                )
                eff.objectChanges.take(10).forEach { ch ->
                    Surface(
                        shape = MaterialTheme.shapes.small,
                        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Column(Modifier.padding(8.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                            Text(ch.changeType, style = MaterialTheme.typography.labelMedium)
                            Text(
                                ch.objectRef.objectId,
                                style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                                maxLines = 1
                            )
                            ch.type?.let {
                                Text(
                                    it,
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                        }
                    }
                }
            }
        }
        tx.moduleFunctions?.takeIf { it.isNotEmpty() }?.let { fns ->
            Text(
                "Functions",
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.Bold,
                modifier = Modifier.padding(top = 8.dp)
            )
            fns.take(10).forEach { Text("• $it", style = MaterialTheme.typography.bodySmall) }
        }
        Spacer(Modifier.height(8.dp))
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            OutlinedButton(
                onClick = { copyToClipboard(context, tx.hash, toast = "Hash copied"); },
                modifier = Modifier.weight(1f)
            ) {
                Icon(Icons.Default.ContentCopy, contentDescription = null, modifier = Modifier.size(18.dp))
                Spacer(Modifier.width(8.dp)); Text("Copy Hash")
            }
            Button(onClick = onDismiss, modifier = Modifier.weight(1f)) { Text("Close") }
        }
        Spacer(Modifier.height(12.dp))
    }
}

@Composable
private fun DetailRow(
    label: String,
    value: String,
    valueColor: androidx.compose.ui.graphics.Color = MaterialTheme.colorScheme.onSurface,
    copyable: Boolean = false
) {
    Column(Modifier.fillMaxWidth()) {
        Text(label, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(
            value,
            style = MaterialTheme.typography.bodyMedium.copy(color = valueColor),
            modifier = Modifier.padding(top = 2.dp)
        )
    }
}

@Composable
private fun CopyableDetailRow(label: String, value: String) {
    val context = LocalContext.current
    Column(Modifier.fillMaxWidth()) {
        Text(label, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Row(
            Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Text(
                value,
                style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                modifier = Modifier.weight(1f).padding(top = 2.dp, end = 8.dp),
                maxLines = 3
            )
            IconButton(onClick = { copyToClipboard(context, value) }, modifier = Modifier.size(28.dp)) {
                Icon(
                    Icons.Default.ContentCopy,
                    contentDescription = "Copy",
                    modifier = Modifier.size(16.dp),
                    tint = MaterialTheme.colorScheme.primary
                )
            }
        }
    }
}