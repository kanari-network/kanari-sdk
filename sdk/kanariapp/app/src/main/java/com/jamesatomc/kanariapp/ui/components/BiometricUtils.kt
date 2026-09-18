package com.jamesatomc.kanariapp.ui.components

import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue

@Composable
fun rememberBiometricPromptLauncher(
    activity: FragmentActivity?,
    title: String,
    subtitle: String,
    onSuccess: () -> Unit,
    onError: ((Int, CharSequence) -> Unit)? = null,
    onFailed: (() -> Unit)? = null,
): () -> Unit {
    var promptShowing by remember { mutableStateOf(false) }
    val latestSuccess by rememberUpdatedState(onSuccess)
    val latestError by rememberUpdatedState(onError)
    val latestFailed by rememberUpdatedState(onFailed)

    return {
        val currentActivity = activity
        if (currentActivity != null && !promptShowing) {
            promptShowing = true
            val started = showBiometricPrompt(
                activity = currentActivity,
                title = title,
                subtitle = subtitle,
                onSuccess = {
                    promptShowing = false
                    latestSuccess()
                },
                onError = { code, message ->
                    promptShowing = false
                    latestError?.invoke(code, message)
                },
                onFailed = { latestFailed?.invoke() },
            )
            if (!started) promptShowing = false
        }
    }
}

private fun showBiometricPrompt(
    activity: FragmentActivity,
    title: String,
    subtitle: String,
    onSuccess: () -> Unit,
    onError: ((Int, CharSequence) -> Unit)? = null,
    onFailed: (() -> Unit)? = null
): Boolean {
    val executor = ContextCompat.getMainExecutor(activity)
    val prompt = BiometricPrompt(
        activity,
        executor,
        object : BiometricPrompt.AuthenticationCallback() {
            override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                super.onAuthenticationSucceeded(result)
                activity.runOnUiThread { onSuccess() }
            }

            override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                super.onAuthenticationError(errorCode, errString)
                activity.runOnUiThread { onError?.invoke(errorCode, errString) }
            }

            override fun onAuthenticationFailed() {
                super.onAuthenticationFailed()
                activity.runOnUiThread { onFailed?.invoke() }
            }
        }
    )

    val promptInfo = BiometricPrompt.PromptInfo.Builder()
        .setTitle(title)
        .setSubtitle(subtitle)
        .setAllowedAuthenticators(
            BiometricManager.Authenticators.BIOMETRIC_STRONG or
                    BiometricManager.Authenticators.BIOMETRIC_WEAK
        )
        .setNegativeButtonText("Cancel")
        .build()

    try {
        prompt.authenticate(promptInfo)
        return true
    } catch (e: Exception) {
        onError?.invoke(BiometricPrompt.ERROR_HW_UNAVAILABLE, e.message ?: "Failed to authenticate")
        return false
    }
}
