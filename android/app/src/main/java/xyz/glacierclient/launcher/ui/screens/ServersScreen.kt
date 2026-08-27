package xyz.glacierclient.launcher.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Bookmark
import androidx.compose.material3.Card
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import xyz.glacierclient.launcher.data.model.SavedServer

/** 1:1 with Pages/Home.razor "servers" panel: saved servers + a "Popular" suggestions list. */
@Composable
fun ServersScreen() {
    var savedServers by remember { mutableStateOf(listOf<SavedServer>()) }

    val suggestions = remember {
        listOf(
            SavedServer("Hive", "geo.hivebedrock.network"),
            SavedServer("CubeCraft", "play.cubecraft.net"),
            SavedServer("Mineplex", "mco.mineplex.com"),
            SavedServer("Lifeboat", "play.lbsg.net"),
            SavedServer("Galaxite", "play.galaxite.net"),
        ).filter { s -> savedServers.none { it.address == s.address } }
    }

    LazyColumn(modifier = Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        if (savedServers.isEmpty()) {
            item { EmptyState("No saved servers yet", "Add a Minecraft Bedrock server to quick-launch into it. The current client will be injected before connecting.") }
        } else {
            items(savedServers) { server ->
                ServerRow(server, savedRow = true, onDelete = { savedServers = savedServers - server }, onSave = {})
            }
        }
        if (suggestions.isNotEmpty()) {
            item { Text("Popular", style = MaterialTheme.typography.labelLarge) }
            items(suggestions) { server ->
                ServerRow(server, savedRow = false, onDelete = {}, onSave = { savedServers = savedServers + server })
            }
        }
    }
}

@Composable
internal fun EmptyState(title: String, caption: String, modifier: Modifier = Modifier) {
    Column(modifier.fillMaxWidth().padding(vertical = 24.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text(title, style = MaterialTheme.typography.titleMedium)
        Text(caption, style = MaterialTheme.typography.bodySmall)
    }
}

@Composable
private fun ServerRow(server: SavedServer, savedRow: Boolean, onDelete: () -> Unit, onSave: () -> Unit) {
    Card(Modifier.fillMaxWidth()) {
        Row(
            Modifier.padding(12.dp).fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Text(server.name, style = MaterialTheme.typography.titleSmall)
                Text("${server.address}:${server.port}", style = MaterialTheme.typography.bodySmall)
            }
            Row {
                if (savedRow) {
                    IconButton(onClick = {}) { Icon(Icons.Filled.Edit, contentDescription = "Edit") }
                    IconButton(onClick = onDelete) { Icon(Icons.Filled.Delete, contentDescription = "Delete") }
                } else {
                    IconButton(onClick = onSave) { Icon(Icons.Filled.Bookmark, contentDescription = "Save") }
                }
                IconButton(onClick = {}) { Icon(Icons.Filled.ContentCopy, contentDescription = "Copy address") }
                IconButton(onClick = {}) { Icon(Icons.Filled.PlayArrow, contentDescription = "Launch & connect") }
            }
        }
    }
}
