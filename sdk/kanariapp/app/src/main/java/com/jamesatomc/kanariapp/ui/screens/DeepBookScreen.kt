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
import com.jamesatomc.kanariapp.network.models.DeepbookBookInfo
import com.jamesatomc.kanariapp.network.models.DeepbookOrderLevel
import com.jamesatomc.kanariapp.network.typeTagsEqual
import com.jamesatomc.kanariapp.ui.components.LoadingButton
import com.jamesatomc.kanariapp.ui.components.ScaffoldWithBackBar
import com.jamesatomc.kanariapp.ui.components.formatAmountExactOrUnknown
import com.jamesatomc.kanariapp.ui.components.parseAmountToMist
import com.jamesatomc.kanariapp.ui.theme.KanariColors
import com.jamesatomc.kanariapp.wallet.WalletViewModel
import com.jamesatomc.kanariapp.wallet.cancelDeepbookOrder
import com.jamesatomc.kanariapp.wallet.discoverDeepbookBooks
import com.jamesatomc.kanariapp.wallet.getAskCount
import com.jamesatomc.kanariapp.wallet.getBidCount
import com.jamesatomc.kanariapp.wallet.getBookAddress
import com.jamesatomc.kanariapp.wallet.getOrderBookLevels
import com.jamesatomc.kanariapp.wallet.placeAsk
import com.jamesatomc.kanariapp.wallet.placeBid
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import java.math.BigDecimal
import java.math.BigInteger
import java.util.Locale

// The on-chain price representation uses nine decimal places regardless of
// the base and quote tokens' own decimal counts.
private const val PRICE_DECIMALS = 9
private val PRICE_SCALE = BigInteger.TEN.pow(PRICE_DECIMALS)

/** Max order book levels fetched per side — enough for a readable depth view. */
private const val MAX_BOOK_LEVELS = 10

private fun quoteAmountRaw(
    priceRaw: ULong,
    quantityRaw: ULong,
    baseDecimals: Int,
    quoteDecimals: Int
): BigInteger {
    val numerator = BigInteger(priceRaw.toString())
        .multiply(BigInteger(quantityRaw.toString()))
        .multiply(BigInteger.TEN.pow(quoteDecimals))
    val denominator = PRICE_SCALE.multiply(BigInteger.TEN.pow(baseDecimals))
    return numerator.divide(denominator)
}

