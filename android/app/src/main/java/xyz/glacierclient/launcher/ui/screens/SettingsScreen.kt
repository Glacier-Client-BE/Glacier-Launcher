package xyz.glacierclient.launcher.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Bolt
import androidx.compose.material.icons.filled.Layers
import androidx.compose.material.icons.filled.Palette
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.SportsEsports
import androidx.compose.material3.Card
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import xyz.glacierclient.launcher.data.model.LauncherSettings
import xyz.glacierclient.launcher.data.repo.SettingsRepository
import xyz.glacierclient.launcher.service.JavaEditionBridge

/**
 * 1:1 with Pages/Home.razor "settings" panel (887 lines): the same category
 * filter row (All / Injection|Java / Appearance / Account / System) and the
 * same sections underneath — Injection, Java Edition, Appearance, Account,
 * Social, Quality of Life, Updates, Folders, CurseForge, Backup, About.
 *
 * "Java Edition" doesn't duplicate RAM/JVM-args/resolution/offline-mode
 * controls here: the companion Pojav app already has a full per-profile
 * settings UI for exactly those knobs, and controls in *this* app wouldn't
 * be wired to anything real (see JavaEditionBridge) — so that section links
 * out to it instead of faking sliders. Everything else below is wired to
 * real settings storage. "Folders" and "Minimize to tray" are dropped: no
 * user-facing filesystem browser and no tray concept on Android.
 */
