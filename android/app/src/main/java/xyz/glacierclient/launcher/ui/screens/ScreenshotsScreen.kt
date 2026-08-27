package xyz.glacierclient.launcher.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import xyz.glacierclient.launcher.service.JavaEditionBridge

/** Mirrors Pages/Home.BedrockScreenshots.cs, reading Java Edition captures from Pojav's shared storage. */
@Composable
fun ScreenshotsScreen() {
    val screenshots = remember { JavaEditionBridge.listScreenshots() }

    if (screenshots.isEmpty()) {
        Text("No screenshots yet.", modifier = Modifier.fillMaxSize().padding(16.dp))
        return
    }

    LazyVerticalGrid(
        columns = GridCells.Adaptive(minSize = 120.dp),
        modifier = Modifier.fillMaxSize().padding(8.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        items(screenshots) { file ->
            AsyncImage(
                model = file,
                contentDescription = file.name,
                contentScale = ContentScale.Crop,
                modifier = Modifier,
            )
        }
    }
}
