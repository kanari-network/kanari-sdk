package com.jamesatomc.kanariapp.compose

import androidx.compose.runtime.Composable
import com.jamesatomc.kanariapp.ui.theme.KanariAppTheme
import com.jamesatomc.kanariapp.ui.theme.ThemeMode

@Composable
fun KanariTheme(
    darkTheme: Boolean? = null,
    themeMode: ThemeMode = if (darkTheme == true) ThemeMode.DARK else if (darkTheme == false) ThemeMode.LIGHT else ThemeMode.SYSTEM,
    dynamicColor: Boolean = false,
    content: @Composable () -> Unit
) {
    KanariAppTheme(themeMode = themeMode, dynamicColor = dynamicColor, content = content)
}
