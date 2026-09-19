package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.animation.core.*
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.FileDownload
import androidx.compose.material.icons.filled.LockOpen
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.res.painterResource
import com.jamesatomc.kanariapp.R
import com.jamesatomc.kanariapp.ui.components.ErrorBanner
import com.jamesatomc.kanariapp.ui.components.IsometricNetworkOrbit
import com.jamesatomc.kanariapp.ui.components.IsometricWalletIllustration
import com.jamesatomc.kanariapp.ui.components.PinGateDialog
import com.jamesatomc.kanariapp.wallet.WalletStorage
import com.jamesatomc.kanariapp.wallet.WalletViewModel
import com.jamesatomc.kanariapp.wallet.zklogin.ZkLoginAuth
import kotlinx.coroutines.launch

@Composable
fun WelcomeScreen(
    viewModel: WalletViewModel,
    onNavigateToWalletGen: () -> Unit,
    onNavigateToUnlock: () -> Unit,
    onLoginSuccess: () -> Unit,
) {
    val context = LocalContext.current
    val walletStorage = remember { WalletStorage(context) }
    var hasWallet by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    var zkLoading by remember { mutableStateOf(value = false) }
    var showPinGate by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        try {
            hasWallet = walletStorage.loadWallets().isNotEmpty()
        } catch (e: Exception) {
            android.util.Log.e("WelcomeScreen", "Failed to check wallets", e)
            hasWallet = false
        }
    }

    val animProgress = remember { Animatable(0f) }
    LaunchedEffect(Unit) {
        animProgress.animateTo(1f, tween(900, easing = EaseOutCubic))
    }

    val doGoogleLogin: suspend () -> Unit = {
        zkLoading = true
        error = null
        try {
            val result = ZkLoginAuth.login(context = context)
            viewModel.addZkLoginWallet(result.session)
            onLoginSuccess()
        } catch (_: ZkLoginAuth.CancelledException) {
            // User dismissed
        } catch (e: Exception) {
            error = e.message ?: "Google login failed"
        } finally {
            zkLoading = false
        }
    }

    if (showPinGate) {
        PinGateDialog(
            title = "Enter PIN",
            subtitle = "Enter 6-digit PIN to continue with Google sign-in",
            onVerifyAsync = { pin -> viewModel.verifyPin(pin) },
            onSuccess = {
                showPinGate = false
                scope.launch { doGoogleLogin() }
            },
            onDismiss = { showPinGate = false }
        )
    }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.surface
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState()),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            // Pill-shaped brand bar
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 12.dp)
                    .clip(RoundedCornerShape(28.dp))
                    .background(MaterialTheme.colorScheme.surfaceContainer)
                    .padding(horizontal = 16.dp, vertical = 10.dp),
                contentAlignment = Alignment.CenterStart
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    com.jamesatomc.kanariapp.ui.components.KanariLogo(
                        modifier = Modifier.size(36.dp)
                    )
                    Spacer(Modifier.width(10.dp))
                    Text(
                        "KANARI",
                        style = MaterialTheme.typography.titleSmall.copy(fontWeight = FontWeight.ExtraBold),
                        modifier = Modifier.weight(1f)
                    )
                }
            }

            Spacer(Modifier.height(16.dp))

            // Hero illustration section
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(240.dp)
                    .graphicsLayer { alpha = animProgress.value; translationY = (1f - animProgress.value) * 30f }
            ) {
                IsometricNetworkOrbit(
                    modifier = Modifier
                        .size(220.dp)
                        .align(Alignment.Center)
                )
                IsometricWalletIllustration(
                    modifier = Modifier
                        .size(140.dp)
                        .align(Alignment.Center)
                )
            }

            Spacer(Modifier.height(8.dp))

            // Tagline
            Column(
                modifier = Modifier
                    .padding(horizontal = 24.dp)
                    .graphicsLayer { alpha = animProgress.value; translationY = (1f - animProgress.value) * 40f },
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Text(
                    "YOUR ASSETS. YOUR CONTROL.",
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.secondary,
                    letterSpacing = MaterialTheme.typography.labelLarge.letterSpacing
                )
                Spacer(Modifier.height(8.dp))
                Text(
                    "Own your\nnext move.",
                    style = MaterialTheme.typography.displaySmall.copy(fontWeight = FontWeight.Bold),
                    textAlign = TextAlign.Center
                )
                Spacer(Modifier.height(12.dp))
                Text(
                    "Manage your Kanari Network assets with post-quantum security.",
                    style = MaterialTheme.typography.bodyLarge,
                    textAlign = TextAlign.Center,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            Spacer(Modifier.height(32.dp))

            // Access panel
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp)
                    .graphicsLayer { alpha = animProgress.value; translationY = (1f - animProgress.value) * 50f },
                shape = RoundedCornerShape(24.dp),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLowest),
                elevation = CardDefaults.cardElevation(defaultElevation = 0.dp)
            ) {
                Column(
                    modifier = Modifier.padding(24.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    if (error != null) {
                        ErrorBanner(error = error!!)
                        Spacer(Modifier.height(4.dp))
                    }

                    if (hasWallet) {
                        Button(
                            onClick = onNavigateToUnlock,
                            modifier = Modifier.fillMaxWidth().height(56.dp),
                            shape = RoundedCornerShape(12.dp)
                        ) {
                            Icon(Icons.Default.LockOpen, contentDescription = null)
                            Spacer(Modifier.width(8.dp))
                            Text("Unlock Saved Wallet", fontWeight = FontWeight.Bold)
                        }
                    }

                    Button(
                        onClick = {
                            scope.launch {
                                if (viewModel.hasPin()) {
                                    showPinGate = true
                                } else {
                                    doGoogleLogin()
                                }
                            }
                        },
                        modifier = Modifier.fillMaxWidth().height(56.dp),
                        shape = RoundedCornerShape(12.dp),
                        enabled = !zkLoading,
                        colors = ButtonDefaults.buttonColors(
                            containerColor = Color.White,
                            contentColor = Color(0xFF1F1F1F)
                        ),
                        elevation = ButtonDefaults.buttonElevation(defaultElevation = 2.dp)
                    ) {
                        if (zkLoading) {
                            CircularProgressIndicator(
                                modifier = Modifier.size(20.dp),
                                strokeWidth = 2.dp,
                                color = Color(0xFF1F1F1F)
                            )
                        } else {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Icon(
                                    painter = painterResource(id = R.drawable.ic_google_logo),
                                    contentDescription = "Google Logo",
                                    modifier = Modifier.size(20.dp),
                                    tint = Color.Unspecified
                                )
                                Spacer(Modifier.width(12.dp))
                                Text("Sign in with Google", fontWeight = FontWeight.Bold)
                            }
                        }
                    }

                    Spacer(Modifier.height(4.dp))
                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.5f))
                    Spacer(Modifier.height(4.dp))

                    FilledTonalButton(
                        onClick = onNavigateToWalletGen,
                        modifier = Modifier.fillMaxWidth().height(56.dp),
                        shape = RoundedCornerShape(12.dp)
                    ) {
                        Icon(Icons.Default.Add, contentDescription = null)
                        Spacer(Modifier.width(8.dp))
                        Text("Create New Wallet", fontWeight = FontWeight.Bold)
                    }
                    OutlinedButton(
                        onClick = onNavigateToWalletGen,
                        modifier = Modifier.fillMaxWidth().height(56.dp),
                        shape = RoundedCornerShape(12.dp)
                    ) {
                        Icon(Icons.Default.FileDownload, contentDescription = null)
                        Spacer(Modifier.width(8.dp))
                        Text("Import Existing Wallet", fontWeight = FontWeight.Bold)
                    }
                }
            }

            Spacer(Modifier.height(24.dp))
        }
    }
}
