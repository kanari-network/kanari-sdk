@file:OptIn(ExperimentalMaterial3Api::class, androidx.compose.animation.ExperimentalAnimationApi::class)

package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.animation.core.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import coil3.compose.AsyncImage
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shadow
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.jamesatomc.kanariapp.ui.components.*
import com.jamesatomc.kanariapp.ui.theme.KanariColors
import com.jamesatomc.kanariapp.ui.theme.LocalKanariGradients
import com.jamesatomc.kanariapp.wallet.WalletRecord
import com.jamesatomc.kanariapp.wallet.WalletViewModel
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DashboardScreen(
    viewModel: WalletViewModel,
    onNavigateToReceive: () -> Unit,
    onNavigateToSettings: () -> Unit,
    onNavigateToSend: () -> Unit,
    onNavigateToWalletGen: () -> Unit,
    onNavigateToNfts: () -> Unit,
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
    var showAllObjectsDialog by remember { mutableStateOf(false) }
    var walletToDelete by remember { mutableStateOf<WalletRecord?>(null) }
    var walletForDetails by remember { mutableStateOf<WalletRecord?>(null) }
    val objects = accountInfo?.ownedObjects ?: emptyList()
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
            Spacer(Modifier.height(4.dp))
            HorizontalPager(
                state = pagerState,
                modifier = Modifier.fillMaxWidth().height(210.dp),
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
                            onViewDetails = { walletForDetails = w })
                    }
                } else AddWalletCard(onNavigateToWalletGen)
            }
            DotsIndicator(
                totalDots = wallets.size + 1,
                selectedIndex = pagerState.currentPage,
                modifier = Modifier.fillMaxWidth()
            )
            // Quick Services Grid (Fintech style matching reference screenshot)
            SectionHeader(title = "Quick Services")
            Card(
                shape = RoundedCornerShape(20.dp),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    Modifier.fillMaxWidth().padding(16.dp),
                    horizontalArrangement = Arrangement.SpaceAround
                ) {
                    QuickServiceItem(
                        icon = Icons.AutoMirrored.Filled.Send,
                        label = "Send",
                        containerColor = KanariColors.Lime,
                        contentColor = KanariColors.Ink,
                        onClick = onNavigateToSend
                    )
                    QuickServiceItem(
                        icon = Icons.Default.QrCode,
                        label = "Receive",
                        containerColor = MaterialTheme.colorScheme.secondaryContainer,
                        contentColor = MaterialTheme.colorScheme.onSecondaryContainer,
                        onClick = onNavigateToReceive
                    )
                    QuickServiceItem(
                        icon = Icons.Default.CollectionsBookmark,
                        label = "NFTs",
                        containerColor = MaterialTheme.colorScheme.tertiaryContainer,
                        contentColor = MaterialTheme.colorScheme.onTertiaryContainer,
                        onClick = onNavigateToNfts
                    )
                    QuickServiceItem(
                        icon = Icons.Default.AddCard,
                        label = "Wallet+",
                        containerColor = MaterialTheme.colorScheme.surfaceContainerHighest,
                        contentColor = MaterialTheme.colorScheme.onSurface,
                        onClick = onNavigateToWalletGen
                    )
                }
            }

            // Promo Banners Pager (3 items with auto-scroll)
            val promoPagerState = rememberPagerState(pageCount = { 3 })
            LaunchedEffect(Unit) {
                while (true) {
                    kotlinx.coroutines.delay(4000L)
                    val nextPage = (promoPagerState.currentPage + 1) % 3
                    promoPagerState.animateScrollToPage(nextPage)
                }
            }

            val banners = listOf(
                Triple("⚡ Kanari Network", "Secure Web3 & ZkLogin Powered", "Manage assets, multi-curve wallets & NFTs securely."),
                Triple("🛡️ Multi-Curve & Post-Quantum", "Advanced Cryptography", "Built-in Dilithium, Falcon & Ed25519 wallet support."),
                Triple("💎 NFTs & Assets", "Instant Transfers & BCS Parsing", "Explore, view and transfer your collectibles effortlessly.")
            )
            val bannerImages = listOf(
                "https://images.unsplash.com/photo-1639762681485-074b7f938ba0?q=80&w=1000&auto=format&fit=crop",
                "https://images.unsplash.com/photo-1642543492481-44e81e3914a7?q=80&w=1000&auto=format&fit=crop",
                "https://images.unsplash.com/photo-1620712943543-bcc4688e7485?q=80&w=1000&auto=format&fit=crop"
            )

            HorizontalPager(
                state = promoPagerState,
                modifier = Modifier.fillMaxWidth().height(130.dp),
                contentPadding = PaddingValues(horizontal = 0.dp),
                pageSpacing = 12.dp
            ) { page ->
                val (tag, title, subtitle) = banners[page]
                Card(
                    shape = RoundedCornerShape(24.dp),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant),
                    modifier = Modifier.fillMaxSize()
                ) {
                    Box(Modifier.fillMaxSize()) {
                        AsyncImage(
                            model = bannerImages[page],
                            contentDescription = null,
                            contentScale = ContentScale.Crop,
                            modifier = Modifier.fillMaxSize()
                        )
                        Box(
                            Modifier
                                .fillMaxSize()
                                .background(
                                    Brush.horizontalGradient(
                                        listOf(Color.Black.copy(alpha = 0.85f), Color.Black.copy(alpha = 0.35f))
                                    )
                                )
                                .padding(20.dp)
                        ) {
                            Column(
                                Modifier.fillMaxHeight(),
                                verticalArrangement = Arrangement.Center
                            ) {
                                Surface(
                                    color = Color.White.copy(alpha = 0.2f),
                                    shape = RoundedCornerShape(8.dp)
                                ) {
                                    Text(
                                        tag,
                                        style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.Bold),
                                        color = Color.White,
                                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                                    )
                                }
                                Spacer(Modifier.height(8.dp))
                                Text(
                                    title,
                                    style = MaterialTheme.typography.titleMedium.copy(
                                        fontWeight = FontWeight.Bold,
                                        color = Color.White
                                    )
                                )
                                Spacer(Modifier.height(2.dp))
                                Text(
                                    subtitle,
                                    style = MaterialTheme.typography.bodySmall.copy(
                                        color = Color.White.copy(alpha = 0.85f)
                                    ),
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis
                                )
                            }
                        }
                    }
                }
            }
            DotsIndicator(
                totalDots = 3,
                selectedIndex = promoPagerState.currentPage,
                modifier = Modifier.fillMaxWidth()
            )
            SectionHeader(title = "Assets")
            error?.let { ErrorBanner(error = it) }
            when {
                isLoading && tokenBalances.isEmpty() -> Box(
                    Modifier.fillMaxWidth().height(100.dp),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator()
                }

                wallets.isEmpty() -> Box(Modifier.fillMaxWidth().height(100.dp), contentAlignment = Alignment.Center) {
                    Text("No wallets found", color = MaterialTheme.colorScheme.onSurfaceVariant)
                }

                else -> tokenBalances.forEach { AssetItem(it) }
            }
            if (objects.isNotEmpty()) {
                SectionHeader(
                    title = "Objects & NFTs",
                    modifier = Modifier.padding(top = 8.dp),
                    trailing = {
                        TextButton(onClick = { showAllObjectsDialog = true }) {
                            Text("All >", style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.Bold))
                        }
                    }
                )
                objects.take(2).forEach { ObjectItem(it) }
                if (objects.size > 2) {
                    TextButton(
                        onClick = { showAllObjectsDialog = true },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("View all ${objects.size} objects >", style = MaterialTheme.typography.labelMedium)
                    }
                }
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
            WalletDetailFullScreen(
                wallet = w,
                viewModel = viewModel,
                onDismiss = { walletForDetails = null },
                onDeleteRequest = {
                    walletToDelete = w
                    walletForDetails = null
                }
            )
        }
    }
    if (showAllObjectsDialog) {
        AlertDialog(
            onDismissRequest = { showAllObjectsDialog = false },
            title = { Text("All Objects & NFTs (${objects.size})", fontWeight = FontWeight.Bold) },
            text = {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(400.dp)
                        .verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    objects.forEach { ObjectItem(it) }
                }
            },
            confirmButton = {
                TextButton(onClick = { showAllObjectsDialog = false }) {
                    Text("Close")
                }
            }
        )
    }
}

