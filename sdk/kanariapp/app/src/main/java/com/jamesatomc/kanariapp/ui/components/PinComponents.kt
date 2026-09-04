package com.jamesatomc.kanariapp.ui.components

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.ExperimentalAnimationApi
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Backspace
import androidx.compose.material.icons.filled.Fingerprint
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp

@Composable
fun PinCircles(
    length: Int,
    totalLength: Int = 6,
    modifier: Modifier = Modifier
) {
    val primary = MaterialTheme.colorScheme.primary
    val outline = MaterialTheme.colorScheme.outlineVariant
    Row(modifier = modifier, horizontalArrangement = Arrangement.spacedBy(12.dp, Alignment.CenterHorizontally), verticalAlignment = Alignment.CenterVertically) {
        repeat(totalLength) { index ->
            val filled = index < length
            Surface(
                shape = CircleShape,
                color = if (filled) primary else androidx.compose.ui.graphics.Color.Transparent,
                border = BorderStroke(2.dp, if (filled) primary else outline),
                modifier = Modifier.size(18.dp)
            ) {}
        }
    }
}

@Composable
fun PinNumberPad(
    onNumberPressed: (String) -> Unit,
    onBackspacePressed: () -> Unit,
    biometricEnabled: Boolean = false,
    onBiometricPressed: (() -> Unit)? = null,
    modifier: Modifier = Modifier
) {
    val numbers = listOf(
        listOf("1", "2", "3"),
        listOf("4", "5", "6"),
        listOf("7", "8", "9")
    )
    Column(modifier = modifier.padding(horizontal = 32.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        numbers.forEach { row ->
            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceEvenly) {
                row.forEach { num ->
                    NumberButton(number = num, onPressed = { onNumberPressed(num) })
                }
            }
        }
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceEvenly, verticalAlignment = Alignment.CenterVertically) {
            if (biometricEnabled) {
                IconButton(onClick = { onBiometricPressed?.invoke() }, modifier = Modifier.size(64.dp)) {
                    Icon(Icons.Default.Fingerprint, contentDescription = "Biometric", modifier = Modifier.size(28.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            } else Spacer(Modifier.size(64.dp))
            NumberButton(number = "0", onPressed = { onNumberPressed("0") })
            IconButton(onClick = onBackspacePressed, modifier = Modifier.size(64.dp)) {
                Icon(Icons.Default.Backspace, contentDescription = "Backspace", modifier = Modifier.size(28.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
    }
}

@Composable
private fun NumberButton(number: String, onPressed: () -> Unit) {
    Surface(
        shape = CircleShape,
        color = MaterialTheme.colorScheme.surfaceContainerHigh,
        tonalElevation = 0.dp,
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
        modifier = Modifier.size(64.dp),
        onClick = onPressed
    ) {
        Box(contentAlignment = Alignment.Center, modifier = Modifier.fillMaxSize()) {
            Text(number, style = MaterialTheme.typography.titleLarge.copy(fontWeight = FontWeight.SemiBold), color = MaterialTheme.colorScheme.onSurface)
        }
    }
}

@OptIn(ExperimentalAnimationApi::class)
@Composable
fun PinEntryHeader(
    title: String,
    subtitle: String,
    enteredLength: Int,
    totalLength: Int = 6,
    errorText: String? = null,
    isChecking: Boolean = false
) {
    Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.fillMaxWidth()) {
        Text(title, style = MaterialTheme.typography.headlineSmall.copy(fontWeight = FontWeight.Bold), color = MaterialTheme.colorScheme.onSurface, textAlign = TextAlign.Center)
        Spacer(Modifier.height(12.dp))
        Text(subtitle, style = MaterialTheme.typography.bodyLarge.copy(color = MaterialTheme.colorScheme.onSurfaceVariant), textAlign = TextAlign.Center, modifier = Modifier.padding(horizontal = 32.dp))
        Spacer(Modifier.height(32.dp))
        PinCircles(length = enteredLength, totalLength = totalLength)
        AnimatedContent(targetState = errorText, label = "error") { err ->
            if (err == null) Spacer(Modifier.height(28.dp))
            else Text(err, style = MaterialTheme.typography.bodyMedium.copy(color = MaterialTheme.colorScheme.error, fontWeight = FontWeight.SemiBold), modifier = Modifier.padding(top = 10.dp))
        }
        if (isChecking) {
            CircularProgressIndicator(modifier = Modifier.size(22.dp).padding(top = 8.dp), strokeWidth = 2.dp, color = MaterialTheme.colorScheme.primary)
        } else Spacer(Modifier.height(30.dp))
    }
}
