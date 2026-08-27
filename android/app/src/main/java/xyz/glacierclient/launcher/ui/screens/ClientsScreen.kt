package xyz.glacierclient.launcher.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.FileDownload
import androidx.compose.material.icons.filled.Upgrade
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Card
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/**
 * 1:1 with Pages/Home.razor "clients" panel (lines ~1909-2179): the same six
 * cards in the same order — Flarial, Latite, OderSo, LeviLamina, Vanilla,
 * Custom DLL — with the same select/download/update/delete affordances.
 * Only the mechanism behind "download" differs (staged for the Java
 * companion app / root-only Bedrock injection — see ClientInjectionService);
 * every button and status the desktop card shows is reproduced here.
 */
private data class DownloadableClientState(
    val name: String,
    val iconRes: String,
    val desc: String,
    val downloaded: Boolean = false,
    val downloading: Boolean = false,
    val upToDate: Boolean = true,
    val progress: Float = 0f,
    val error: String? = null,
)

@Composable
fun ClientsScreen() {
    var selectedClient by remember { mutableStateOf("Latite Client") }
    var flarial by remember { mutableStateOf(DownloadableClientState("Flarial Client", "flarial", "Feature-rich Bedrock client with modules, HUD customization, and active development.")) }
    var oderso by remember { mutableStateOf(DownloadableClientState("OderSo Client", "oderso", "OderSo Client — curated Minecraft Bedrock experience by MasonOderSo.")) }
    var levilamina by remember { mutableStateOf(DownloadableClientState("LeviLamina Client", "levilamina", "LeviLamina — open-source native Bedrock mod loader by LiteLDev, injected the same way as the other clients here.")) }
    var customDllPath by remember { mutableStateOf<String?>(null) }

    LazyColumn(
        modifier = Modifier.fillMaxWidth().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        item {
            DownloadableClientCard(
                state = flarial,
                selected = selectedClient == flarial.name,
                onSelect = { selectedClient = flarial.name },
                onDownload = { flarial = flarial.copy(downloading = true, progress = 0f) },
                onDelete = { flarial = flarial.copy(downloaded = false) },
            )
        }
        item { LatiteCard(selected = selectedClient == "Latite Client", onSelect = { selectedClient = "Latite Client" }) }
        item {
            DownloadableClientCard(
                state = oderso,
                selected = selectedClient == oderso.name,
                onSelect = { selectedClient = oderso.name },
                onDownload = { oderso = oderso.copy(downloading = true, progress = 0f) },
                onDelete = { oderso = oderso.copy(downloaded = false) },
            )
        }
        item {
            DownloadableClientCard(
                state = levilamina,
                selected = selectedClient == levilamina.name,
                onSelect = { selectedClient = levilamina.name },
                onDownload = { levilamina = levilamina.copy(downloading = true, progress = 0f) },
                onDelete = { levilamina = levilamina.copy(downloaded = false) },
                extraAction = if (levilamina.downloaded) "Mods" else null,
            )
        }
        item { VanillaCard(selected = selectedClient == "Vanilla", onSelect = { selectedClient = "Vanilla" }) }
        customDllPath?.let { path ->
            item {
                CustomDllCard(
                    path = path,
                    selected = selectedClient == "Custom DLL",
                    onSelect = { selectedClient = "Custom DLL" },
                    onRemove = { customDllPath = null },
                )
            }
        }
        item {
            OutlinedButton(onClick = { /* Android SAF file picker for a .jar/.so mod, since raw .dll injection has no Android target */ }) {
                Text("Browse for a mod file…")
            }
        }
    }
}

@Composable
private fun ClientCardShell(
    name: String,
    subtitle: @Composable () -> Unit,
    desc: String,
    selected: Boolean,
    error: String? = null,
    actions: @Composable () -> Unit,
) {
    Card(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Row(verticalAlignment = androidx.compose.ui.Alignment.CenterVertically, horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                Column(Modifier.weight(1f)) {
                    Text(name, style = MaterialTheme.typography.titleMedium)
                    subtitle()
                }
                actions()
            }
            error?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall) }
            Text(desc, style = MaterialTheme.typography.bodySmall)
            if (selected) AssistChip(onClick = {}, label = { Text("Active") })
        }
    }
}

@Composable
private fun DownloadableClientCard(
    state: DownloadableClientState,
    selected: Boolean,
    onSelect: () -> Unit,
    onDownload: () -> Unit,
    onDelete: () -> Unit,
    extraAction: String? = null,
) {
    ClientCardShell(
        name = state.name,
        subtitle = {
            Text(
                when {
                    !state.downloaded -> "Not downloaded"
                    state.upToDate -> "Up to date"
                    else -> "Update available"
                },
                style = MaterialTheme.typography.bodySmall,
            )
        },
        desc = state.desc,
        selected = selected,
        error = state.error,
    ) {
        Row {
            if (!selected && state.downloaded) {
                IconButton(onClick = onSelect) { Icon(Icons.Filled.Check, contentDescription = "Select") }
            }
            if (state.downloading) {
                LinearProgressIndicator(progress = { state.progress }, modifier = Modifier.padding(8.dp))
            } else {
                if (!state.downloaded || !state.upToDate) {
                    IconButton(onClick = onDownload) {
                        Icon(if (state.downloaded) Icons.Filled.Upgrade else Icons.Filled.FileDownload, contentDescription = "Download")
                    }
                }
                if (state.downloaded) {
                    IconButton(onClick = onDelete) { Icon(Icons.Filled.Delete, contentDescription = "Delete") }
                }
            }
        }
    }
}

@Composable
private fun LatiteCard(selected: Boolean, onSelect: () -> Unit) {
    ClientCardShell(
        name = "Latite Client",
        subtitle = { Text("Versioned GitHub releases", style = MaterialTheme.typography.bodySmall) },
        desc = "Classic Minecraft Bedrock client. Choose a specific release from the Versions panel.",
        selected = selected,
    ) {
        if (!selected) IconButton(onClick = onSelect) { Icon(Icons.Filled.Check, contentDescription = "Select") }
    }
}

@Composable
private fun VanillaCard(selected: Boolean, onSelect: () -> Unit) {
    ClientCardShell(
        name = "Vanilla",
        subtitle = { Text("Launches Minecraft with no DLL injection", style = MaterialTheme.typography.bodySmall) },
        desc = "Pure stock Minecraft Bedrock — useful for diagnostics, multiplayer realms, or just playing un-modified.",
        selected = selected,
    ) {
        if (!selected) IconButton(onClick = onSelect) { Icon(Icons.Filled.Check, contentDescription = "Select") }
    }
}

@Composable
private fun CustomDllCard(path: String, selected: Boolean, onSelect: () -> Unit, onRemove: () -> Unit) {
    ClientCardShell(
        name = path.substringAfterLast('/'),
        subtitle = { Text(path, style = MaterialTheme.typography.bodySmall) },
        desc = "Custom client loaded via file picker.",
        selected = selected,
    ) {
        Row {
            if (!selected) IconButton(onClick = onSelect) { Icon(Icons.Filled.Check, contentDescription = "Select") }
            IconButton(onClick = onRemove) { Icon(Icons.Filled.Delete, contentDescription = "Remove") }
        }
    }
}
