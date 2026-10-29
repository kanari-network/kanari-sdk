// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.jamesatomc.kanariapp.network.models.DEEPBOOK_ORDER_TYPES
import com.jamesatomc.kanariapp.network.models.DEEPBOOK_REGISTRY_ID
import com.jamesatomc.kanariapp.network.models.DeepbookOrderLevel
import com.jamesatomc.kanariapp.network.models.TOKEN_KANARI
import com.jamesatomc.kanariapp.network.models.TOKEN_USD
import com.jamesatomc.kanariapp.ui.components.LoadingButton
import com.jamesatomc.kanariapp.ui.components.ScaffoldWithBackBar
import com.jamesatomc.kanariapp.ui.components.formatAmountExactOrUnknown
import com.jamesatomc.kanariapp.ui.components.parseAmountToMist
import com.jamesatomc.kanariapp.ui.theme.KanariColors
import com.jamesatomc.kanariapp.wallet.WalletViewModel
import com.jamesatomc.kanariapp.wallet.cancelDeepbookOrder
import com.jamesatomc.kanariapp.wallet.getBookAddress
import com.jamesatomc.kanariapp.wallet.getOrderBookLevels
import com.jamesatomc.kanariapp.wallet.placeAsk
import com.jamesatomc.kanariapp.wallet.placeBid
import kotlinx.coroutines.launch
import java.util.Locale

// PRICE_SCALE of the on-chain module: price is stored as quote * 10^9 / base.
private const val PRICE_SCALE = 1_000_000_000.0

