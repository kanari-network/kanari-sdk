package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.navigation.NavController
import com.jamesatomc.kanariapp.ui.components.RecipientAddressField
import com.jamesatomc.kanariapp.ui.components.parseAmountToMist
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
        NavItem("Escrow", Icons.Default.Security, Icons.Outlined.Security),
        NavItem("Settings", Icons.Default.Settings, Icons.Outlined.Settings)
    )

    var bottomBarVisible by remember { mutableStateOf(true) }
    var accumulated by remember { mutableStateOf(0f) }
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
                Column {
                    // 2px tail / handle that moves with navbar - stays visible as peek
                    HorizontalDivider(
                        thickness = 2.dp,
                        color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.35f)
                    )
                    NavigationBar(
                        containerColor = MaterialTheme.colorScheme.surfaceContainer,
                        contentColor = MaterialTheme.colorScheme.onSurface
                    ) {
                        navItems.forEachIndexed { index, item ->
                            NavigationBarItem(
                                icon = {
                                    Icon(
                                        if (selectedItem == index) item.activeIcon else item.inactiveIcon,
                                        contentDescription = item.label
                                    )
                                },
                                label = { Text(item.label) },
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
    ) { innerPadding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(bottom = innerPadding.calculateBottomPadding()) // Only apply bottom padding for the nav bar
        ) {
            when (selectedItem) {
                0 -> DashboardScreen(
                    viewModel = viewModel,
                    onNavigateToReceive = { navController.navigate(Screen.Receive.route) },
                    onNavigateToSettings = { selectedItem = 3 },
                    onNavigateToSend = { showSendSheet = true },
                    onNavigateToWalletGen = { navController.navigate(Screen.WalletGeneration.route) }
                )

                1 -> HistoryScreen(viewModel = viewModel)
                2 -> EscrowScreen(walletViewModel = viewModel)
                3 -> SettingsScreen(
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
                                    String.format(
                                        java.util.Locale.US,
                                        "%.4f",
                                        token.getEffectiveAmount() / Math.pow(10.0, token.decimals.toDouble())
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
            label = "Recipient Address"
        )

        OutlinedTextField(
            value = amount,
            onValueChange = { amount = it; error = null },
            label = { Text("Amount") },
            modifier = Modifier.fillMaxWidth(),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal)
        )

        if (error != null) {
            Text(error!!, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
        }

        Button(
            onClick = {
                val decimals = selectedToken?.decimals ?: 9
                val tokenType = selectedToken?.tokenType ?: "0x2::kanari::KANARI"
                val amt = parseAmountToMist(amount, decimals)
                if (amt == null || amt == 0uL) {
                    error = "Invalid amount"
                    return@Button
                }
                val recipientTrimmed = recipient.trim()
                if (recipientTrimmed.isEmpty()) {
                    error = "Recipient address required"
                    return@Button
                }
                val clean = recipientTrimmed.removePrefix("0x")
                if (clean.isEmpty() || clean.length > 64 || !clean.matches(Regex("^[0-9a-fA-F]+$"))) {
                    error = "Invalid recipient address format"
                    return@Button
                }
                scope.launch {
                    isLoading = true
                    error = null
                    if (viewModel.transfer(recipientTrimmed, amt, tokenType)) onBack()
                    else error = viewModel.error.value ?: "Transaction failed"
                    isLoading = false
                }
            },
            modifier = Modifier.fillMaxWidth(),
            enabled = !isLoading && recipient.isNotEmpty() && amount.isNotEmpty() && selectedToken != null
        ) {
            if (isLoading) CircularProgressIndicator(modifier = Modifier.size(24.dp))
            else Text("Send")
        }
    }
}

data class NavItem(
    val label: String,
    val activeIcon: androidx.compose.ui.graphics.vector.ImageVector,
    val inactiveIcon: androidx.compose.ui.graphics.vector.ImageVector
)
