package com.jamesatomc.kanariapp.ui.theme

import android.app.Activity
import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat

private val KanariLightScheme = lightColorScheme(
    primary = KanariPrimaryLight,
    onPrimary = KanariOnPrimaryLight,
    primaryContainer = KanariPrimaryContainerLight,
    onPrimaryContainer = KanariOnPrimaryContainerLight,
    secondary = KanariSecondaryLight,
    onSecondary = KanariOnSecondaryLight,
    secondaryContainer = KanariSecondaryContainerLight,
    onSecondaryContainer = KanariOnSecondaryContainerLight,
    tertiary = KanariTertiaryLight,
    onTertiary = KanariOnTertiaryLight,
    tertiaryContainer = KanariTertiaryContainerLight,
    onTertiaryContainer = KanariOnTertiaryContainerLight,
    error = KanariErrorLight,
    onError = KanariOnErrorLight,
    errorContainer = KanariErrorContainerLight,
    background = KanariBackgroundLight,
    onBackground = KanariOnBackgroundLight,
    surface = KanariSurfaceLight,
    onSurface = KanariOnSurfaceLight,
    surfaceVariant = KanariSurfaceVariantLight,
    onSurfaceVariant = KanariOnSurfaceVariantLight,
    outline = KanariOutlineLight,
    surfaceContainer = KanariSurfaceContainerLight,
    surfaceContainerHigh = KanariSurfaceContainerHighLight,
    scrim = KanariOnBackgroundLight.copy(alpha = 0.3f)
)

private val KanariDarkScheme = darkColorScheme(
    primary = KanariPrimaryDark,
    onPrimary = KanariOnPrimaryDark,
    primaryContainer = KanariPrimaryContainerDark,
    onPrimaryContainer = KanariOnPrimaryContainerDark,
    secondary = KanariSecondaryDark,
    onSecondary = KanariOnSecondaryDark,
    secondaryContainer = KanariSecondaryContainerDark,
    onSecondaryContainer = KanariOnSecondaryContainerDark,
    tertiary = KanariTertiaryDark,
    onTertiary = KanariOnTertiaryDark,
    tertiaryContainer = KanariTertiaryContainerDark,
    onTertiaryContainer = KanariOnTertiaryContainerDark,
    error = KanariErrorDark,
    onError = KanariOnErrorDark,
    errorContainer = KanariErrorContainerDark,
    background = KanariBackgroundDark,
    onBackground = KanariOnBackgroundDark,
    surface = KanariSurfaceDark,
    onSurface = KanariOnSurfaceDark,
    surfaceVariant = KanariSurfaceVariantDark,
    onSurfaceVariant = KanariOnSurfaceVariantDark,
    outline = KanariOutlineDark,
    surfaceContainer = KanariSurfaceContainerDark,
    surfaceContainerHigh = KanariSurfaceContainerHighDark,
    scrim = KanariBackgroundDark.copy(alpha = 0.5f)
)

enum class ThemeMode { LIGHT, DARK, SYSTEM }

@Composable
fun KanariAppTheme(
    themeMode: ThemeMode = ThemeMode.SYSTEM,
    dynamicColor: Boolean = false,
    content: @Composable () -> Unit
) {
    val systemDark = isSystemInDarkTheme()
    val darkTheme = when (themeMode) {
        ThemeMode.LIGHT -> false
        ThemeMode.DARK -> true
        ThemeMode.SYSTEM -> systemDark
    }

    val colorScheme = when {
        dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }
        darkTheme -> KanariDarkScheme
        else -> KanariLightScheme
    }

    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as Activity).window
            window.statusBarColor = colorScheme.background.toArgb()
            window.navigationBarColor = colorScheme.surface.toArgb()
            WindowCompat.getInsetsController(window, view).apply {
                isAppearanceLightStatusBars = !darkTheme
                isAppearanceLightNavigationBars = !darkTheme
            }
        }
    }

    MaterialTheme(colorScheme = colorScheme, typography = Typography, content = content)
}
