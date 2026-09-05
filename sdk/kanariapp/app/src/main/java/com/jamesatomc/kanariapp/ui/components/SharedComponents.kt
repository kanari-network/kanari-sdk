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
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.MenuBook
import androidx.compose.material.icons.filled.AccountBalanceWallet
import androidx.compose.material.icons.filled.ArrowDropDown
import androidx.compose.material.icons.filled.Contacts
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.QrCode
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material.icons.filled.Settings
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
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import com.google.zxing.BarcodeFormat
import com.google.zxing.qrcode.QRCodeWriter
import com.jamesatomc.kanariapp.network.models.TokenBalance
import com.jamesatomc.kanariapp.wallet.WalletRecord
import com.journeyapps.barcodescanner.ScanContract
import com.journeyapps.barcodescanner.ScanOptions
import androidx.compose.material.icons.filled.CurrencyExchange
import androidx.compose.material.icons.filled.Security
import androidx.compose.material.icons.filled.Visibility
import androidx.compose.ui.text.font.FontWeight
import kotlin.math.pow
import androidx.core.graphics.set
import androidx.core.graphics.createBitmap

// ---------- Utils ----------

fun String.toShortAddress(): String =
    if (length <= 16) this else take(8) + "..." + takeLast(8)

fun formatMist(amount: Long, decimals: Int): Double =
    amount / 10.0.pow(decimals.toDouble())

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

// ---------- Curve Info ----------

data class CurveInfo(val displayName: String, val description: String, val isPostQuantum: Boolean)

fun getCurveInfo(curveType: String): CurveInfo = when (curveType) {
    "K256" -> CurveInfo("K256 (secp256k1)", "Bitcoin/Ethereum", false)
    "P256" -> CurveInfo("P256 (secp256r1)", "NIST P-256", false)
    "Ed25519" -> CurveInfo("Ed25519", "EdDSA - Fast modern", false)
    "Dilithium2" -> CurveInfo("Dilithium2", "Post-Quantum NIST Level 2", true)
    "Dilithium3" -> CurveInfo("Dilithium3", "Post-Quantum NIST Level 3", true)
    "Dilithium5" -> CurveInfo("Dilithium5", "Post-Quantum NIST Level 5", true)
    "SphincsPlusSha256Robust" -> CurveInfo("SPHINCS+-SHA256", "Hash-based Post-Quantum", true)
    "Falcon512", "FnDsa512" -> CurveInfo("Falcon-512", "Compact Lattice PQ", true)
    "Falcon1024", "FnDsa1024" -> CurveInfo("Falcon-1024", "Compact Lattice PQ Level 5", true)
    "Ed25519Dilithium3" -> CurveInfo("Ed25519 + Dilithium3", "Hybrid Classical + PQ", true)
    "K256Dilithium3" -> CurveInfo("K256 + Dilithium3", "Hybrid Classical + PQ", true)
    else -> CurveInfo(
        curveType,
        if (curveType.contains("Dilithium") || curveType.contains("Falcon") || curveType.contains("Sphincs")) "Post-Quantum" else "Classical",
        curveType.contains("Dilithium") || curveType.contains("Falcon")
    )
}

// ---------- Shared UI ----------

@Composable
fun SecurityWarningCard(text: String, modifier: Modifier = Modifier) {
    Surface(
        color = MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.3f),
        shape = RoundedCornerShape(12.dp),
        modifier = modifier.fillMaxWidth()
    ) {
        Row(
            Modifier.padding(12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Icon(
                Icons.Default.Security,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.error,
                modifier = Modifier.size(18.dp)
            )
            Text(
                text,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onErrorContainer
            )
        }
    }
}

@Composable
fun SecretRevealCard(
    title: String,
    secret: String?,
    isVisible: Boolean,
    onToggleVisibility: () -> Unit,
    onCopy: () -> Unit,
    modifier: Modifier = Modifier
) {
    val icon = remember(title) {
        when (title) {
            "Private Key" -> Icons.Filled.Visibility
            else -> Icons.AutoMirrored.Filled.MenuBook
        }
    }
    val containerColor = when (title) {
        "Private Key" -> MaterialTheme.colorScheme.primaryContainer
        else -> MaterialTheme.colorScheme.secondaryContainer
    }
    val contentColor = when (title) {
        "Private Key" -> MaterialTheme.colorScheme.onPrimaryContainer
        else -> MaterialTheme.colorScheme.onSecondaryContainer
    }
    Card(
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
        shape = RoundedCornerShape(16.dp),
        modifier = modifier.fillMaxWidth()
    ) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Surface(
                    shape = CircleShape,
                    color = containerColor,
                    modifier = Modifier.size(32.dp)
                ) {
                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        Icon(icon, contentDescription = null, modifier = Modifier.size(18.dp), tint = contentColor)
                    }
                }
                Text(
                    title,
                    style = MaterialTheme.typography.titleSmall.copy(fontWeight = FontWeight.Bold)
                )
            }
            Surface(
                color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                shape = RoundedCornerShape(12.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                Text(
                    if (isVisible && secret != null) secret else "•".repeat(32),
                    style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                    color = MaterialTheme.colorScheme.onSurface,
                    modifier = Modifier.padding(12.dp)
                )
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                OutlinedButton(onClick = onCopy, modifier = Modifier.weight(1f)) {
                    Icon(Icons.Default.ContentCopy, contentDescription = null, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(6.dp)); Text("Copy")
                }
                Button(onClick = onToggleVisibility, modifier = Modifier.weight(1f)) {
                    Text(if (isVisible) "Hide" else "Show")
                }
            }
        }
    }
}

