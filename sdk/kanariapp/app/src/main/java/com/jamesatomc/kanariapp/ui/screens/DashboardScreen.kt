@file:OptIn(ExperimentalMaterial3Api::class, androidx.compose.animation.ExperimentalAnimationApi::class)

package com.jamesatomc.kanariapp.ui.screens

import android.content.Context
import android.content.ContextWrapper
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
import androidx.compose.material.icons.filled.Security
import androidx.compose.material.icons.filled.VerifiedUser
import androidx.compose.material.icons.filled.Visibility
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.jamesatomc.kanariapp.ui.components.CopyableAddressRow
import com.jamesatomc.kanariapp.ui.components.CurveBadge
import com.jamesatomc.kanariapp.ui.components.DetailSectionCard
import com.jamesatomc.kanariapp.ui.components.FullScreenDialog
import com.jamesatomc.kanariapp.ui.components.IsometricCoinStack
import com.jamesatomc.kanariapp.ui.components.IsometricGlobeChain
import com.jamesatomc.kanariapp.ui.components.IsometricShieldLock
import com.jamesatomc.kanariapp.ui.components.KanariTopBar
import com.jamesatomc.kanariapp.ui.components.LoadingEmptyState
import com.jamesatomc.kanariapp.ui.components.PinVerificationContent
import com.jamesatomc.kanariapp.ui.components.SecretRevealCard
import com.jamesatomc.kanariapp.ui.components.SecurityWarningCard
import com.jamesatomc.kanariapp.ui.components.TokenIcon
import com.jamesatomc.kanariapp.ui.components.copyToClipboard
import com.jamesatomc.kanariapp.ui.components.findFragmentActivity
import com.jamesatomc.kanariapp.ui.components.formatAmount
import com.jamesatomc.kanariapp.ui.components.formatMist
import com.jamesatomc.kanariapp.ui.components.getCurveInfo
import com.jamesatomc.kanariapp.ui.components.rememberBiometricAvailable
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
    var walletForDetails by remember { mutableStateOf<WalletRecord?>(null) }
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
                if (page < wallets.size) {
                    val w = wallets[page]
                    Box(
                        modifier = Modifier.graphicsLayer {
                            val pageOffset = (pagerState.currentPage - page) + pagerState.currentPageOffsetFraction
                            val scale = androidx.compose.ui.util.lerp(
                                0.88f,
                                1f,
                                1f - kotlin.math.abs(pageOffset).coerceIn(0f, 1f)
                            )
                            scaleX = scale; scaleY = scale
                            alpha = androidx.compose.ui.util.lerp(
                                0.6f,
                                1f,
                                1f - kotlin.math.abs(pageOffset).coerceIn(0f, 1f)
                            )
                        }
                    ) {
                        WalletCard(
                            w,
                            if (activeWallet?.id == w.id) tokenBalances else emptyList(),
                            onDelete = { walletToDelete = w },
                            onViewDetails = { walletForDetails = w })
                    }
                } else AddWalletCard(onNavigateToWalletGen)
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
    walletForDetails?.let { w ->
        FullScreenDialog(onDismiss = { walletForDetails = null }) {
            WalletDetailFullScreen(wallet = w, viewModel = viewModel, onDismiss = { walletForDetails = null })
        }
    }
}

@Composable
fun WalletCard(
    wallet: WalletRecord,
    tokenBalances: List<com.jamesatomc.kanariapp.network.models.TokenBalance>,
    onDelete: () -> Unit,
    onViewDetails: () -> Unit
) {
    val curveInfo = remember(wallet.curveType) { getCurveInfo(wallet.curveType) }
    Card(
        Modifier.fillMaxSize(),
        shape = androidx.compose.foundation.shape.RoundedCornerShape(20.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh),
        elevation = CardDefaults.cardElevation(2.dp)
    ) {
        Box(Modifier.fillMaxSize()) {
            IsometricCoinStack(
                modifier = Modifier
                    .size(100.dp)
                    .align(Alignment.BottomEnd)
                    .padding(end = 8.dp, bottom = 8.dp)
                    .graphicsLayer { alpha = 0.15f }
            )
            Column(Modifier.fillMaxSize().padding(16.dp)) {
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
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        IconButton(
                            onClick = onViewDetails,
                            modifier = Modifier.size(32.dp)
                        ) {
                            Icon(
                                Icons.Default.Visibility,
                                contentDescription = "View details",
                                modifier = Modifier.size(20.dp),
                                tint = MaterialTheme.colorScheme.primary
                            )
                        }
                        IconButton(onClick = onDelete, modifier = Modifier.size(32.dp)) {
                            Icon(
                                Icons.Default.Delete,
                                contentDescription = "Delete",
                                modifier = Modifier.size(18.dp),
                                tint = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                }
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    modifier = Modifier.padding(top = 6.dp)
                ) {
                    CurveBadge(curveInfo)
                }
                Spacer(Modifier.height(8.dp))
                val t = tokenBalances.find { it.tokenType == "0x2::kanari::KANARI" }
                Text(
                    "Balance",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Text(
                    "${formatAmount(t?.getEffectiveAmount() ?: 0L, t?.decimals ?: 9, 2)} KANARI",
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
            Spacer(Modifier.height(8.dp)); Text("Add Wallet", style = MaterialTheme.typography.titleSmall)
        }
    }
}

@Composable
fun AssetItem(token: com.jamesatomc.kanariapp.network.models.TokenBalance) {
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
                Surface(
                    color = MaterialTheme.colorScheme.primaryContainer,
                    shape = androidx.compose.foundation.shape.RoundedCornerShape(12.dp)
                ) {
                    Text(
                        formatAmount(token.getEffectiveAmount(), token.decimals),
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.onPrimaryContainer,
                        modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp)
                    )
                }
            },
            colors = ListItemDefaults.colors(containerColor = androidx.compose.ui.graphics.Color.Transparent)
        )
    }
}

