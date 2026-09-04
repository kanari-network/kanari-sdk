package com.jamesatomc.kanariapp.ui.components

import android.Manifest
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.pm.PackageManager
import android.graphics.Bitmap
import android.util.Log
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.AccountBalanceWallet
import androidx.compose.material.icons.filled.Contacts
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import com.google.zxing.BarcodeFormat
import com.google.zxing.qrcode.QRCodeWriter
import com.jamesatomc.kanariapp.wallet.WalletRecord
import com.journeyapps.barcodescanner.ScanContract
import com.journeyapps.barcodescanner.ScanOptions

// ---------- Utils ----------

fun String.toShortAddress(): String =
    if (length <= 16) this else take(8) + "..." + takeLast(8)

fun formatMist(amount: Long, decimals: Int): Double =
    amount / Math.pow(10.0, decimals.toDouble())

fun parseAmountToMist(amountStr: String, decimals: Int): ULong? {
    val trimmed = amountStr.trim()
    if (trimmed.isEmpty() || trimmed.startsWith("-")) return null
    val parts = trimmed.split(".")
    if (parts.size > 2) return null
    val intPart = parts[0].ifEmpty { "0" }
    val fracPart = if (parts.size == 2) parts[1] else ""
    if (fracPart.length > decimals) return null
    if (!intPart.matches(Regex("\\d+"))) return null
    if (fracPart.isNotEmpty() && !fracPart.matches(Regex("\\d+"))) return null
    val fracPadded = fracPart.padEnd(decimals, '0')
    val combined = intPart + fracPadded
    val normalized = combined.trimStart('0').ifEmpty { "0" }
    return try {
        normalized.toULong()
    } catch (_: Exception) {
        null
    }
}

fun copyToClipboard(
    context: Context,
    text: String,
    label: String = "kanari_address",
    toast: String = "Address copied"
) {
    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    clipboard.setPrimaryClip(ClipData.newPlainText(label, text))
    Toast.makeText(context, toast, Toast.LENGTH_SHORT).show()
}

fun extractAddressFromQr(raw: String): String {
    val regex = Regex("0x[0-9a-fA-F]{1,64}")
    return regex.find(raw.trim())?.value ?: raw.trim()
}

fun generateQrBitmap(text: String, size: Int): Bitmap? = try {
    val writer = QRCodeWriter()
    val bitMatrix = writer.encode(text, BarcodeFormat.QR_CODE, size, size)
    val w = bitMatrix.width
    val h = bitMatrix.height
    val bmp = Bitmap.createBitmap(w, h, Bitmap.Config.RGB_565)
    for (x in 0 until w) for (y in 0 until h) {
        bmp.setPixel(x, y, if (bitMatrix.get(x, y)) android.graphics.Color.BLACK else android.graphics.Color.WHITE)
    }
    bmp
} catch (_: Exception) {
    null
}

// ---------- Reusable UI ----------

@Composable
fun CopyableAddressRow(
    address: String,
    modifier: Modifier = Modifier,
    short: Boolean = true,
    copyToast: String = "Address copied",
    textStyle: androidx.compose.ui.text.TextStyle = MaterialTheme.typography.bodySmall,
) {
    val context = LocalContext.current
    Row(
        modifier = modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceBetween
    ) {
        Text(
            text = if (short) address.toShortAddress() else address,
            style = textStyle.copy(
                fontFamily = FontFamily.Monospace,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            ),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f)
        )
        IconButton(
            onClick = { copyToClipboard(context, address, toast = copyToast) },
            modifier = Modifier.size(28.dp)
        ) {
            Icon(
                Icons.Default.ContentCopy,
                contentDescription = "Copy",
                modifier = Modifier.size(18.dp),
                tint = MaterialTheme.colorScheme.primary
            )
        }
    }
}

@Composable
fun QrCodeImage(
    address: String,
    modifier: Modifier = Modifier,
    size: Dp = 240.dp,
    placeholder: @Composable () -> Unit = {
        Box(
            Modifier.fillMaxSize(),
            contentAlignment = Alignment.Center
        ) { CircularProgressIndicator() }
    }
) {
    val bmp = remember(address) { if (address.isNotEmpty()) generateQrBitmap(address, 512) else null }
    Surface(
        modifier = modifier.size(size).padding(8.dp),
        shape = RoundedCornerShape(16.dp),
        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)
    ) {
        if (bmp != null) Image(
            bitmap = bmp.asImageBitmap(),
            contentDescription = "QR",
            modifier = Modifier.fillMaxSize()
        )
        else placeholder()
    }
}

