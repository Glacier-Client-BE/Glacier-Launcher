package xyz.glacierclient.launcher.ui.screens

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/** Mirrors Pages/Home.Worlds.cs — lists Bedrock worlds under
 *  /storage/emulated/0/Android/data/com.mojang.minecraftpe/files/games/com.mojang/minecraftWorlds. */
@Composable
fun WorldsScreen() {
    Text(
        "Bedrock worlds (requires storage access on Android 11+ via " +
            "MANAGE_EXTERNAL_STORAGE or the Storage Access Framework)",
        modifier = Modifier.fillMaxSize().padding(16.dp),
    )
}
