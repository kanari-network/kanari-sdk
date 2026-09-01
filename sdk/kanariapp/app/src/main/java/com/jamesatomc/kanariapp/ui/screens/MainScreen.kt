package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.*
import androidx.compose.material.icons.outlined.Home
import androidx.compose.material.icons.outlined.Security
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
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

    Scaffold(
        bottomBar = {
            KanariBottomNav(
                currentIndex = selectedItem,
                onTabTapped = { selectedItem = it },
                onSendTapped = { showSendSheet = true }
            )
        }
    ) { padding ->
        Box(modifier = Modifier.fillMaxSize().padding(padding).padding(bottom = 80.dp)) {
            when (selectedItem) {
                0 -> DashboardScreen(
                    viewModel = viewModel,
                    onNavigateToReceive = { navController.navigate(Screen.Receive.route) },
                    onNavigateToSettings = { selectedItem = 2 },
                    onNavigateToSend = { showSendSheet = true },
                    onNavigateToWalletGen = { navController.navigate(Screen.WalletGeneration.route) }
                )
                1 -> EscrowScreen()
                2 -> SettingsScreen(
                    viewModel = viewModel,
                    onLogout = onLogout,
                    onBack = { selectedItem = 0 }
                )
            }
        }
    }

    if (showSendSheet) {
        ModalBottomSheet(
            onDismissRequest = { showSendSheet = false },
            containerColor = MaterialTheme.colorScheme.surfaceContainerLowest,
        ) {
            SendScreenContent(
                viewModel = viewModel,
                onBack = { showSendSheet = false }
            )
        }
    }
}

@Composable
fun KanariBottomNav(
    currentIndex: Int,
    onTabTapped: (Int) -> Unit,
    onSendTapped: () -> Unit
) {
    Surface(
        modifier = Modifier
            .padding(horizontal = 16.dp, vertical = 12.dp)
            .fillMaxWidth()
            .height(80.dp)
            .clip(RoundedCornerShape(24.dp))
            .border(1.dp, MaterialTheme.colorScheme.outlineVariant, RoundedCornerShape(24.dp)),
        color = MaterialTheme.colorScheme.surfaceContainerLowest,
        tonalElevation = 0.dp
    ) {
        Row(
            modifier = Modifier.fillMaxSize(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceEvenly
        ) {
            KanariNavItem(
                icon = if (currentIndex == 0) Icons.Default.Home else Icons.Outlined.Home,
                label = "Home",
                isActive = currentIndex == 0,
                onClick = { onTabTapped(0) }
            )
            KanariNavItem(
                icon = Icons.AutoMirrored.Filled.Send,
                label = "Send",
                isActive = false,
                onClick = onSendTapped
            )
            KanariNavItem(
                icon = if (currentIndex == 1) Icons.Default.Security else Icons.Outlined.Security,
                label = "Escrow",
                isActive = currentIndex == 1,
                onClick = { onTabTapped(1) }
            )
            KanariNavItem(
                icon = Icons.Default.Settings,
                label = "Settings",
                isActive = currentIndex == 2,
                onClick = { onTabTapped(2) }
            )
        }
    }
}

@Composable
fun RowScope.KanariNavItem(
    icon: ImageVector,
    label: String,
    isActive: Boolean,
    onClick: () -> Unit
) {
    val contentColor = if (isActive) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurfaceVariant
    val containerColor = if (isActive) MaterialTheme.colorScheme.primary else Color.Transparent

    Box(
        modifier = Modifier
            .weight(1f)
            .fillMaxHeight()
            .padding(4.dp)
            .clip(RoundedCornerShape(16.dp))
            .background(containerColor)
            .clickable { onClick() },
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            Icon(
                icon, 
                contentDescription = label, 
                tint = contentColor,
                modifier = Modifier.size(if (isActive) 26.dp else 24.dp)
            )
            Spacer(Modifier.height(4.dp))
            Text(
                text = label,
                style = MaterialTheme.typography.labelSmall.copy(
                    fontWeight = if (isActive) FontWeight.ExtraBold else FontWeight.Bold,
                    fontSize = 10.sp
                ),
                color = contentColor
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SendScreenContent(viewModel: WalletViewModel, onBack: () -> Unit) {
    var recipient by remember { mutableStateOf("") }
    var amount by remember { mutableStateOf("") }
    var isLoading by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(16.dp)
            .navigationBarsPadding()
    ) {
        Text("Send Tokens", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        Spacer(Modifier.height(16.dp))
        
        OutlinedTextField(
            value = recipient,
            onValueChange = { recipient = it },
            label = { Text("Recipient Address") },
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(12.dp)
        )
        Spacer(Modifier.height(8.dp))
        
        OutlinedTextField(
            value = amount,
            onValueChange = { amount = it },
            label = { Text("Amount (KANARI)") },
            modifier = Modifier.fillMaxWidth(),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
            shape = RoundedCornerShape(12.dp)
        )
        
        if (error != null) {
            Text(error!!, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
        }
        
        Spacer(Modifier.height(24.dp))
        
        Button(
            onClick = {
                val amtLong = ((amount.toDoubleOrNull() ?: 0.0) * 1_000_000_000).toLong()
                if (amtLong > 0 && recipient.isNotEmpty()) {
                    scope.launch {
                        isLoading = true
                        if (viewModel.transfer(recipient, amtLong)) {
                            onBack()
                        } else {
                            error = "Transaction failed"
                        }
                        isLoading = false
                    }
                }
            },
            modifier = Modifier.fillMaxWidth().height(56.dp),
            shape = RoundedCornerShape(12.dp),
            enabled = !isLoading && recipient.isNotEmpty() && amount.isNotEmpty()
        ) {
            if (isLoading) CircularProgressIndicator(modifier = Modifier.size(24.dp), color = Color.White)
            else Text("Send")
        }
        Spacer(Modifier.height(16.dp))
    }
}