@Composable
fun ObjectItem(obj: com.jamesatomc.kanariapp.network.models.ObjectInfo) {
    ListItem(
        headlineContent = { Text(obj.getEffectiveType().split("::").last()) },
        supportingContent = {
            Text(
                obj.id,
                maxLines = 1,
                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis
            )
        },
        leadingContent = { Icon(Icons.Default.Public, contentDescription = null) })
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WalletDetailFullScreen(wallet: WalletRecord, viewModel: WalletViewModel, onDismiss: () -> Unit) {
    val ctx = LocalContext.current
    val activity = ctx.findFragmentActivity()
    var isVerified by remember { mutableStateOf(false) }
    var revealedKey by remember { mutableStateOf<String?>(null) }
    var revealedSeed by remember { mutableStateOf<String?>(null) }
    var keyVisible by remember { mutableStateOf(false) }
    var seedVisible by remember { mutableStateOf(false) }
    val hasSeed = wallet.mnemonicEncrypted != null
    val curveInfo = remember(wallet.curveType) { getCurveInfo(wallet.curveType) }
    val canUseBiometric = rememberBiometricAvailable(viewModel)

    fun onBiometric() {
        if (activity == null) return
        com.jamesatomc.kanariapp.ui.components.showBiometricPrompt(
            activity = activity,
            title = "Reveal Wallet Secrets",
            subtitle = "Use biometrics to unlock",
            onSuccess = {
                val k = viewModel.revealPrivateKeyWithBiometric(wallet)
                if (k != null) {
                    revealedKey = k
                    revealedSeed = if (hasSeed) viewModel.revealMnemonicWithBiometric(wallet) else null
                    isVerified = true
                }
            }
        )
    }
    Scaffold(topBar = {
        TopAppBar(
            title = { Text("Wallet Details") },
            navigationIcon = {
                IconButton(onClick = onDismiss) {
                    Icon(
                        Icons.AutoMirrored.Filled.ArrowBack,
                        contentDescription = "Back"
                    )
                }
            },
            colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background)
        )
    }, containerColor = MaterialTheme.colorScheme.background) { padding ->
        if (!isVerified) {
            PinVerificationContent(
                title = "Enter PIN",
                subtitle = "Enter 6-digit PIN to reveal secrets",
                onVerify = { pin -> viewModel.verifyPin(pin) },
                onSuccess = { pin ->
                    val k = viewModel.revealPrivateKey(wallet, pin)
                    if (k != null) {
                        revealedKey = k
                        revealedSeed = if (hasSeed) viewModel.revealMnemonic(wallet, pin) else null
                        isVerified = true
                    }
                },
                biometricEnabled = canUseBiometric,
                onBiometric = ::onBiometric,
                modifier = Modifier.fillMaxSize().padding(padding)
            )
        } else {
            Column(
                Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()).padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp)
            ) {
                DetailSectionCard {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        CurveBadge(curveInfo)
                    }
                    Text(
                        curveInfo.displayName,
                        style = MaterialTheme.typography.titleMedium.copy(
                            fontFamily = FontFamily.Monospace,
                            fontWeight = androidx.compose.ui.text.font.FontWeight.Bold
                        )
                    )
                    Text(
                        curveInfo.description,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Text(
                        "Curve: ${wallet.curveType}",
                        style = MaterialTheme.typography.labelSmall.copy(fontFamily = FontFamily.Monospace),
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
                DetailSectionCard {
                    Text("Address", style = MaterialTheme.typography.titleSmall)
                    Text(
                        wallet.address,
                        style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                        color = MaterialTheme.colorScheme.onSurface
                    )
                    Button(onClick = {
                        copyToClipboard(ctx, wallet.address, toast = "Address copied")
                    }, modifier = Modifier.fillMaxWidth()) {
                        Icon(
                            Icons.Default.Public,
                            contentDescription = null,
                            modifier = Modifier.size(18.dp)
                        ); Spacer(Modifier.width(8.dp)); Text("Copy Address")
                    }
                }
                Box(
                    modifier = Modifier.fillMaxWidth(),
                    contentAlignment = Alignment.Center
                ) {
                    IsometricShieldLock(
                        modifier = Modifier.size(120.dp)
                    )
                }
                DetailSectionCard {
                    SecretRevealCard(
                        title = "Private Key",
                        secret = revealedKey,
                        isVisible = keyVisible,
                        onToggleVisibility = { keyVisible = !keyVisible },
                        onCopy = {
                            copyToClipboard(
                                ctx,
                                revealedKey!!,
                                label = "Private Key",
                                toast = "Private key copied"
                            )
                        }
                    )
                    SecurityWarningCard("Never share your private key")
                }
                DetailSectionCard {
                    if (!hasSeed) {
                        Text(
                            "This wallet was imported from private key - no seed phrase available",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    } else {
                        SecretRevealCard(
                            title = "Seed Phrase",
                            secret = revealedSeed,
                            isVisible = seedVisible,
                            onToggleVisibility = { seedVisible = !seedVisible },
                            onCopy = {
                                copyToClipboard(
                                    ctx,
                                    revealedSeed!!,
                                    label = "Seed",
                                    toast = "Seed phrase copied"
                                )
                            }
                        )
                        SecurityWarningCard("Never share your seed phrase")
                    }
                }
                OutlinedButton(
                    onClick = { isVerified = false; revealedKey = null; revealedSeed = null },
                    modifier = Modifier.fillMaxWidth()
                ) { Text("Lock") }
            }
        }
    }
}