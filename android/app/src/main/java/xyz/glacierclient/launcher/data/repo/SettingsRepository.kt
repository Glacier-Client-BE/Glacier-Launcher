package xyz.glacierclient.launcher.data.repo

import android.content.Context
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.serialization.json.Json
import xyz.glacierclient.launcher.data.model.LauncherSettings

private val Context.dataStore by preferencesDataStore(name = "glacier_settings")

/**
 * JSON-blob-in-DataStore persistence, the Android analogue of the desktop
 * app's JsonStore (Services/JsonStore.cs) writing settings.json to disk.
 */
class SettingsRepository(private val context: Context) {

    private val key = stringPreferencesKey("settings_json")
    private val json = Json { ignoreUnknownKeys = true; encodeDefaults = true }

    val settingsFlow = context.dataStore.data.map { prefs ->
        prefs[key]?.let { runCatching { json.decodeFromString<LauncherSettings>(it) }.getOrNull() }
            ?: LauncherSettings()
    }

    suspend fun current(): LauncherSettings = settingsFlow.first()

    suspend fun update(transform: (LauncherSettings) -> LauncherSettings) {
        context.dataStore.edit { prefs ->
            val existing = prefs[key]?.let {
                runCatching { json.decodeFromString<LauncherSettings>(it) }.getOrNull()
            } ?: LauncherSettings()
            prefs[key] = json.encodeToString(LauncherSettings.serializer(), transform(existing))
        }
    }
}
