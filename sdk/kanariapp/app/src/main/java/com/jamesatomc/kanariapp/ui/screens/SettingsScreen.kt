package com.jamesatomc.kanariapp.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Logout
import android.widget.Toast
import androidx.biometric.BiometricManager
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.filled.BrightnessAuto
import androidx.compose.material.icons.filled.ChevronRight
import androidx.compose.material.icons.filled.DarkMode
import androidx.compose.material.icons.filled.Fingerprint
import androidx.compose.material.icons.filled.LightMode
import androidx.compose.material.icons.filled.Pin
import androidx.compose.material.icons.filled.Public
import androidx.compose.material.icons.filled.Security
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import com.jamesatomc.kanariapp.network.models.KanariEnvironment
import com.jamesatomc.kanariapp.ui.theme.ThemeMode
import com.jamesatomc.kanariapp.wallet.WalletViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    viewModel: WalletViewModel,
    onLogout: () -> Unit,
    onBack: () -> Unit
) {
    val currentEnv by viewModel.environment.collectAsState()
    val themeMode by viewModel.themeMode.collectAsState()
    var showPinDialog by remember { mutableStateOf(false) }
    var showEnvDialog by remember { mutableStateOf(false) }
    var showThemeDialog by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Settings", fontWeight = FontWeight.Black) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.background,
                    titleContentColor = MaterialTheme.colorScheme.onBackground
                )
            )
        },
        containerColor = MaterialTheme.colorScheme.background
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp)
        ) {
            Text(
                "APPEARANCE",
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(Modifier.height(8.dp))
            SettingsItem(
                title = "Theme",
                subtitle = when (themeMode) {
                    ThemeMode.LIGHT -> "Light"; ThemeMode.DARK -> "Dark"; ThemeMode.SYSTEM -> "System"
                },
                icon = when (themeMode) {
                    ThemeMode.LIGHT -> Icons.Default.LightMode; ThemeMode.DARK -> Icons.Default.DarkMode; ThemeMode.SYSTEM -> Icons.Default.BrightnessAuto
                },
                onClick = { showThemeDialog = true }
            )

            Spacer(Modifier.height(24.dp))

            Text(
                "SECURITY",
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(Modifier.height(8.dp))
            SettingsItem(
                title = "Change PIN",
                subtitle = "Update your 6-digit PIN",
                icon = Icons.Default.Security,
                onClick = { showPinDialog = true }
            )
            val context = LocalContext.current
            val activity = context as? androidx.fragment.app.FragmentActivity
            val biometricManager = remember { BiometricManager.from(context) }
            val biometricAvailable = remember {
                try {
                    biometricManager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG) == BiometricManager.BIOMETRIC_SUCCESS
                } catch (_: Exception) {
                    false
                }
            }
            if (biometricAvailable) {
                val biometricEnabled by viewModel.biometricEnabled.collectAsState()
                SettingsSwitchItem(
                    title = "Biometric Unlock",
                    subtitle = if (biometricEnabled) "Enabled - use fingerprint to unlock" else "Tap to enable fingerprint / face unlock",
                    icon = Icons.Default.Fingerprint,
                    checked = biometricEnabled,
                    onCheckedChange = { enabled ->
                        if (!enabled) {
                            viewModel.setBiometricEnabled(false)
                            Toast.makeText(context, "Biometric disabled", Toast.LENGTH_SHORT).show()
                        } else {
                            if (activity == null) {
                                val ok = viewModel.setBiometricEnabled(true)
                                Toast.makeText(
                                    context,
                                    if (ok) "Biometric enabled" else "Unlock with PIN first",
                                    Toast.LENGTH_SHORT
                                ).show()
                                return@SettingsSwitchItem
                            }
                            val executor = androidx.core.content.ContextCompat.getMainExecutor(activity)
                            val prompt = androidx.biometric.BiometricPrompt(
                                activity,
                                executor,
                                object : androidx.biometric.BiometricPrompt.AuthenticationCallback() {
                                    override fun onAuthenticationSucceeded(result: androidx.biometric.BiometricPrompt.AuthenticationResult) {
                                        super.onAuthenticationSucceeded(result)
                                        activity.runOnUiThread {
                                            val ok = viewModel.setBiometricEnabled(true)
                                            Toast.makeText(
                                                context,
                                                if (ok) "Biometric enabled" else "Unlock with PIN first",
                                                Toast.LENGTH_SHORT
                                            ).show()
                                        }
                                    }

                                    override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                                        super.onAuthenticationError(errorCode, errString)
                                        activity.runOnUiThread {
                                            if (errorCode != androidx.biometric.BiometricPrompt.ERROR_USER_CANCELED && errorCode != androidx.biometric.BiometricPrompt.ERROR_NEGATIVE_BUTTON) {
                                                Toast.makeText(
                                                    context,
                                                    "Biometric error: $errString",
                                                    Toast.LENGTH_SHORT
                                                ).show()
                                            }
                                        }
                                    }

                                    override fun onAuthenticationFailed() {
                                        super.onAuthenticationFailed()
                                        activity.runOnUiThread {
                                            Toast.makeText(
                                                context,
                                                "Biometric not recognized",
                                                Toast.LENGTH_SHORT
                                            ).show()
                                        }
                                    }
                                })
                            val promptInfo = androidx.biometric.BiometricPrompt.PromptInfo.Builder()
                                .setTitle("Enable Biometric Unlock")
                                .setSubtitle("Authenticate to enable fingerprint unlock")
                                .setNegativeButtonText("Cancel")
                                .build()
                            prompt.authenticate(promptInfo)
                        }
                    }
                )
            }

            Spacer(Modifier.height(24.dp))

            Text(
                "NETWORK",
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(Modifier.height(8.dp))
            SettingsItem(
                title = "Environment",
                subtitle = currentEnv.name,
                icon = Icons.Default.Public,
                onClick = { showEnvDialog = true }
            )

            Spacer(Modifier.weight(1f))

            Button(
                onClick = onLogout,
                modifier = Modifier.fillMaxWidth().height(56.dp),
                shape = RoundedCornerShape(12.dp),
                colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error)
            ) {
                Icon(Icons.AutoMirrored.Filled.Logout, contentDescription = null)
                Spacer(Modifier.width(12.dp))
                Text("Logout", fontWeight = FontWeight.Bold)
            }
        }
    }

    if (showPinDialog) {
        ChangePinDialog(
            onDismiss = { showPinDialog = false },
            onConfirm = { old, new -> viewModel.changePin(old, new) }
        )
    }

    if (showEnvDialog) {
        EnvironmentDialog(
            currentEnv = currentEnv,
            onDismiss = { showEnvDialog = false },
            onSelect = { env ->
                viewModel.setEnvironment(env)
                showEnvDialog = false
            }
        )
    }

    if (showThemeDialog) {
        ThemeDialog(
            current = themeMode,
            onDismiss = { showThemeDialog = false },
            onSelect = { viewModel.setThemeMode(it); showThemeDialog = false })
    }
}

