package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowDropDown
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Public
import androidx.compose.material.icons.filled.QrCode
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.jamesatomc.kanariapp.network.models.AccountInfo
import com.jamesatomc.kanariapp.network.models.TokenBalance
import com.jamesatomc.kanariapp.ui.components.CopyableAddressRow
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
        val index = wallets.indexOfFirst { it.id == activeWallet?.id }
        if (index >= 0 && pagerState.currentPage != index) {
            pagerState.animateScrollToPage(index)
        }
    }

    LaunchedEffect(pagerState.currentPage) {
        if (pagerState.currentPage < wallets.size) {
            viewModel.switchWallet(wallets[pagerState.currentPage])
        }
    }

    Scaffold(
        modifier = modifier,
        topBar = {
            TopAppBar(
                title = {
                    Row(
                        modifier = Modifier.clickable { showEnvDialog = true },
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Text("Kanari Wallet", style = MaterialTheme.typography.titleLarge, fontWeight = androidx.compose.ui.text.font.FontWeight.Bold)
                        Surface(
                            color = when (environment.name.lowercase()) {
                                "dev" -> MaterialTheme.colorScheme.tertiaryContainer
                                "mainnet" -> MaterialTheme.colorScheme.errorContainer
                                else -> MaterialTheme.colorScheme.secondaryContainer
                            },
                            shape = RoundedCornerShape(6.dp)
                        ) {
                            Text(
                                environment.name.uppercase(),
                                style = MaterialTheme.typography.labelSmall.copy(fontWeight = androidx.compose.ui.text.font.FontWeight.Bold),
                                color = when (environment.name.lowercase()) {
                                    "dev" -> MaterialTheme.colorScheme.onTertiaryContainer
                                    "mainnet" -> MaterialTheme.colorScheme.onErrorContainer
                                    else -> MaterialTheme.colorScheme.onSecondaryContainer
                                },
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp)
                            )
                        }
                        Icon(Icons.Default.ArrowDropDown, contentDescription = null, modifier = Modifier.size(18.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                },
                actions = {
                    IconButton(onClick = onNavigateToReceive) {
                        Icon(Icons.Default.QrCode, contentDescription = "Receive")
                    }
                    IconButton(onClick = onNavigateToSettings) {
                        Icon(Icons.Default.Settings, contentDescription = "Settings")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface,
                    scrolledContainerColor = MaterialTheme.colorScheme.surfaceContainer
                )
            )
        },
        containerColor = MaterialTheme.colorScheme.background
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            HorizontalPager(
                state = pagerState,
                modifier = Modifier
                    .fillMaxWidth()
                    .height(180.dp),
                contentPadding = PaddingValues(horizontal = 16.dp),
                pageSpacing = 12.dp
            ) { page ->
                if (page < wallets.size) {
                    val wallet = wallets[page]
                    WalletCard(
                        wallet = wallet,
                        tokenBalances = if (activeWallet?.id == wallet.id) tokenBalances else emptyList(),
                        onDelete = { walletToDelete = wallet }
                    )
                } else {
                    AddWalletCard(onClick = onNavigateToWalletGen)
                }
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                Button(
                    onClick = onNavigateToSend,
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Send")
                }
                OutlinedButton(
                    onClick = onNavigateToReceive,
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Receive")
                }
            }

            Text(
                text = "Assets",
                style = MaterialTheme.typography.titleMedium
            )

            error?.let {
                Text(
                    text = it,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.fillMaxWidth()
                )
            }

            if (isLoading && tokenBalances.isEmpty()) {
                CircularProgressIndicator(modifier = Modifier.align(Alignment.CenterHorizontally))
            } else if (wallets.isEmpty()) {
                Text(
                    "No wallets found",
                    modifier = Modifier.align(Alignment.CenterHorizontally),
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            } else {
                tokenBalances.forEach { token ->
                    AssetItem(token)
                }
            }

            val objects = accountInfo?.ownedObjects ?: emptyList()
            if (objects.isNotEmpty()) {
                Text(
                    text = "Objects & NFTs",
                    style = MaterialTheme.typography.titleMedium
                )
                objects.forEach { obj ->
                    ObjectItem(obj)
                }
            }

            Spacer(modifier = Modifier.height(24.dp))
        }
    }

    if (showEnvDialog) {
        EnvironmentDialog(
            currentEnv = environment,
            onDismiss = { showEnvDialog = false },
            onSelect = { env ->
                viewModel.setEnvironment(env)
                showEnvDialog = false
            }
        )
    }

    walletToDelete?.let { wallet ->
        AlertDialog(
            onDismissRequest = { walletToDelete = null },
            title = { Text("Delete Wallet") },
            text = { Text("Are you sure you want to delete '${wallet.name}'?") },
            confirmButton = {
                TextButton(
                    onClick = {
                        viewModel.deleteWallet(wallet)
                        walletToDelete = null
                    },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error)
                ) {
                    Text("Delete")
                }
            },
            dismissButton = {
                TextButton(onClick = { walletToDelete = null }) {
                    Text("Cancel")
                }
            }
        )
    }
}

@Composable
fun WalletCard(wallet: WalletRecord, tokenBalances: List<TokenBalance>, onDelete: () -> Unit) {
    Card(
        modifier = Modifier.fillMaxSize(),
        shape = androidx.compose.foundation.shape.RoundedCornerShape(20.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp)
    ) {
        Column(modifier = Modifier.fillMaxSize().padding(20.dp)) {
            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Surface(color = MaterialTheme.colorScheme.primary.copy(alpha = 0.12f), shape = androidx.compose.foundation.shape.RoundedCornerShape(8.dp)) {
                    Text(wallet.name, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurface, modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp))
                }
                IconButton(onClick = onDelete) { Icon(Icons.Default.Delete, contentDescription = "Delete", modifier = Modifier.size(20.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant) }
            }
            Spacer(Modifier.height(12.dp))
            val kanariToken = tokenBalances.find { it.tokenType == "0x2::kanari::KANARI" }
            val balance = kanariToken?.getEffectiveAmount() ?: 0L
            val decimals = kanariToken?.decimals ?: 9
            Text("Balance", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Text(String.format(Locale.US, "%.2f KANARI", balance / Math.pow(10.0, decimals.toDouble())), style = MaterialTheme.typography.headlineMedium.copy(fontWeight = androidx.compose.ui.text.font.FontWeight.Bold), color = MaterialTheme.colorScheme.onSurface)
            Spacer(Modifier.weight(1f))
            Surface(color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f), shape = androidx.compose.foundation.shape.RoundedCornerShape(12.dp)) {
                Box(Modifier.padding(horizontal = 8.dp, vertical = 2.dp)) { CopyableAddressRow(address = wallet.address, short = true, textStyle = MaterialTheme.typography.labelMedium) }
            }
        }
    }
}

