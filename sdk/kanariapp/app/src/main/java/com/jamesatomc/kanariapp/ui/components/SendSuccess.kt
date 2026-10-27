// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

package com.jamesatomc.kanariapp.ui.components

import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Popup
import androidx.compose.ui.window.PopupProperties
import com.jamesatomc.kanariapp.ui.Sfx
import com.jamesatomc.kanariapp.ui.theme.KanariColors

/**
 * Full-screen "Send Successful" confirmation in the spirit of Slush: a
 * spring-in lime checkmark, a one-line English summary, and a single Done
 * action. Plays the success chime and a haptic tick when it appears.
 *
 * Render this as the last layer of a screen while the owning state holds a
 * non-null summary; [onDone] fires from the button, back press, or tap-outside.
 */
@Composable
fun SendSuccessOverlay(
    summary: String,
    onDone: () -> Unit,
) {
    val context = LocalContext.current
    val haptic = LocalHapticFeedback.current
    var shown by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) {
        Sfx.success(context)
        haptic.performHapticFeedback(HapticFeedbackType.LongPress)
        shown = true
    }
    // Back is handled by the focusable Popup (dismissOnBackPress -> onDone).
    val checkScale by animateFloatAsState(
        targetValue = if (shown) 1f else 0.3f,
        animationSpec = spring(dampingRatio = 0.35f, stiffness = 380f),
        label = "checkScale",
    )
    val contentAlpha by animateFloatAsState(
        targetValue = if (shown) 1f else 0f,
        animationSpec = tween(durationMillis = 350, delayMillis = 120, easing = FastOutSlowInEasing),
        label = "contentAlpha",
    )
    val dismissInteraction = remember { MutableInteractionSource() }

    // The send sheet lives inside a ModalBottomSheet, so a plain fillMaxSize
    // box would only cover the sheet. A width-free Popup covers the whole
    // window — the full-screen takeover Slush does on success.
    Popup(
        alignment = Alignment.TopStart,
        offset = IntOffset.Zero,
        onDismissRequest = onDone,
        properties = PopupProperties(
            focusable = true,
            dismissOnBackPress = true,
            dismissOnClickOutside = false,
            usePlatformDefaultWidth = false,
        ),
    ) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(Color(0xF2000000))
                .clickable(interactionSource = dismissInteraction, indication = null) { onDone() },
            contentAlignment = Alignment.Center,
        ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Icon(
                imageVector = Icons.Default.CheckCircle,
                contentDescription = null,
                tint = KanariColors.Lime,
                modifier = Modifier
                    .size(120.dp)
                    .graphicsLayer {
                        scaleX = checkScale
                        scaleY = checkScale
                        alpha = contentAlpha.coerceAtLeast(0.35f)
                    },
            )
            Text(
                "Send Successful",
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.Bold,
                color = Color.White,
                textAlign = TextAlign.Center,
                modifier = Modifier.graphicsLayer { alpha = contentAlpha },
            )
            Text(
                summary,
                style = MaterialTheme.typography.bodyMedium,
                color = Color.White.copy(alpha = 0.7f),
                textAlign = TextAlign.Center,
                modifier = Modifier
                    .width(280.dp)
                    .graphicsLayer { alpha = contentAlpha },
            )
            Box(Modifier.size(24.dp))
            LoadingButton(
                onClick = onDone,
                text = "Done",
                modifier = Modifier
                    .width(220.dp)
                    .padding(top = 8.dp)
                    .graphicsLayer { alpha = contentAlpha },
            )
            }
        }
    }
}
