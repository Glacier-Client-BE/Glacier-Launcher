package xyz.glacierclient.launcher.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import xyz.glacierclient.launcher.service.ClientInjectionService

/** Mirrors Pages/Home.Launch.cs — quick-launch + recently launched. */
@Composable
fun HomeScreen() {
    val rootHint = rootAvailabilityHint()
    LazyColumn(
        modifier = Modifier.fillMaxSize().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            Text("Glacier Launcher", style = androidx.compose.material3.MaterialTheme.typography.headlineMedium)
            Text("Minecraft Bedrock client manager", style = androidx.compose.material3.MaterialTheme.typography.bodyMedium)
        }
        item {
            Card(modifier = Modifier.fillMaxSize()) {
                Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Launch Minecraft Bedrock")
                    Button(onClick = { launchBedrock() }) { Text("Launch") }
                    Text(rootHint, style = androidx.compose.material3.MaterialTheme.typography.bodySmall)
                }
            }
        }
        item { Text("Recently Launched", style = androidx.compose.material3.MaterialTheme.typography.titleMedium) }
        items(emptyList<String>()) { }
    }
}

private fun rootAvailabilityHint(): String =
    if (ClientInjectionService.isRootAvailable())
        "Root detected — client injection available (experimental)."
    else
        "No root detected — client injection is unavailable on this device; Minecraft still launches normally."

private fun launchBedrock() {
    // Intent handoff to the installed com.mojang.minecraftpe package happens
    // from the Activity context (see MainActivity / LauncherIntents helper).
}