@Composable
fun ChangePinDialog(onDismiss: () -> Unit, onConfirm: (String, String) -> Boolean) {
    var oldPin by remember { mutableStateOf("") }
    var newPin by remember { mutableStateOf("") }
    var confirmPin by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    val context = LocalContext.current

    AlertDialog(
        onDismissRequest = onDismiss,
        icon = { Icon(Icons.Default.Pin, contentDescription = null, tint = MaterialTheme.colorScheme.primary) },
        title = { Text("Change PIN") },
        text = {
            Column(
                modifier = Modifier.verticalScroll(rememberScrollState()).imePadding(),
                verticalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                OutlinedTextField(
                    value = oldPin,
                    onValueChange = {
                        if (it.length <= 6 && it.all { c -> c.isDigit() }) {
                            oldPin = it; error = null
                        }
                    },
                    label = { Text("Current PIN") },
                    placeholder = { Text("••••••") },
                    visualTransformation = PasswordVisualTransformation(),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.NumberPassword),
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                    isError = error != null && oldPin.length != 6
                )
                OutlinedTextField(
                    value = newPin,
                    onValueChange = {
                        if (it.length <= 6 && it.all { c -> c.isDigit() }) {
                            newPin = it; error = null
                        }
                    },
                    label = { Text("New PIN") },
                    placeholder = { Text("••••••") },
                    visualTransformation = PasswordVisualTransformation(),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.NumberPassword),
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true
                )
                OutlinedTextField(
                    value = confirmPin,
                    onValueChange = {
                        if (it.length <= 6 && it.all { c -> c.isDigit() }) {
                            confirmPin = it; error = null
                        }
                    },
                    label = { Text("Confirm New PIN") },
                    placeholder = { Text("••••••") },
                    visualTransformation = PasswordVisualTransformation(),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.NumberPassword),
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                    isError = error != null && newPin != confirmPin
                )
                if (error != null) {
                    Text(
                        error!!,
                        color = MaterialTheme.colorScheme.error,
                        style = MaterialTheme.typography.bodySmall,
                        modifier = Modifier.padding(top = 4.dp)
                    )
                }
            }
        },
        confirmButton = {
            Button(
                onClick = {
                    when {
                        oldPin.length != 6 || newPin.length != 6 || confirmPin.length != 6 -> error =
                            "PIN must be 6 digits"

                        newPin != confirmPin -> error = "PINs do not match"
                        oldPin == newPin -> error = "New PIN must be different"
                        else -> {
                            val ok = onConfirm(oldPin, newPin)
                            if (ok) {
                                Toast.makeText(context, "PIN changed successfully", Toast.LENGTH_SHORT).show()
                                onDismiss()
                            } else error = "Incorrect current PIN"
                        }
                    }
                },
                enabled = oldPin.length == 6 && newPin.length == 6 && confirmPin.length == 6
            ) { Text("Update") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
        modifier = Modifier.navigationBarsPadding().imePadding()
    )
}

