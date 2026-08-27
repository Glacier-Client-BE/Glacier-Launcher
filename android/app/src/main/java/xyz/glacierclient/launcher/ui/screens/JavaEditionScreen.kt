package xyz.glacierclient.launcher.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import xyz.glacierclient.launcher.service.JavaEditionBridge

/**
 * Java Edition tab — hands off to our rebranded PojavLauncher build
 * (android/pojavlauncher, applicationId xyz.glacierclient.launcher.java)
 * rather than reimplementing an ARM JVM/JNI runtime here. See
 * android/README.md for why this is a companion APK, not a merged module.
 */
@Composable
fun JavaEditionScreen() {
    val context = LocalContext.current
    val installed = JavaEditionBridge.isInstalled(context)

    Column(Modifier.fillMaxSize().padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Java Edition", style = androidx.compose.material3.MaterialTheme.typography.headlineSmall)
        Text(
            if (installed)
                "Glacier Launcher (Java Edition) is installed."
            else
                "Glacier Launcher (Java Edition) isn't installed yet. Install the companion " +
                    "APK from the same release to play Java Edition — it's our rebranded, " +
                    "open-source PojavLauncher build.",
        )
        Button(enabled = installed, onClick = { JavaEditionBridge.launch(context) }) {
            Text("Launch Java Edition")
        }
        Text(
            "Glacier Client jars installed from the Clients tab are copied into its " +
                "shared mods folder automatically.",
            style = androidx.compose.material3.MaterialTheme.typography.bodySmall,
        )
    }
}