/** Max order book levels fetched per side — enough for a readable depth view. */
private const val MAX_BOOK_LEVELS = 10

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DeepBookScreen(
    viewModel: WalletViewModel,
    onBack: () -> Unit
) {
    val scope = rememberCoroutineScope()
    val isLoading by viewModel.isLoading.collectAsStateWithLifecycle()
    val tokenBalances by viewModel.tokenBalances.collectAsStateWithLifecycle()
    val activeWallet by viewModel.activeWallet.collectAsStateWithLifecycle()

    // --- Dynamic Trading Pair ---
    var baseType by remember { mutableStateOf(TOKEN_KANARI) }
    var quoteType by remember { mutableStateOf(TOKEN_USD) }
    val baseSymbol = remember(baseType) { baseType.split("::").lastOrNull() ?: "BASE" }
    val quoteSymbol = remember(quoteType) { quoteType.split("::").lastOrNull() ?: "QUOTE" }

    // --- Order form state ---
    var bookId by remember { mutableStateOf("") }
    var price by remember { mutableStateOf("") }
    var quantity by remember { mutableStateOf("") }
    var orderTypeIndex by remember { mutableStateOf(0) }
    var isBidSide by remember { mutableStateOf(true) } // true = Bid (buy), false = Ask (sell)
    var dropdownExpanded by remember { mutableStateOf(false) }
    var orderResult by remember { mutableStateOf<String?>(null) }
    var bidLevels by remember { mutableStateOf<List<DeepbookOrderLevel>>(emptyList()) }
    var askLevels by remember { mutableStateOf<List<DeepbookOrderLevel>>(emptyList()) }

    // Normalized (padded, lowercase) address of the active wallet — used to
    // mark the user's own orders in the book and offer cancel on them.
    val myAddress = remember(activeWallet) {
        activeWallet?.address
            ?.removePrefix("0x")?.removePrefix("0X")
            ?.lowercase()?.padStart(64, '0')
    }

    val refreshOrderBook = {
        scope.launch {
            bidLevels = viewModel.getOrderBookLevels(bookId, isBid = true, MAX_BOOK_LEVELS, baseType, quoteType)
            askLevels = viewModel.getOrderBookLevels(bookId, isBid = false, MAX_BOOK_LEVELS, baseType, quoteType)
            orderResult = "Order book refreshed"
        }
    }

    // Dynamically fetch bookId from the registry view function whenever the
    // trading pair changes, then load the book.
    LaunchedEffect(baseType, quoteType) {
        val fetched = viewModel.getBookAddress(DEEPBOOK_REGISTRY_ID, baseType, quoteType)
        if (!fetched.isNullOrBlank()) {
            bookId = fetched
            orderResult = "Book address discovered from registry"
        } else {
            orderResult = "Book address not found in registry"
        }
        refreshOrderBook()
    }

    val orderType = DEEPBOOK_ORDER_TYPES[orderTypeIndex].value

    // Available balance for base token
    val baseBalance = remember(tokenBalances, baseType) {
        tokenBalances.find { it.tokenType.equals(baseType, ignoreCase = true) }?.getEffectiveAmount() ?: 0L
    }
    val baseDecimals = remember(tokenBalances, baseType) {
        tokenBalances.find { it.tokenType.equals(baseType, ignoreCase = true) }?.decimals ?: 9
    }

    ScaffoldWithBackBar(title = "DeepBook", onBack = onBack) { padding ->
        Column(
            Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 16.dp)
                .verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            Text("Trade on the CLOB", style = MaterialTheme.typography.headlineSmall.copy(fontWeight = FontWeight.Bold))

            // Registry Status Banner
            Surface(
                color = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.4f),
                shape = RoundedCornerShape(16.dp),
                border = androidx.compose.foundation.BorderStroke(1.dp, MaterialTheme.colorScheme.primary.copy(alpha = 0.3f)),
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    Modifier.padding(16.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Icon(Icons.Default.CheckCircle, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(20.dp))
                        Text("Registry & Order Book Discovery", style = MaterialTheme.typography.bodySmall.copy(fontWeight = FontWeight.SemiBold))
                    }
                    OutlinedButton(
                        onClick = { refreshOrderBook() },
                        shape = RoundedCornerShape(10.dp),
                        contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp),
                        modifier = Modifier.height(32.dp)
                    ) {
                        Icon(Icons.Default.Refresh, contentDescription = null, modifier = Modifier.size(14.dp))
                        Spacer(Modifier.width(4.dp))
                        Text("Refresh", style = MaterialTheme.typography.labelSmall)
                    }
                }
            }

            // Order Book Section
            OrderBookSection(
                baseSymbol = baseSymbol,
                quoteSymbol = quoteSymbol,
                bidLevels = bidLevels,
                askLevels = askLevels,
                myAddress = myAddress,
                baseDecimals = baseDecimals,
                onRefresh = { refreshOrderBook() },
                onCancelOrder = { orderId ->
                    scope.launch {
                        val ok = viewModel.cancelDeepbookOrder(bookId, orderId, baseType, quoteType)
                        orderResult = if (ok) "Order $orderId cancelled" else viewModel.error.value
                        if (ok) refreshOrderBook()
                    }
                }
            )

            // Order Form Card
            Card(
                shape = RoundedCornerShape(24.dp),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {

                    // Row 1: Base Type & Quote Type Inputs + Side Selector (Buy/Sell)
                    Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        Column(Modifier.weight(1.3f), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                            Text("Base Type", style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.Bold))
                            OutlinedTextField(
                                value = baseType,
                                onValueChange = { baseType = it.trim() },
                                singleLine = true,
                                shape = RoundedCornerShape(12.dp),
                                modifier = Modifier.fillMaxWidth()
                            )
                            Text("Quote Type", style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.Bold))
                            OutlinedTextField(
                                value = quoteType,
                                onValueChange = { quoteType = it.trim() },
                                singleLine = true,
                                shape = RoundedCornerShape(12.dp),
                                modifier = Modifier.fillMaxWidth()
                            )
                        }
                        Column(Modifier.weight(1f)) {
                            Text("Side", style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.Bold))
                            Spacer(Modifier.height(4.dp))
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .height(116.dp)
                                    .background(MaterialTheme.colorScheme.surfaceContainerHighest, RoundedCornerShape(12.dp))
                                    .padding(4.dp),
                                horizontalArrangement = Arrangement.spacedBy(4.dp)
                            ) {
                                Surface(
                                    onClick = { isBidSide = true },
                                    shape = RoundedCornerShape(10.dp),
                                    color = if (isBidSide) KanariColors.Lime else Color.Transparent,
                                    modifier = Modifier.weight(1f).fillMaxHeight()
                                ) {
                                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                                        Text(
                                            "Buy",
                                            style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.Bold),
                                            color = if (isBidSide) KanariColors.Ink else MaterialTheme.colorScheme.onSurfaceVariant
                                        )
                                    }
                                }
                                Surface(
                                    onClick = { isBidSide = false },
                                    shape = RoundedCornerShape(10.dp),
                                    color = if (!isBidSide) MaterialTheme.colorScheme.errorContainer else Color.Transparent,
                                    modifier = Modifier.weight(1f).fillMaxHeight()
                                ) {
                                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                                        Text(
                                            "Sell",
                                            style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.Bold),
                                            color = if (!isBidSide) MaterialTheme.colorScheme.onErrorContainer else MaterialTheme.colorScheme.onSurfaceVariant
                                        )
                                    }
                                }
                            }
                        }
                    }

                    // Selected Pair Book ID Box
                    Surface(
                        color = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.2f),
                        shape = RoundedCornerShape(12.dp),
                        border = androidx.compose.foundation.BorderStroke(1.dp, MaterialTheme.colorScheme.primary.copy(alpha = 0.2f)),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Column(Modifier.padding(12.dp)) {
                            Text(
                                "Discovered Pair: $baseSymbol / $quoteSymbol",
                                style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.Bold),
                                color = MaterialTheme.colorScheme.primary
                            )
                            Spacer(Modifier.height(2.dp))
                            Text(
                                "Book ID: ${bookId.ifEmpty { "(discovering…)" }}",
                                style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis
                            )
                        }
                    }

                    // Order Type
                    Column {
                        Text("Order Type", style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.Bold))
                        Spacer(Modifier.height(4.dp))
                        ExposedDropdownMenuBox(
                            expanded = dropdownExpanded,
                            onExpandedChange = { dropdownExpanded = it },
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            OutlinedTextField(
                                value = DEEPBOOK_ORDER_TYPES[orderTypeIndex].label,
                                onValueChange = {},
                                readOnly = true,
                                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = dropdownExpanded) },
                                shape = RoundedCornerShape(12.dp),
                                modifier = Modifier.menuAnchor(type = ExposedDropdownMenuAnchorType.PrimaryEditable, enabled = true).fillMaxWidth()
                            )
                            ExposedDropdownMenu(
                                expanded = dropdownExpanded,
                                onDismissRequest = { dropdownExpanded = false }
                            ) {
                                DEEPBOOK_ORDER_TYPES.forEachIndexed { index, ot ->
                                    DropdownMenuItem(
                                        text = { Text(ot.label) },
                                        onClick = {
                                            orderTypeIndex = index
                                            dropdownExpanded = false
                                        }
                                    )
                                }
                            }
                        }
                    }

                    // Price + Quantity Row
                    Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        Column(Modifier.weight(1f)) {
                            Text("Price (human)", style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.Bold))
                            Spacer(Modifier.height(4.dp))
                            OutlinedTextField(
                                value = price,
                                onValueChange = { price = it },
                                placeholder = { Text("e.g. 1.234") },
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                                shape = RoundedCornerShape(12.dp),
                                modifier = Modifier.fillMaxWidth()
                            )
                        }
                        Column(Modifier.weight(1f)) {
                            Text("Quantity (base)", style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.Bold))
                            Spacer(Modifier.height(4.dp))
                            OutlinedTextField(
                                value = quantity,
                                onValueChange = { quantity = it },
                                placeholder = { Text("e.g. 100") },
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                                shape = RoundedCornerShape(12.dp),
                                modifier = Modifier.fillMaxWidth()
                            )
                            Spacer(Modifier.height(2.dp))
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Text(
                                    "Avail: ${formatAmountExactOrUnknown(baseBalance, baseDecimals)} $baseSymbol",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                                Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                                    OutlinedButton(
                                        onClick = {
                                            if (baseBalance > 0) {
                                                quantity = formatAmountExactOrUnknown(baseBalance / 2, baseDecimals)
                                            }
                                        },
                                        shape = RoundedCornerShape(6.dp),
                                        contentPadding = PaddingValues(horizontal = 6.dp, vertical = 2.dp),
                                        modifier = Modifier.height(24.dp)
                                    ) {
                                        Text("50%", style = MaterialTheme.typography.labelSmall)
                                    }
                                    OutlinedButton(
                                        onClick = {
                                            if (baseBalance > 0) {
                                                quantity = formatAmountExactOrUnknown(baseBalance, baseDecimals)
                                            }
                                        },
                                        shape = RoundedCornerShape(6.dp),
                                        contentPadding = PaddingValues(horizontal = 6.dp, vertical = 2.dp),
                                        modifier = Modifier.height(24.dp)
                                    ) {
                                        Text("100%", style = MaterialTheme.typography.labelSmall)
                                    }
                                }
                            }
                        }
                    }

                    // Preview Quote Box
                    Surface(
                        color = MaterialTheme.colorScheme.surfaceContainerHighest,
                        shape = RoundedCornerShape(12.dp),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Column(Modifier.padding(12.dp)) {
                            Text("Preview Quote", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Spacer(Modifier.height(2.dp))
                            val p = price.toDoubleOrNull() ?: 0.0
                            val q = quantity.toDoubleOrNull() ?: 0.0
                            val estimatedQuote = if (p > 0 && q > 0) p * q else 0.0
                            Text(
                                text = if (estimatedQuote > 0) "~${String.format(Locale.US, "%.4f", estimatedQuote)} $quoteSymbol" else "-",
                                style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
                                color = MaterialTheme.colorScheme.onSurface
                            )
                        }
                    }

                    Spacer(Modifier.height(8.dp))

                    // Place Order Button
                    LoadingButton(
                        onClick = {
                            scope.launch {
                                val book = bookId.trim()
                                if (book.isEmpty()) {
                                    orderResult = "Book address required"
                                    return@launch
                                }
                                val priceDouble = price.toDoubleOrNull() ?: run {
                                    orderResult = "Invalid price"; return@launch
                                }
                                val p = (priceDouble * PRICE_SCALE).toULong()
                                if (p == 0uL) {
                                    orderResult = "Price rounds to zero"
                                    return@launch
                                }
                                val q = parseAmountToMist(quantity, baseDecimals) ?: run {
                                    orderResult = "Invalid quantity"; return@launch
                                }
                                // Pass the order TYPE VALUE (0..3), not the list index.
                                val ok = if (isBidSide) {
                                    viewModel.placeBid(book, p, q, "", orderType, baseType, quoteType)
                                } else {
                                    viewModel.placeAsk(book, p, q, "", orderType, baseType, quoteType)
                                }
                                orderResult = if (ok) {
                                    if (isBidSide) "Bid placed successfully" else "Ask placed successfully"
                                } else {
                                    viewModel.error.value
                                }
                                if (ok) refreshOrderBook()
                            }
                        },
                        text = if (isBidSide) "Place Buy Order" else "Place Sell Order",
                        enabled = bookId.isNotEmpty() && price.isNotEmpty() && quantity.isNotEmpty(),
                        isLoading = isLoading,
                        modifier = Modifier.fillMaxWidth().height(54.dp),
                        colors = ButtonDefaults.buttonColors(
                            containerColor = if (isBidSide) KanariColors.Lime else MaterialTheme.colorScheme.error,
                            contentColor = if (isBidSide) KanariColors.Ink else MaterialTheme.colorScheme.onError
                        )
                    )
                }
            }

            // Result banner
            orderResult?.let { result ->
                val isSuccess = result.contains("successfully", ignoreCase = true) || result.contains("refreshed", ignoreCase = true)
                Surface(
                    color = if (isSuccess) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.errorContainer,
                    shape = RoundedCornerShape(12.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text(
                        result,
                        style = MaterialTheme.typography.bodyMedium,
                        color = if (isSuccess) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onErrorContainer,
                        modifier = Modifier.padding(12.dp)
                    )
                }
            }
        }
    }
}

