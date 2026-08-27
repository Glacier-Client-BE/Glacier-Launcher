package xyz.glacierclient.launcher.ui.screens

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/** Mirrors Pages/Home.razor "bedrockinstances" panel (copy-sync world/save isolation). Listing queued. */
@Composable
fun InstancesScreen() {
    EmptyState(
        "No instances yet",
        "Isolated Bedrock instances will list here once wired to shared storage.",
        modifier = Modifier.fillMaxSize().padding(16.dp),
    )
}