private fun formatRawAmount(amount: BigInteger, decimals: Int): String =
    BigDecimal(amount).movePointLeft(decimals).stripTrailingZeros().toPlainString()

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
    val environment by viewModel.environment.collectAsStateWithLifecycle()

    // --- Discovered Books (dropdown, Book ID selector) ---
    var discoveredBooks by remember { mutableStateOf<List<DeepbookBookInfo>>(emptyList()) }
    var selectedBook by remember { mutableStateOf<DeepbookBookInfo?>(null) }
    var bookDropdownExpanded by remember { mutableStateOf(false) }
    var discoveringBooks by remember { mutableStateOf(false) }

    // Defaults come from the active network deployment, while the selected
    // on-chain book remains authoritative for its token metadata.
    val defaultPair = environment.deepbook.pairs.first()
    val baseType = selectedBook?.baseType ?: defaultPair.baseType
    val quoteType = selectedBook?.quoteType ?: defaultPair.quoteType
    val baseSymbol = selectedBook?.baseSymbol ?: defaultPair.baseType.substringAfterLast("::")
    val quoteSymbol = selectedBook?.quoteSymbol ?: defaultPair.quoteType.substringAfterLast("::")
    val baseDecimals = selectedBook?.baseDecimals ?: defaultPair.baseDecimals
    val quoteDecimals = selectedBook?.quoteDecimals ?: defaultPair.quoteDecimals

    // bookId is driven by selected book
    val bookId = selectedBook?.bookId ?: ""

    // --- Order form state ---
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

    // Loads both depth sides. Deliberately leaves `orderResult` alone so a
    // caller's success message (e.g. "Bid submitted") is not overwritten.
    val refreshOrderBook = {
        scope.launch {
            bidLevels = viewModel.getOrderBookLevels(bookId, isBid = true, MAX_BOOK_LEVELS, baseType, quoteType)
            askLevels = viewModel.getOrderBookLevels(bookId, isBid = false, MAX_BOOK_LEVELS, baseType, quoteType)
        }
    }

    // Discover books on chain when "Discover" is pressed
    val discoverBooks = {
        discoveringBooks = true
        scope.launch {
            val books = viewModel.discoverDeepbookBooks()
            discoveredBooks = books
            // Auto-select first book if available
            selectedBook = books.firstOrNull()
            discoveringBooks = false
        }
    }

    // Auto-discover books on initial load
    LaunchedEffect(environment) {
        discoverBooks()
    }

    // Refresh depth when book selection changes
    LaunchedEffect(selectedBook) {
        if (selectedBook != null) {
            refreshOrderBook()
        }
    }

    // Keep the depth view in sync with the chain. A placed order is submitted
    // as pending and only becomes visible after it commits, so poll the two
    // counts (cheap view calls) and reload the levels only when the book
    // actually changed.
    LaunchedEffect(bookId, baseType, quoteType) {
        if (bookId.isBlank()) return@LaunchedEffect
        var lastBid: Int? = null
        var lastAsk: Int? = null
        while (true) {
            val bidCount = viewModel.getBidCount(bookId, baseType, quoteType)
            val askCount = viewModel.getAskCount(bookId, baseType, quoteType)
            if (bidCount != null && askCount != null &&
                (bidCount != lastBid || askCount != lastAsk)
            ) {
                lastBid = bidCount
                lastAsk = askCount
                bidLevels = viewModel.getOrderBookLevels(bookId, isBid = true, MAX_BOOK_LEVELS, baseType, quoteType)
                askLevels = viewModel.getOrderBookLevels(bookId, isBid = false, MAX_BOOK_LEVELS, baseType, quoteType)
            }
            delay(4_000)
        }
    }

    val orderType = DEEPBOOK_ORDER_TYPES[orderTypeIndex].value

    // Available balance for base token (decimals from selected pair)
    val baseBalance = remember(tokenBalances, baseType) {
        tokenBalances.find { typeTagsEqual(it.tokenType, baseType) }?.getEffectiveAmount() ?: 0L
    }

    // Quote balance — bids are paid in the quote token (USD), so a wallet
    // without it can never place a bid.
    val quoteBalance = remember(tokenBalances, quoteType) {
        tokenBalances.find { typeTagsEqual(it.tokenType, quoteType) }?.getEffectiveAmount() ?: 0L
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
            // Active Wallet Indicator
            activeWallet?.let { wallet ->
                Surface(
                    color = MaterialTheme.colorScheme.surfaceContainerHighest,
                    shape = RoundedCornerShape(12.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Row(
                        modifier = Modifier.padding(12.dp),
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Icon(
                            Icons.Default.AccountCircle,
                            contentDescription = null,
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
                        Column(
                            Modifier.weight(1f),
                            verticalArrangement = Arrangement.spacedBy(4.dp)
                        ) {
                            Text(
                                "Base: ${formatAmountExactOrUnknown(baseBalance, baseDecimals)} $baseSymbol",
                                style = MaterialTheme.typography.labelSmall,
                                color = if (baseBalance > 0) MaterialTheme.colorScheme.onSurfaceVariant else MaterialTheme.colorScheme.error
                            )
                            Text(
                                "Quote: ${formatAmountExactOrUnknown(quoteBalance, quoteDecimals)} $quoteSymbol",
                                style = MaterialTheme.typography.labelSmall,
                                color = if (quoteBalance > 0) MaterialTheme.colorScheme.onSurfaceVariant else MaterialTheme.colorScheme.error
                            )
                        }
                    }
                }
            }

            // Registry Status Banner
            Surface(
                color = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.4f),
                shape = RoundedCornerShape(16.dp),
                border = androidx.compose.foundation.BorderStroke(
                    1.dp,
                    MaterialTheme.colorScheme.primary.copy(alpha = 0.3f)
                ),
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    Modifier.padding(16.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Icon(
                            Icons.Default.CheckCircle,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(20.dp)
                        )
                        Text(
                            "Registry & Order Book Discovery",
                            style = MaterialTheme.typography.bodySmall.copy(fontWeight = FontWeight.SemiBold)
                        )
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

                    // Row 1: Book ID Dropdown + Side Selector (Buy/Sell)
                    Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        Column(Modifier.weight(1.3f), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.spacedBy(8.dp),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Text(
                                    "Order Book",
                                    style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.Bold)
                                )
                                OutlinedButton(
                                    onClick = { discoverBooks() },
                                    enabled = !discoveringBooks,
                                    shape = RoundedCornerShape(8.dp),
                                    contentPadding = PaddingValues(horizontal = 8.dp, vertical = 2.dp),
                                    modifier = Modifier.height(28.dp)
                                ) {
                                    if (discoveringBooks) {
                                        androidx.compose.material3.CircularProgressIndicator(modifier = Modifier.size(14.dp))
                                    } else {
                                        Icon(
                                            Icons.Default.Refresh,
                                            contentDescription = null,
                                            modifier = Modifier.size(14.dp)
                                        )
                                    }
                                    Spacer(Modifier.width(4.dp))
                                    Text("Discover", style = MaterialTheme.typography.labelSmall)
                                }
                            }
                            Spacer(Modifier.height(4.dp))
                            ExposedDropdownMenuBox(
                                expanded = bookDropdownExpanded,
                                onExpandedChange = { bookDropdownExpanded = it },
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                OutlinedTextField(
                                    value = selectedBook?.displayLabel ?: "Select a book…",
                                    onValueChange = {},
                                    readOnly = true,
                                    singleLine = true,
                                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = bookDropdownExpanded) },
                                    shape = RoundedCornerShape(12.dp),
                                    modifier = Modifier.menuAnchor(
                                        type = ExposedDropdownMenuAnchorType.PrimaryEditable,
                                        enabled = true
                                    ).fillMaxWidth()
                                )
                                ExposedDropdownMenu(
                                    expanded = bookDropdownExpanded,
                                    onDismissRequest = { bookDropdownExpanded = false }
                                ) {
                                    discoveredBooks.forEach { book ->
                                        DropdownMenuItem(
                                            text = {
                                                Text(
                                                    book.displayLabel,
                                                    style = MaterialTheme.typography.labelMedium,
                                                    maxLines = 1,
                                                    overflow = TextOverflow.Ellipsis
                                                )
                                            },
                                            onClick = {
                                                selectedBook = book
                                                bookDropdownExpanded = false
                                            }
                                        )
                                    }
                                }
                            }
                        }
                        Column(Modifier.weight(1f)) {
                            Text(
                                "Side",
                                style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.Bold)
                            )
                            Spacer(Modifier.height(4.dp))
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .height(56.dp)
                                    .background(
                                        MaterialTheme.colorScheme.surfaceContainerHighest,
                                        RoundedCornerShape(12.dp)
                                    )
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

                    // Selected Pair & Book ID Box
                    Surface(
                        color = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.2f),
                        shape = RoundedCornerShape(12.dp),
                        border = androidx.compose.foundation.BorderStroke(
                            1.dp,
                            MaterialTheme.colorScheme.primary.copy(alpha = 0.2f)
                        ),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Column(Modifier.padding(12.dp)) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Column {
                                    selectedBook?.let { book ->
                                        Text(
                                            "Book: ${book.baseSymbol} / ${book.quoteSymbol}",
                                            style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.Bold),
                                            color = MaterialTheme.colorScheme.primary
                                        )
                                        Spacer(Modifier.height(2.dp))
                                        Text(
                                            "ID: ${book.bookId}",
                                            style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            maxLines = 1,
                                            overflow = TextOverflow.Ellipsis
                                        )
                                    } ?: run {
                                        Text(
                                            "No book selected",
                                            style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.Bold),
                                            color = MaterialTheme.colorScheme.onSurfaceVariant
                                        )
                                    }
                                }
                            }
                        }
                    }

                    // Wallet Capability Check
                    activeWallet?.let { wallet ->
                        val canBid = quoteBalance > 0
                        val canAsk = baseBalance > 0
                        val hasGas = tokenBalances.any {
                            typeTagsEqual(
                                it.tokenType,
                                environment.deepbook.nativeTokenType
                            ) && it.getEffectiveAmount() > 0
                        }
                        if (!canBid || !canAsk || !hasGas) {
                            Surface(
                                color = MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.9f),
                                shape = RoundedCornerShape(10.dp),
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .padding(top = 8.dp)
                            ) {
                                Column(Modifier.padding(12.dp)) {
                                    Row(
                                        modifier = Modifier.fillMaxWidth(),
                                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                                    ) {
                                        Icon(
                                            Icons.Default.WarningAmber,
                                            contentDescription = null,
                                            tint = MaterialTheme.colorScheme.onErrorContainer,
                                            modifier = Modifier.size(16.dp)
                                        )
                                        Text(
                                            "Wallet Limitations",
                                            style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.Bold),
                                            color = MaterialTheme.colorScheme.onErrorContainer
                                        )
                                    }
                                    Spacer(Modifier.height(4.dp))
                                    if (!canBid) {
                                        Text(
                                            "✕ Cannot Bid — need $quoteSymbol (have ${
                                                formatAmountExactOrUnknown(
                                                    quoteBalance,
                                                    quoteDecimals
                                                )
                                            })",
                                            style = MaterialTheme.typography.labelSmall,
                                            color = MaterialTheme.colorScheme.onErrorContainer
                                        )
                                    }
                                    if (!canAsk) {
                                        Text(
                                            "✕ Cannot Ask — need $baseSymbol (have ${
                                                formatAmountExactOrUnknown(
                                                    baseBalance,
                                                    baseDecimals
                                                )
                                            })",
                                            style = MaterialTheme.typography.labelSmall,
                                            color = MaterialTheme.colorScheme.onErrorContainer
                                        )
                                    }
                                    if (!hasGas) {
                                        Text(
                                            "✕ Cannot Pay Gas — need KANARI for gas",
                                            style = MaterialTheme.typography.labelSmall,
                                            color = MaterialTheme.colorScheme.onErrorContainer
                                        )
                                    }
                                    Text(
                                        "Switch wallet in Dashboard or claim devnet tokens",
                                        style = MaterialTheme.typography.labelSmall,
                                        color = MaterialTheme.colorScheme.onErrorContainer.copy(alpha = 0.8f)
                                    )
                                }
                            }
                        }
                    }

                    // Order Type
                    Column {
                        Text(
                            "Order Type",
                            style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.Bold)
                        )
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
                                modifier = Modifier.menuAnchor(
                                    type = ExposedDropdownMenuAnchorType.PrimaryEditable,
                                    enabled = true
                                ).fillMaxWidth()
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
                            Text(
                                "Price ($quoteSymbol per $baseSymbol)",
                                style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.Bold)
                            )
                            Spacer(Modifier.height(4.dp))
                            OutlinedTextField(
                                value = price,
                                onValueChange = { price = it },
                                placeholder = { Text("e.g. 1.234") },
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                                shape = RoundedCornerShape(12.dp),
                                modifier = Modifier.fillMaxWidth()
                            )
                            Text(
                                "Up to $PRICE_DECIMALS decimal places",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                        Column(Modifier.weight(1f)) {
                            Text(
                                "Quantity ($baseSymbol)",
                                style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.Bold)
                            )
                            Spacer(Modifier.height(4.dp))
                            OutlinedTextField(
                                value = quantity,
                                onValueChange = { quantity = it },
                                placeholder = { Text("e.g. 100") },
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                                shape = RoundedCornerShape(12.dp),
                                modifier = Modifier.fillMaxWidth()
                            )
                            Text(
                                "Up to $baseDecimals decimal places",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
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
                            Text(
                                "Order Summary",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                            Text(
                                text = if (price.isNotBlank() && quantity.isNotBlank()) {
                                    "${if (isBidSide) "Buy" else "Sell"} $quantity $baseSymbol at $price $quoteSymbol/$baseSymbol"
                                } else "Enter price and quantity",
                                style = MaterialTheme.typography.bodyMedium.copy(fontWeight = FontWeight.Medium),
                                color = MaterialTheme.colorScheme.onSurface
                            )
                            Spacer(Modifier.height(2.dp))
                            val priceRaw = parseAmountToMist(price, PRICE_DECIMALS)
                            val quantityRaw = parseAmountToMist(quantity, baseDecimals)
                            val estimatedQuote =
                                if (priceRaw != null && quantityRaw != null && priceRaw > 0uL && quantityRaw > 0uL) {
                                    formatRawAmount(
                                        quoteAmountRaw(priceRaw, quantityRaw, baseDecimals, quoteDecimals),
                                        quoteDecimals
                                    )
                                } else null
                            Text(
                                text = estimatedQuote?.let { "~$it $quoteSymbol" } ?: "-",
                                style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
                                color = MaterialTheme.colorScheme.onSurface
                            )
                            Spacer(Modifier.height(2.dp))
                            Text(
                                text = "Pay with: ${
                                    formatAmountExactOrUnknown(
                                        quoteBalance,
                                        quoteDecimals
                                    )
                                } $quoteSymbol",
                                style = MaterialTheme.typography.labelSmall,
                                color = if (quoteBalance > 0) MaterialTheme.colorScheme.onSurfaceVariant
                                else MaterialTheme.colorScheme.error
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
                                val p = parseAmountToMist(price, PRICE_DECIMALS) ?: run {
                                    orderResult = "Invalid price (up to $PRICE_DECIMALS decimal places)"; return@launch
                                }
                                if (p == 0uL) {
                                    orderResult = "Price rounds to zero"
                                    return@launch
                                }
                                val q = parseAmountToMist(quantity, baseDecimals) ?: run {
                                    orderResult = "Invalid quantity"; return@launch
                                }
                                // Fail fast on a balance the VM would reject anyway:
                                // a bid is paid in the quote token, an ask must be
                                // backed by base tokens.
                                if (isBidSide) {
                                    val needed = quoteAmountRaw(p, q, baseDecimals, quoteDecimals)
                                    if (needed > BigInteger.valueOf(quoteBalance.coerceAtLeast(0))) {
                                        orderResult = "Not enough $quoteSymbol for this bid — " +
                                                "need ~${formatRawAmount(needed, quoteDecimals)} $quoteSymbol, " +
                                                "have ${formatAmountExactOrUnknown(quoteBalance, quoteDecimals)}. " +
                                                "Claim devnet USD from the faucet first."
                                        return@launch
                                    }
                                } else if (q > baseBalance.toULong()) {
                                    orderResult = "Not enough $baseSymbol — have " +
                                            "${formatAmountExactOrUnknown(baseBalance, baseDecimals)} $baseSymbol"
                                    return@launch
                                }
                                // Pass the order TYPE VALUE (0..3), not the list index.
                                val ok = if (isBidSide) {
                                    viewModel.placeBid(book, p, q, "", orderType, baseType, quoteType)
                                } else {
                                    viewModel.placeAsk(book, p, q, "", orderType, baseType, quoteType)
                                }
                                orderResult = if (ok) {
                                    if (isBidSide) "Bid submitted — waiting for commit"
                                    else "Ask submitted — waiting for commit"
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
                val isSuccess = result.contains("successfully", ignoreCase = true) || result.contains(
                    "refreshed",
                    ignoreCase = true
                )
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
                    Text(
                        "$baseSymbol / $quoteSymbol",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
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
            Text(
                "$title - ${levels.size}",
                style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.Bold),
                color = accent
            )
        }
        if (levels.isEmpty()) {
            Surface(
                color = MaterialTheme.colorScheme.surfaceContainerHighest,
                shape = RoundedCornerShape(12.dp),
                modifier = Modifier.fillMaxWidth().height(76.dp)
            ) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text(
                        emptyText,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        } else {
            // Header row
            Row(
                Modifier.fillMaxWidth().padding(horizontal = 8.dp),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text(
                    "Price",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Text(
                    "Qty",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            levels.forEach { level ->
                val isMine = myAddress != null && level.maker.removePrefix("0x").removePrefix("0X")
                    .equals(myAddress, ignoreCase = true)
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
                                formatRawAmount(BigInteger.valueOf(level.price.toLong()), PRICE_DECIMALS),
                                style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.Bold),
                                color = accent
                            )
                            Text(
                                "#${level.orderId}" + if (isMine) " • yours" else "",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(4.dp)
                        ) {
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
