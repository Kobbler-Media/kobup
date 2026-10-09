package com.kobup.app.ui.theme

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext

private val KobupBlue = Color(0xFF1565C0)
private val KobupBlueLight = Color(0xFF5E92F3)
private val KobupBlueDark = Color(0xFF003C8F)

private val LightColors = lightColorScheme(
    primary = KobupBlue,
    onPrimary = Color.White,
    primaryContainer = KobupBlueLight.copy(alpha = 0.2f),
    secondary = Color(0xFF546E7A),
    surface = Color.White,
    background = Color(0xFFF5F5F5),
)

private val DarkColors = darkColorScheme(
    primary = KobupBlueLight,
    onPrimary = Color.Black,
    primaryContainer = KobupBlueDark,
    secondary = Color(0xFF90A4AE),
    surface = Color(0xFF1E1E1E),
    background = Color(0xFF121212),
)

@Composable
fun KobupTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    val colorScheme = when {
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }
        darkTheme -> DarkColors
        else -> LightColors
    }

    MaterialTheme(
        colorScheme = colorScheme,
        content = content,
    )
}
