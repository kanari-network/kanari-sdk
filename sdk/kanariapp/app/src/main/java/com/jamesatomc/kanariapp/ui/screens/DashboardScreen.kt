package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Public
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.jamesatomc.kanariapp.network.models.TokenBalance
import com.jamesatomc.kanariapp.ui.components.CopyableAddressRow
import com.jamesatomc.kanariapp.ui.components.KanariTopBar
import com.jamesatomc.kanariapp.ui.components.TokenIcon
import com.jamesatomc.kanariapp.wallet.WalletRecord
import com.jamesatomc.kanariapp.wallet.WalletViewModel
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DashboardScreen(
    viewModel: WalletViewModel,
    onNavigateToReceive: () -> Unit,
    onNavigateToSettings: () -> Unit,
    onNavigateToSend: () -> Unit,
    onNavigateToWalletGen: () -> Unit,
    modifier: Modifier = Modifier
) {
    val accountInfo by viewModel.accountInfo.collectAsStateWithLifecycle()
    val tokenBalances by viewModel.tokenBalances.collectAsStateWithLifecycle()
    val wallets by viewModel.wallets.collectAsStateWithLifecycle()
    val activeWallet by viewModel.activeWallet.collectAsStateWithLifecycle()
    val isLoading by viewModel.isLoading.collectAsStateWithLifecycle()
    val environment by viewModel.environment.collectAsStateWithLifecycle()
    val error by viewModel.error.collectAsStateWithLifecycle()
    val pagerState = rememberPagerState(pageCount = { wallets.size + 1 })
    var showEnvDialog by remember { mutableStateOf(false) }
    var walletToDelete by remember { mutableStateOf<WalletRecord?>(null) }
    LaunchedEffect(activeWallet) {
        val idx =
            wallets.indexOfFirst { it.id == activeWallet?.id }; if (idx >= 0 && pagerState.currentPage != idx) pagerState.animateScrollToPage(
        idx
    )
    }
    LaunchedEffect(pagerState.currentPage) { if (pagerState.currentPage < wallets.size) viewModel.switchWallet(wallets[pagerState.currentPage]) }
    Scaffold(
        modifier = modifier,
        topBar = {
            KanariTopBar(
                environmentName = environment.name,
                onEnvClick = { showEnvDialog = true },
                onReceive = onNavigateToReceive,
                onSettings = onNavigateToSettings
            )
        },
        containerColor = MaterialTheme.colorScheme.background
    ) { padding ->
        Column(
            Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            HorizontalPager(
                state = pagerState,
                modifier = Modifier.fillMaxWidth().height(180.dp),
                contentPadding = PaddingValues(horizontal = 16.dp),
                pageSpacing = 12.dp
            ) { page ->
                if (page < wallets.size) WalletCard(
                    wallets[page],
                    if (activeWallet?.id == wallets[page].id) tokenBalances else emptyList()
                ) { walletToDelete = wallets[page] } else AddWalletCard(onNavigateToWalletGen)
            }
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Button(onClick = onNavigateToSend, modifier = Modifier.weight(1f)) { Text("Send") }
                OutlinedButton(onClick = onNavigateToReceive, modifier = Modifier.weight(1f)) { Text("Receive") }
            }
            Text("Assets", style = MaterialTheme.typography.titleMedium)
            error?.let {
                Text(
                    it,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.fillMaxWidth()
                )
            }
            when {
                isLoading && tokenBalances.isEmpty() -> CircularProgressIndicator(Modifier.align(Alignment.CenterHorizontally))
                wallets.isEmpty() -> Text(
                    "No wallets found",
                    Modifier.align(Alignment.CenterHorizontally),
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )

                else -> tokenBalances.forEach { AssetItem(it) }
            }
            val objects = accountInfo?.ownedObjects ?: emptyList()
            if (objects.isNotEmpty()) {
                Text("Objects & NFTs", style = MaterialTheme.typography.titleMedium)
                objects.forEach { ObjectItem(it) }
            }
            Spacer(Modifier.height(24.dp))
        }
    }
    if (showEnvDialog) EnvironmentDialog(
        currentEnv = environment,
        onDismiss = { showEnvDialog = false },
        onSelect = { viewModel.setEnvironment(it); showEnvDialog = false })
    walletToDelete?.let { w ->
        AlertDialog(
            onDismissRequest = { walletToDelete = null },
            title = { Text("Delete Wallet") },
            text = { Text("Are you sure you want to delete '${w.name}'?") },
            confirmButton = {
                TextButton(
                    onClick = { viewModel.deleteWallet(w); walletToDelete = null },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error)
                ) { Text("Delete") }
            },
            dismissButton = { TextButton(onClick = { walletToDelete = null }) { Text("Cancel") } })
    }
}

