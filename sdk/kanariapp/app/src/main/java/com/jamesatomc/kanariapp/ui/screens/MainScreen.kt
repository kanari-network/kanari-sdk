package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.navigation.NavController
import com.jamesatomc.kanariapp.ui.components.LoadingButton
import com.jamesatomc.kanariapp.ui.components.RecipientAddressField
import com.jamesatomc.kanariapp.ui.components.SendCountdownDialog
import com.jamesatomc.kanariapp.ui.components.SendSuccessOverlay
import com.jamesatomc.kanariapp.ui.components.formatAmountExactOrUnknown
import com.jamesatomc.kanariapp.ui.components.parseAmountToMist
import com.jamesatomc.kanariapp.ui.components.validateAddress
import com.jamesatomc.kanariapp.wallet.WalletViewModel
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MainScreen(
    navController: NavController,
    viewModel: WalletViewModel,
    onLogout: () -> Unit
) {
    var selectedItem by remember { mutableIntStateOf(0) }
    var showSendSheet by remember { mutableStateOf(false) }

    val navItems = listOf(
        NavItem("Home", Icons.Default.Home, Icons.Outlined.Home),
        NavItem("History", Icons.Default.History, Icons.Outlined.History),
        NavItem("Settings", Icons.Default.Settings, Icons.Outlined.Settings)
    )

    var bottomBarVisible by remember { mutableStateOf(true) }
    var accumulated by remember { mutableFloatStateOf(0f) }
    val density = androidx.compose.ui.platform.LocalDensity.current
    val peekPx = with(density) { 2.dp.toPx() }
    val nestedScrollConnection = remember {
        object : NestedScrollConnection {
            override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
                accumulated += available.y
                // increase threshold to avoid jitter - hide only after 28px down, show after 16px up
                if (available.y < 0 && accumulated < -28) {
                    bottomBarVisible = false
                    accumulated = 0f
                } else if (available.y > 0 && accumulated > 16) {
                    bottomBarVisible = true
                    accumulated = 0f
                }
                // reset if direction changes
                if ((available.y < 0 && accumulated > 0) || (available.y > 0 && accumulated < 0)) accumulated =
                    available.y
                return Offset.Zero
            }
        }
    }

    Scaffold(
        modifier = Modifier.nestedScroll(nestedScrollConnection),
        contentWindowInsets = WindowInsets(0, 0, 0, 0), // Disable automatic inset handling in the top-level Scaffold
        bottomBar = {
            AnimatedVisibility(
                visible = bottomBarVisible,
                enter = slideInVertically(
                    initialOffsetY = { it },
                    animationSpec = androidx.compose.animation.core.tween(
                        320,
                        easing = androidx.compose.animation.core.FastOutSlowInEasing
                    )
                ),
                exit = slideOutVertically(
                    // leave 2px peek (tail) visible when hidden
                    targetOffsetY = { fullHeight -> fullHeight - peekPx.toInt() },
                    animationSpec = androidx.compose.animation.core.tween(
                        280,
                        easing = androidx.compose.animation.core.FastOutSlowInEasing
                    )
                )
            ) {
                Column(
                    modifier = Modifier
                        .padding(horizontal = 8.dp)
                        .padding(bottom = 8.dp) // Offset from the bottom edge
                        .navigationBarsPadding()
                ) {
                    Surface(
                        color = MaterialTheme.colorScheme.surfaceContainer.copy(alpha = 0.95f),
                        shape = RoundedCornerShape(20.dp), // Reduced curvature for a more sophisticated look
                        shadowElevation = 2.dp,
                        border = androidx.compose.foundation.BorderStroke(
                            1.dp,
                            MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f)
                        ),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        NavigationBar(
                            containerColor = Color.Transparent, // Color handled by Surface
                            contentColor = MaterialTheme.colorScheme.onSurface,
                            tonalElevation = 0.dp,
                            modifier = Modifier.height(80.dp)
                        ) {
                            navItems.forEachIndexed { index, item ->
                                NavigationBarItem(
                                    icon = {
                                        Icon(
                                            if (selectedItem == index) item.activeIcon else item.inactiveIcon,
                                            contentDescription = item.label
                                        )
                                    },
                                    label = { Text(item.label, style = MaterialTheme.typography.labelSmall) },
                                    selected = selectedItem == index,
                                    onClick = { selectedItem = index },
                                    colors = NavigationBarItemDefaults.colors(
                                        selectedIconColor = MaterialTheme.colorScheme.onSecondaryContainer,
                                        selectedTextColor = MaterialTheme.colorScheme.onSurface,
                                        indicatorColor = MaterialTheme.colorScheme.secondaryContainer
                                    )
                                )
                            }
                        }
                    }
                }
            }
        }
    ) { innerPadding ->
        // Re-entering Home/History should show the latest chain state — a send
        // may settle after the post-send polling window has already closed.
        LaunchedEffect(selectedItem) {
            if (selectedItem == 0 || selectedItem == 1) viewModel.refreshBalance()
        }

        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(bottom = innerPadding.calculateBottomPadding()) // Only apply bottom padding for the nav bar
        ) {
            when (selectedItem) {
                0 -> DashboardScreen(
                    viewModel = viewModel,
                    onNavigateToReceive = { navController.navigate(Screen.Receive.route) },
                    onNavigateToSettings = { selectedItem = 2 },
                    onNavigateToSend = { showSendSheet = true },
                    onNavigateToWalletGen = { navController.navigate(Screen.WalletGeneration.route) },
                    onNavigateToNfts = { navController.navigate(Screen.Nfts.route) },
                    onNavigateToObjects = { navController.navigate(Screen.Objects.route) }
                )

                1 -> HistoryScreen(viewModel = viewModel)
                2 -> SettingsScreen(
                    viewModel = viewModel,
                    onLogout = onLogout,
                    onBack = { selectedItem = 0 }
                )
            }
        }

        if (showSendSheet) {
            ModalBottomSheet(
                onDismissRequest = { showSendSheet = false }
            ) {
                SendScreenContent(
                    viewModel = viewModel,
                    onBack = { showSendSheet = false }
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SendScreenContent(viewModel: WalletViewModel, onBack: () -> Unit) {
    val tokenBalances by viewModel.tokenBalances.collectAsState()
    val wallets by viewModel.wallets.collectAsState()
    var recipient by remember { mutableStateOf("") }
    var amount by remember { mutableStateOf("") }
    var isLoading by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    var expanded by remember { mutableStateOf(false) }
    // Armed send: (recipient, amount in mist, token type) waiting on the
    // 10-second countdown dialog before anything touches the network.
    var pendingSend by remember { mutableStateOf<Triple<String, ULong, String>?>(null) }
    // Non-null while the Slush-style success overlay is showing.
    var successSummary by remember { mutableStateOf<String?>(null) }
    var selectedToken by remember {
        mutableStateOf(tokenBalances.find { it.symbol == "KANARI" } ?: tokenBalances.firstOrNull())
    }

    LaunchedEffect(tokenBalances) {
        if (selectedToken == null && tokenBalances.isNotEmpty()) {
            selectedToken = tokenBalances.find { it.symbol == "KANARI" } ?: tokenBalances.first()
        }
    }

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(16.dp)
            .navigationBarsPadding(),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text("Send Tokens", style = MaterialTheme.typography.titleLarge)

        ExposedDropdownMenuBox(
            expanded = expanded,
            onExpandedChange = { expanded = !expanded },
            modifier = Modifier.fillMaxWidth()
        ) {
            OutlinedTextField(
                value = selectedToken?.symbol ?: "Select Token",
                onValueChange = {},
                readOnly = true,
                label = { Text("Token") },
                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
                shape = androidx.compose.foundation.shape.RoundedCornerShape(12.dp),
                modifier = Modifier.menuAnchor(type = ExposedDropdownMenuAnchorType.PrimaryEditable, enabled = true)
                    .fillMaxWidth()
            )

            ExposedDropdownMenu(
                expanded = expanded,
                onDismissRequest = { expanded = false }
            ) {
                tokenBalances.forEach { token ->
                    DropdownMenuItem(
                        text = {
                            Text(
                                "${token.symbol} (Balance: ${
                                    formatAmountExactOrUnknown(
                                        token.getEffectiveAmount(),
                                        token.decimals
                                    )
                                })"
                            )
                        },
                        onClick = {
                            selectedToken = token
                            expanded = false
                        }
                    )
                }
            }
        }

        RecipientAddressField(
            value = recipient,
            onValueChange = { recipient = it; error = null },
            wallets = wallets,
            label = "Recipient Address",
            // Payment QR (`kanari:pay?…`) also pre-fills amount and token so
            // the sender just presses Send on the reviewed numbers.
            onPaymentScanned = { payment ->
                payment.amount?.let { amount = it }
                payment.token?.let { symbol ->
                    tokenBalances
                        .find { it.symbol.equals(symbol, ignoreCase = true) }
                        ?.let { selectedToken = it }
                }
            },
        )

        OutlinedTextField(
            value = amount,
            onValueChange = { amount = it; error = null },
            label = { Text("Amount") },
            shape = androidx.compose.foundation.shape.RoundedCornerShape(12.dp),
            modifier = Modifier.fillMaxWidth(),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal)
        )

        if (error != null) {
            Text(error!!, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
        }

        LoadingButton(
            onClick = {
                // No fallback: decimals must come from API metadata
                val decimals = selectedToken?.decimals
                if (decimals == null) {
                    error = "Decimals unknown — refresh balance and try again"; return@LoadingButton
                }
                val tokenType = selectedToken?.tokenType ?: "0x2::kanari::KANARI"
                val amt = parseAmountToMist(amount, decimals)
                if (amt == null || amt == 0uL) {
                    error = "Invalid amount"; return@LoadingButton
                }
                val adderError = validateAddress(recipient)
                if (adderError != null) {
                    error = adderError; return@LoadingButton
                }
                // Validation passed — arm the countdown; the real send runs
                // from SendCountdownDialog's onConfirmed below.
                pendingSend = Triple(recipient.trim(), amt, tokenType)
            },
            text = "Send",
            enabled = recipient.isNotEmpty() && amount.isNotEmpty() && selectedToken != null,
            isLoading = isLoading,
            modifier = Modifier.fillMaxWidth()
        )
    }

    pendingSend?.let { (to, mist, type) ->
        SendCountdownDialog(
            title = "Confirm send",
            detail = "Sending $amount ${selectedToken?.symbol ?: ""} to " +
                    "${to.take(12)}\u2026${to.takeLast(6)}",
            onCancel = { pendingSend = null },
            onConfirmed = {
                pendingSend = null
                scope.launch {
                    isLoading = true; error = null
                    if (viewModel.transfer(to, mist, type)) {
                        successSummary = "$amount ${selectedToken?.symbol ?: ""} sent to " +
                            "${to.take(10)}\u2026${to.takeLast(6)}"
                    } else {
                        error = viewModel.error.value ?: "Transaction failed"
                    }
                    isLoading = false
                }
            },
        )
    }

    successSummary?.let { message ->
        SendSuccessOverlay(
            summary = message,
            onDone = {
                successSummary = null
                onBack()
            },
        )
    }
}

data class NavItem(
    val label: String,
    val activeIcon: androidx.compose.ui.graphics.vector.ImageVector,
    val inactiveIcon: androidx.compose.ui.graphics.vector.ImageVector
)