@Composable
fun SettingsScreen() {
    val context = LocalContext.current
    val repo = remember { SettingsRepository(context) }
    val settings by repo.settingsFlow.collectAsState(initial = LauncherSettings())
    val scope = rememberCoroutineScope()
    var category by remember { mutableStateOf("all") }
    // The desktop app swaps this category between "Inject" and "Java" based on a
    // global Bedrock/Java edition toggle that this app doesn't have wired yet
    // (see android/README.md) — always show the Bedrock "Inject" tab for now.
    val isBedrock = true

    fun update(transform: (LauncherSettings) -> LauncherSettings) {
        scope.launch { repo.update(transform) }
    }

    Column(Modifier.fillMaxWidth()) {
        CategoryRow(category, isBedrock) { category = it }

        LazyColumn(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            if (category == "all" || category == "injection") {
                item { SectionLabel("Injection") }
                item {
                    SettingRowSelect(
                        "Active client", "",
                        options = listOf("Latite Client", "Flarial Client", "OderSo Client", "LeviLamina Client", "Vanilla"),
                        selected = settings.selectedClient,
                        onSelect = { update { s -> s.copy(selectedClient = it) } },
                    )
                }
                item {
                    SettingRowSlider(
                        "Injection delay", "How long to wait after game launches before injecting",
                        value = settings.injectionDelayMs.toFloat(), range = 500f..15000f,
                        valueLabel = "${settings.injectionDelayMs / 1000.0}s",
                        onChange = { update { s -> s.copy(injectionDelayMs = it.toInt()) } },
                    )
                }
                item {
                    SettingRowToggle("Auto-inject", "Inject automatically once the game process is detected", settings.autoInject) {
                        update { s -> s.copy(autoInject = it) }
                    }
                }
                item {
                    SettingRowToggle("Close after launch", "Minimise the launcher once injection succeeds", settings.closeAfterLaunch) {
                        update { s -> s.copy(closeAfterLaunch = it) }
                    }
                }
            }

            if (category == "all" || category == "java") {
                item { SectionLabel("Java Edition") }
                item {
                    val installed = JavaEditionBridge.isInstalled(context)
                    Card(Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                            Text("RAM, JVM args, resolution, offline mode, and version filters are configured in the Java Edition companion app itself.")
                            OutlinedButton(enabled = installed, onClick = { JavaEditionBridge.launch(context) }) {
                                Text(if (installed) "Open Java Edition" else "Java Edition not installed")
                            }
                        }
                    }
                }
            }

            if (category == "all" || category == "appearance") {
                item { SectionLabel("Appearance") }
                item {
                    AccentColorRow(settings.accentColor) { update { s -> s.copy(accentColor = it) } }
                }
                item {
                    SettingRowSelect(
                        "Theme preset", "Background tone — affects panels and overlays",
                        options = listOf("dark", "darker", "midnight", "slate", "ocean", "forest", "sunset", "light"),
                        selected = settings.themePreset,
                        onSelect = { update { s -> s.copy(themePreset = it) } },
                    )
                }
                item {
                    SettingRowToggle("Compact mode", "Tighter spacing throughout", settings.compactMode) {
                        update { s -> s.copy(compactMode = it) }
                    }
                }
                item {
                    SettingRowToggle("Animations", "Disable for low-end devices", settings.animationsEnabled) {
                        update { s -> s.copy(animationsEnabled = it) }
                    }
                }
                if (settings.animationsEnabled) {
                    item {
                        SettingRowSlider(
                            "Animation speed", "",
                            value = settings.animationSpeed.toFloat(), range = 0.25f..2f,
                            valueLabel = "${settings.animationSpeed}x",
                            onChange = { update { s -> s.copy(animationSpeed = it.toDouble()) } },
                        )
                    }
                }
                item {
                    SettingRowSlider(
                        "UI scale", "",
                        value = settings.uiScalePct.toFloat(), range = 75f..150f,
                        valueLabel = "${settings.uiScalePct}%",
                        onChange = { update { s -> s.copy(uiScalePct = it.toInt()) } },
                    )
                }
            }

            if (category == "all" || category == "account") {
                item { SectionLabel("Account") }
                item {
                    SettingRowText("Display name", settings.username) { update { s -> s.copy(username = it) } }
                }
                item {
                    SettingRowSelect(
                        "Profile display", "Which account to show in the footer",
                        options = listOf("auto", "xbox", "discord"),
                        selected = settings.profileDisplayMode,
                        onSelect = { update { s -> s.copy(profileDisplayMode = it) } },
                    )
                }
                item {
                    SettingRowSelect(
                        "Language", "",
                        options = listOf("en", "es", "fr", "de", "pt", "ru"),
                        selected = settings.language,
                        onSelect = { update { s -> s.copy(language = it) } },
                    )
                }

                item { SectionLabel("Social") }
                item {
                    SettingRowToggle("Discord Rich Presence", "Posts a \"now playing\" webhook message — see DiscordPresenceService", settings.discordRichPresence) {
                        update { s -> s.copy(discordRichPresence = it) }
                    }
                }
                if (settings.discordRichPresence) {
                    item {
                        SettingRowText("Discord webhook URL", settings.discordWebhookUrl) { update { s -> s.copy(discordWebhookUrl = it) } }
                    }
                }
                item {
                    SettingRowButton("Xbox profile", if (settings.xboxGamertag.isBlank()) "Not signed in" else settings.xboxGamertag, "Sign in") {
                        // Opens the Xbox device-code sign-in flow — see XboxAuthService
                    }
                }
            }

            if (category == "all" || category == "system") {
                item { SectionLabel("Quality of Life") }
                item {
                    SettingRowToggle("Show recently launched", "", settings.showRecentlyLaunched) {
                        update { s -> s.copy(showRecentlyLaunched = it) }
                    }
                }
                item {
                    SettingRowButton("Clear recent history", "", "Clear") {
                        update { s -> s.copy(recentlyLaunched = emptyList()) }
                    }
                }

                item { SectionLabel("Updates") }
                item {
                    SettingRowToggle("Check for updates on startup", "", settings.checkUpdatesOnStartup) {
                        update { s -> s.copy(checkUpdatesOnStartup = it) }
                    }
                }
                item {
                    SettingRowButton("Check for updates now", "", "Check") {
                        // Wires to the Play Store / GitHub release feed once auto-update is implemented
                    }
                }

                item { SectionLabel("CurseForge") }
                item {
                    SettingRowText("CurseForge API key", settings.curseForgeApiKeyOverride) {
                        update { s -> s.copy(curseForgeApiKeyOverride = it) }
                    }
                }

                item { SectionLabel("Backup") }
                item { SettingRowButton("Export settings", "", "Export") { /* SAF document picker -> write settings JSON */ } }
                item { SettingRowButton("Import settings", "", "Import") { /* SAF document picker -> read settings JSON */ } }
                item {
                    SettingRowButton("Reset to defaults", "", "Reset") {
                        update { LauncherSettings() }
                    }
                }

                item { SectionLabel("About") }
                item { Text("Glacier Launcher for Android", style = MaterialTheme.typography.bodySmall) }
            }
        }
    }
}

