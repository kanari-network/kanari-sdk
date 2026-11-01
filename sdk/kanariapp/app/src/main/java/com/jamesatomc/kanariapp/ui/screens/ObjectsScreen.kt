// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Public
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.jamesatomc.kanariapp.network.models.ObjectInfo
import com.jamesatomc.kanariapp.network.models.ObjectOwnerKind
import com.jamesatomc.kanariapp.ui.components.ScaffoldWithBackBar
import com.jamesatomc.kanariapp.ui.components.copyToClipboard
import com.jamesatomc.kanariapp.wallet.WalletViewModel

/** Module path (`package::module`) that an object type belongs to. */
private fun objectGroup(type: String): String {
    val outer = type.substringBefore('<')
    val parts = outer.split("::")
    return if (parts.size >= 2) "${parts[0]}::${parts[1]}" else outer
}

/** Last readable segment of a type, e.g. `Coin<0x2::kanari::KANARI>`. */
private fun objectLabel(type: String): String {
    val outer = type.substringBefore('<')
    val rest = type.removePrefix(outer) // "<...>" generics or empty
    return outer.substringAfterLast("::") + rest
}

private fun ownerLabel(obj: ObjectInfo): String = when (obj.ownerKind) {
    is ObjectOwnerKind.AddressOwner -> "Owned"
    ObjectOwnerKind.Shared -> "Shared"
    ObjectOwnerKind.Immutable -> "Immutable"
    null -> "Owned"
}

/**
 * All on-chain objects owned by the active wallet, grouped by module.
 * Tap a row (or its copy button) to copy the object id.
 */
@Composable
fun ObjectsScreen(viewModel: WalletViewModel, onBack: () -> Unit) {
    val accountInfo by viewModel.accountInfo.collectAsStateWithLifecycle()
    val isLoading by viewModel.isLoading.collectAsStateWithLifecycle()
    val context = LocalContext.current
    var query by remember { mutableStateOf("") }

    val objects = accountInfo?.ownedObjects ?: emptyList()
    val needle = query.trim().removePrefix("0x")
    val filtered = if (needle.isEmpty()) objects else objects.filter {
        it.getEffectiveType().contains(needle, ignoreCase = true) ||
            it.id.contains(needle, ignoreCase = true)
    }
    val groups = filtered
        .groupBy { objectGroup(it.getEffectiveType()) }
        .toList()
        .sortedBy { it.first }

    ScaffoldWithBackBar(title = "Objects", onBack = onBack) { padding ->
        Column(
            Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 16.dp),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                OutlinedTextField(
                    value = query,
                    onValueChange = { query = it },
                    placeholder = { Text("Search type or id") },
                    leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
                    singleLine = true,
                    shape = RoundedCornerShape(14.dp),
                    modifier = Modifier.weight(1f),
                )
                IconButton(onClick = { viewModel.refreshBalance() }) {
                    Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                }
            }
            Spacer(Modifier.height(8.dp))
            Text(
                "${filtered.size} of ${objects.size} objects",
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.height(8.dp))

            when {
                isLoading && objects.isEmpty() -> Box(
                    Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center,
                ) { CircularProgressIndicator() }

                groups.isEmpty() -> Box(
                    Modifier.fillMaxSize().padding(32.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        if (query.isBlank()) "No objects owned by this wallet"
                        else "No objects match \"$query\"",
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }

                else -> LazyColumn(
                    contentPadding = PaddingValues(bottom = 24.dp),
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    groups.forEach { (group, items) ->
                        item(key = "header-$group") {
                            Text(
                                group,
                                style = MaterialTheme.typography.labelMedium.copy(
                                    fontWeight = FontWeight.Bold,
                                ),
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.padding(top = 12.dp, bottom = 4.dp),
                            )
                        }
                        items(items, key = { it.id }) { obj ->
                            ObjectRow(
                                obj = obj,
                                onCopy = {
                                    copyToClipboard(context, obj.id, toast = "Object id copied")
                                },
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun ObjectRow(obj: ObjectInfo, onCopy: () -> Unit) {
    Surface(
        color = MaterialTheme.colorScheme.surfaceContainer,
        shape = RoundedCornerShape(14.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        ListItem(
            headlineContent = {
                Text(
                    objectLabel(obj.getEffectiveType()),
                    style = MaterialTheme.typography.bodyMedium.copy(fontWeight = FontWeight.SemiBold),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            },
            supportingContent = {
                Column {
                    Text(
                        "${obj.id.take(14)}…${obj.id.takeLast(8)}",
                        style = MaterialTheme.typography.bodySmall,
                    )
                    Text(
                        "v${obj.version} · ${ownerLabel(obj)}",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            },
            leadingContent = {
                Icon(Icons.Default.Public, contentDescription = null)
            },
            trailingContent = {
                IconButton(onClick = onCopy) {
                    Icon(Icons.Default.ContentCopy, contentDescription = "Copy object id")
                }
            },
            modifier = Modifier.padding(vertical = 2.dp),
        )
    }
}
