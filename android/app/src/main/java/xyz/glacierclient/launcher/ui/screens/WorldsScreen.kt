package xyz.glacierclient.launcher.ui.screens

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/**
 * Mirrors Pages/Home.razor "bedrockworlds" panel. World listing itself is
 * queued (needs Storage Access Framework wiring to Pojav's shared
 * .minecraft/saves — see JavaEditionBridge); the empty-state matches the
 * desktop's placeholder pattern until that's wired.
 */
@Composable
fun WorldsScreen() {
    EmptyState(
        "No worlds found",
        "Bedrock worlds live under the Java Edition companion app's shared storage " +
            "once installed — this list will populate from there.",
        modifier = Modifier.fillMaxSize().padding(16.dp),
    )
}
