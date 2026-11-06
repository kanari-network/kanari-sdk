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
                    val tokenType = balance.tokenType
                    if (tokenType.isNullOrBlank()) {
                        sendStatus = "This balance has no token type"
                        isSending = false
                    } else {
                        // One call: the RPC selects the Balance objects that
                        // cover `amount` and joins up to three of them in a
                        // single transaction, like sending a coin.
                        val ok = viewModel.sendKrc1155(to, tokenType, amount)
                        if (ok) {
                            isSending = false
                            sendingGroup = null
                            recipientInput = ""
                            amountInput = ""
                            success = "${amount} sent to ${to.take(10)}…${to.takeLast(6)}"
                        } else {
                            sendStatus = viewModel.error.value ?: "Transfer failed"
                            isSending = false
                        }
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