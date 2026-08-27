package xyz.glacierclient.launcher.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Language
import androidx.compose.material3.Card
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.unit.dp

private data class Credit(val name: String, val by: String, val links: List<Pair<String, String>>)

/** 1:1 with Pages/Home.razor "credits" panel content (same names/links/order). */
@Composable
fun CreditsScreen() {
    val uriHandler = LocalUriHandler.current
    val launcher = Credit("Glacier Launcher", "Built by Pepe · Glacier Productions", listOf(
        "GitHub" to "https://github.com/Glacier-Client-BE",
        "Website" to "https://glacierclient.xyz",
        "Discord" to "https://discord.glacierclient.xyz",
    ))
    val clients = listOf(
        Credit("Latite Client", "by Imrglop & contributors", listOf(
            "GitHub Releases" to "https://github.com/Imrglop/Latite-Releases",
            "Discord" to "https://discord.gg/latite",
        )),
        Credit("Flarial Client", "by the Flarial team", listOf(
            "Website" to "https://flarial.xyz",
            "Discord" to "https://discord.gg/flarial",
        )),
        Credit("OderSo Client", "by MasonOderSo", listOf(
            "GitHub" to "https://github.com/MasonOderSo/oderso-data",
        )),
    )

    LazyColumn(modifier = Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        item { Text("Launcher", style = MaterialTheme.typography.labelLarge) }
        item { CreditCard(launcher, uriHandler::openUri) }
        item { Text("Clients", style = MaterialTheme.typography.labelLarge) }
        items(clients) { CreditCard(it, uriHandler::openUri) }
        item { Text("Open Source", style = MaterialTheme.typography.labelLarge) }
        item {
            Row(Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                Text(
                    "Glacier Launcher is open source. Contributions and forks are welcome.",
                    modifier = Modifier.weight(1f),
                    style = MaterialTheme.typography.bodySmall,
                )
                OutlinedButton(onClick = { uriHandler.openUri("https://github.com/Glacier-Client-BE/Glacier-Launcher") }) {
                    Text("View")
                }
            }
        }
    }
}

@Composable
private fun CreditCard(credit: Credit, openUri: (String) -> Unit) {
    Card(Modifier.fillMaxWidth()) {
        Row(Modifier.padding(12.dp), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text(credit.name, style = MaterialTheme.typography.titleSmall)
                Text(credit.by, style = MaterialTheme.typography.bodySmall)
            }
            credit.links.forEach { (label, url) ->
                IconButton(onClick = { openUri(url) }) { Icon(Icons.Filled.Language, contentDescription = label) }
            }
        }
    }
}
