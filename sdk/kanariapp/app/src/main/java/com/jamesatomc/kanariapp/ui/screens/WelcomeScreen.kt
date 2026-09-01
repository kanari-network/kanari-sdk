package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.animation.core.*
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Login
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.FileDownload
import androidx.compose.material.icons.filled.LockOpen
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.jamesatomc.kanariapp.wallet.WalletStorage
import kotlin.math.cos
import kotlin.math.sin

@Composable
fun WelcomeScreen(
    onNavigateToLogin: () -> Unit,
    onNavigateToRegister: () -> Unit,
    onNavigateToWalletGen: () -> Unit,
    onNavigateToUnlock: () -> Unit
) {
    val context = LocalContext.current
    val walletStorage = remember { WalletStorage(context) }
    var hasWallet by remember { mutableStateOf(walletStorage.loadWallets().isNotEmpty()) }
    
    LaunchedEffect(Unit) {
        hasWallet = walletStorage.loadWallets().isNotEmpty()
    }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            // Brand Bar
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(64.dp)
                    .clip(RoundedCornerShape(32.dp))
                    .background(MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f))
                    .padding(horizontal = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(
                        modifier = Modifier
                            .size(46.dp)
                            .clip(CircleShape)
                            .background(MaterialTheme.colorScheme.primary),
                        contentAlignment = Alignment.Center
                    ) {
                        Text("K", fontWeight = FontWeight.Black, color = MaterialTheme.colorScheme.onPrimary)
                    }
                    Spacer(Modifier.width(12.dp))
                    Text(
                        text = "KANARI",
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.Black,
                        letterSpacing = 1.sp
                    )
                }
                
                Button(
                    onClick = onNavigateToRegister,
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.primary,
                        contentColor = MaterialTheme.colorScheme.onPrimary
                    ),
                    shape = RoundedCornerShape(22.dp),
                    modifier = Modifier.height(44.dp)
                ) {
                    Text("REGISTER", fontWeight = FontWeight.Bold, style = MaterialTheme.typography.labelLarge)
                }
            }
            
            Spacer(Modifier.height(60.dp))
            
            // Hero Section
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Text(
                    "YOUR ASSETS. YOUR CONTROL.",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.secondary,
                    fontWeight = FontWeight.Bold
                )
                Spacer(Modifier.height(22.dp))
                Text(
                    "Own your\nnext move.",
                    style = MaterialTheme.typography.displayLarge.copy(
                        fontSize = 54.sp,
                        fontWeight = FontWeight.Black,
                        lineHeight = 52.sp
                    ),
                    textAlign = TextAlign.Center
                )
                Spacer(Modifier.height(18.dp))
                Text(
                    "A secure Kanari wallet for Move-powered digital assets.",
                    style = MaterialTheme.typography.bodyLarge,
                    textAlign = TextAlign.Center,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            
            Spacer(Modifier.height(30.dp))
            
            NetworkMark()
            
            Spacer(Modifier.height(48.dp))
            
            // Access Panel
            ElevatedCard(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(24.dp),
                colors = CardDefaults.elevatedCardColors(
                    containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f)
                )
            ) {
                Column(
                    modifier = Modifier.padding(24.dp),
                    horizontalAlignment = Alignment.Start
                ) {
                    Text(
                        "WALLET ACCESS", 
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Spacer(Modifier.height(10.dp))
                    Text(
                        if (hasWallet) "Welcome back." else "Start with Kanari.",
                        style = MaterialTheme.typography.headlineLarge,
                        fontWeight = FontWeight.ExtraBold
                    )
                    Spacer(Modifier.height(8.dp))
                    Text(
                        if (hasWallet) "Unlock your saved wallet or add another account."
                        else "Create a wallet or import an existing private key.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    
                    Spacer(Modifier.height(26.dp))
                    
                    if (hasWallet) {
                        AccessButton(
                            onClick = onNavigateToUnlock,
                            icon = Icons.Default.LockOpen,
                            label = "Unlock Saved Wallet",
                            containerColor = MaterialTheme.colorScheme.primary,
                            contentColor = MaterialTheme.colorScheme.onPrimary
                        )
                        Spacer(Modifier.height(10.dp))
                    }
                    
                    AccessButton(
                        onClick = onNavigateToWalletGen,
                        icon = Icons.Default.Add,
                        label = "Create New Wallet",
                        containerColor = MaterialTheme.colorScheme.secondary.copy(alpha = 0.2f),
                        contentColor = MaterialTheme.colorScheme.secondary
                    )
                    
                    Spacer(Modifier.height(10.dp))
                    
                    OutlinedButton(
                        onClick = onNavigateToWalletGen,
                        modifier = Modifier.fillMaxWidth().height(56.dp),
                        shape = RoundedCornerShape(12.dp),
                        border = androidx.compose.foundation.BorderStroke(1.dp, MaterialTheme.colorScheme.outline)
                    ) {
                        Icon(Icons.Default.FileDownload, contentDescription = null)
                        Spacer(Modifier.width(12.dp))
                        Text("Import Existing Wallet", fontWeight = FontWeight.Bold)
                    }
                    
                    Spacer(Modifier.height(22.dp))
                    
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        HorizontalDivider(modifier = Modifier.weight(1f))
                        Text(
                            " ACCOUNT ", 
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(horizontal = 8.dp)
                        )
                        HorizontalDivider(modifier = Modifier.weight(1f))
                    }
                    
                    Spacer(Modifier.height(8.dp))
                    
                    TextButton(
                        onClick = onNavigateToLogin,
                        modifier = Modifier.align(Alignment.CenterHorizontally)
                    ) {
                        Icon(Icons.AutoMirrored.Filled.Login, contentDescription = null)
                        Spacer(Modifier.width(8.dp))
                        Text("Login to Kanari account", fontWeight = FontWeight.Bold)
                    }
                }
            }
        }
    }
}