@Composable
fun WalletCard(
    wallet: WalletRecord,
    tokenBalances: List<com.jamesatomc.kanariapp.network.models.TokenBalance>,
    onViewDetails: () -> Unit
) {
    val curveInfo = remember(wallet.curveType) { getCurveInfo(wallet.curveType) }
    val gradients = LocalKanariGradients.current
    val infiniteTransition = rememberInfiniteTransition(label = "holographic")
    val holoOffset by infiniteTransition.animateFloat(
        initialValue = -1f,
        targetValue = 2f,
        animationSpec = infiniteRepeatable(tween(4000, easing = LinearEasing)),
        label = "holoOffset"
    )

    Card(
        Modifier.fillMaxSize(),
        shape = androidx.compose.foundation.shape.RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(containerColor = Color.Transparent),
        elevation = CardDefaults.cardElevation(4.dp)
    ) {
        Box(
            Modifier
                .fillMaxSize()
                .background(
                    if (curveInfo.isPostQuantum) gradients.multi else gradients.secondary
                )
        ) {
            // Holographic sweep for PQ wallets
            if (curveInfo.isPostQuantum) {
                Box(
                    Modifier
                        .fillMaxSize()
                        .background(
                            Brush.linearGradient(
                                0.0f to Color.Transparent,
                                0.5f to Color.White.copy(alpha = 0.15f),
                                1.0f to Color.Transparent,
                                start = Offset(100f * holoOffset, 0f),
                                end = Offset(100f * holoOffset + 200f, 400f)
                            )
                        )
                )
            }

            IsometricCoinStack(
                modifier = Modifier
                    .size(130.dp)
                    .align(Alignment.BottomEnd)
                    .offset(x = 10.dp, y = 10.dp)
                    .graphicsLayer { alpha = 0.2f }
            )

            Column(Modifier.fillMaxSize().padding(24.dp)) {
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            Icons.Default.AccountBalanceWallet,
                            contentDescription = null,
                            tint = Color.White.copy(alpha = 0.8f),
                            modifier = Modifier.size(16.dp)
                        )
                        Spacer(Modifier.width(8.dp))
                        Text(
                            wallet.name,
                            style = MaterialTheme.typography.labelLarge.copy(
                                fontWeight = FontWeight.Bold,
                                shadow = Shadow(
                                    color = Color.Black.copy(alpha = 0.3f),
                                    offset = Offset(0f, 1f),
                                    blurRadius = 2f
                                )
                            ),
                            color = Color.White
                        )
                    }
                    IconButton(
                        onClick = onViewDetails,
                        modifier = Modifier.size(36.dp)
                    ) {
                        Icon(
                            Icons.Default.Visibility,
                            contentDescription = "View details",
                            modifier = Modifier.size(22.dp),
                            tint = Color.White
                        )
                    }
                }

                Spacer(Modifier.height(10.dp))
                CurveBadge(curveInfo)

                Spacer(Modifier.height(20.dp))
                val t = tokenBalances.find { it.tokenType == "0x2::kanari::KANARI" }
                Text(
                    "Balance",
                    style = MaterialTheme.typography.labelLarge.copy(
                        fontWeight = FontWeight.Bold,
                        shadow = Shadow(
                            color = Color.Black.copy(alpha = 0.3f),
                            offset = Offset(0f, 2f),
                            blurRadius = 4f
                        )
                    ),
                    color = Color.White.copy(alpha = 0.9f)
                )
                val balanceStr = formatAmountExactOrUnknown(t?.getEffectiveAmount() ?: 0L, t?.decimals ?: 9)
                val balanceTextStyle = when {
                    balanceStr.length > 15 -> MaterialTheme.typography.titleLarge
                    balanceStr.length > 10 -> MaterialTheme.typography.headlineSmall
                    else -> MaterialTheme.typography.headlineMedium
                }.copy(
                    fontWeight = FontWeight.Bold,
                    color = Color.White,
                    letterSpacing = 0.5.sp,
                    shadow = Shadow(
                        color = Color.Black.copy(alpha = 0.4f),
                        offset = Offset(0f, 4f),
                        blurRadius = 8f
                    )
                )
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.Bottom
                ) {
                    Text(
                        text = balanceStr,
                        style = balanceTextStyle,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.weight(1f, fill = false)
                    )
                    Text(
                        text = " KANARI",
                        style = MaterialTheme.typography.titleSmall.copy(
                            fontWeight = FontWeight.Bold,
                            color = Color.White.copy(alpha = 0.75f),
                            shadow = Shadow(
                                color = Color.Black.copy(alpha = 0.3f),
                                offset = Offset(0f, 2f),
                                blurRadius = 4f
                            )
                        ),
                        maxLines = 1,
                        softWrap = false,
                        modifier = Modifier.padding(bottom = 4.dp, start = 4.dp)
                    )
                }

                Spacer(Modifier.weight(1f))

                CopyableAddressRow(
                    wallet.address,
                    short = false,
                    textStyle = MaterialTheme.typography.labelMedium.copy(
                        color = Color.White.copy(alpha = 0.9f),
                        fontWeight = FontWeight.Bold,
                        shadow = Shadow(
                            color = Color.Black.copy(alpha = 0.3f),
                            offset = Offset(0f, 1f),
                            blurRadius = 2f
                        )
                    )
                )
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
    val displayType = remember(token.tokenType) {
        val parts = token.tokenType.split("::")
        if (parts.size >= 3) "${parts[parts.size - 2]}::${parts.last()}" else token.tokenType
    }
    Card(
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
        shape = androidx.compose.foundation.shape.RoundedCornerShape(16.dp),
        modifier = Modifier.fillMaxWidth().padding(vertical = 2.dp)
    ) {
        ListItem(
            leadingContent = { TokenIcon(token) },
            headlineContent = {
                Text(
                    token.symbol,
                    style = MaterialTheme.typography.titleSmall.copy(fontWeight = FontWeight.Bold)
                )
            },
            supportingContent = {
                Text(
                    displayType,
                    maxLines = 1,
                    style = MaterialTheme.typography.bodySmall.copy(
                        color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f)
                    )
                )
            },
            trailingContent = {
                val decimals = token.decimals ?: if (token.tokenType == "0x2::kanari::KANARI" || token.symbol.equals(
                        "KANARI",
                        true
                    )
                ) 9 else null
                Text(
                    formatAmountExactOrUnknown(token.getEffectiveAmount(), decimals),
                    style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.ExtraBold),
                    color = MaterialTheme.colorScheme.onSurface
                )
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent)
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
                overflow = TextOverflow.Ellipsis
            )
        },
        leadingContent = { Icon(Icons.Default.Public, contentDescription = null) })
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WalletDetailFullScreen(
    wallet: WalletRecord,
    viewModel: WalletViewModel,
    onDismiss: () -> Unit,
    onDeleteRequest: () -> Unit
) {
    val ctx = LocalContext.current
    val activity = ctx.findFragmentActivity()
    val scope = rememberCoroutineScope()
    var isVerified by remember { mutableStateOf(false) }
    var revealedKey by remember { mutableStateOf<String?>(null) }
    var revealedSeed by remember { mutableStateOf<String?>(null) }
    var keyVisible by remember { mutableStateOf(false) }
    var seedVisible by remember { mutableStateOf(false) }
    val hasSeed = wallet.mnemonicEncrypted != null
    val hasPrivateKey = wallet.privateKeyEncrypted != null
    val curveInfo = remember(wallet.curveType) { getCurveInfo(wallet.curveType) }
    val canUseBiometric = rememberBiometricAvailable(viewModel)

    val onBiometric = rememberBiometricPromptLauncher(
        activity = activity,
        title = "Reveal Wallet Secrets",
        subtitle = "Use biometrics to unlock",
        onSuccess = {
            scope.launch {
                val k = viewModel.revealPrivateKeyWithBiometric(wallet)
                if (k != null) {
                    revealedKey = k
                    revealedSeed = if (hasSeed) viewModel.revealMnemonicWithBiometric(wallet) else null
                    isVerified = true
                }
            }
        },
    )
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
        if (!isVerified && hasPrivateKey) {
            PinVerificationContent(
                title = "Enter PIN",
                subtitle = "Enter 6-digit PIN to reveal secrets",
                onVerifyAsync = { pin -> viewModel.verifyPin(pin) },
                onSuccess = { pin ->
                    scope.launch {
                        val k = viewModel.revealPrivateKey(wallet, pin)
                        if (k != null) {
                            revealedKey = k
                            revealedSeed = if (hasSeed) viewModel.revealMnemonic(wallet, pin) else null
                            isVerified = true
                        }
                    }
                },
                biometricEnabled = canUseBiometric,
                onBiometric = onBiometric,
                modifier = Modifier.fillMaxSize().padding(padding)
            )
        } else {
            Column(
                Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()).padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp)
            ) {
                DetailSectionCard(glass = true) {
                    CurveBadge(curveInfo)
                    Spacer(Modifier.height(8.dp))
                    Text(
                        curveInfo.displayName,
                        style = MaterialTheme.typography.titleLarge.copy(
                            fontFamily = FontFamily.Monospace,
                            fontWeight = FontWeight.Bold
                        )
                    )
                    Text(
                        curveInfo.description,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Surface(
                        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                        shape = RoundedCornerShape(8.dp)
                    ) {
                        Text(
                            "Curve ID: ${wallet.curveType}",
                            style = MaterialTheme.typography.labelSmall.copy(fontFamily = FontFamily.Monospace),
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                        )
                    }
                }
                DetailSectionCard {
                    Text(
                        "Public Address",
                        style = MaterialTheme.typography.titleSmall.copy(fontWeight = FontWeight.Bold),
                        color = MaterialTheme.colorScheme.primary
                    )
                    Surface(
                        color = MaterialTheme.colorScheme.surfaceContainerHighest,
                        shape = RoundedCornerShape(16.dp),
                        modifier = Modifier.fillMaxWidth(),
                        border = androidx.compose.foundation.BorderStroke(
                            1.5.dp,
                            MaterialTheme.colorScheme.outline.copy(alpha = 0.3f)
                        )
                    ) {
                        Text(
                            wallet.address,
                            style = MaterialTheme.typography.bodyMedium.copy(
                                fontFamily = FontFamily.Monospace,
                                letterSpacing = 0.5.sp,
                                fontWeight = FontWeight.SemiBold
                            ),
                            color = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.padding(16.dp)
                        )
                    }
                    LoadingButton(
                        onClick = { copyToClipboard(ctx, wallet.address, toast = "Address copied") },
                        text = "Copy Address",
                        icon = Icons.Default.Public,
                        modifier = Modifier.fillMaxWidth(),
                        colors = ButtonDefaults.buttonColors(
                            containerColor = KanariColors.Lime,
                            contentColor = KanariColors.Ink
                        )
                    )
                }
                Spacer(Modifier.height(8.dp)) // Extra spacing
                DetailSectionCard {
                    if (!hasPrivateKey) {
                        Text(
                            "Session wallet — no private key exists. " +
                                    "Transfers are authorized automatically with your Google zkLogin session.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    } else {
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
                LoadingButton(
                    onClick = { isVerified = false; revealedKey = null; revealedSeed = null },
                    text = "Lock & Protect",
                    icon = Icons.Default.Security,
                    modifier = Modifier.fillMaxWidth(),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.surfaceVariant,
                        contentColor = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                )

                Spacer(Modifier.height(12.dp))

                TextButton(
                    onClick = onDeleteRequest,
                    modifier = Modifier.fillMaxWidth(),
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error.copy(alpha = 0.8f))
                ) {
                    Icon(Icons.Default.Delete, contentDescription = null, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text("Delete Wallet Permanent", style = MaterialTheme.typography.labelLarge)
                }
            }
        }
    }
}

@Composable
fun QuickServiceItem(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    label: String,
    containerColor: Color,
    contentColor: Color,
    onClick: () -> Unit
) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier
            .clickable(onClick = onClick)
            .padding(4.dp)
            .width(68.dp)
    ) {
        Surface(
            color = containerColor,
            shape = RoundedCornerShape(18.dp),
            modifier = Modifier.size(52.dp)
        ) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Icon(
                    icon,
                    contentDescription = label,
                    tint = contentColor,
                    modifier = Modifier.size(24.dp)
                )
            }
        }
        Spacer(Modifier.height(6.dp))
        Text(
            label,
            style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.SemiBold),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            color = MaterialTheme.colorScheme.onSurface
        )
    }
}