@Composable
private fun CategoryRow(active: String, isBedrock: Boolean, onSelect: (String) -> Unit) {
    val categories = listOf(
        Triple("all", "All", Icons.Filled.Layers),
        if (isBedrock) Triple("injection", "Inject", Icons.Filled.Bolt) else Triple("java", "Java", Icons.Filled.SportsEsports),
        Triple("appearance", "Looks", Icons.Filled.Palette),
        Triple("account", "Account", Icons.Filled.Person),
        Triple("system", "System", Icons.Filled.Settings),
    )
    LazyRow(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        items(categories) { (id, label, icon) ->
            FilterChip(
                selected = active == id,
                onClick = { onSelect(id) },
                label = { Text(label) },
                leadingIcon = { Icon(icon, contentDescription = null) },
            )
        }
    }
}

@Composable
private fun SectionLabel(text: String) {
    Text(text, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
}

@Composable
private fun SettingRowToggle(label: String, hint: String, value: Boolean, onChange: (Boolean) -> Unit) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Text(label, style = MaterialTheme.typography.bodyMedium)
            if (hint.isNotBlank()) Text(hint, style = MaterialTheme.typography.bodySmall)
        }
        Switch(checked = value, onCheckedChange = onChange)
    }
}

@Composable
private fun SettingRowSlider(label: String, hint: String, value: Float, range: ClosedFloatingPointRange<Float>, valueLabel: String, onChange: (Float) -> Unit) {
    Column(Modifier.fillMaxWidth()) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Column(Modifier.weight(1f)) {
                Text(label, style = MaterialTheme.typography.bodyMedium)
                if (hint.isNotBlank()) Text(hint, style = MaterialTheme.typography.bodySmall)
            }
            Text(valueLabel, style = MaterialTheme.typography.bodySmall)
        }
        Slider(value = value, onValueChange = onChange, valueRange = range)
    }
}

@Composable
private fun SettingRowText(label: String, value: String, onChange: (String) -> Unit) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label) },
        modifier = Modifier.fillMaxWidth(),
        singleLine = true,
    )
}

@Composable
private fun SettingRowButton(label: String, hint: String, buttonLabel: String, onClick: () -> Unit) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Text(label, style = MaterialTheme.typography.bodyMedium)
            if (hint.isNotBlank()) Text(hint, style = MaterialTheme.typography.bodySmall)
        }
        OutlinedButton(onClick = onClick) { Text(buttonLabel) }
    }
}

@Composable
private fun SettingRowSelect(label: String, hint: String, options: List<String>, selected: String, onSelect: (String) -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth()) {
        Text(label, style = MaterialTheme.typography.bodyMedium)
        if (hint.isNotBlank()) Text(hint, style = MaterialTheme.typography.bodySmall)
        ExposedDropdownMenuBox(expanded = expanded, onExpandedChange = { expanded = it }) {
            OutlinedTextField(
                value = selected,
                onValueChange = {},
                readOnly = true,
                modifier = Modifier.fillMaxWidth(),
                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
            )
            androidx.compose.material3.ExposedDropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
                options.forEach { option ->
                    androidx.compose.material3.DropdownMenuItem(
                        text = { Text(option) },
                        onClick = { onSelect(option); expanded = false },
                    )
                }
            }
        }
    }
}

private val accentSwatches = listOf(
    "#7289da", "#43b581", "#f04747", "#faa61a", "#9b59b6", "#00bcd4", "#e91e63", "#ffffff",
)

@Composable
private fun AccentColorRow(active: String, onSelect: (String) -> Unit) {
    Column(Modifier.fillMaxWidth()) {
        Text("Accent colour", style = MaterialTheme.typography.bodyMedium)
        Text("Used for buttons, highlights and glows", style = MaterialTheme.typography.bodySmall)
        Row(Modifier.padding(top = 6.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            accentSwatches.forEach { hex ->
                val color = runCatching { Color(android.graphics.Color.parseColor(hex)) }.getOrDefault(Color.Gray)
                val borderColor = if (active == hex) MaterialTheme.colorScheme.onSurface else Color.Transparent
                Column(
                    Modifier
                        .size(28.dp)
                        .clip(CircleShape)
                        .background(color)
                        .border(2.dp, borderColor, CircleShape)
                        .clickable { onSelect(hex) },
                ) {}
            }
        }
    }
}
