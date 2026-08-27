package xyz.glacierclient.launcher.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

// Matches the desktop app's default accent (#7289da) and dark glass theme preset.
val GlacierAccent = Color(0xFF7289DA)
val GlacierBackground = Color(0xFF0B0F14)
val GlacierSurface = Color(0xFF141A22)

private val DarkColors = darkColorScheme(
    primary = GlacierAccent,
    background = GlacierBackground,
    surface = GlacierSurface,
    onPrimary = Color.White,
    onBackground = Color(0xFFE6E9EF),
    onSurface = Color(0xFFE6E9EF),
)

private val LightColors = lightColorScheme(
    primary = GlacierAccent,
)

@Composable
fun GlacierTheme(useDark: Boolean = true, content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = if (useDark) DarkColors else LightColors,
        content = content,
    )
}