@Composable
fun WalletPickerDialog(
    wallets: List<WalletRecord>,
    onPick: (WalletRecord) -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Select wallet address") },
        text = {
            Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                if (wallets.isEmpty()) {
                    Text(
                        "No saved wallets",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                } else {
                    wallets.forEach { w ->
                        ListItem(
                            headlineContent = { Text(w.name, style = MaterialTheme.typography.titleSmall) },
                            supportingContent = {
                                Text(
                                    w.address,
                                    style = MaterialTheme.typography.bodySmall,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis
                                )
                            },
                            leadingContent = { Icon(Icons.Default.AccountBalanceWallet, contentDescription = null) },
                            modifier = Modifier.clickable { onPick(w) }
                        )
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f))
                    }
                }
            }
        },
        confirmButton = {},
        dismissButton = { TextButton(onClick = onDismiss) { Text("Close") } }
    )
}

@Composable
fun RecipientAddressField(
    value: String,
    onValueChange: (String) -> Unit,
    wallets: List<WalletRecord>,
    modifier: Modifier = Modifier,
    label: String = "Recipient Address",
) {
    val context = LocalContext.current
    var showPicker by remember { mutableStateOf(false) }

    val qrLauncher = rememberLauncherForActivityResult(ScanContract()) { result ->
        if (result.contents != null) {
            onValueChange(extractAddressFromQr(result.contents))
            Toast.makeText(context, "Scan successful", Toast.LENGTH_SHORT).show()
        }
    }
    val permLauncher =
        rememberLauncherForActivityResult(androidx.activity.result.contract.ActivityResultContracts.RequestPermission()) { granted ->
            if (granted) {
                try {
                    qrLauncher.launch(ScanOptions().apply {
                        setDesiredBarcodeFormats(ScanOptions.QR_CODE)
                        setPrompt("Scan wallet QR")
                        setBeepEnabled(true)
                        setBarcodeImageEnabled(true)
                        setOrientationLocked(true)
                        setCaptureActivity(com.journeyapps.barcodescanner.CaptureActivity::class.java)
                    })
                } catch (e: Exception) {
                    Log.e("RecipientField", "scanner failed", e)
                    Toast.makeText(context, "Failed to open camera: ${e.message}", Toast.LENGTH_LONG).show()
                }
            } else Toast.makeText(context, "Camera permission required", Toast.LENGTH_SHORT).show()
        }

    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        label = { Text(label) },
        modifier = modifier.fillMaxWidth(),
        singleLine = false,
        maxLines = 3,
        trailingIcon = {
            Row {
                IconButton(onClick = { showPicker = true }) {
                    Icon(
                        Icons.Default.Contacts,
                        contentDescription = "Select from wallets",
                        tint = MaterialTheme.colorScheme.primary
                    )
                }
                IconButton(onClick = {
                    try {
                        if (ContextCompat.checkSelfPermission(
                                context,
                                Manifest.permission.CAMERA
                            ) == PackageManager.PERMISSION_GRANTED
                        ) {
                            qrLauncher.launch(ScanOptions().apply {
                                setDesiredBarcodeFormats(ScanOptions.QR_CODE)
                                setPrompt("Scan wallet QR")
                                setBeepEnabled(true)
                                setBarcodeImageEnabled(true)
                                setOrientationLocked(true)
                                setCaptureActivity(com.journeyapps.barcodescanner.CaptureActivity::class.java)
                            })
                        } else permLauncher.launch(Manifest.permission.CAMERA)
                    } catch (e: Exception) {
                        Log.e("RecipientField", "launch error", e)
                        Toast.makeText(context, "Cannot open camera: ${e.message}", Toast.LENGTH_LONG).show()
                    }
                }) {
                    Icon(
                        Icons.Default.QrCodeScanner,
                        contentDescription = "Scan QR",
                        tint = MaterialTheme.colorScheme.primary
                    )
                }
            }
        }
    )
    if (showPicker) WalletPickerDialog(
        wallets = wallets,
        onPick = { onValueChange(it.address); showPicker = false },
        onDismiss = { showPicker = false })
}
