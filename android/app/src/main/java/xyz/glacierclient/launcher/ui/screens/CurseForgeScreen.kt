package xyz.glacierclient.launcher.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Card
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import xyz.glacierclient.launcher.BuildConfig
import xyz.glacierclient.launcher.data.remote.CurseForgeRepository

/** Mirrors Services/CurseForgeService.cs mod-browsing surfaced in Components/ModpacksPanel.razor. */
@Composable
fun CurseForgeScreen() {
    val repo = remember { CurseForgeRepository(BuildConfig.CURSEFORGE_API_KEY) }
    val scope = rememberCoroutineScope()
    var query by remember { mutableStateOf("") }
    var results by remember { mutableStateOf(listOf<CurseForgeRepository.CurseForgeMod>()) }
    var error by remember { mutableStateOf<String?>(null) }

    Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("CurseForge — Java Mods", style = androidx.compose.material3.MaterialTheme.typography.headlineSmall)
        OutlinedTextField(
            value = query,
            onValueChange = {
                query = it
                scope.launch {
                    error = null
                    results = try {
                        repo.search(CurseForgeRepository.GAME_ID_JAVA, CurseForgeRepository.JAVA_CLASS_MODS, it)
                    } catch (e: Exception) {
                        error = "Search failed: ${e.message}"
                        emptyList()
                    }
                }
            },
            label = { Text("Search mods") },
            modifier = Modifier.fillMaxWidth(),
        )
        error?.let { Text(it) }
        LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            items(results) { mod ->
                Card(Modifier.fillMaxWidth()) {
                    Column(Modifier.padding(12.dp)) {
                        Text(mod.name, style = androidx.compose.material3.MaterialTheme.typography.titleMedium)
                        Text(mod.summary, style = androidx.compose.material3.MaterialTheme.typography.bodySmall)
                        Text("${mod.downloadCount} downloads", style = androidx.compose.material3.MaterialTheme.typography.labelSmall)
                    }
                }
            }
        }
    }
}
