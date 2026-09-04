@file:OptIn(ExperimentalMaterial3Api::class, androidx.compose.animation.ExperimentalAnimationApi::class)

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
import androidx.compose.material.icons.filled.Security
import androidx.compose.material.icons.filled.VerifiedUser
import androidx.compose.material.icons.filled.Visibility
import androidx.compose.material.icons.filled.VisibilityOff
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.ContentPaste
import androidx.compose.material.icons.filled.MenuBook
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.jamesatomc.kanariapp.ui.components.CopyableAddressRow
import com.jamesatomc.kanariapp.ui.components.KanariTopBar
import com.jamesatomc.kanariapp.ui.components.PinCircles
import com.jamesatomc.kanariapp.ui.components.PinNumberPad
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
                    WalletCard(
                        w,
                        if (activeWallet?.id == w.id) tokenBalances else emptyList(),
                        onDelete = { walletToDelete = w },
                        onViewDetails = { walletForDetails = w })
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
        androidx.compose.ui.window.Dialog(
            onDismissRequest = { walletForDetails = null },
            properties = androidx.compose.ui.window.DialogProperties(usePlatformDefaultWidth = false)
        ) {
            Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
                WalletDetailFullScreen(wallet = w, viewModel = viewModel, onDismiss = { walletForDetails = null })
            }
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
    val (displayName, desc, isPQ) = remember(wallet.curveType) {
        when (wallet.curveType) {
            "K256" -> Triple("K256 (secp256k1)", "Bitcoin/Ethereum", false)
            "P256" -> Triple("P256", "NIST P-256", false)
            "Ed25519" -> Triple("Ed25519", "Fast modern", false)
            "Dilithium2" -> Triple("Dilithium2", "Post-Quantum Level 2", true)
            "Dilithium3" -> Triple("Dilithium3", "Post-Quantum Level 3", true)
            "Dilithium5" -> Triple("Dilithium5", "Post-Quantum Level 5", true)
            "SphincsPlusSha256Robust" -> Triple("SPHINCS+", "Hash-based PQ", true)
            "Falcon512", "FnDsa512" -> Triple("Falcon-512", "Post-Quantum Compact", true)
            "Falcon1024", "FnDsa1024" -> Triple("Falcon-1024", "Post-Quantum Level 5", true)
            "Ed25519Dilithium3" -> Triple("Ed25519+Dilithium3", "Hybrid Quantum-Safe", true)
            "K256Dilithium3" -> Triple("K256+Dilithium3", "Hybrid Quantum-Safe", true)
            else -> Triple(
                wallet.curveType,
                if (wallet.curveType.contains("Dilithium") || wallet.curveType.contains("Falcon") || wallet.curveType.contains(
                        "Sphincs"
                    )
                ) "Post-Quantum" else "Classical",
                wallet.curveType.contains("Dilithium") || wallet.curveType.contains("Falcon")
            )
        }
    }
    Card(
        Modifier.fillMaxSize(),
        shape = androidx.compose.foundation.shape.RoundedCornerShape(20.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh),
        elevation = CardDefaults.cardElevation(2.dp)
    ) {
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
                Icon(
                    if (isPQ) Icons.Default.Security else Icons.Default.VerifiedUser,
                    contentDescription = null,
                    modifier = Modifier.size(14.dp),
                    tint = if (isPQ) MaterialTheme.colorScheme.tertiary else MaterialTheme.colorScheme.primary
                )
                Text(
                    displayName,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                if (isPQ) Surface(
                    color = MaterialTheme.colorScheme.tertiaryContainer,
                    shape = androidx.compose.foundation.shape.RoundedCornerShape(6.dp)
                ) {
                    Text(
                        "PQ",
                        style = MaterialTheme.typography.labelSmall.copy(fontWeight = androidx.compose.ui.text.font.FontWeight.Bold),
                        color = MaterialTheme.colorScheme.onTertiaryContainer,
                        modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
                    )
                }
            }
            Spacer(Modifier.height(8.dp))
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

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WalletDetailFullScreen(wallet: WalletRecord, viewModel: WalletViewModel, onDismiss: () -> Unit) {
    val ctx = LocalContext.current
    var pinInput by remember { mutableStateOf("") }
    var isVerified by remember { mutableStateOf(false) }
    var pinError by remember { mutableStateOf<String?>(null) }
    var isChecking by remember { mutableStateOf(false) }
    var revealedKey by remember { mutableStateOf<String?>(null) }
    var revealedSeed by remember { mutableStateOf<String?>(null) }
    var keyVisible by remember { mutableStateOf(false) }
    var seedVisible by remember { mutableStateOf(false) }
    val hasSeed = wallet.mnemonicEncrypted != null
    val (displayName, desc, isPQ) = remember(wallet.curveType) {
        when (wallet.curveType) {
            "K256" -> Triple("K256 (secp256k1)", "Bitcoin/Ethereum compatible - Classical", false)
            "P256" -> Triple("P256 (secp256r1)", "NIST P-256 - Classical", false)
            "Ed25519" -> Triple("Ed25519", "EdDSA - Fast modern - Classical", false)
            "Dilithium2" -> Triple("Dilithium2", "Post-Quantum NIST Level 2", true)
            "Dilithium3" -> Triple("Dilithium3", "Post-Quantum NIST Level 3 (Recommended)", true)
            "Dilithium5" -> Triple("Dilithium5", "Post-Quantum NIST Level 5", true)
            "SphincsPlusSha256Robust" -> Triple("SPHINCS+-SHA256", "Hash-based Post-Quantum", true)
            "Falcon512", "FnDsa512" -> Triple("Falcon-512", "Compact Lattice PQ", true)
            "Falcon1024", "FnDsa1024" -> Triple("Falcon-1024", "Compact Lattice PQ Level 5", true)
            "Ed25519Dilithium3" -> Triple("Ed25519 + Dilithium3", "Hybrid Classical + Post-Quantum", true)
            "K256Dilithium3" -> Triple("K256 + Dilithium3", "Hybrid Classical + Post-Quantum", true)
            else -> Triple(
                wallet.curveType,
                "Unknown curve type",
                wallet.curveType.contains("Dilithium") || wallet.curveType.contains("Falcon")
            )
        }
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
            Column(
                Modifier.fillMaxSize().padding(padding).padding(20.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.SpaceBetween
            ) {
                com.jamesatomc.kanariapp.ui.components.PinEntryHeader(
                    title = "Enter PIN",
                    subtitle = "Enter 6-digit PIN to reveal secrets",
                    enteredLength = pinInput.length,
                    errorText = pinError,
                    isChecking = isChecking
                )
                com.jamesatomc.kanariapp.ui.components.PinNumberPad(
                    onNumberPressed = {
                        if (pinInput.length < 6) {
                            pinInput += it; pinError = null
                        }
                    },
                    onBackspacePressed = { if (pinInput.isNotEmpty()) pinInput = pinInput.dropLast(1) },
                    biometricEnabled = false,
                    modifier = Modifier.fillMaxWidth()
                )
            }
            LaunchedEffect(pinInput) {
                if (pinInput.length == 6) {
                    isChecking = true
                    if (viewModel.verifyPin(pinInput)) {
                        val k = viewModel.revealPrivateKey(wallet, pinInput)
                        if (k != null) {
                            revealedKey = k; revealedSeed =
                                if (hasSeed) viewModel.revealMnemonic(wallet, pinInput) else null; isVerified = true
                        } else pinError = "Invalid PIN"
                    } else pinError = "Invalid PIN"
                    if (!isVerified) pinInput = ""
                    isChecking = false
                }
            }
        } else {
            Column(
                Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()).padding(20.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp)
            ) {
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
                    shape = androidx.compose.foundation.shape.RoundedCornerShape(16.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Icon(
                                if (isPQ) Icons.Default.Security else Icons.Default.VerifiedUser,
                                contentDescription = null,
                                tint = if (isPQ) MaterialTheme.colorScheme.tertiary else MaterialTheme.colorScheme.primary
                            )
                            Text(
                                "Algorithm",
                                style = MaterialTheme.typography.titleSmall.copy(fontWeight = androidx.compose.ui.text.font.FontWeight.Bold)
                            )
                            if (isPQ) Surface(
                                color = MaterialTheme.colorScheme.tertiaryContainer,
                                shape = androidx.compose.foundation.shape.RoundedCornerShape(6.dp)
                            ) {
                                Text(
                                    "POST-QUANTUM",
                                    style = MaterialTheme.typography.labelSmall.copy(fontWeight = androidx.compose.ui.text.font.FontWeight.Bold),
                                    color = MaterialTheme.colorScheme.onTertiaryContainer,
                                    modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
                                )
                            }
                        }
                        Text(
                            displayName,
                            style = MaterialTheme.typography.titleMedium.copy(
                                fontFamily = FontFamily.Monospace,
                                fontWeight = androidx.compose.ui.text.font.FontWeight.Bold
                            )
                        )
                        Text(
                            desc,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Text(
                            "Curve: ${wallet.curveType}",
                            style = MaterialTheme.typography.labelSmall.copy(fontFamily = FontFamily.Monospace),
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
                    shape = androidx.compose.foundation.shape.RoundedCornerShape(16.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text("Address", style = MaterialTheme.typography.titleSmall)
                        Text(
                            wallet.address,
                            style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                            color = MaterialTheme.colorScheme.onSurface
                        )
                        Button(onClick = {
                            val c =
                                ctx.getSystemService(android.content.Context.CLIPBOARD_SERVICE) as android.content.ClipboardManager; c.setPrimaryClip(
                            android.content.ClipData.newPlainText("Address", wallet.address)
                        ); android.widget.Toast.makeText(ctx, "Address copied", android.widget.Toast.LENGTH_SHORT)
                            .show()
                        }, modifier = Modifier.fillMaxWidth()) {
                            Icon(
                                Icons.Default.Public,
                                contentDescription = null,
                                modifier = Modifier.size(18.dp)
                            ); Spacer(Modifier.width(8.dp)); Text("Copy Address")
                        }
                    }
                }
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
                    shape = androidx.compose.foundation.shape.RoundedCornerShape(16.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        Row(
                            Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(8.dp)
                            ) {
                                Surface(
                                    shape = androidx.compose.foundation.shape.CircleShape,
                                    color = MaterialTheme.colorScheme.primaryContainer,
                                    modifier = Modifier.size(32.dp)
                                ) {
                                    Box(
                                        Modifier.fillMaxSize(),
                                        contentAlignment = Alignment.Center
                                    ) {
                                        Icon(
                                            Icons.Filled.Visibility,
                                            contentDescription = null,
                                            modifier = Modifier.size(18.dp),
                                            tint = MaterialTheme.colorScheme.onPrimaryContainer
                                        )
                                    }
                                }
                                Text(
                                    "Private Key",
                                    style = MaterialTheme.typography.titleSmall.copy(fontWeight = androidx.compose.ui.text.font.FontWeight.Bold)
                                )
                            }
                            IconButton(onClick = {
                                keyVisible = !keyVisible
                            }) {
                                Icon(
                                    if (keyVisible) Icons.Default.VisibilityOff else Icons.Default.Visibility,
                                    contentDescription = "Toggle"
                                )
                            }
                        }
                        Surface(
                            color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                            shape = androidx.compose.foundation.shape.RoundedCornerShape(12.dp),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Text(
                                if (keyVisible) revealedKey!! else "•".repeat(32),
                                style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                                color = MaterialTheme.colorScheme.onSurface,
                                modifier = Modifier.padding(12.dp)
                            )
                        }
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                            OutlinedButton(onClick = {
                                val c =
                                    ctx.getSystemService(android.content.Context.CLIPBOARD_SERVICE) as android.content.ClipboardManager; c.setPrimaryClip(
                                android.content.ClipData.newPlainText("Private Key", revealedKey!!)
                            ); android.widget.Toast.makeText(
                                ctx,
                                "Private key copied",
                                android.widget.Toast.LENGTH_SHORT
                            ).show()
                            }, modifier = Modifier.weight(1f)) {
                                Icon(
                                    Icons.Default.ContentPaste,
                                    contentDescription = null,
                                    modifier = Modifier.size(18.dp)
                                ); Spacer(Modifier.width(6.dp)); Text("Copy")
                            }
                            Button(
                                onClick = { keyVisible = !keyVisible },
                                modifier = Modifier.weight(1f)
                            ) { Text(if (keyVisible) "Hide" else "Show") }
                        }
                        Surface(
                            color = MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.3f),
                            shape = androidx.compose.foundation.shape.RoundedCornerShape(12.dp),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Row(
                                Modifier.padding(12.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(8.dp)
                            ) {
                                Icon(
                                    Icons.Default.Security,
                                    contentDescription = null,
                                    tint = MaterialTheme.colorScheme.error,
                                    modifier = Modifier.size(18.dp)
                                ); Text(
                                "Never share your private key",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onErrorContainer
                            )
                            }
                        }
                    }
                }
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
                    shape = androidx.compose.foundation.shape.RoundedCornerShape(16.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Surface(
                                shape = androidx.compose.foundation.shape.CircleShape,
                                color = MaterialTheme.colorScheme.secondaryContainer,
                                modifier = Modifier.size(32.dp)
                            ) {
                                Box(
                                    Modifier.fillMaxSize(),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Icon(
                                        Icons.Filled.MenuBook,
                                        contentDescription = null,
                                        modifier = Modifier.size(18.dp),
                                        tint = MaterialTheme.colorScheme.onSecondaryContainer
                                    )
                                }
                            }
                            Text(
                                "Seed Phrase",
                                style = MaterialTheme.typography.titleSmall.copy(fontWeight = androidx.compose.ui.text.font.FontWeight.Bold)
                            )
                        }
                        if (!hasSeed) {
                            Text(
                                "This wallet was imported from private key - no seed phrase available",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        } else {
                            Surface(
                                color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                                shape = androidx.compose.foundation.shape.RoundedCornerShape(12.dp),
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Text(
                                    if (seedVisible) revealedSeed!! else "•".repeat(48),
                                    style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                                    color = MaterialTheme.colorScheme.onSurface,
                                    modifier = Modifier.padding(12.dp)
                                )
                            }
                            Row(
                                horizontalArrangement = Arrangement.spacedBy(8.dp),
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                IconButton(onClick = {
                                    seedVisible = !seedVisible
                                }) {
                                    Icon(
                                        if (seedVisible) Icons.Default.VisibilityOff else Icons.Default.Visibility,
                                        contentDescription = "Toggle"
                                    )
                                }
                                OutlinedButton(onClick = {
                                    val c =
                                        ctx.getSystemService(android.content.Context.CLIPBOARD_SERVICE) as android.content.ClipboardManager; c.setPrimaryClip(
                                    android.content.ClipData.newPlainText("Seed", revealedSeed!!)
                                ); android.widget.Toast.makeText(
                                    ctx,
                                    "Seed phrase copied",
                                    android.widget.Toast.LENGTH_SHORT
                                ).show()
                                }, modifier = Modifier.weight(1f)) { Text("Copy") }
                                Button(onClick = { seedVisible = !seedVisible }, modifier = Modifier.weight(1f)) {
                                    Text(
                                        if (seedVisible) "Hide" else "Show"
                                    )
                                }
                            }
                            Surface(
                                color = MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.3f),
                                shape = androidx.compose.foundation.shape.RoundedCornerShape(12.dp),
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Row(
                                    Modifier.padding(12.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                                ) {
                                    Icon(
                                        Icons.Default.Security,
                                        contentDescription = null,
                                        tint = MaterialTheme.colorScheme.error,
                                        modifier = Modifier.size(18.dp)
                                    ); Text(
                                    "Never share your seed phrase",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onErrorContainer
                                )
                                }
                            }
                        }
                    }
                }
                OutlinedButton(
                    onClick = { isVerified = false; pinInput = ""; revealedKey = null; revealedSeed = null },
                    modifier = Modifier.fillMaxWidth()
                ) { Text("Lock") }
            }
        }
    }
}