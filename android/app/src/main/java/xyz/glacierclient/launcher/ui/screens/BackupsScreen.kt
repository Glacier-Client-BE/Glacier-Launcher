package xyz.glacierclient.launcher.ui.screens

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/** Mirrors Pages/Home.Backups.cs — world backup/restore. */
@Composable
fun BackupsScreen() {
    Text("World backups", modifier = Modifier.fillMaxSize().padding(16.dp))
}
