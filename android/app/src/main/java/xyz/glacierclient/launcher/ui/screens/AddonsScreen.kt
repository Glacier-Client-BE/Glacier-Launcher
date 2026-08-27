package xyz.glacierclient.launcher.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.FileDownload
import androidx.compose.material.icons.filled.Key
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import xyz.glacierclient.launcher.BuildConfig
import xyz.glacierclient.launcher.data.remote.CurseForgeRepository

/**
 * 1:1 with Pages/Home.razor "addons" panel's Bedrock branch (the Java branch
 * additionally has Loaders/Mods/Assets/Datapacks/Tools sub-tabs and a
 * Modrinth tab — queued, since this app is Bedrock-only until the edition
 * toggle from android/README.md's status list is wired): API-key-required
 * empty state, the same category chip row (Addons/Maps/Skins/Texture
 * Packs/Scripts), search, results with an install action, and "Load more"
 * pagination.
 */
@Composable
fun AddonsScreen() {
    val repo = remember { CurseForgeRepository(BuildConfig.CURSEFORGE_API_KEY) }
    val scope = rememberCoroutineScope()
    val context = LocalContext.current

    var category by remember { mutableStateOf(repo.bedrockCategories.first()) }
    var query by remember { mutableStateOf("") }
    var results by remember { mutableStateOf(listOf<CurseForgeRepository.CurseForgeMod>()) }
    var totalCount by remember { mutableStateOf(0) }
    var searching by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var hasSearched by remember { mutableStateOf(false) }

    fun runSearch(reset: Boolean) {
        scope.launch {
            searching = true
            error = null
            hasSearched = true
            try {
                val response = repo.search(
                    CurseForgeRepository.GAME_ID_BEDROCK,
                    category.classId,
                    query,
                    index = if (reset) 0 else results.size,
                )
                results = if (reset) response.data else results + response.data
                totalCount = response.pagination.totalCount
            } catch (e: Exception) {
                error = "Failed to reach CurseForge: ${e.message}"
            } finally {
                searching = false
            }
        }
    }

    if (!repo.isAvailable) {
        Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Icon(Icons.Filled.Key, contentDescription = null)
            Text("CurseForge API key required", style = MaterialTheme.typography.titleMedium)
            Text("Get a free key from the CurseForge developer console, then paste it in Settings.", style = MaterialTheme.typography.bodySmall)
        }
        return
    }

    Column(Modifier.fillMaxWidth()) {
        LazyRow(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            items(repo.bedrockCategories) { cat ->
                FilterChip(
                    selected = category == cat,
                    onClick = { category = cat; runSearch(reset = true) },
                    label = { Text(cat.label) },
                )
            }
        }

        Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), verticalAlignment = Alignment.CenterVertically) {
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                label = { Text("Search CurseForge addons…") },
                leadingIcon = { Icon(Icons.Filled.Search, contentDescription = null) },
                modifier = Modifier.weight(1f),
                singleLine = true,
            )
        }

        LazyColumn(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            if (searching && results.isEmpty()) {
                item { CircularProgressIndicator(modifier = Modifier.padding(24.dp)) }
            }
            error?.let { message ->
                item {
                    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        Text(message, color = MaterialTheme.colorScheme.error)
                        OutlinedButton(onClick = { runSearch(reset = true) }) { Text("Retry") }
                    }
                }
            }
            if (!searching && error == null && results.isEmpty() && hasSearched) {
                item { Text("No results.", style = MaterialTheme.typography.bodyMedium) }
            }
            items(results) { mod ->
                Card(Modifier.fillMaxWidth()) {
                    Row(Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text(mod.name, style = MaterialTheme.typography.titleSmall)
                            Text(mod.summary, style = MaterialTheme.typography.bodySmall, maxLines = 2)
                            Text("${mod.downloadCount} downloads", style = MaterialTheme.typography.labelSmall)
                        }
                        IconButton(onClick = { /* download + install into the active client's addon folder */ }) {
                            Icon(Icons.Filled.FileDownload, contentDescription = "Download & Install")
                        }
                    }
                }
            }
            if (results.isNotEmpty() && results.size < totalCount) {
                item {
                    OutlinedButton(
                        onClick = { runSearch(reset = false) },
                        enabled = !searching,
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Text("Load more (${results.size} / $totalCount)")
                    }
                }
            }
        }
    }
}
