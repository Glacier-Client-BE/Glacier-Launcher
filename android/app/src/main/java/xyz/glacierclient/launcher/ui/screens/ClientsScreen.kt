package xyz.glacierclient.launcher.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import xyz.glacierclient.launcher.data.model.ClientInfo
import xyz.glacierclient.launcher.data.model.ClientType
import xyz.glacierclient.launcher.data.repo.GlacierClientRepository

/** Mirrors Pages/Home.Clients.cs + Components/ClientCard.razor. */
@Composable
fun ClientsScreen() {
    val context = LocalContext.current
    val repo = remember { GlacierClientRepository(context) }
    val scope = rememberCoroutineScopeCompat()

    var clients by remember {
        mutableStateOf(
            listOf(
                ClientInfo(ClientType.Latite, "Latite Client"),
                ClientInfo(ClientType.Flarial, "Flarial Client"),
                ClientInfo(ClientType.OderSo, "OderSo Client"),
            )
        )
    }

    LazyColumn(modifier = Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        item {
            Text(
                "Client injection on Android is experimental and requires root — " +
                    "see Settings for details.",
                style = androidx.compose.material3.MaterialTheme.typography.bodySmall,
            )
        }
        items(clients) { client ->
            Card(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(client.name, style = androidx.compose.material3.MaterialTheme.typography.titleMedium)
                    Text(client.version ?: "Not installed", style = androidx.compose.material3.MaterialTheme.typography.bodySmall)
                    if (client.isDownloading) {
                        LinearProgressIndicator(progress = { client.progress.toFloat() }, modifier = Modifier.fillMaxWidth())
                    }
                    Button(onClick = {
                        clients = clients.map { if (it.type == client.type) it.copy(isDownloading = true, progress = 0.0) else it }
                        scope.launch {
                            repo.refreshManifest()
                            clients = clients.map {
                                if (it.type == client.type) it.copy(isDownloading = false, isDownloaded = true, progress = 1.0)
                                else it
                            }
                        }
                    }) {
                        Text(if (client.isDownloaded) "Reinstall" else "Install")
                    }
                }
            }
        }
    }
}

@Composable
private fun rememberCoroutineScopeCompat() = androidx.compose.runtime.rememberCoroutineScope()
