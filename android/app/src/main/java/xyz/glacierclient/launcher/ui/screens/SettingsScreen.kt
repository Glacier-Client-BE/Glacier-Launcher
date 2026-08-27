package xyz.glacierclient.launcher.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Divider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import xyz.glacierclient.launcher.data.model.LauncherSettings
import xyz.glacierclient.launcher.data.repo.SettingsRepository
import xyz.glacierclient.launcher.service.ClientInjectionService

/** Mirrors Pages/Home.Settings.cs. */
@Composable
fun SettingsScreen() {
    val context = LocalContext.current
    val repo = remember { SettingsRepository(context) }
    val settings by repo.settingsFlow.collectAsState(initial = LauncherSettings())
    val scope = rememberCoroutineScope()

    Column(Modifier.fillMaxSize().padding(16.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Text("Settings", style = androidx.compose.material3.MaterialTheme.typography.headlineSmall)

        Row(verticalAlignment = androidx.compose.ui.Alignment.CenterVertically, horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxSize()) {
            Text("Discord Rich Presence")
            Switch(
                checked = settings.discordRichPresence,
                onCheckedChange = { checked ->
                    scope.launch { repo.update { it.copy(discordRichPresence = checked) } }
                },
            )
        }

        Row(verticalAlignment = androidx.compose.ui.Alignment.CenterVertically, horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxSize()) {
            Text("Auto-inject on launch")
            Switch(
                checked = settings.autoInject,
                onCheckedChange = { checked ->
                    scope.launch { repo.update { it.copy(autoInject = checked) } }
                },
            )
        }

        Divider()

        Text("Client injection status", style = androidx.compose.material3.MaterialTheme.typography.titleMedium)
        Text(
            if (ClientInjectionService.isRootAvailable())
                "Root available — experimental injection can be attempted."
            else
                "No root available. Windows-style DLL injection (Latite/Flarial/OderSo) " +
                    "has no equivalent on non-rooted Android; this is a platform limitation, " +
                    "not a missing feature.",
        )

        Divider()
        Text("Version: Glacier Launcher for Android")
    }
}