@Composable
fun OrderBookSection(
    baseSymbol: String,
    quoteSymbol: String,
    bidLevels: List<DeepbookOrderLevel>,
    askLevels: List<DeepbookOrderLevel>,
    myAddress: String?,
    baseDecimals: Int,
    onRefresh: () -> Unit,
    onCancelOrder: (ULong) -> Unit
) {
    Card(
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column {
                    Text("Order Book", style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold))
                    Text("$baseSymbol / $quoteSymbol", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                OutlinedButton(
                    onClick = onRefresh,
                    shape = RoundedCornerShape(10.dp),
                    contentPadding = PaddingValues(horizontal = 10.dp, vertical = 4.dp),
                    modifier = Modifier.height(32.dp)
                ) {
                    Icon(Icons.Default.Refresh, contentDescription = null, modifier = Modifier.size(14.dp))
                    Spacer(Modifier.width(4.dp))
                    Text("Refresh", style = MaterialTheme.typography.labelSmall)
                }
            }

            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                OrderBookColumn(
                    title = "Bids (Buy)",
                    accent = Color(0xFF16A34A),
                    dot = Color(0xFF22C55E),
                    levels = bidLevels,
                    myAddress = myAddress,
                    baseDecimals = baseDecimals,
                    emptyText = "No active bids",
                    onCancelOrder = onCancelOrder,
                    modifier = Modifier.weight(1f)
                )
                OrderBookColumn(
                    title = "Asks (Sell)",
                    accent = Color(0xFFDC2626),
                    dot = Color(0xFFEF4444),
                    levels = askLevels,
                    myAddress = myAddress,
                    baseDecimals = baseDecimals,
                    emptyText = "No active asks",
                    onCancelOrder = onCancelOrder,
                    modifier = Modifier.weight(1f)
                )
            }
        }
    }
}

