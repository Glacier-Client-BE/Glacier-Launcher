package xyz.glacierclient.launcher.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.History
import androidx.compose.material.icons.filled.SportsEsports
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import xyz.glacierclient.launcher.data.model.LauncherSettings
import xyz.glacierclient.launcher.data.repo.SettingsRepository
import xyz.glacierclient.launcher.service.ClientInjectionService
import xyz.glacierclient.launcher.service.JavaEditionBridge

/**
 * Mirrors Pages/Home.razor "home" view + the always-visible footer: status
 * message, recently-launched chips, and the footer profile/Xbox/Discord row.
 */
@Composable
fun HomeScreen() {
    val context = LocalContext.current
    val settingsRepo = remember { SettingsRepository(context) }
    val settings by settingsRepo.settingsFlow.collectAsState(initial = LauncherSettings())
    val rootHint = rootAvailabilityHint()
    val javaInstalled = remember { JavaEditionBridge.isInstalled(context) }

    Column(Modifier.fillMaxSize()) {
        LazyColumn(
            modifier = Modifier.weight(1f).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            item {
                Text("Glacier Launcher", style = MaterialTheme.typography.headlineMedium)
                Text("Minecraft Bedrock client manager", style = MaterialTheme.typography.bodyMedium)
            }
            item {
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text("Launch Minecraft Bedrock (${settings.selectedClient})")
                        Button(onClick = { launchBedrock(context) }) { Text("Launch") }
                        Text(rootHint, style = MaterialTheme.typography.bodySmall)
                    }
                }
            }
            item {
                Card(modifier = Modifier.fillMaxWidth()) {
                    Row(Modifier.padding(16.dp).fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                        Column {
                            Text("Java Edition", style = MaterialTheme.typography.titleSmall)
                            Text(if (javaInstalled) "Companion app installed" else "Companion app not installed", style = MaterialTheme.typography.bodySmall)
                        }
                        OutlinedButton(enabled = javaInstalled, onClick = { JavaEditionBridge.launch(context) }) {
                            Icon(Icons.Filled.SportsEsports, contentDescription = null)
                            Text(" Launch")
                        }
                    }
                }
            }
            if (settings.showRecentlyLaunched && settings.recentlyLaunched.isNotEmpty()) {
                item { Text("Recently Launched", style = MaterialTheme.typography.titleMedium) }
                item {
                    LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        items(settings.recentlyLaunched.take(3)) { entry ->
                            AssistChip(
                                onClick = {},
                                label = { Text(entry) },
                                leadingIcon = { Icon(Icons.Filled.History, contentDescription = null) },
                            )
                        }
                    }
                }
            }
        }
        HomeFooter(settings)
    }
}

@Composable
private fun HomeFooter(settings: LauncherSettings) {
    Row(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 10.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column {
            Text(
                settings.xboxGamertag.ifBlank { "Not signed in" },
                style = MaterialTheme.typography.bodyMedium,
            )
            Text("Xbox Live", style = MaterialTheme.typography.labelSmall)
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(onClick = { /* Xbox device-code sign-in modal, see XboxAuthService */ }) {
                Text(if (settings.xboxGamertag.isBlank()) "Xbox" else settings.xboxGamertag)
            }
            OutlinedButton(onClick = { /* Discord OAuth connect modal */ }) {
                Text("Discord")
            }
        }
    }
}

private fun rootAvailabilityHint(): String =
    if (ClientInjectionService.isRootAvailable())
        "Root detected — client injection available (experimental)."
    else
        "No root detected — client injection is unavailable on this device; Minecraft still launches normally."

private fun launchBedrock(context: android.content.Context) {
    val pm = context.packageManager
    val intent = pm.getLaunchIntentForPackage("com.mojang.minecraftpe")
    if (intent != null) context.startActivity(intent)
}
