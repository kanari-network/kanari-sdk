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
                    Column(modifier = Modifier.clickable { showEnvDialog = true }) {
                        Text("Kanari Wallet", style = MaterialTheme.typography.titleLarge)
                        Text(
                            environment.name,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.primary
                        )
                    }
                },
                actions = {
                    IconButton(onClick = onNavigateToReceive) {
                        Icon(Icons.Default.QrCode, contentDescription = "Receive")
                    }
                    IconButton(onClick = onNavigateToSettings) {
                        Icon(Icons.Default.Settings, contentDescription = "Settings")
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
    ElevatedCard(modifier = Modifier.fillMaxSize()) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(wallet.name, style = MaterialTheme.typography.titleMedium)
                IconButton(onClick = onDelete) {
                    Icon(
                        Icons.Default.Delete,
                        contentDescription = "Delete",
                        modifier = Modifier.size(20.dp)
                    )
                }
            }
            val kanariToken = tokenBalances.find { it.tokenType == "0x2::kanari::KANARI" }
            val balance = kanariToken?.getEffectiveAmount() ?: 0L
            val decimals = kanariToken?.decimals ?: 9
            Text(
                String.format(Locale.US, "%.2f KANARI", balance / Math.pow(10.0, decimals.toDouble())),
                style = MaterialTheme.typography.headlineMedium
            )
            Spacer(Modifier.weight(1f))
            CopyableAddressRow(address = wallet.address, short = true)
        }
    }
}

@Composable
fun AddWalletCard(onClick: () -> Unit) {
    OutlinedCard(
        onClick = onClick,
        modifier = Modifier.fillMaxSize()
    ) {
        Column(
            modifier = Modifier.fillMaxSize(),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            Icon(Icons.Default.Add, contentDescription = null)
            Text("Add Wallet")
        }
    }
}

@Composable
fun AssetItem(token: TokenBalance) {
    ListItem(
        headlineContent = { Text(token.symbol) },
        supportingContent = { Text(token.tokenType, maxLines = 1) },
        trailingContent = {
            val formatted = token.getEffectiveAmount() / Math.pow(10.0, token.decimals.toDouble())
            Text(String.format(Locale.US, "%.4f", formatted))
        },
        modifier = Modifier.clickable { /* Detail */ }
    )
}

@Composable
fun ObjectItem(obj: com.jamesatomc.kanariapp.network.models.ObjectInfo) {
    ListItem(
        headlineContent = { Text(obj.getEffectiveType().split("::").last()) },
        supportingContent = { Text(obj.id, maxLines = 1) },
        leadingContent = { Icon(Icons.Default.Public, contentDescription = null) }
    )
}