fun generateQrBitmap(text: String, size: Int): Bitmap? = try {
    val writer = QRCodeWriter()
    val bitMatrix = writer.encode(text, BarcodeFormat.QR_CODE, size, size)
    val w = bitMatrix.width
    val h = bitMatrix.height
    val bmp = createBitmap(w, h, Bitmap.Config.RGB_565)
    for (x in 0 until w) for (y in 0 until h) {
        bmp[x, y] =
            if (bitMatrix.get(x, y)) android.graphics.Color.BLACK else android.graphics.Color.WHITE
    }
    bmp
} catch (_: Exception) {
    null
}

// ---------- Reusable UI (shared across Dashboard/History/Receive) ----------

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun KanariTopBar(
    environmentName: String,
    onEnvClick: () -> Unit,
    onReceive: () -> Unit,
    onSettings: () -> Unit,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier) {
        TopAppBar(
            title = {
                Row(
                    modifier = Modifier.clickable { onEnvClick() },
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(10.dp)
                ) {
                    Text(
                        "Kanari Wallet",
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.Bold
                    )
                    Surface(
                        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.6f),
                        shape = RoundedCornerShape(20.dp),
                        border = androidx.compose.foundation.BorderStroke(
                            1.dp,
                            MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.5f)
                        )
                    ) {
                        Row(
                            Modifier.padding(horizontal = 10.dp, vertical = 4.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(6.dp)
                        ) {
                            Box(
                                Modifier.size(8.dp).background(
                                    when (environmentName.lowercase()) {
                                        "dev" -> com.jamesatomc.kanariapp.ui.theme.KanariColors.Lime
                                        "mainnet" -> com.jamesatomc.kanariapp.ui.theme.KanariColors.Lavender
                                        else -> MaterialTheme.colorScheme.primary
                                    }, CircleShape
                                )
                            )
                            Text(
                                environmentName.uppercase(),
                                style = MaterialTheme.typography.labelSmall.copy(
                                    fontWeight = FontWeight.Bold,
                                    letterSpacing = 0.5.sp
                                ),
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                            Icon(
                                Icons.Default.ArrowDropDown,
                                contentDescription = null,
                                modifier = Modifier.size(16.dp),
                                tint = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                }
            },
            actions = {
                IconButton(onClick = onReceive) { Icon(Icons.Default.QrCode, contentDescription = "Receive") }
                IconButton(onClick = onSettings) { Icon(Icons.Default.Settings, contentDescription = "Settings") }
            },
            colors = TopAppBarDefaults.topAppBarColors(
                containerColor = MaterialTheme.colorScheme.surface,
                scrolledContainerColor = MaterialTheme.colorScheme.surfaceContainer
            )
        )
        HorizontalDivider(thickness = 2.dp, color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.2f))
    }
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
fun TokenIcon(token: TokenBalance, modifier: Modifier = Modifier) {
    val isKanari = token.symbol.equals("KANARI", true) || token.tokenType.endsWith("::KANARI", true)
    val isUsdc = token.symbol.equals("USDC", true)
    val container = when {
        isKanari -> MaterialTheme.colorScheme.primaryContainer; isUsdc -> MaterialTheme.colorScheme.secondaryContainer; else -> MaterialTheme.colorScheme.tertiaryContainer
    }
    val content = when {
        isKanari -> MaterialTheme.colorScheme.onPrimaryContainer; isUsdc -> MaterialTheme.colorScheme.onSecondaryContainer; else -> MaterialTheme.colorScheme.onTertiaryContainer
    }
    Surface(shape = CircleShape, color = container, modifier = modifier.size(40.dp)) {
        Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            if (!token.iconUrl.isNullOrBlank()) {
                coil.compose.AsyncImage(
                    model = token.iconUrl,
                    contentDescription = token.symbol,
                    modifier = Modifier.fillMaxSize().padding(6.dp)
                )
            } else {
                if (isKanari || isUsdc) Text(
                    token.symbol.take(1).uppercase(),
                    style = MaterialTheme.typography.titleSmall.copy(fontWeight = FontWeight.Bold),
                    color = content
                )
                else Icon(
                    Icons.Default.CurrencyExchange,
                    contentDescription = null,
                    modifier = Modifier.size(20.dp),
                    tint = content
                )
            }
        }
    }
}

@Composable
fun DetailRowShared(
    label: String,
    value: String,
    valueColor: androidx.compose.ui.graphics.Color = MaterialTheme.colorScheme.onSurface
) {
    Column(Modifier.fillMaxWidth()) {
        Text(label, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(
            value,
            style = MaterialTheme.typography.bodyMedium.copy(color = valueColor),
            modifier = Modifier.padding(top = 2.dp)
        )
    }
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