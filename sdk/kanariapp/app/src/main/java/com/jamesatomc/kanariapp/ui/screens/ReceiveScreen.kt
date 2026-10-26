// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.ExpandMore
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.jamesatomc.kanariapp.ui.components.LoadingButton
import com.jamesatomc.kanariapp.ui.components.QrCodeImage
import com.jamesatomc.kanariapp.ui.components.ScaffoldWithBackBar
import com.jamesatomc.kanariapp.ui.components.buildPaymentUri
import com.jamesatomc.kanariapp.ui.components.copyToClipboard
import com.jamesatomc.kanariapp.wallet.WalletViewModel

/**
 * Receive screen: shows the wallet QR. Setting an optional amount (and token)
 * turns the QR into a payment request (`kanari:pay?address=…&amount=…&token=…`)
 * so the sender's scan fills address *and* amount in one tap. Leaving the
 * amount empty keeps a plain-address QR, which older app versions scan fine.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReceiveScreen(viewModel: WalletViewModel, onBack: () -> Unit) {
    val activeWallet by viewModel.activeWallet.collectAsStateWithLifecycle()
    val tokenBalances by viewModel.tokenBalances.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val address = activeWallet?.address ?: ""

    var amount by remember { mutableStateOf("") }
    var expanded by remember { mutableStateOf(false) }
    var selectedToken by remember {
        mutableStateOf(tokenBalances.find { it.symbol.equals("KANARI", true) } ?: tokenBalances.firstOrNull())
    }

    val qrContent = buildPaymentUri(
        address = address,
        amount = amount.takeIf { it.isNotBlank() },
        token = selectedToken?.symbol,
    )
    val requesting = amount.isNotBlank() && amount.trim() != "0"

    ScaffoldWithBackBar(title = "Receive", onBack = onBack) { padding ->
        Column(
            Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text(
                if (requesting) "Requesting" else "Your Wallet Address",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
            )
            if (requesting) {
                Text(
                    "$amount ${selectedToken?.symbol ?: ""}",
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.primary,
                )
            }

            QrCodeImage(content = qrContent, size = 220.dp)

            Surface(
                Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(12.dp),
                color = MaterialTheme.colorScheme.surfaceContainerLow,
            ) {
                Text(
                    address.ifEmpty { "No wallet selected" },
                    style = MaterialTheme.typography.bodySmall,
                    textAlign = TextAlign.Center,
                    maxLines = 3,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.padding(16.dp),
                )
            }

            // Full-width token selector — side-by-side squeezed the symbol
            // into "KAN…"; the dropdown needs the whole row to show "KANARI".
            ExposedDropdownMenuBox(
                expanded = expanded,
                onExpandedChange = { expanded = !expanded },
                modifier = Modifier.fillMaxWidth(),
            ) {
                OutlinedTextField(
                    value = selectedToken?.symbol ?: "Token",
                    onValueChange = {},
                    readOnly = true,
                    label = { Text("Token") },
                    trailingIcon = { Icon(Icons.Default.ExpandMore, contentDescription = null) },
                    shape = RoundedCornerShape(12.dp),
                    singleLine = true,
                    modifier = Modifier
                        .menuAnchor(
                            type = ExposedDropdownMenuAnchorType.PrimaryEditable,
                            enabled = true,
                        )
                        .fillMaxWidth(),
                )
                ExposedDropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
                    tokenBalances.forEach { token ->
                        DropdownMenuItem(
                            text = { Text(token.symbol) },
                            onClick = {
                                selectedToken = token
                                expanded = false
                            },
                        )
                    }
                }
            }
            OutlinedTextField(
                value = amount,
                onValueChange = { input ->
                    amount = input.trimStart()
                },
                label = { Text("Amount (optional)") },
                singleLine = true,
                shape = RoundedCornerShape(12.dp),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                supportingText = {
                    Text(
                        if (requesting) "Sender scans — address & amount fill in"
                        else "Leave empty for a plain address QR",
                        style = MaterialTheme.typography.bodySmall,
                    )
                },
                modifier = Modifier.fillMaxWidth(),
            )

            LoadingButton(
                onClick = {
                    if (address.isNotEmpty()) {
                        copyToClipboard(
                            context,
                            qrContent,
                            toast = if (requesting) "Payment link copied" else "Address copied"
                        )
                    }
                },
                text = if (requesting) "Copy Payment Details" else "Copy Address",
                icon = Icons.Default.ContentCopy,
                modifier = Modifier.fillMaxWidth(),
            )
        }
    }
}
