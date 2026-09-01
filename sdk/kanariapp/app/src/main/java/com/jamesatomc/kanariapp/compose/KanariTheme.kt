package com.jamesatomc.kanariapp.compose

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

// Kanari Brand Colors from Flutter
val KanariInk = Color(0xFF111B18)
val KanariCream = Color(0xFFF7F4EB)
val KanariPaper = Color(0xFFFFFDF7)
val KanariLime = Color(0xFFC8F43D)
val KanariLavender = Color(0xFFCABDFF)
val KanariPurple = Color(0xFF7868DA)
val KanariDarkPaper = Color(0xFF17211E)
val KanariDarkSurface = Color(0xFF0E1513)

private val DarkColorScheme = darkColorScheme(
    primary = KanariLime,
    onPrimary = KanariInk,
    primaryContainer = KanariLime,
    onPrimaryContainer = KanariInk,
    secondary = KanariLavender,
    onSecondary = KanariInk,
    background = KanariDarkSurface,
    onBackground = KanariCream,
    surface = KanariDarkSurface,
    onSurface = KanariCream,
    surfaceVariant = KanariDarkPaper,
    onSurfaceVariant = Color(0xFFB8C0BC),
    outline = Color(0xFF3C4642)
)

private val LightColorScheme = lightColorScheme(
    primary = KanariInk,
    onPrimary = KanariCream,
    primaryContainer = KanariLime,
    onPrimaryContainer = KanariInk,
    secondary = KanariPurple,
    onSecondary = Color.White,
    background = KanariCream,
    onBackground = KanariInk,
    surface = KanariCream,
    onSurface = KanariInk,
    surfaceVariant = Color(0xFFF1EEE5),
    onSurfaceVariant = Color(0xFF59625F),
    outline = Color(0xFFD5D5CC)
)

@Composable
fun KanariTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit
) {
    val colorScheme = if (darkTheme) DarkColorScheme else DarkColorScheme // Forcing dark for "Premium Dark" feel or could fallback

    MaterialTheme(
        colorScheme = colorScheme,
        content = content
    )
}
