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
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color

import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat

@Immutable
data class KanariGradients(
    val primary: Brush,
    val secondary: Brush,
    val multi: Brush,
    val glass: Brush
)

val LocalKanariGradients = staticCompositionLocalOf<KanariGradients> {
    error("No KanariGradients provided")
}

private val KanariLightScheme = lightColorScheme(
    primary = Color(0xFF111B18),                // Ink
    onPrimary = Color(0xFFF7F4EB),              // Cream
    primaryContainer = Color(0xFFC8F43D),       // Lime
    onPrimaryContainer = Color(0xFF111B18),     // Ink
    secondary = Color(0xFF7868DA),              // Purple
    onSecondary = Color(0xFFFFFFFF),
    secondaryContainer = Color(0xFFCABDFF),     // Lavender
    onSecondaryContainer = Color(0xFF111B18),   // Ink
    tertiary = Color(0xFFC8F43D),               // Lime
    onTertiary = Color(0xFF111B18),             // Ink
    tertiaryContainer = Color(0xFFE8F9A8),
    onTertiaryContainer = Color(0xFF111B18),    // Ink
    error = Color(0xFFB42318),
    onError = Color(0xFFFFFFFF),
    errorContainer = Color(0xFFFFDAD6),
    onErrorContainer = Color(0xFF410002),
    background = Color(0xFFF7F4EB),             // Cream
    onBackground = Color(0xFF111B18),           // Ink
    surface = Color(0xFFF7F4EB),                // Cream
    onSurface = Color(0xFF111B18),              // Ink
    surfaceVariant = Color(0xFFD5D5CC),         // outlineVariant
    onSurfaceVariant = Color(0xFF59625F),
    outline = Color(0xFF8C9491),
    outlineVariant = Color(0xFFD5D5CC),
    scrim = Color(0xFF111B18),
    inverseSurface = Color(0xFF111B18),
    inversePrimary = Color(0xFFC8F43D),
    surfaceTint = Color.Transparent,
    surfaceDim = Color(0xFFE8E5DC),
    surfaceBright = Color(0xFFFFFDF7),
    surfaceContainerLowest = Color(0xFFFFFDF7),
    surfaceContainerLow = Color(0xFFFBF8EF),
    surfaceContainer = Color(0xFFEBE8DB),      // Darker for better contrast
    surfaceContainerHigh = Color(0xFFDFDCD3),
    surfaceContainerHighest = Color(0xFFD5D2C9)
)

private val KanariDarkScheme = darkColorScheme(
    primary = Color(0xFFC8F43D),               // Lime
    onPrimary = Color(0xFF111B18),             // Ink
    primaryContainer = Color(0xFFC8F43D),      // Lime
    onPrimaryContainer = Color(0xFF111B18),    // Ink
    secondary = Color(0xFFCABDFF),             // Lavender
    onSecondary = Color(0xFF111B18),           // Ink
    secondaryContainer = Color(0xFF4E4674),
    onSecondaryContainer = Color(0xFFF1ECFF),
    tertiary = Color(0xFFC8F43D),              // Lime
    onTertiary = Color(0xFF111B18),            // Ink
    tertiaryContainer = Color(0xFF405013),
    onTertiaryContainer = Color(0xFFE8F9A8),
    error = Color(0xFFFFB4AB),
    onError = Color(0xFF690005),
    errorContainer = Color(0xFF93000A),
    onErrorContainer = Color(0xFFFFDAD6),
    // True-black OLED base: brand lime/lavender pop, battery-friendly panels.
    background = Color(0xFF000000),            // pure black
    onBackground = Color(0xFFF7F4EB),          // Cream
    surface = Color(0xFF000000),               // pure black
    onSurface = Color(0xFFF7F4EB),             // Cream
    surfaceVariant = Color(0xFF222725),        // outlineVariant (on black)
    onSurfaceVariant = Color(0xFFBFC7C2),
    outline = Color(0xFF8A938E),
    outlineVariant = Color(0xFF2A302D),
    scrim = Color(0xFF000000),
    inverseSurface = Color(0xFFF7F4EB),
    inversePrimary = Color(0xFF7868DA),
    surfaceTint = Color.Transparent,
    surfaceDim = Color(0xFF000000),
    surfaceBright = Color(0xFF2E3431),
    // Container ladder lifts one step at a time off black so cards, sheets
    // and dialogs stay distinguishable without a grey wash.
    surfaceContainerLowest = Color(0xFF000000),
    surfaceContainerLow = Color(0xFF0B0E0D),
    surfaceContainer = Color(0xFF141817),
    surfaceContainerHigh = Color(0xFF1E2422),
    surfaceContainerHighest = Color(0xFF28302D)
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

    val gradients = KanariGradients(
        primary = Brush.linearGradient(KanariColors.LimeGradient),
        secondary = Brush.linearGradient(KanariColors.PurpleGradient),
        multi = Brush.linearGradient(KanariColors.MultiGradient),
        glass = Brush.verticalGradient(listOf(Color.White.copy(alpha = 0.2f), Color.White.copy(alpha = 0.05f)))
    )

    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as Activity).window
            WindowCompat.getInsetsController(window, view).apply {
                isAppearanceLightStatusBars = !darkTheme
                isAppearanceLightNavigationBars = !darkTheme
            }
        }
    }

    CompositionLocalProvider(LocalKanariGradients provides gradients) {
        MaterialTheme(colorScheme = colorScheme, typography = Typography, content = content)
    }
}
