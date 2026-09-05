package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Security
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import com.jamesatomc.kanariapp.network.EscrowViewModel
import com.jamesatomc.kanariapp.network.models.EscrowConstants
import com.jamesatomc.kanariapp.network.models.EscrowDeal
import com.jamesatomc.kanariapp.ui.components.ErrorBanner
import com.jamesatomc.kanariapp.ui.components.formatAmount
import com.jamesatomc.kanariapp.ui.components.formatMist
import com.jamesatomc.kanariapp.ui.components.LoadingEmptyState
import com.jamesatomc.kanariapp.ui.components.parseAmountToMist
import com.jamesatomc.kanariapp.ui.components.SmartTabRow
import com.jamesatomc.kanariapp.wallet.WalletViewModel
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EscrowScreen(
    walletViewModel: WalletViewModel,
    escrowViewModel: EscrowViewModel = viewModel(factory = EscrowViewModel.Factory(walletViewModel.getClient()))
) {
    val isLoading by escrowViewModel.isLoading.collectAsStateWithLifecycle()
    val deals by escrowViewModel.deals.collectAsStateWithLifecycle()
    val error by escrowViewModel.error.collectAsStateWithLifecycle()
    val activeWallet by walletViewModel.activeWallet.collectAsStateWithLifecycle()

    var selectedTab by remember { mutableIntStateOf(0) }
    val tabs = listOf("Buy", "Sell", "History")

    LaunchedEffect(activeWallet) {
        activeWallet?.let { escrowViewModel.loadDeals(it.address) }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Kanari Escrow", fontWeight = FontWeight.Black) },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.background,
                    titleContentColor = MaterialTheme.colorScheme.onBackground
                ),
                actions = {
                    IconButton(onClick = { activeWallet?.let { escrowViewModel.loadDeals(it.address) } }) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                }
            )
        },
        containerColor = MaterialTheme.colorScheme.background
    ) { padding ->
        Column(modifier = Modifier.padding(padding)) {
            if (error != null) ErrorBanner(
                error = error!!,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
            )
            SmartTabRow(selectedTabIndex = selectedTab, tabs = tabs, onTabSelected = { selectedTab = it })

            when (selectedTab) {
                0 -> CreateDealTab(activeWallet?.address ?: "", escrowViewModel)
                1 -> ActiveDealsTab(deals.filter { it.state < 2 }, isLoading, activeWallet?.address, escrowViewModel)
                2 -> ActiveDealsTab(deals.filter { it.state >= 2 }, isLoading, activeWallet?.address, escrowViewModel)
            }
        }
    }
}

@Composable
fun CreateDealTab(walletAddress: String, viewModel: EscrowViewModel) {
    var sellerAddress by remember { mutableStateOf("") }
    var amount by remember { mutableStateOf("") }
    var description by remember { mutableStateOf("") }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text("Create Secure Escrow", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
        Text(
            "Lock funds in a smart contract. The seller only receives payment after you confirm delivery.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )

        Spacer(Modifier.height(8.dp))

        OutlinedTextField(
            value = sellerAddress,
            onValueChange = { sellerAddress = it },
            label = { Text("Seller Address") },
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(12.dp)
        )

        OutlinedTextField(
            value = amount,
            onValueChange = { amount = it },
            label = { Text("Amount (KANARI)") },
            modifier = Modifier.fillMaxWidth(),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
            shape = RoundedCornerShape(12.dp)
        )

        OutlinedTextField(
            value = description,
            onValueChange = { description = it },
            label = { Text("Description (Optional)") },
            modifier = Modifier.fillMaxWidth(),
            minLines = 2,
            shape = RoundedCornerShape(12.dp)
        )

        Spacer(Modifier.weight(1f))

        Button(
            onClick = {
                val amtLong = (parseAmountToMist(amount, 9) ?: 0uL).toLong()
                viewModel.createDeal(walletAddress, sellerAddress, amtLong, "0x2::kanari::KANARI", description)
            },
            modifier = Modifier.fillMaxWidth().height(56.dp),
            shape = RoundedCornerShape(12.dp),
            enabled = sellerAddress.isNotEmpty() && amount.isNotEmpty()
        ) {
            Icon(Icons.Default.Security, contentDescription = null)
            Spacer(Modifier.width(12.dp))
            Text("Create Escrow Deal", fontWeight = FontWeight.Bold)
        }
    }
}

@Composable
fun ActiveDealsTab(
    deals: List<EscrowDeal>,
    isLoading: Boolean,
    walletAddress: String? = null,
    viewModel: EscrowViewModel? = null
) {
    LoadingEmptyState(
        isLoading = isLoading,
        items = deals,
        emptyIcon = Icons.Default.Security,
        emptyText = "No deals found"
    ) {
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            items(deals) { deal ->
                DealCard(deal, walletAddress, viewModel)
            }
        }
    }
}

@Composable
fun DealCard(deal: EscrowDeal, walletAddress: String? = null, viewModel: EscrowViewModel? = null) {
    ElevatedCard(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.elevatedCardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)
        )
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "Deal ID: ${deal.dealId.take(8)}...",
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.primary
                )
                Badge(
                    containerColor = when (deal.state) {
                        0 -> MaterialTheme.colorScheme.primary.copy(alpha = 0.2f)
                        3 -> MaterialTheme.colorScheme.error.copy(alpha = 0.2f)
                        else -> MaterialTheme.colorScheme.secondary.copy(alpha = 0.2f)
                    }
                ) {
                    Text(
                        EscrowConstants.getStateName(deal.state),
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
                        style = MaterialTheme.typography.labelSmall
                    )
                }
            }

            Spacer(Modifier.height(12.dp))

            Text(
                text = "${formatAmount(deal.amount, 9, 2)} KANARI",
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.Black
            )

            if (deal.description.isNotEmpty()) {
                Text(
                    deal.description,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1
                )
            }

            Spacer(Modifier.height(16.dp))

            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (deal.state == 0) {
                    Button(
                        onClick = {
                            if (walletAddress != null && viewModel != null) viewModel.confirmDelivery(
                                walletAddress,
                                deal
                            )
                        },
                        modifier = Modifier.weight(1f).height(40.dp),
                        shape = RoundedCornerShape(8.dp)
                    ) {
                        Text("Confirm Delivery", fontSize = 12.sp)
                    }
                    OutlinedButton(
                        onClick = { /* Dispute */ },
                        modifier = Modifier.weight(1f).height(40.dp),
                        shape = RoundedCornerShape(8.dp),
                        colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error)
                    ) {
                        Text("Dispute", fontSize = 12.sp)
                    }
                }
            }
        }
    }
}