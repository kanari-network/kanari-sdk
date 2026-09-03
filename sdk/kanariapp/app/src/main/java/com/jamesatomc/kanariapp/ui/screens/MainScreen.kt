package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.*
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.*
import androidx.compose.material3.adaptive.navigationsuite.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.navigation.NavController
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

    NavigationSuiteScaffold(
        navigationSuiteItems = {
            navItems.forEachIndexed { index, item ->
                item(
                    icon = { 
                        Icon(
                            if (selectedItem == index) item.activeIcon else item.inactiveIcon, 
                            contentDescription = item.label
                        ) 
                    },
                    label = { Text(item.label) },
                    selected = selectedItem == index,
                    onClick = { selectedItem = index }
                )
            }
        }
    ) {
        Box(modifier = Modifier.fillMaxSize()) {
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
                modifier = Modifier.menuAnchor(type = ExposedDropdownMenuAnchorType.PrimaryEditable, enabled = true).fillMaxWidth()
            )

            ExposedDropdownMenu(
                expanded = expanded,
                onDismissRequest = { expanded = false }
            ) {
                tokenBalances.forEach { token ->
                    DropdownMenuItem(
                        text = {
                            Text("${token.symbol} (Balance: ${String.format(java.util.Locale.US, "%.4f", token.getEffectiveAmount() / Math.pow(10.0, token.decimals.toDouble()))})")
                        },
                        onClick = {
                            selectedToken = token
                            expanded = false
                        }
                    )
                }
            }
        }

        OutlinedTextField(
            value = recipient,
            onValueChange = { recipient = it },
            label = { Text("Recipient Address") },
            modifier = Modifier.fillMaxWidth()
        )
        
        OutlinedTextField(
            value = amount,
            onValueChange = { amount = it },
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
                val amtLong = ((amount.toDoubleOrNull() ?: 0.0) * Math.pow(10.0, decimals.toDouble())).toLong().toULong()
                val tokenType = selectedToken?.tokenType ?: "0x2::kanari::KANARI"
                
                if (amtLong > 0uL && recipient.isNotEmpty()) {
                    scope.launch {
                        isLoading = true
                        if (viewModel.transfer(recipient, amtLong, tokenType)) {
                            onBack()
                        } else {
                            error = viewModel.error.value ?: "Transaction failed"
                        }
                        isLoading = false
                    }
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
