// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.jamesatomc.kanariapp.ui.components.ScaffoldWithBackBar
import com.jamesatomc.kanariapp.wallet.WalletViewModel
import com.jamesatomc.kanariapp.wallet.runTemplateAction
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TemplateScreen(
    viewModel: WalletViewModel,
    onBack: () -> Unit
) {
    val scope = rememberCoroutineScope()
    var status by remember { mutableStateOf<String?>(null) }
    var loading by remember { mutableStateOf(false) }

    ScaffoldWithBackBar(title = "Template", onBack = onBack) { padding ->
        Column(
            modifier = Modifier
                .padding(padding)
                .fillMaxSize()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Button(
                onClick = {
                    loading = true
                    scope.launch {
                        try {
                            val ok = viewModel.runTemplateAction()
                            status = if (ok) "Success" else (viewModel.error.value ?: "Failed")
                        } catch (e: Exception) {
                            status = e.message
                        } finally {
                            loading = false
                        }
                    }
                },
                enabled = !loading
            ) {
                if (loading) CircularProgressIndicator(modifier = Modifier.size(18.dp))
                else Text("Run Template Action")
            }
            status?.let { Text(it) }
        }
    }
}
