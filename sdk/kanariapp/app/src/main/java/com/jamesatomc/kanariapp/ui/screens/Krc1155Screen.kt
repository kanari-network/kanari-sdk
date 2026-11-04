@file:OptIn(ExperimentalMaterial3Api::class)

package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import com.jamesatomc.kanariapp.network.models.OwnedKrc1155Balance
import com.jamesatomc.kanariapp.ui.components.LoadingButton
import com.jamesatomc.kanariapp.ui.components.RecipientAddressField
import com.jamesatomc.kanariapp.ui.components.SendCountdownDialog
import com.jamesatomc.kanariapp.ui.components.SendSuccessOverlay
import com.jamesatomc.kanariapp.ui.theme.KanariColors
import com.jamesatomc.kanariapp.wallet.WalletViewModel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

data class Krc1155Group(
    val tokenType: String?,
    val name: String?,
    val imageUrl: String?,
    val maxSupply: Long?,
    val balances: List<OwnedKrc1155Balance>,
) {
    val totalAmount: Long get() = balances.sumOf { it.amount ?: 0L }
}

/**
 * `refreshBalance()` only launches its query, so a fixed delay can still read
 * the previous snapshot. Wait for the refresh cycle to settle before deciding
 * how much each balance object can cover.
 */
private suspend fun awaitBalanceRefresh(viewModel: WalletViewModel) {
    viewModel.refreshBalance()
    val startedAt = System.currentTimeMillis()
    while (!viewModel.isLoading.value && System.currentTimeMillis() - startedAt < 1_000L) {
        delay(50)
    }
    val deadline = System.currentTimeMillis() + 6_000L
    while (viewModel.isLoading.value && System.currentTimeMillis() < deadline) {
        delay(100)
    }
}