@Composable
fun NetworkMark() {
    val infiniteTransition = rememberInfiniteTransition(label = "orbit")
    val angle by infiniteTransition.animateFloat(
        initialValue = 0f,
        targetValue = 2 * Math.PI.toFloat(),
        animationSpec = infiniteRepeatable(
            animation = tween(26000, easing = LinearEasing),
            repeatMode = RepeatMode.Restart
        ),
        label = "angle"
    )
    
    val floatAnim by infiniteTransition.animateFloat(
        initialValue = -7f,
        targetValue = 7f,
        animationSpec = infiniteRepeatable(
            animation = tween(2000, easing = LinearOutSlowInEasing),
            repeatMode = RepeatMode.Reverse
        ),
        label = "float"
    )

    val pulseAnim by infiniteTransition.animateFloat(
        initialValue = 0.975f,
        targetValue = 1.025f,
        animationSpec = infiniteRepeatable(
            animation = tween(1000, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse
        ),
        label = "pulse"
    )

    Box(
        modifier = Modifier
            .size(240.dp)
            .offset(y = floatAnim.dp),
        contentAlignment = Alignment.Center
    ) {
        // Outer glow
        Box(
            modifier = Modifier
                .fillMaxSize()
                .clip(CircleShape)
                .background(MaterialTheme.colorScheme.secondary.copy(alpha = 0.16f))
        )
        
        // Orbit rings
        Canvas(modifier = Modifier.fillMaxSize()) {
            val color = Color(0xFF111B18).copy(alpha = 0.2f)
            drawCircle(color, radius = size.minDimension / 2 * 0.72f, style = androidx.compose.ui.graphics.drawscope.Stroke(width = 1.dp.toPx()))
            drawCircle(color, radius = size.minDimension / 2 * 0.48f, style = androidx.compose.ui.graphics.drawscope.Stroke(width = 1.dp.toPx()))
        }
        
        // Nodes
        val radius = 120.dp * 0.36f
        
        OrbitNode(label = "K", angle = angle - (Math.PI * 0.78).toFloat(), radius = radius)
        OrbitNode(label = "M", angle = angle + (Math.PI * 0.05).toFloat(), radius = radius * 0.82f, dark = true)
        OrbitNode(label = "01", angle = angle + (Math.PI * 0.78).toFloat(), radius = radius)

        // Center Icon
        Box(
            modifier = Modifier
                .size(64.dp)
                .scale(pulseAnim)
                .clip(CircleShape)
                .background(MaterialTheme.colorScheme.primary)
                .padding(10.dp),
            contentAlignment = Alignment.Center
        ) {
            Text("K", fontWeight = FontWeight.Black, color = MaterialTheme.colorScheme.onPrimary, fontSize = 24.sp)
        }
    }
}

@Composable
fun OrbitNode(label: String, angle: Float, radius: androidx.compose.ui.unit.Dp, dark: Boolean = false) {
    val x = (cos(angle.toDouble()) * radius.value).dp
    val y = (sin(angle.toDouble()) * radius.value).dp
    
    Box(
        modifier = Modifier
            .offset(x = x, y = y)
            .size(38.dp)
            .clip(CircleShape)
            .background(if (dark) Color(0xFF111B18) else MaterialTheme.colorScheme.surface)
            .border(1.dp, Color(0xFF111B18).copy(alpha = 0.16f), CircleShape),
        contentAlignment = Alignment.Center
    ) {
        Text(
            label, 
            fontWeight = FontWeight.Black, 
            fontSize = 12.sp,
            color = if (dark) Color.White else Color(0xFF111B18)
        )
    }
}

@Composable
fun AccessButton(
    onClick: () -> Unit,
    icon: ImageVector,
    label: String,
    containerColor: androidx.compose.ui.graphics.Color,
    contentColor: androidx.compose.ui.graphics.Color = contentColorFor(containerColor)
) {
    Button(
        onClick = onClick,
        modifier = Modifier.fillMaxWidth().height(56.dp),
        shape = RoundedCornerShape(12.dp),
        colors = ButtonDefaults.buttonColors(
            containerColor = containerColor,
            contentColor = contentColor
        )
    ) {
        Icon(icon, contentDescription = null)
        Spacer(Modifier.width(12.dp))
        Text(label, fontWeight = FontWeight.Bold)
    }
}
