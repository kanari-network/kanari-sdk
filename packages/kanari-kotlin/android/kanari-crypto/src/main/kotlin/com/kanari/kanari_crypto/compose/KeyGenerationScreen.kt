package com.kanari.kanari_crypto.compose

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.kanari.kanari_crypto.KanariCrypto
import com.kanari.kanari_crypto.model.CurveInfoModel
import com.kanari.kanari_crypto.model.KeyPairModel
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun KeyGenerationScreen(
    modifier: Modifier = Modifier,
    defaultCurve: String = KanariCrypto.DEFAULT_CURVE,
    onKeyPairGenerated: ((KeyPairModel) -> Unit)? = null,
) {
    val snackbarHostState = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()

    var curves by remember { mutableStateOf<List<CurveInfoModel>>(emptyList()) }
    var selectedCurve by remember { mutableStateOf(defaultCurve) }
    var mnemonic by remember { mutableStateOf<String?>(null) }
    var keyPair by remember { mutableStateOf<KeyPairModel?>(null) }
    var isLoading by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        isLoading = true
        runCatching {
            withContext(Dispatchers.Default) {
                KanariCrypto.listSupportedCurves()
            }
        }.onSuccess { curves = it }
            .onFailure { errorMessage = it.message }
        isLoading = false
    }

    LaunchedEffect(errorMessage) {
        errorMessage?.let {
            snackbarHostState.showSnackbar(it)
            errorMessage = null
        }
    }

    Scaffold(
        modifier = modifier,
        snackbarHost = { SnackbarHost(snackbarHostState) },
        topBar = {
            TopAppBar(title = { Text("Kanari Wallet") })
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp)
                .verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text(
                text = "Select curve",
                style = MaterialTheme.typography.titleMedium,
            )

            if (curves.isNotEmpty()) {
                CurveSelector(
                    curves = curves,
                    selectedCurve = selectedCurve,
                    onCurveSelected = { selectedCurve = it },
                )
            }

            Button(
                onClick = {
                    scope.launch {
                        isLoading = true
                        runCatching {
                            withContext(Dispatchers.Default) {
                                val words = KanariCrypto.generateMnemonic(12)
                                val pair = KanariCrypto.deriveKeypairFromMnemonic(words, selectedCurve)
                                words to pair
                            }
                        }.onSuccess { (words, pair) ->
                            mnemonic = words
                            keyPair = pair
                            onKeyPairGenerated?.invoke(pair)
                        }.onFailure {
                            errorMessage = it.message ?: "Key generation failed"
                        }
                        isLoading = false
                    }
                },
                enabled = !isLoading,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text("Generate new wallet")
            }

            if (isLoading) {
                CircularProgressIndicator(modifier = Modifier.align(Alignment.CenterHorizontally))
            }

            mnemonic?.let { words ->
                Text(
                    text = "Recovery phrase",
                    style = MaterialTheme.typography.titleMedium,
                )
                MnemonicDisplay(mnemonic = words)
            }

            keyPair?.let { pair ->
                WalletAddressCard(
                    address = pair.address,
                    onCopied = {
                        scope.launch {
                            snackbarHostState.showSnackbar("Address copied")
                        }
                    },
                )
                Text(
                    text = "Curve: ${pair.curveType}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