@Composable
fun AddWalletCard(onClick: () -> Unit) {
    Card(
        onClick = onClick,
        modifier = Modifier.fillMaxSize(),
        shape = androidx.compose.foundation.shape.RoundedCornerShape(20.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
        border = androidx.compose.foundation.BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
    ) {
        Column(modifier = Modifier.fillMaxSize(), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.Center) {
            FilledTonalIconButton(onClick = onClick) { Icon(Icons.Default.Add, contentDescription = null) }
            Spacer(Modifier.height(8.dp))
            Text("Add Wallet", style = MaterialTheme.typography.titleSmall)
        }
    }
}

@Composable
fun AssetItem(token: TokenBalance) {
    Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer), shape = androidx.compose.foundation.shape.RoundedCornerShape(16.dp), modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
        ListItem(
            headlineContent = { Text(token.symbol, style = MaterialTheme.typography.titleSmall) },
            supportingContent = { Text(token.tokenType, maxLines = 1, style = MaterialTheme.typography.bodySmall) },
            trailingContent = {
                val formatted = token.getEffectiveAmount() / Math.pow(10.0, token.decimals.toDouble())
                Surface(color = MaterialTheme.colorScheme.primaryContainer, shape = androidx.compose.foundation.shape.RoundedCornerShape(12.dp)) {
                    Text(String.format(Locale.US, "%.4f", formatted), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onPrimaryContainer, modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp))
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
        leadingContent = { Icon(Icons.Default.Public, contentDescription = null) }
    )
}