@Composable
fun EnvironmentDialog(currentEnv: KanariEnvironment, onDismiss: () -> Unit, onSelect: (KanariEnvironment) -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Select Environment") },
        text = {
            Column {
                KanariEnvironment.entries.forEach { env ->
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { onSelect(env) }
                            .padding(vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        RadioButton(selected = env == currentEnv, onClick = { onSelect(env) })
                        Spacer(Modifier.width(8.dp))
                        Text(env.name)
                    }
                }
            }
        },
        confirmButton = {}
    )
}

@Composable
fun ThemeDialog(current: ThemeMode, onDismiss: () -> Unit, onSelect: (ThemeMode) -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Select Theme") },
        text = {
            Column {
                ThemeMode.entries.forEach { mode ->
                    val title = when (mode) {
                        ThemeMode.LIGHT -> "Light"; ThemeMode.DARK -> "Dark"; ThemeMode.SYSTEM -> "System"
                    }
                    val icon = when (mode) {
                        ThemeMode.LIGHT -> Icons.Default.LightMode; ThemeMode.DARK -> Icons.Default.DarkMode; ThemeMode.SYSTEM -> Icons.Default.BrightnessAuto
                    }
                    Row(
                        modifier = Modifier.fillMaxWidth().clickable { onSelect(mode) }.padding(vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Icon(icon, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
                        Spacer(Modifier.width(16.dp))
                        Text(title, modifier = Modifier.weight(1f))
                        RadioButton(selected = mode == current, onClick = { onSelect(mode) })
                    }
                }
            }
        },
        confirmButton = {}
    )
}

@Composable
fun SettingsItem(
    title: String,
    subtitle: String? = null,
    icon: ImageVector,
    onClick: () -> Unit
) {
    Surface(onClick = onClick, color = Color.Transparent) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(icon, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(Modifier.width(16.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(title, fontWeight = FontWeight.Bold)
                if (subtitle != null) Text(
                    subtitle,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            Icon(
                Icons.Default.ChevronRight,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
fun SettingsSwitchItem(
    title: String,
    subtitle: String? = null,
    icon: ImageVector,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit
) {
    Surface(color = Color.Transparent) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(icon, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(Modifier.width(16.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(title, fontWeight = FontWeight.Bold)
                if (subtitle != null) Text(
                    subtitle,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            Switch(checked = checked, onCheckedChange = onCheckedChange)
        }
    }
}