@Composable
fun Krc1155Screen(
    viewModel: WalletViewModel,
    onBack: () -> Unit,
    modifier: Modifier = Modifier
) {
    val balances by viewModel.ownedKrc1155Balances.collectAsState()
    val wallets by viewModel.wallets.collectAsState()
    val isLoading by viewModel.isLoading.collectAsState()
    val scope = rememberCoroutineScope()

    val groups = remember(balances) {
        balances.groupBy { it.tokenType ?: it.objectId ?: "" }
            .map { (key, list) ->
                Krc1155Group(
                    tokenType = key,
                    name = list.firstOrNull()?.name,
                    imageUrl = list.firstOrNull()?.imageUrl,
                    maxSupply = list.firstOrNull()?.maxSupply,
                    balances = list,
                )
            }
    }

    var sendingGroup by remember { mutableStateOf<Krc1155Group?>(null) }
    var recipientInput by remember { mutableStateOf("") }
    var amountInput by remember { mutableStateOf("") }
    var isSending by remember { mutableStateOf(false) }
    var sendStatus by remember { mutableStateOf<String?>(null) }
    var pendingSend by remember { mutableStateOf<Triple<String, Long, Krc1155Group>?>(null) }
    var success by remember { mutableStateOf<String?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("KRC-1155 Balances") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { padding ->
        Column(
            Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp)
        ) {
            if (balances.isEmpty()) {
                Text(if (isLoading) "Loading..." else "No KRC-1155 balances found.")
            } else {
                LazyVerticalGrid(
                    columns = GridCells.Fixed(2),
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                    contentPadding = PaddingValues(bottom = 24.dp)
                ) {
                    items(groups, key = { it.tokenType ?: "" }) { group ->
                        Krc1155Card(group) {
                            sendingGroup = group
                            recipientInput = ""
                            amountInput = group.totalAmount.toString()
                            sendStatus = null
                        }
                    }
                }
            }
        }
    }

    sendingGroup?.let { balance ->
        AlertDialog(
            onDismissRequest = { if (!isSending) sendingGroup = null },
            title = { Text("Send KRC-1155", fontWeight = FontWeight.Bold) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    sendStatus?.let {
                        Text(
                            it,
                            style = MaterialTheme.typography.bodySmall,
                            color = if (it.startsWith("Sent")) MaterialTheme.colorScheme.primary
                            else MaterialTheme.colorScheme.error
                        )
                    }
                    Text("Total available: ${balance.totalAmount}")
                    RecipientAddressField(
                        value = recipientInput,
                        onValueChange = { recipientInput = it },
                        wallets = wallets,
                        modifier = Modifier.fillMaxWidth()
                    )
                    OutlinedTextField(
                        value = amountInput,
                        onValueChange = { amountInput = it.filter { c -> c.isDigit() } },
                        label = { Text("Amount") },
                        singleLine = true,
                        enabled = !isSending,
                        modifier = Modifier.fillMaxWidth()
                    )
                }
            },
            confirmButton = {
                LoadingButton(
                    onClick = {
                        val recipient = recipientInput.trim()
                        val amount = amountInput.toLongOrNull() ?: 0L
                        if (recipient.isBlank() || !recipient.startsWith("0x")) {
                            sendStatus = "Enter a valid 0x address"
                        } else if (amount <= 0) {
                            sendStatus = "Enter a valid amount"
                        } else {
                            pendingSend = Triple(recipient, amount, balance)
                        }
                    },
                    text = "Send",
                    icon = Icons.AutoMirrored.Filled.Send,
                    isLoading = isSending,
                    enabled = !isSending,
                    colors = ButtonDefaults.buttonColors(
                        containerColor = KanariColors.Lime,
                        contentColor = KanariColors.Ink
                    )
                )
            },
            dismissButton = {
                TextButton(onClick = { if (!isSending) sendingGroup = null }) { Text("Cancel") }
            }
        )
    }

    pendingSend?.let { (to, amount, balance) ->
        SendCountdownDialog(
            title = "Confirm KRC-1155 send",
            detail = "Sending ${amount} of ${balance.name ?: balance.tokenType ?: ""} to " +
                    "${to.take(12)}…${to.takeLast(6)}",
            onCancel = { pendingSend = null },
            onConfirmed = {
                pendingSend = null
                isSending = true
                sendStatus = null
                scope.launch {
                    awaitBalanceRefresh(viewModel)
                    val fresh = viewModel.ownedKrc1155Balances.value
                    val grp = fresh.groupBy { it.tokenType ?: it.objectId ?: "" }
                        .map { (k, l) ->
                            Krc1155Group(
                                k,
                                l.firstOrNull()?.name,
                                l.firstOrNull()?.imageUrl,
                                l.firstOrNull()?.maxSupply,
                                l
                            )
                        }
                        .firstOrNull { it.tokenType == balance.tokenType } ?: balance
                    // The RPC groups KRC-1155 balances by token type: `amount` is
                    // the total over every Balance object while `balances` lists
                    // each object with its own amount. Send object by object - one
                    // object rarely holds the whole total, and krc_1155::split
                    // aborts with E_INSUFFICIENT_BALANCE for anything larger.
                    val sources = grp.balances
                        .flatMap { entry ->
                            val parts = entry.balances.orEmpty()
                                .map { part -> part.objectId to (part.amount ?: 0L) }
                                .filter { it.second > 0 }
                            if (parts.isNotEmpty()) parts
                            else listOfNotNull(entry.objectId?.let { it to (entry.amount ?: 0L) })
                        }
                        .distinctBy { it.first }
                        .sortedByDescending { it.second }
                    var remaining = amount
                    var ok = true
                    for ((oid, available) in sources) {
                        if (remaining <= 0) break
                        val chunk = minOf(available, remaining)
                        if (chunk <= 0) continue
                        val success = viewModel.sendKrc1155(to, oid, chunk)
                        if (!success) {
                            ok = false; break
                        }
                        remaining -= chunk
                    }
                    if (ok && remaining <= 0) {
                        isSending = false
                        sendingGroup = null
                        recipientInput = ""
                        amountInput = ""
                        success = "${amount} sent to ${to.take(10)}…${to.takeLast(6)}"
                    } else {
                        val sent = amount - remaining
                        sendStatus = when {
                            !ok -> viewModel.error.value ?: "Transfer failed"
                            sent <= 0 -> "Insufficient balance: ${amount} is more than this wallet's KRC-1155 balances hold. Send a smaller amount."
                            else -> "Sent ${sent} of ${amount}. Refresh and send the remaining ${remaining}."
                        }
                        isSending = false
                    }
                }
            },
        )
    }

    success?.let { message ->
        SendSuccessOverlay(
            summary = message,
            onDone = { success = null },
        )
    }
}

@Composable
private fun Krc1155Card(group: Krc1155Group, onClick: () -> Unit) {
    Card(
        onClick = onClick,
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
        modifier = Modifier.fillMaxWidth()
    ) {
        Column {
            val img = group.imageUrl
            if (!img.isNullOrBlank()) {
                AsyncImage(
                    model = img,
                    contentDescription = group.name,
                    contentScale = ContentScale.Crop,
                    modifier = Modifier.fillMaxWidth().aspectRatio(1f)
                )
            } else {
                Box(
                    Modifier.fillMaxWidth().aspectRatio(1f),
                    contentAlignment = Alignment.Center
                ) {
                    Text("1155", style = MaterialTheme.typography.headlineMedium)
                }
            }
            Column(Modifier.padding(10.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(
                    group.name ?: "KRC-1155",
                    style = MaterialTheme.typography.titleSmall.copy(fontWeight = FontWeight.Bold),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
                Text(
                    "Total: ${group.totalAmount}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                if (group.maxSupply != null) {
                    Text(
                        "Max supply: ${group.maxSupply}",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        }
    }
}