@Composable
fun WalletCard(wallet: WalletRecord, tokenBalances: List<TokenBalance>, onDelete: () -> Unit) {
    Card(
        Modifier.fillMaxSize(),
        shape = androidx.compose.foundation.shape.RoundedCornerShape(20.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh),
        elevation = CardDefaults.cardElevation(2.dp)
    ) {
        Column(Modifier.fillMaxSize().padding(20.dp)) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Surface(
                    color = MaterialTheme.colorScheme.primary.copy(alpha = 0.12f),
                    shape = androidx.compose.foundation.shape.RoundedCornerShape(8.dp)
                ) {
                    Text(
                        wallet.name,
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.onSurface,
                        modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp)
                    )
                }
                IconButton(onClick = onDelete) {
                    Icon(
                        Icons.Default.Delete,
                        contentDescription = "Delete",
                        modifier = Modifier.size(20.dp),
                        tint = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
            Spacer(Modifier.height(12.dp))
            val t = tokenBalances.find { it.tokenType == "0x2::kanari::KANARI" }
            Text(
                "Balance",
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Text(
                String.format(
                    Locale.US,
                    "%.2f KANARI",
                    (t?.getEffectiveAmount() ?: 0L) / Math.pow(10.0, (t?.decimals ?: 9).toDouble())
                ),
                style = MaterialTheme.typography.headlineMedium.copy(fontWeight = androidx.compose.ui.text.font.FontWeight.Bold)
            )
            Spacer(Modifier.weight(1f))
            Surface(
                color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                shape = androidx.compose.foundation.shape.RoundedCornerShape(12.dp)
            ) {
                Box(Modifier.padding(horizontal = 8.dp, vertical = 2.dp)) {
                    CopyableAddressRow(
                        wallet.address,
                        short = true,
                        textStyle = MaterialTheme.typography.labelMedium
                    )
                }
            }
        }
    }
}

@Composable
fun AddWalletCard(onClick: () -> Unit) {
    Card(
        onClick = onClick,
        Modifier.fillMaxSize(),
        shape = androidx.compose.foundation.shape.RoundedCornerShape(20.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
        border = androidx.compose.foundation.BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
    ) {
        Column(
            Modifier.fillMaxSize(),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            FilledTonalIconButton(onClick = onClick) { Icon(Icons.Default.Add, contentDescription = null) }
            Spacer(Modifier.height(8.dp))
            Text("Add Wallet", style = MaterialTheme.typography.titleSmall)
        }
    }
}

@Composable
fun AssetItem(token: TokenBalance) {
    Card(
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
        shape = androidx.compose.foundation.shape.RoundedCornerShape(16.dp),
        modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp)
    ) {
        ListItem(
            leadingContent = { TokenIcon(token) },
            headlineContent = { Text(token.symbol, style = MaterialTheme.typography.titleSmall) },
            supportingContent = { Text(token.tokenType, maxLines = 1, style = MaterialTheme.typography.bodySmall) },
            trailingContent = {
                val f = token.getEffectiveAmount() / Math.pow(10.0, token.decimals.toDouble())
                Surface(
                    color = MaterialTheme.colorScheme.primaryContainer,
                    shape = androidx.compose.foundation.shape.RoundedCornerShape(12.dp)
                ) {
                    Text(
                        String.format(Locale.US, "%.4f", f),
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.onPrimaryContainer,
                        modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp)
                    )
                }
            },
            colors = ListItemDefaults.colors(containerColor = androidx.compose.ui.graphics.Color.Transparent),
            modifier = Modifier.clickable { }
        )
    }
}

@Composable
fun ObjectItem(obj: com.jamesatomc.kanariapp.network.models.ObjectInfo) {
    ListItem(
        headlineContent = { Text(obj.getEffectiveType().split("::").last()) },
        supportingContent = { Text(obj.id, maxLines = 1) },
        leadingContent = { Icon(Icons.Default.Public, contentDescription = null) })
}