@Composable
private fun OrderBookColumn(
    title: String,
    accent: Color,
    dot: Color,
    levels: List<DeepbookOrderLevel>,
    myAddress: String?,
    baseDecimals: Int,
    emptyText: String,
    onCancelOrder: (ULong) -> Unit,
    modifier: Modifier = Modifier
) {
    Column(modifier, verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            Box(Modifier.size(8.dp).background(dot, CircleShape))
            Text("$title - ${levels.size}", style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.Bold), color = accent)
        }
        if (levels.isEmpty()) {
            Surface(
                color = MaterialTheme.colorScheme.surfaceContainerHighest,
                shape = RoundedCornerShape(12.dp),
                modifier = Modifier.fillMaxWidth().height(76.dp)
            ) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text(emptyText, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        } else {
            // Header row
            Row(
                Modifier.fillMaxWidth().padding(horizontal = 8.dp),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("Price", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text("Qty", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            levels.forEach { level ->
                val isMine = myAddress != null && level.maker.equals(myAddress, ignoreCase = true)
                Surface(
                    color = if (isMine) MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.35f)
                    else MaterialTheme.colorScheme.surfaceContainerHighest,
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Row(
                        Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Column {
                            Text(
                                String.format(Locale.US, "%.4f", level.price.toLong() / PRICE_SCALE),
                                style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.Bold),
                                color = accent
                            )
                            Text(
                                "#${level.orderId}" + if (isMine) " • yours" else "",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                            Text(
                                formatAmountExactOrUnknown(level.remaining.toLong(), baseDecimals),
                                style = MaterialTheme.typography.labelMedium
                            )
                            if (isMine) {
                                IconButton(
                                    onClick = { onCancelOrder(level.orderId) },
                                    modifier = Modifier.size(24.dp)
                                ) {
                                    Icon(
                                        Icons.Default.Close,
                                        contentDescription = "Cancel order ${level.orderId}",
                                        tint = MaterialTheme.colorScheme.error,
                                        modifier = Modifier.size(16.dp)
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
