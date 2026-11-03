// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.jamesatomc.kanariapp.ui.components.ScaffoldWithBackBar
import com.jamesatomc.kanariapp.ui.components.TokenIcon
import com.jamesatomc.kanariapp.ui.theme.KanariColors
import com.jamesatomc.kanariapp.wallet.WalletViewModel
import com.jamesatomc.kanariapp.wallet.createEscrowDealAndGetId
import com.jamesatomc.kanariapp.wallet.confirmEscrowDelivery
import com.jamesatomc.kanariapp.wallet.releaseEscrowFunds
import com.jamesatomc.kanariapp.wallet.raiseEscrowDispute
import com.jamesatomc.kanariapp.wallet.getEscrowDealState
import com.jamesatomc.kanariapp.wallet.getAllEscrowDeals
import kotlinx.coroutines.launch
import java.math.BigDecimal
import java.math.RoundingMode

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EscrowScreen(
    viewModel: WalletViewModel,
    onBack: () -> Unit
) {
    val scope = rememberCoroutineScope()
    val activeWallet by viewModel.activeWallet.collectAsStateWithLifecycle()
    val tokenBalances by viewModel.tokenBalances.collectAsStateWithLifecycle()

    var dealId by remember { mutableStateOf("") }
    var seller by remember { mutableStateOf("") }
    var amount by remember { mutableStateOf("") }
    var description by remember { mutableStateOf("") }
    var tokenType by remember { mutableStateOf("") }
    var dealObjectId by remember { mutableStateOf("") }
    var proofObjectId by remember { mutableStateOf("") }
    var disputeReason by remember { mutableStateOf("") }
    var checkObjectId by remember { mutableStateOf("") }
    var checkTokenType by remember { mutableStateOf("") }
    var buyerAddress by remember { mutableStateOf("") }
    var allDeals by remember { mutableStateOf<List<Map<String, Any>>>(emptyList()) }
    var selectedDealIdx by remember { mutableStateOf(0) }
    var status by remember { mutableStateOf<String?>(null) }
    var loading by remember { mutableStateOf(false) }
    var dealState by remember { mutableStateOf<Int?>(null) }
    var selectedTab by remember { mutableStateOf(0) }

    LaunchedEffect(tokenBalances) {
        if (tokenType.isEmpty() && tokenBalances.isNotEmpty()) tokenType = tokenBalances.first().tokenType
        if (checkTokenType.isEmpty() && tokenBalances.isNotEmpty()) checkTokenType = tokenBalances.first().tokenType
    }
    LaunchedEffect(activeWallet) {
        if (seller.isEmpty()) activeWallet?.address?.let { seller = it }
        if (buyerAddress.isEmpty()) activeWallet?.address?.let { buyerAddress = it }
    }
    LaunchedEffect(Unit) { if (dealId.isEmpty()) dealId = makeDealId() }

    ScaffoldWithBackBar(title = "Escrow", onBack = onBack) { padding ->
        Column(
            Modifier.fillMaxSize().padding(padding).padding(horizontal = 16.dp).verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            activeWallet?.let { wallet ->
                Surface(
                    color = MaterialTheme.colorScheme.surfaceContainerHighest,
                    shape = RoundedCornerShape(12.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Row(
                        Modifier.padding(12.dp),
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Icon(
                            Icons.Default.AccountCircle,
                            null,
                            tint = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(24.dp)
                        )
                        Column(Modifier.weight(1f)) {
                            Text(
                                wallet.name ?: "Wallet",
                                style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.Bold)
                            )
                            Text(
                                "${wallet.address?.take(8) ?: ""}...${wallet.address?.takeLast(8) ?: ""}",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                }
            }
            status?.let { msg ->
                Surface(
                    color = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.4f),
                    shape = RoundedCornerShape(16.dp),
                    border = androidx.compose.foundation.BorderStroke(
                        1.dp,
                        MaterialTheme.colorScheme.primary.copy(alpha = 0.3f)
                    ),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text(
                        msg,
                        style = MaterialTheme.typography.bodySmall.copy(fontWeight = FontWeight.SemiBold),
                        modifier = Modifier.padding(16.dp)
                    )
                }
            }
            TabRow(selectedTabIndex = selectedTab) {
                Tab(selected = selectedTab == 0, onClick = { selectedTab = 0 }, text = { Text("Create") })
                Tab(selected = selectedTab == 1, onClick = { selectedTab = 1 }, text = { Text("Manage") })
                Tab(selected = selectedTab == 2, onClick = { selectedTab = 2 }, text = { Text("Check") })
            }
            when (selectedTab) {
                0 -> CreateDealCard(
                    dealId,
                    { dealId = it },
                    { dealId = makeDealId() },
                    seller,
                    { seller = it },
                    { activeWallet?.address?.let { seller = it } },
                    amount,
                    { amount = it },
                    description,
                    { description = it },
                    tokenType,
                    { tokenType = it },
                    tokenBalances,
                    loading
                ) {
                    loading = true
                    scope.launch {
                        try {
                            val decimals = tokenBalances.firstOrNull { it.tokenType == tokenType }?.decimals
                            val rawAmount = parseTokenAmount(amount, decimals)
                                ?: throw IllegalArgumentException("Enter a valid amount for the selected token")
                            val ok = viewModel.createEscrowDealAndGetId(
                                dealId,
                                seller,
                                rawAmount,
                                description,
                                tokenType
                            )
                            if (ok != null) {
                                status = "Deal created: $ok"
                                dealObjectId = ok; checkObjectId = ok
                                checkTokenType = tokenType
                                dealId = makeDealId(); amount = ""; description = ""
                            } else status = viewModel.error.value ?: "Failed"
                        } catch (e: Exception) {
                            status = e.message
                        } finally {
                            loading = false
                        }
                    }
                }

                1 -> ManageDealCard(
                    dealObjectId,
                    { dealObjectId = it },
                    proofObjectId,
                    { proofObjectId = it },
                    tokenType,
                    { tokenType = it },
                    tokenBalances,
                    disputeReason,
                    { disputeReason = it },
                    buyerAddress,
                    { buyerAddress = it },
                    allDeals,
                    selectedDealIdx,
                    { idx ->
                        selectedDealIdx = idx
                        val d = allDeals.getOrElse(idx) { emptyMap() }
                        dealObjectId = d["object_id"] as? String ?: dealObjectId
                        proofObjectId = d["proof_id"] as? String ?: proofObjectId
                        (d["coin_type"] as? String)?.let { tokenType = it }
                    },
                    loading,
                    {
                        scope.launch {
                            try {
                                val deals =
                                    viewModel.getAllEscrowDeals(buyerAddress.ifBlank { activeWallet?.address ?: "" })
                                allDeals = deals
                                status = if (deals.isEmpty()) "No deals found" else "Loaded ${deals.size} deal(s)"
                                if (deals.isNotEmpty()) {
                                    selectedDealIdx = 0
                                    val d = deals[0]
                                    dealObjectId = d["object_id"] as? String ?: ""
                                    proofObjectId = d["proof_id"] as? String ?: ""
                                    (d["coin_type"] as? String)?.let { tokenType = it }
                                }
                            } catch (e: Exception) {
                                status = e.message
                            }
                        }
                    },
                    {
                        loading = true
                        scope.launch {
                            try {
                                val ok = viewModel.confirmEscrowDelivery(dealObjectId, proofObjectId, tokenType)
                                status = if (ok) "Delivery confirmed" else (viewModel.error.value ?: "Failed")
                            } catch (e: Exception) {
                                status = e.message
                            } finally {
                                loading = false
                            }
                        }
                    },
                    {
                        loading = true
                        scope.launch {
                            try {
                                val ok = viewModel.releaseEscrowFunds(dealObjectId, proofObjectId, tokenType)
                                status = if (ok) "Funds released" else (viewModel.error.value ?: "Failed")
                            } catch (e: Exception) {
                                status = e.message
                            } finally {
                                loading = false
                            }
                        }
                    },
                    {
                        loading = true
                        scope.launch {
                            try {
                                val ok = viewModel.raiseEscrowDispute(
                                    dealObjectId,
                                    proofObjectId,
                                    disputeReason.ifBlank { "No reason" },
                                    tokenType
                                )
                                status = if (ok) "Dispute raised" else (viewModel.error.value ?: "Failed")
                            } catch (e: Exception) {
                                status = e.message
                            } finally {
                                loading = false
                            }
                        }
                    })

                2 -> CheckDealCard(
                    checkObjectId,
                    { checkObjectId = it },
                    checkTokenType,
                    { checkTokenType = it },
                    tokenBalances,
                    dealState,
                    {
                        scope.launch {
                            try {
                                val st = viewModel.getEscrowDealState(checkObjectId, checkTokenType)
                                dealState = st
                                status = "State: $st"
                            } catch (e: Exception) {
                                status = e.message
                            }
                        }
                    },
                    {
                        scope.launch {
                            try {
                                val deals =
                                    viewModel.getAllEscrowDeals(buyerAddress.ifBlank { activeWallet?.address ?: "" })
                                allDeals = deals
                                status = if (deals.isEmpty()) "No deals found" else "Loaded ${deals.size} deal(s)"
                                if (deals.isNotEmpty()) {
                                    val d = deals[0]
                                    checkObjectId = d["object_id"] as? String ?: checkObjectId
                                    (d["coin_type"] as? String)?.let { checkTokenType = it }
                                }
                            } catch (e: Exception) {
                                status = e.message
                            }
                        }
                    },
                    allDeals,
                    selectedDealIdx,
                    {
                        selectedDealIdx = it;
                        val d = allDeals.getOrElse(it) { emptyMap() }; checkObjectId = d["object_id"] as? String
                        ?: checkObjectId; (d["coin_type"] as? String)?.let { checkTokenType = it }
                    })
            }
        }
    }
}

@Composable
private fun CreateDealCard(
    dealId: String, onDealIdChange: (String) -> Unit, onRefreshDealId: () -> Unit,
    seller: String, onSellerChange: (String) -> Unit, onAutofillSeller: () -> Unit,
    amount: String, onAmountChange: (String) -> Unit,
    description: String, onDescriptionChange: (String) -> Unit,
    tokenType: String, onTokenTypeChange: (String) -> Unit,
    tokenBalances: List<com.jamesatomc.kanariapp.network.models.TokenBalance>,
    loading: Boolean,
    onCreate: () -> Unit
) {
    Card(
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text("Create Escrow Deal", style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold))
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                OutlinedTextField(
                    value = dealId,
                    onValueChange = onDealIdChange,
                    label = { Text("Deal ID") },
                    modifier = Modifier.weight(1f)
                )
                IconButton(onClick = onRefreshDealId) { Icon(Icons.Default.Refresh, "Regenerate") }
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                OutlinedTextField(
                    value = seller,
                    onValueChange = onSellerChange,
                    label = { Text("Seller Address") },
                    modifier = Modifier.weight(1f)
                )
                IconButton(onClick = onAutofillSeller) { Icon(Icons.Default.PersonPin, "Use my address") }
            }
            OutlinedTextField(
                value = amount,
                onValueChange = onAmountChange,
                label = { Text("Amount (token units)") },
                modifier = Modifier.fillMaxWidth()
            )
            OutlinedTextField(
                value = description,
                onValueChange = onDescriptionChange,
                label = { Text("Description") },
                modifier = Modifier.fillMaxWidth()
            )
            TokenDropdown(tokenType, onTokenTypeChange, tokenBalances)
            Button(onClick = onCreate, enabled = !loading, modifier = Modifier.fillMaxWidth()) {
                if (loading) CircularProgressIndicator(modifier = Modifier.size(18.dp)) else Text("Create Deal & Lock Funds")
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ManageDealCard(
    dealObjectId: String, onDealObjectIdChange: (String) -> Unit,
    proofObjectId: String, onProofObjectIdChange: (String) -> Unit,
    tokenType: String, onTokenTypeChange: (String) -> Unit,
    tokenBalances: List<com.jamesatomc.kanariapp.network.models.TokenBalance>,
    disputeReason: String, onDisputeReasonChange: (String) -> Unit,
    buyerAddress: String, onBuyerAddressChange: (String) -> Unit,
    allDeals: List<Map<String, Any>>, selectedDealIdx: Int, onSelectDeal: (Int) -> Unit,
    loading: Boolean,
    onLoadDeals: () -> Unit,
    onConfirm: () -> Unit, onRelease: () -> Unit, onDispute: () -> Unit
) {
    Card(
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text("Manage Escrow Deal", style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold))
            OutlinedTextField(
                value = buyerAddress,
                onValueChange = onBuyerAddressChange,
                label = { Text("Buyer Address (load deals)") },
                modifier = Modifier.fillMaxWidth()
            )
            Button(onClick = onLoadDeals, modifier = Modifier.fillMaxWidth()) { Text("Load My Deals") }
            if (allDeals.isNotEmpty()) {
                var expanded by remember { mutableStateOf(false) }
                ExposedDropdownMenuBox(expanded = expanded, onExpandedChange = { expanded = it }) {
                    OutlinedTextField(
                        value = allDeals.getOrElse(selectedDealIdx) { emptyMap() }["object_id"] as? String ?: "",
                        onValueChange = {},
                        readOnly = true,
                        label = { Text("Select Deal") },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
                        modifier = Modifier.menuAnchor().fillMaxWidth()
                    )
                    ExposedDropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
                        allDeals.forEachIndexed { idx, d ->
                            DropdownMenuItem(
                                text = { Text("${(d["object_id"] as? String)?.take(12) ?: ""}... (state ${d["state"]})") },
                                onClick = { onSelectDeal(idx); expanded = false }
                            )
                        }
                    }
                }
            }
            OutlinedTextField(
                value = dealObjectId,
                onValueChange = onDealObjectIdChange,
                label = { Text("Deal Object ID (auto-filled)") },
                modifier = Modifier.fillMaxWidth(),
                enabled = false
            )
            OutlinedTextField(
                value = proofObjectId,
                onValueChange = onProofObjectIdChange,
                label = { Text("Proof Object ID (auto-filled)") },
                modifier = Modifier.fillMaxWidth(),
                enabled = false
            )
            SelectedTokenField(tokenType, tokenBalances)
            OutlinedTextField(
                value = disputeReason,
                onValueChange = onDisputeReasonChange,
                label = { Text("Dispute Reason") },
                modifier = Modifier.fillMaxWidth()
            )
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                OutlinedButton(
                    onClick = onConfirm,
                    enabled = !loading,
                    modifier = Modifier.weight(1f)
                ) { Text("Confirm Delivery") }
                Button(
                    onClick = onRelease,
                    enabled = !loading,
                    modifier = Modifier.weight(1f)
                ) { Text("Release Funds") }
            }
            OutlinedButton(
                onClick = onDispute,
                enabled = !loading,
                modifier = Modifier.fillMaxWidth()
            ) { Text("Raise Dispute & Refund") }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun CheckDealCard(
    checkObjectId: String, onCheckObjectIdChange: (String) -> Unit,
    tokenType: String, onTokenTypeChange: (String) -> Unit,
    tokenBalances: List<com.jamesatomc.kanariapp.network.models.TokenBalance>,
    dealState: Int?,
    onCheck: () -> Unit,
    onLoadDeals: () -> Unit,
    allDeals: List<Map<String, Any>>,
    selectedDealIdx: Int,
    onSelectDeal: (Int) -> Unit
) {
    Card(
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text("Check Deal State", style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold))
            OutlinedTextField(
                value = checkObjectId,
                onValueChange = onCheckObjectIdChange,
                label = { Text("Deal Object ID") },
                modifier = Modifier.fillMaxWidth(),
                enabled = false
            )
            Button(onClick = onLoadDeals, modifier = Modifier.fillMaxWidth()) { Text("Load My Deals") }
            if (allDeals.isNotEmpty()) {
                var expanded by remember { mutableStateOf(false) }
                ExposedDropdownMenuBox(expanded = expanded, onExpandedChange = { expanded = it }) {
                    OutlinedTextField(
                        value = allDeals.getOrElse(selectedDealIdx) { emptyMap() }["object_id"] as? String
                            ?: checkObjectId,
                        onValueChange = {},
                        readOnly = true,
                        label = { Text("Select Deal") },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
                        modifier = Modifier.menuAnchor().fillMaxWidth()
                    )
                    ExposedDropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
                        allDeals.forEachIndexed { idx, d ->
                            DropdownMenuItem(
                                text = { Text("${(d["object_id"] as? String)?.take(12) ?: ""}... (state ${d["state"]})") },
                                onClick = { onSelectDeal(idx); expanded = false }
                            )
                        }
                    }
                }
            }
            SelectedTokenField(tokenType, tokenBalances)
            Button(onClick = onCheck, modifier = Modifier.fillMaxWidth()) { Text("Check State") }
            dealState?.let {
                Text(
                    "Current state: $it  (1=Locked, 2=Delivered, 3=Completed, 4=Disputed)",
                    style = MaterialTheme.typography.bodySmall
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun TokenDropdown(
    selected: String,
    onSelect: (String) -> Unit,
    options: List<com.jamesatomc.kanariapp.network.models.TokenBalance>
) {
    var expanded by remember { mutableStateOf(false) }
    val selectedToken = options.firstOrNull { it.tokenType == selected }
    ExposedDropdownMenuBox(expanded = expanded, onExpandedChange = { expanded = it }) {
        OutlinedTextField(
            value = selectedToken?.symbol ?: selected,
            onValueChange = {},
            readOnly = true,
            label = { Text("Token") },
            leadingIcon = { selectedToken?.let { TokenIcon(it, Modifier.size(24.dp)) } },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
            modifier = Modifier.menuAnchor().fillMaxWidth()
        )
        ExposedDropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            options.forEach { token ->
                DropdownMenuItem(
                    text = { Text(token.symbol.ifBlank { token.tokenType }) },
                    leadingIcon = { TokenIcon(token, Modifier.size(24.dp)) },
                    onClick = { onSelect(token.tokenType); expanded = false }
                )
            }
        }
    }
}

private fun makeDealId(): String {
    val ts = System.currentTimeMillis()
    val rnd = (System.nanoTime() % 10000).toString().padStart(4, '0')
    return "deal-$ts-$rnd"
}

private fun parseTokenAmount(input: String, decimals: Int?): Long? {
    if (decimals == null || decimals < 0) return null
    return try {
        BigDecimal(input.trim())
            .movePointRight(decimals)
            .setScale(0, RoundingMode.UNNECESSARY)
            .longValueExact()
            .takeIf { it > 0L }
    } catch (_: Exception) {
        null
    }
}

/**
 * Read-only token field for Manage/Check tabs: the coin is derived from the
 * selected EscrowDeal's generic type, so the user never has to pick manually.
 */
@Composable
private fun SelectedTokenField(
    selected: String,
    options: List<com.jamesatomc.kanariapp.network.models.TokenBalance>
) {
    val selectedToken = options.firstOrNull { it.tokenType == selected }
    OutlinedTextField(
        value = selectedToken?.symbol?.takeIf { it.isNotBlank() } ?: selected,
        onValueChange = {},
        readOnly = true,
        label = { Text("Token (auto)") },
        leadingIcon = { selectedToken?.let { TokenIcon(it, Modifier.size(24.dp)) } },
        modifier = Modifier.fillMaxWidth()
    )
}
