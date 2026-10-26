// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

package com.jamesatomc.kanariapp.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay

/**
 * Countdown confirmation gate for irreversible sends (tokens and NFTs).
 *
 * Shows the pending transfer for [seconds] seconds with a Cancel button and a
 * "Send now" button that skips the remaining wait. Nothing is submitted while
 * the countdown runs; cancelling (button or back press) discards the action.
 * When the countdown reaches zero — or the user taps "Send now" — [onConfirmed]
 * fires exactly once and the caller performs the real send.
 */
@Composable
fun SendCountdownDialog(
    title: String,
    detail: String,
    seconds: Int = 10,
    onCancel: () -> Unit,
    onConfirmed: () -> Unit,
) {
    var remaining by remember { mutableIntStateOf(seconds) }
    // Exactly-once guard: the final tick and a "Send now" tap can race when
    // the user clicks as the countdown reaches zero.
    var fired by remember { mutableStateOf(false) }
    fun fire() {
        if (!fired) {
            fired = true
            onConfirmed()
        }
    }
    LaunchedEffect(Unit) {
        while (remaining > 0) {
            delay(1_000L)
            remaining -= 1
        }
        fire()
    }
    AlertDialog(
        onDismissRequest = onCancel,
        title = { Text(title, fontWeight = FontWeight.Bold) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text(detail, style = MaterialTheme.typography.bodyMedium)
                Text(
                    "Sending in $remaining…",
                    style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.primary,
                )
                LinearProgressIndicator(
                    progress = { (seconds - remaining).toFloat() / seconds.toFloat() },
                    modifier = Modifier.fillMaxWidth(),
                )
                Text(
                    "Press Cancel to abort or Send now to skip the wait.",
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        },
        confirmButton = {
            Button(onClick = { fire() }) {
                Text("Send now", fontWeight = FontWeight.SemiBold)
            }
        },
        dismissButton = {
            TextButton(onClick = onCancel) {
                Text("Cancel", fontWeight = FontWeight.SemiBold)
            }
        },
    )
}
