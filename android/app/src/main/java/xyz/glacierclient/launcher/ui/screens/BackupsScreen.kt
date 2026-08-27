package xyz.glacierclient.launcher.ui.screens

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/** Mirrors Pages/Home.razor "bedrockbackups" panel (world backup/restore). Listing queued. */
@Composable
fun BackupsScreen() {
    EmptyState(
        "No backups yet",
        "World backups will list here once wired to shared storage.",
        modifier = Modifier.fillMaxSize().padding(16.dp),
    )
}
