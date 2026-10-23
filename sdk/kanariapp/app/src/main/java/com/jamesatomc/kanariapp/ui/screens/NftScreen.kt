@file:OptIn(ExperimentalMaterial3Api::class)

package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.CollectionsBookmark
import androidx.compose.material.icons.filled.Image
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import coil3.compose.AsyncImage
import com.jamesatomc.kanariapp.network.models.OwnedNft
import com.jamesatomc.kanariapp.ui.components.ErrorBanner
import com.jamesatomc.kanariapp.ui.components.LoadingButton
import com.jamesatomc.kanariapp.ui.theme.KanariColors
import com.jamesatomc.kanariapp.wallet.WalletViewModel
import kotlinx.coroutines.launch

/**
 * "My NFTs": every NFT-like object owned by the active wallet, with a detail
 * + send flow. Sending builds the canonical
 * `0x2::transfer::public_transfer<T>(obj, recipient)` call server-side and
 * signs it locally, so both key wallets and zkLogin sessions can transfer.
 */
@Composable
fun NftScreen(
    viewModel: WalletViewModel,
    onBack: () -> Unit,
    modifier: Modifier = Modifier
) {
    val nfts by viewModel.ownedNfts.collectAsStateWithLifecycle()
    val activeWallet by viewModel.activeWallet.collectAsStateWithLifecycle()
    val wallets by viewModel.wallets.collectAsStateWithLifecycle()
    val isLoading by viewModel.isLoading.collectAsStateWithLifecycle()
    val error by viewModel.error.collectAsStateWithLifecycle()
    val scope = androidx.compose.runtime.rememberCoroutineScope()

    var selectedNft by remember { mutableStateOf<OwnedNft?>(null) }
    var sendingNft by remember { mutableStateOf<OwnedNft?>(null) }
    var recipientInput by remember { mutableStateOf("") }
    var isSending by remember { mutableStateOf(false) }
    var sendStatus by remember { mutableStateOf<String?>(null) }
    var walletMenuExpanded by remember { mutableStateOf(false) }

    Scaffold(
        modifier = modifier,
        topBar = {
            TopAppBar(
                title = { Text("My NFTs") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.background
                )
            )
        },
        containerColor = MaterialTheme.colorScheme.background
    ) { padding ->
        Column(
            Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp)
        ) {
            if (wallets.size > 1) {
                Box {
                    TextButton(onClick = { walletMenuExpanded = true }) {
                        Text(
                            "Wallet: ${activeWallet?.name ?: "Select wallet"} · ${activeWallet?.address?.takeLast(8) ?: ""}"
                        )
                    }
                    DropdownMenu(
                        expanded = walletMenuExpanded,
                        onDismissRequest = { walletMenuExpanded = false }
                    ) {
                        wallets.forEach { wallet ->
                            DropdownMenuItem(
                                text = {
                                    Text("${wallet.name} · ${wallet.address.takeLast(8)}")
                                },
                                onClick = {
                                    walletMenuExpanded = false
                                    selectedNft = null
                                    sendingNft = null
                                    sendStatus = null
                                    viewModel.switchWallet(wallet)
                                }
                            )
                        }
                    }
                }
            }
            error?.let { ErrorBanner(error = it) }

            when {
                activeWallet == null -> {
                    EmptyState(text = "No wallet selected — create or unlock a wallet first")
                }

                isLoading && nfts.isEmpty() -> {
                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        androidx.compose.material3.CircularProgressIndicator()
                    }
                }

                nfts.isEmpty() -> {
                    EmptyState(text = "No NFTs in this wallet yet")
                }

                else -> {
                    Text(
                        "${nfts.size} item${if (nfts.size == 1) "" else "s"}",
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Spacer(Modifier.height(8.dp))
                    LazyVerticalGrid(
                        columns = GridCells.Fixed(2),
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp),
                        contentPadding = PaddingValues(bottom = 24.dp)
                    ) {
                        items(nfts, key = { it.objectId }) { nft ->
                            NftCard(nft) { selectedNft = nft }
                        }
                    }
                }
            }
        }
    }

    // Detail view
    selectedNft?.let { nft ->
        AlertDialog(
            onDismissRequest = { selectedNft = null },
            containerColor = MaterialTheme.colorScheme.surfaceContainerHigh,
            shape = RoundedCornerShape(28.dp),
            title = {
                Text(
                    nft.displayName,
                    style = MaterialTheme.typography.titleLarge.copy(fontWeight = FontWeight.Bold),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
            },
            text = {
                Column(
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                    modifier = Modifier.fillMaxWidth().verticalScroll(rememberScrollState())
                ) {
                    Card(
                        shape = RoundedCornerShape(20.dp),
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHighest),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        NftImage(nft, Modifier.fillMaxWidth().aspectRatio(1f))
                    }

                    Surface(
                        color = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.2f),
                        shape = RoundedCornerShape(8.dp)
                    ) {
                        Text(
                            nft.shortType,
                            style = MaterialTheme.typography.labelMedium.copy(
                                fontFamily = FontFamily.Monospace,
                                fontWeight = FontWeight.SemiBold
                            ),
                            color = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                        )
                    }

                    nft.parsed?.description?.takeIf { it.isNotBlank() }?.let { desc ->
                        Text(
                            desc,
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurface
                        )
                    }

                    if (nft.attributes.isNotEmpty()) {
                        Text(
                            "Attributes",
                            style = MaterialTheme.typography.labelLarge.copy(fontWeight = FontWeight.Bold),
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Surface(
                            color = MaterialTheme.colorScheme.surfaceContainer,
                            shape = RoundedCornerShape(16.dp),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Column(
                                modifier = Modifier.padding(12.dp),
                                verticalArrangement = Arrangement.spacedBy(6.dp)
                            ) {
                                nft.attributes.forEach { (key, value) ->
                                    Row(
                                        Modifier.fillMaxWidth(),
                                        horizontalArrangement = Arrangement.SpaceBetween
                                    ) {
                                        Text(
                                            key,
                                            style = MaterialTheme.typography.bodySmall.copy(fontWeight = FontWeight.SemiBold),
                                            color = MaterialTheme.colorScheme.onSurfaceVariant
                                        )
                                        Text(
                                            value,
                                            style = MaterialTheme.typography.bodySmall.copy(fontWeight = FontWeight.Bold),
                                            color = MaterialTheme.colorScheme.onSurface
                                        )
                                    }
                                }
                            }
                        }
                    }

                    Surface(
                        color = MaterialTheme.colorScheme.surfaceContainerHighest,
                        shape = RoundedCornerShape(12.dp),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Column(Modifier.padding(10.dp)) {
                            Text(
                                "Object ID",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                            Spacer(Modifier.height(2.dp))
                            Text(
                                nft.objectId,
                                style = MaterialTheme.typography.bodySmall.copy(
                                    fontFamily = FontFamily.Monospace,
                                    fontWeight = FontWeight.Medium
                                ),
                                color = MaterialTheme.colorScheme.primary,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis
                            )
                        }
                    }
                }
            },
            confirmButton = {
                LoadingButton(
                    onClick = {
                        sendingNft = nft
                        sendStatus = null
                        selectedNft = null
                    },
                    text = "Send",
                    icon = Icons.AutoMirrored.Filled.Send,
                    colors = ButtonDefaults.buttonColors(
                        containerColor = KanariColors.Lime,
                        contentColor = KanariColors.Ink
                    ),
                    modifier = Modifier.height(48.dp)
                )
            },
            dismissButton = {
                TextButton(onClick = { selectedNft = null }) {
                    Text("Close", fontWeight = FontWeight.SemiBold)
                }
            }
        )
    }

    // Send flow
    sendingNft?.let { nft ->
        AlertDialog(
            onDismissRequest = { if (!isSending) sendingNft = null },
            title = { Text("Send NFT", fontWeight = FontWeight.Bold) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        "Send \"${nft.displayName}\" to another address. The transfer is final once confirmed.",
                        style = MaterialTheme.typography.bodyMedium
                    )
                    sendStatus?.let {
                        Text(
                            it,
                            style = MaterialTheme.typography.bodySmall,
                            color = if (it.startsWith("Sent")) MaterialTheme.colorScheme.primary
                            else MaterialTheme.colorScheme.error
                        )
                    }
                    OutlinedTextField(
                        value = recipientInput,
                        onValueChange = { recipientInput = it.trimStart() },
                        label = { Text("Recipient address") },
                        placeholder = { Text("0x…") },
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
                        if (recipient.isBlank() || !recipient.startsWith("0x")) {
                            sendStatus = "Enter a valid 0x address"
                        } else {
                            val target: OwnedNft = nft
                            isSending = true
                            sendStatus = null
                            scope.launch {
                                if (viewModel.sendNft(recipient, target.objectId)) {
                                    sendStatus = "Sent! The NFT is on its way."
                                    isSending = false
                                    sendingNft = null
                                    recipientInput = ""
                                } else {
                                    sendStatus = viewModel.error.value ?: "Transfer failed"
                                    isSending = false
                                }
                            }
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
                TextButton(onClick = { if (!isSending) sendingNft = null }) { Text("Cancel") }
            }
        )
    }
}

@Composable
private fun NftCard(nft: OwnedNft, onClick: () -> Unit) {
    Card(
        onClick = onClick,
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
        modifier = Modifier.fillMaxWidth()
    ) {
        Column {
            NftImage(nft, Modifier.fillMaxWidth().aspectRatio(1f).clip(RoundedCornerShape(16.dp)))
            Column(Modifier.padding(10.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(
                    nft.displayName,
                    style = MaterialTheme.typography.titleSmall.copy(fontWeight = FontWeight.Bold),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
                Text(
                    nft.shortType,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
            }
        }
    }
}

@Composable
private fun NftImage(nft: OwnedNft, modifier: Modifier = Modifier) {
    val url = nft.imageUrl
    Box(modifier.background(MaterialTheme.colorScheme.surfaceVariant)) {
        if (url != null) {
            AsyncImage(
                model = url,
                contentDescription = nft.displayName,
                contentScale = ContentScale.Crop,
                modifier = Modifier.fillMaxSize()
            )
        } else {
            Icon(
                Icons.Default.Image,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f),
                modifier = Modifier.align(Alignment.Center).size(40.dp)
            )
        }
    }
}

@Composable
private fun EmptyState(text: String) {
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Icon(
                Icons.Default.CollectionsBookmark,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f),
                modifier = Modifier.size(64.dp)
            )
            Spacer(Modifier.height(12.dp))
            Text(
                text,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}
