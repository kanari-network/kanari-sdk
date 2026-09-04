package com.jamesatomc.kanariapp.ui.components

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.ExperimentalAnimationApi
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Backspace
import androidx.compose.material.icons.filled.Fingerprint
import androidx.compose.material3.*
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.window.Dialog
import android.widget.Toast
import androidx.compose.ui.window.DialogProperties
import androidx.compose.runtime.*
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
    Row(
        modifier = modifier,
        horizontalArrangement = Arrangement.spacedBy(12.dp, Alignment.CenterHorizontally),
        verticalAlignment = Alignment.CenterVertically
    ) {
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
    Column(modifier = modifier.padding(horizontal = 12.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        numbers.forEach { row ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceEvenly) {
                row.forEach { num -> NumberButton(number = num, onPressed = { onNumberPressed(num) }) }
            }
        }
        Row(
            Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceEvenly,
            verticalAlignment = Alignment.CenterVertically
        ) {
            if (biometricEnabled) IconButton(
                onClick = { onBiometricPressed?.invoke() },
                modifier = Modifier.size(56.dp)
            ) {
                Icon(
                    Icons.Default.Fingerprint,
                    contentDescription = "Biometric",
                    modifier = Modifier.size(26.dp),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            else Spacer(Modifier.size(56.dp))
            NumberButton(number = "0", onPressed = { onNumberPressed("0") })
            IconButton(onClick = onBackspacePressed, modifier = Modifier.size(56.dp)) {
                Icon(
                    Icons.Default.Backspace,
                    contentDescription = "Backspace",
                    modifier = Modifier.size(26.dp),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}

@Composable
private fun NumberButton(number: String, onPressed: () -> Unit) {
    Surface(
        shape = CircleShape,
        color = MaterialTheme.colorScheme.surfaceContainerHigh,
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
        modifier = Modifier.size(56.dp),
        onClick = onPressed
    ) {
        Box(contentAlignment = Alignment.Center, modifier = Modifier.fillMaxSize()) {
            Text(
                number,
                style = MaterialTheme.typography.titleLarge.copy(fontWeight = FontWeight.SemiBold),
                color = MaterialTheme.colorScheme.onSurface
            )
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
        Text(
            title,
            style = MaterialTheme.typography.headlineSmall.copy(fontWeight = FontWeight.Bold),
            color = MaterialTheme.colorScheme.onSurface,
            textAlign = TextAlign.Center
        )
        Spacer(Modifier.height(12.dp))
        Text(
            subtitle,
            style = MaterialTheme.typography.bodyLarge.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(horizontal = 32.dp)
        )
        Spacer(Modifier.height(32.dp))
        PinCircles(length = enteredLength, totalLength = totalLength)
        AnimatedContent(targetState = errorText, label = "error") { err ->
            if (err == null) Spacer(Modifier.height(28.dp))
            else Text(
                err,
                style = MaterialTheme.typography.bodyMedium.copy(
                    color = MaterialTheme.colorScheme.error,
                    fontWeight = FontWeight.SemiBold
                ),
                modifier = Modifier.padding(top = 10.dp)
            )
        }
        if (isChecking) {
            CircularProgressIndicator(
                modifier = Modifier.size(22.dp).padding(top = 8.dp),
                strokeWidth = 2.dp,
                color = MaterialTheme.colorScheme.primary
            )
        } else Spacer(Modifier.height(30.dp))
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FullScreenPinDialog(
    title: String,
    subtitle: String,
    onDismiss: () -> Unit,
    onPinComplete: (String) -> Boolean
) {
    var entered by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var isChecking by remember { mutableStateOf(false) }
    fun onNumber(num: String) {
        if (isChecking || entered.length >= 6) return
        val next = entered + num
        entered = next
        error = null
        if (next.length == 6) {
            isChecking = true
            val ok = onPinComplete(next)
            if (ok) onDismiss() else {
                error = "Invalid PIN"; entered = ""; isChecking = false
            }
        }
    }

    fun onBackspace() {
        if (entered.isNotEmpty() && !isChecking) {
            entered = entered.dropLast(1); error = null
        }
    }
    Dialog(onDismissRequest = onDismiss, properties = DialogProperties(usePlatformDefaultWidth = false)) {
        Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
            Scaffold(topBar = {
                TopAppBar(
                    title = { Text(title) },
                    navigationIcon = {
                        IconButton(onClick = onDismiss) {
                            Icon(
                                Icons.AutoMirrored.Filled.ArrowBack,
                                contentDescription = "Back"
                            )
                        }
                    },
                    colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background)
                )
            }) { padding ->
                Column(
                    Modifier.fillMaxSize().padding(padding).padding(horizontal = 24.dp, vertical = 16.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.SpaceBetween
                ) {
                    PinEntryHeader(
                        title = title,
                        subtitle = subtitle,
                        enteredLength = entered.length,
                        errorText = error,
                        isChecking = isChecking
                    )
                    PinNumberPad(
                        onNumberPressed = ::onNumber,
                        onBackspacePressed = ::onBackspace,
                        biometricEnabled = false,
                        modifier = Modifier.fillMaxWidth()
                    )
                }
            }
        }
    }
}
@Composable
fun PinVerificationContent(
    title: String,
    subtitle: String,
    onVerify: (String) -> Boolean,
    onSuccess: () -> Unit,
    biometricEnabled: Boolean = false,
    onBiometric: (() -> Unit)? = null,
    modifier: Modifier = Modifier
) {
    var entered by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var isChecking by remember { mutableStateOf(false) }
    fun onNumber(num: String) {
        if (isChecking || entered.length >= 6) return
        val next = entered + num
        entered = next
        error = null
        if (next.length == 6) {
            isChecking = true
            val ok = onVerify(next)
            if (ok) onSuccess() else { error = "Invalid PIN"; entered = ""; isChecking = false }
        }
    }
    fun onBackspace() { if (entered.isNotEmpty() && !isChecking) { entered = entered.dropLast(1); error = null } }
    Column(modifier.fillMaxSize().padding(horizontal = 24.dp, vertical = 16.dp), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.SpaceBetween) {
        PinEntryHeader(title = title, subtitle = subtitle, enteredLength = entered.length, errorText = error, isChecking = isChecking)
        PinNumberPad(onNumberPressed = ::onNumber, onBackspacePressed = ::onBackspace, biometricEnabled = biometricEnabled, onBiometricPressed = onBiometric, modifier = Modifier.fillMaxWidth())
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ChangePinFullScreenContent(
    onDismiss: () -> Unit,
    onConfirm: (String, String) -> Boolean
) {
    var step by remember { mutableIntStateOf(0) }
    var currentPin by remember { mutableStateOf("") }
    var newPin by remember { mutableStateOf("") }
    var entered by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var isChecking by remember { mutableStateOf(false) }
    val context = LocalContext.current
    val title = when (step) { 0 -> "Current PIN"; 1 -> "New PIN"; else -> "Confirm New PIN" }
    val subtitle = when (step) { 0 -> "Enter current 6-digit PIN"; 1 -> "Enter new 6-digit PIN"; else -> "Re-enter new PIN to confirm" }
    fun onNumber(num: String) {
        if (isChecking || entered.length >= 6) return
        val next = entered + num
        entered = next
        error = null
        if (next.length == 6) {
            when (step) {
                0 -> { currentPin = next; entered = ""; step = 1 }
                1 -> { newPin = next; entered = ""; step = 2 }
                2 -> {
                    if (next != newPin) { error = "PINs do not match"; entered = "" }
                    else if (currentPin == newPin) { error = "New PIN must be different"; entered = "" }
                    else {
                        isChecking = true
                        val ok = onConfirm(currentPin, newPin)
                        if (ok) { Toast.makeText(context, "PIN changed successfully", Toast.LENGTH_SHORT).show(); onDismiss() }
                        else { error = "Incorrect current PIN"; entered = ""; isChecking = false }
                    }
                }
            }
        }
    }
    fun onBackspace() { if (entered.isNotEmpty() && !isChecking) { entered = entered.dropLast(1); error = null } }
    Scaffold(topBar = { TopAppBar(title = { Text("Change PIN") }, navigationIcon = { IconButton(onClick = { if (step > 0) { step--; entered = ""; error = null } else onDismiss() }) { Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back") } }) }) { padding ->
        Column(Modifier.fillMaxSize().padding(padding).padding(horizontal = 24.dp, vertical = 16.dp), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.SpaceBetween) {
            PinEntryHeader(title = title, subtitle = subtitle, enteredLength = entered.length, errorText = error, isChecking = isChecking)
            PinNumberPad(onNumberPressed = ::onNumber, onBackspacePressed = ::onBackspace, biometricEnabled = false, modifier = Modifier.fillMaxWidth())
        }
    }
}