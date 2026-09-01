package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.QrCode
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.jamesatomc.kanariapp.network.models.AccountInfo
import com.jamesatomc.kanariapp.network.models.TokenBalance
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
    
    val pagerState = rememberPagerState(pageCount = { wallets.size + 1 })

    // Sync pager state with active wallet
    LaunchedEffect(activeWallet) {
        val index = wallets.indexOfFirst { it.id == activeWallet?.id }
        if (index >= 0 && pagerState.currentPage != index) {
            pagerState.animateScrollToPage(index)
        }
    }

    // Sync active wallet with pager state
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
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Box(
                            modifier = Modifier
                                .size(32.dp)
                                .clip(CircleShape)
                                .background(MaterialTheme.colorScheme.primary),
                            contentAlignment = Alignment.Center
                        ) {
                            Text("K", fontWeight = FontWeight.Black, color = MaterialTheme.colorScheme.onPrimary, fontSize = 14.sp)
                        }
                        Spacer(Modifier.width(10.dp))
                        Text("KANARI", fontWeight = FontWeight.Black)
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
                    containerColor = MaterialTheme.colorScheme.background,
                    titleContentColor = MaterialTheme.colorScheme.onBackground
                )
            )
        },
        containerColor = MaterialTheme.colorScheme.background
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(top = 8.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            HorizontalPager(
                state = pagerState,
                modifier = Modifier
                    .fillMaxWidth()
                    .height(220.dp),
                contentPadding = PaddingValues(horizontal = 32.dp),
                pageSpacing = 16.dp
            ) { page ->
                if (page < wallets.size) {
                    val wallet = wallets[page]
                    WalletCard(
                        wallet = wallet,
                        tokenBalances = if (activeWallet?.id == wallet.id) tokenBalances else emptyList()
                    )
                } else {
                    AddWalletCard(onClick = onNavigateToWalletGen)
                }
            }
            
            Spacer(modifier = Modifier.height(32.dp))
            
            Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        "Assets", 
                        style = MaterialTheme.typography.titleLarge, 
                        fontWeight = FontWeight.Bold
                    )
                    if (isLoading) {
                        CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                    }
                }
                
                Spacer(modifier = Modifier.height(16.dp))
                
                if (wallets.isEmpty() && !isLoading) {
                    Text("No wallets found", color = MaterialTheme.colorScheme.onSurfaceVariant)
                } else {
                    if (tokenBalances.isEmpty()) {
                        Text("No tokens found", color = MaterialTheme.colorScheme.onSurfaceVariant)
                    } else {
                        tokenBalances.forEach { token ->
                            AssetItem(
                                symbol = token.symbol,
                                address = token.tokenType,
                                balance = token.getEffectiveAmount(),
                                decimals = token.decimals
                            )
                        }
                    }
                }
            }
            
            Spacer(Modifier.weight(1f))
            
            // Bottom Action Button as in Dashboard
            Button(
                onClick = onNavigateToSend,
                modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp).height(56.dp).padding(bottom = 8.dp),
                shape = RoundedCornerShape(12.dp)
            ) {
                Text("Send Tokens", fontWeight = FontWeight.Bold)
            }
        }
    }
}

@Composable
fun WalletCard(wallet: WalletRecord, tokenBalances: List<TokenBalance>) {
    ElevatedCard(
        modifier = Modifier.fillMaxSize(),
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.elevatedCardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)
        )
    ) {
        Column(modifier = Modifier.padding(24.dp)) {
            Text(
                wallet.name, 
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(modifier = Modifier.height(8.dp))
            
            val kanariToken = tokenBalances.find { it.tokenType == "0x2::kanari::KANARI" }
            val balance = kanariToken?.getEffectiveAmount() ?: 0L
            val decimals = kanariToken?.decimals ?: 9
            
            Text(
                text = String.format(Locale.US, "%.2f KANARI", balance / Math.pow(10.0, decimals.toDouble())),
                style = MaterialTheme.typography.headlineMedium,
                fontWeight = FontWeight.Black
            )
            
            Spacer(Modifier.weight(1f))
            
            Text(
                wallet.address, 
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1
            )
        }
    }
}

@Composable
fun AddWalletCard(onClick: () -> Unit) {
    OutlinedCard(
        onClick = onClick,
        modifier = Modifier.fillMaxSize(),
        shape = RoundedCornerShape(24.dp),
        border = androidx.compose.foundation.BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.3f))
    ) {
        Column(
            modifier = Modifier.fillMaxSize(),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            Icon(Icons.Default.Add, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
            Spacer(Modifier.height(8.dp))
            Text("Add Wallet", fontWeight = FontWeight.Bold)
        }
    }
}

@Composable
fun AssetItem(symbol: String, address: String, balance: Long, decimals: Int) {
    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp)
            .clip(RoundedCornerShape(16.dp))
            .clickable { /* Detail */ },
        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.2f)
    ) {
        Row(
            modifier = Modifier.padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Box(
                modifier = Modifier
                    .size(44.dp)
                    .clip(CircleShape)
                    .background(MaterialTheme.colorScheme.primary.copy(alpha = 0.15f)),
                contentAlignment = Alignment.Center
            ) {
                Text(
                    symbol.take(1), 
                    color = MaterialTheme.colorScheme.primary, 
                    fontWeight = FontWeight.Black,
                    fontSize = 18.sp
                )
            }
            
            Spacer(Modifier.width(16.dp))
            
            Column(modifier = Modifier.weight(1f)) {
                Text(symbol.ifEmpty { "Unknown" }, fontWeight = FontWeight.Bold, style = MaterialTheme.typography.titleMedium)
                Text(
                    if (address.length > 10) address.take(6) + "..." + address.takeLast(4) else address, 
                    style = MaterialTheme.typography.bodySmall, 
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            
            Column(horizontalAlignment = Alignment.End) {
                val formattedBalance = balance / Math.pow(10.0, decimals.toDouble())
                Text(
                    text = String.format(Locale.US, "%.4f", formattedBalance),
                    fontWeight = FontWeight.Black,
                    style = MaterialTheme.typography.titleMedium
                )
                Text(
                    symbol, 
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}
